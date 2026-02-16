use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::debug;

use crate::config::AppConfig;
use crate::error::{GcopError, Result};
use crate::llm::{
    CommitContext, LLMProvider, ProgressReporter, ReviewResult, ReviewType, StreamChunk,
    StreamHandle,
};
use crate::ui::colors;

use super::create_single_provider;

/// Fallback Provider - wraps multiple providers and automatically switches when failure occurs
pub struct FallbackProvider {
    providers: Vec<Arc<dyn LLMProvider>>,
    colored: bool,
}

impl FallbackProvider {
    /// Creates a fallback wrapper from a prepared provider chain.
    pub fn new(providers: Vec<Arc<dyn LLMProvider>>, colored: bool) -> Self {
        Self { providers, colored }
    }

    /// Create FallbackProvider from configuration
    ///
    /// Collect main providers and fallback providers, and only record debug logs if they fail during creation.
    /// Return the wrapped provider (if only one succeeds, return it directly).
    pub fn from_config(
        config: &AppConfig,
        provider_name: Option<&str>,
    ) -> Result<Arc<dyn LLMProvider>> {
        let colored = config.ui.colored;
        let main_name = provider_name.unwrap_or(&config.llm.default_provider);

        // Collect all provider names to try
        let mut provider_names: Vec<&str> = vec![main_name];
        provider_names.extend(config.llm.fallback_providers.iter().map(String::as_str));

        // If there is only one provider (no fallback), create it directly
        if provider_names.len() == 1 {
            return create_single_provider(config, provider_names[0], colored);
        }

        // Create all providers and record debug logs on failure
        let mut providers: Vec<Arc<dyn LLMProvider>> = Vec::new();

        for (i, &name) in provider_names.iter().enumerate() {
            match create_single_provider(config, name, colored) {
                Ok(p) => providers.push(p),
                Err(e) => {
                    if i == 0 {
                        debug!("Primary provider '{}' failed to create: {}", name, e);
                    } else {
                        debug!("Fallback provider '{}' failed to create: {}", name, e);
                    }
                }
            }
        }

        if providers.is_empty() {
            return Err(GcopError::Config(
                rust_i18n::t!("provider.no_valid_providers").to_string(),
            ));
        }

        // If exactly one provider is available, return it directly.
        if providers.len() == 1 {
            // SAFETY: len() == 1 guarantees that there are elements
            return Ok(providers
                .into_iter()
                .next()
                .expect("providers is non-empty: len() == 1"));
        }

        Ok(Arc::new(Self::new(providers, colored)))
    }
}

#[async_trait]
impl LLMProvider for FallbackProvider {
    fn name(&self) -> &str {
        "fallback"
    }

    fn supports_streaming(&self) -> bool {
        // Ability to use the first provider
        self.providers
            .first()
            .map(|p| p.supports_streaming())
            .unwrap_or(false)
    }

    async fn validate(&self) -> Result<()> {
        if self.providers.is_empty() {
            return Err(GcopError::Config(
                rust_i18n::t!("provider.no_providers_configured").to_string(),
            ));
        }

        // Validate all providers and collect results
        let mut all_failed = true;

        for provider in &self.providers {
            tracing::debug!("Validating provider '{}'...", provider.name());

            match provider.validate().await {
                Ok(_) => {
                    all_failed = false;
                    tracing::debug!("Provider '{}' validated successfully", provider.name());
                }
                Err(e) => {
                    tracing::debug!("Provider '{}' validation failed: {}", provider.name(), e);
                }
            }
        }

        if all_failed {
            return Err(GcopError::Config(
                rust_i18n::t!(
                    "provider.all_providers_failed_validation",
                    count = self.providers.len()
                )
                .to_string(),
            ));
        }

        Ok(())
    }

    async fn generate_commit_message(
        &self,
        diff: &str,
        context: Option<CommitContext>,
        progress: Option<&dyn ProgressReporter>,
    ) -> Result<String> {
        let mut last_error = None;

        for (i, provider) in self.providers.iter().enumerate() {
            // If it is fallback (not the first provider), update the spinner display
            if i > 0
                && let Some(p) = progress
            {
                p.append_suffix(&rust_i18n::t!(
                    "provider.fallback_suffix",
                    provider = provider.name()
                ));
            }

            match provider
                .generate_commit_message(diff, context.clone(), progress)
                .await
            {
                Ok(msg) => return Ok(msg),
                Err(e) => {
                    // If it is not the last provider, show a warning and continue
                    if i < self.providers.len() - 1 {
                        colors::warning(
                            &rust_i18n::t!(
                                "provider.fallback_provider_failed",
                                provider = provider.name(),
                                error = e.to_string()
                            ),
                            self.colored,
                        );
                    }
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            GcopError::Llm(rust_i18n::t!("provider.no_providers_available").to_string())
        }))
    }

    async fn review_code(
        &self,
        diff: &str,
        review_type: ReviewType,
        custom_prompt: Option<&str>,
        progress: Option<&dyn ProgressReporter>,
    ) -> Result<ReviewResult> {
        let mut last_error = None;

        for (i, provider) in self.providers.iter().enumerate() {
            // If it is fallback (not the first provider), update the spinner display
            if i > 0
                && let Some(p) = progress
            {
                p.append_suffix(&rust_i18n::t!(
                    "provider.fallback_suffix",
                    provider = provider.name()
                ));
            }

            match provider
                .review_code(diff, review_type.clone(), custom_prompt, progress)
                .await
            {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if i < self.providers.len() - 1 {
                        colors::warning(
                            &rust_i18n::t!(
                                "provider.fallback_provider_failed",
                                provider = provider.name(),
                                error = e.to_string()
                            ),
                            self.colored,
                        );
                    }
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            GcopError::Llm(rust_i18n::t!("provider.no_providers_available").to_string())
        }))
    }

    async fn generate_commit_message_streaming(
        &self,
        diff: &str,
        context: Option<CommitContext>,
    ) -> Result<StreamHandle> {
        let mut last_error = None;
        let mut tried_streaming = false;

        // Try all providers that support streaming
        for provider in &self.providers {
            if !provider.supports_streaming() {
                continue;
            }
            tried_streaming = true;

            match provider
                .generate_commit_message_streaming(diff, context.clone())
                .await
            {
                Ok(handle) => return Ok(handle),
                Err(e) => {
                    colors::warning(
                        &rust_i18n::t!(
                            "provider.fallback_streaming_failed",
                            provider = provider.name(),
                            error = e.to_string()
                        ),
                        self.colored,
                    );
                    last_error = Some(e);
                }
            }
        }

        // All streaming providers failed and fellback to non-streaming mode
        if tried_streaming {
            colors::warning(
                &rust_i18n::t!("provider.all_streaming_failed"),
                self.colored,
            );
        }

        let (tx, rx) = mpsc::channel(32);
        let result = self.generate_commit_message(diff, context, None).await;

        match result {
            Ok(message) => {
                let _ = tx.send(StreamChunk::Delta(message)).await;
                let _ = tx.send(StreamChunk::Done).await;
            }
            Err(e) => {
                // If the non-streaming method also fails, the streaming error will be returned first (more meaningful)
                let error = last_error.map(|le| le.to_string()).unwrap_or(e.to_string());
                let _ = tx.send(StreamChunk::Error(error)).await;
            }
        }

        Ok(StreamHandle { receiver: rx })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Simple Mock Provider for testing
    struct TestProvider {
        name: String,
        should_fail: bool,
        supports_streaming: bool,
        message: String,
    }

    impl TestProvider {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                should_fail: false,
                supports_streaming: false,
                message: format!("message from {}", name),
            }
        }

        fn with_failure(mut self) -> Self {
            self.should_fail = true;
            self
        }

        fn with_streaming(mut self) -> Self {
            self.supports_streaming = true;
            self
        }
    }

    #[async_trait]
    impl LLMProvider for TestProvider {
        fn name(&self) -> &str {
            &self.name
        }

        fn supports_streaming(&self) -> bool {
            self.supports_streaming
        }

        async fn validate(&self) -> Result<()> {
            if self.should_fail {
                Err(GcopError::Config("validation failed".to_string()))
            } else {
                Ok(())
            }
        }

        async fn generate_commit_message(
            &self,
            _diff: &str,
            _context: Option<CommitContext>,
            _progress: Option<&dyn ProgressReporter>,
        ) -> Result<String> {
            if self.should_fail {
                Err(GcopError::Llm(format!("{} failed", self.name)))
            } else {
                Ok(self.message.clone())
            }
        }

        async fn review_code(
            &self,
            _diff: &str,
            _review_type: ReviewType,
            _custom_prompt: Option<&str>,
            _progress: Option<&dyn ProgressReporter>,
        ) -> Result<ReviewResult> {
            if self.should_fail {
                Err(GcopError::Llm(format!("{} failed", self.name)))
            } else {
                Ok(ReviewResult {
                    summary: self.message.clone(),
                    issues: vec![],
                    suggestions: vec![],
                })
            }
        }

        async fn generate_commit_message_streaming(
            &self,
            _diff: &str,
            _context: Option<CommitContext>,
        ) -> Result<StreamHandle> {
            if self.should_fail {
                Err(GcopError::Llm(format!("{} streaming failed", self.name)))
            } else {
                let (tx, rx) = mpsc::channel(32);
                let message = self.message.clone();
                tokio::spawn(async move {
                    let _ = tx.send(StreamChunk::Delta(message)).await;
                    let _ = tx.send(StreamChunk::Done).await;
                });
                Ok(StreamHandle { receiver: rx })
            }
        }
    }

    // === Test supports_streaming ===

    #[test]
    fn test_supports_streaming_true() {
        let provider = TestProvider::new("test").with_streaming();
        let fallback = FallbackProvider::new(vec![Arc::new(provider)], false);
        assert!(fallback.supports_streaming());
    }

    #[test]
    fn test_supports_streaming_false() {
        let provider = TestProvider::new("test");
        let fallback = FallbackProvider::new(vec![Arc::new(provider)], false);
        assert!(!fallback.supports_streaming());
    }

    #[test]
    fn test_supports_streaming_empty() {
        let fallback = FallbackProvider::new(vec![], false);
        assert!(!fallback.supports_streaming());
    }

    // === Test validate ===

    #[tokio::test]
    async fn test_validate_empty_providers() {
        let fallback = FallbackProvider::new(vec![], false);
        let result = fallback.validate().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_success() {
        let provider = TestProvider::new("test");
        let fallback = FallbackProvider::new(vec![Arc::new(provider)], false);
        assert!(fallback.validate().await.is_ok());
    }

    #[tokio::test]
    async fn test_validate_all_fail() {
        let provider1 = TestProvider::new("p1").with_failure();
        let provider2 = TestProvider::new("p2").with_failure();
        let fallback = FallbackProvider::new(vec![Arc::new(provider1), Arc::new(provider2)], false);
        let result = fallback.validate().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_partial_success() {
        let provider1 = TestProvider::new("p1").with_failure();
        let provider2 = TestProvider::new("p2"); // success
        let fallback = FallbackProvider::new(vec![Arc::new(provider1), Arc::new(provider2)], false);
        assert!(fallback.validate().await.is_ok());
    }

    // === Test generate_commit_message ===

    #[tokio::test]
    async fn test_generate_commit_message_primary_success() {
        let provider = TestProvider::new("primary");
        let fallback = FallbackProvider::new(vec![Arc::new(provider)], false);
        let result = fallback.generate_commit_message("diff", None, None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "message from primary");
    }

    #[tokio::test]
    async fn test_generate_commit_message_fallback_on_failure() {
        let provider1 = TestProvider::new("primary").with_failure();
        let provider2 = TestProvider::new("fallback");
        let fallback = FallbackProvider::new(vec![Arc::new(provider1), Arc::new(provider2)], false);
        let result = fallback.generate_commit_message("diff", None, None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "message from fallback");
    }

    #[tokio::test]
    async fn test_generate_commit_message_all_fail() {
        let provider1 = TestProvider::new("primary").with_failure();
        let provider2 = TestProvider::new("fallback").with_failure();
        let fallback = FallbackProvider::new(vec![Arc::new(provider1), Arc::new(provider2)], false);
        let result = fallback.generate_commit_message("diff", None, None).await;
        assert!(result.is_err());
    }

    // === Test review_code ===

    #[tokio::test]
    async fn test_review_code_primary_success() {
        let provider = TestProvider::new("primary");
        let fallback = FallbackProvider::new(vec![Arc::new(provider)], false);
        let result = fallback
            .review_code("diff", ReviewType::UncommittedChanges, None, None)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().summary, "message from primary");
    }

    #[tokio::test]
    async fn test_review_code_fallback_on_failure() {
        let provider1 = TestProvider::new("primary").with_failure();
        let provider2 = TestProvider::new("fallback");
        let fallback = FallbackProvider::new(vec![Arc::new(provider1), Arc::new(provider2)], false);
        let result = fallback
            .review_code("diff", ReviewType::UncommittedChanges, None, None)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().summary, "message from fallback");
    }

    // === Test generate_commit_message_streaming ===

    #[tokio::test]
    async fn test_streaming_primary_success() {
        let provider = TestProvider::new("primary").with_streaming();
        let fallback = FallbackProvider::new(vec![Arc::new(provider)], false);
        let result = fallback
            .generate_commit_message_streaming("diff", None)
            .await;
        assert!(result.is_ok());

        let mut handle = result.unwrap();
        let chunk = handle.receiver.recv().await;
        assert!(chunk.is_some());
        match chunk.unwrap() {
            StreamChunk::Delta(msg) => assert_eq!(msg, "message from primary"),
            _ => panic!("Expected Delta chunk"),
        }
    }

    #[tokio::test]
    async fn test_streaming_fallback_to_non_streaming() {
        let provider = TestProvider::new("primary").with_streaming().with_failure();
        let fallback = FallbackProvider::new(vec![Arc::new(provider)], false);
        let result = fallback
            .generate_commit_message_streaming("diff", None)
            .await;
        // Should fallback to non-streaming mode, but since that also fails, you get an error
        assert!(result.is_ok());

        let mut handle = result.unwrap();
        let chunk = handle.receiver.recv().await;
        assert!(chunk.is_some());
        // Error chunk should be received (other chunks may also be received)
        if let StreamChunk::Error(_) = chunk.unwrap() {
            // OK
        }
    }
}
