use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::debug;

use crate::config::AppConfig;
use crate::error::{GcopError, Result};
use crate::llm::{CommitContext, LLMProvider, ReviewResult, ReviewType, StreamChunk, StreamHandle};
use crate::ui::{Spinner, colors};

use super::create_single_provider;

/// Fallback Provider - 包装多个 provider，失败时自动切换
pub struct FallbackProvider {
    providers: Vec<Arc<dyn LLMProvider>>,
    colored: bool,
}

impl FallbackProvider {
    pub fn new(providers: Vec<Arc<dyn LLMProvider>>, colored: bool) -> Self {
        Self { providers, colored }
    }

    /// 从配置创建 FallbackProvider
    ///
    /// 收集主 provider 和 fallback providers，创建时失败只记录 debug 日志。
    /// 返回包装好的 provider（如果只有一个成功则直接返回它）。
    pub fn from_config(
        config: &AppConfig,
        provider_name: Option<&str>,
    ) -> Result<Arc<dyn LLMProvider>> {
        let colored = config.ui.colored;
        let main_name = provider_name.unwrap_or(&config.llm.default_provider);

        // 收集所有要尝试的 provider 名称
        let mut provider_names: Vec<&str> = vec![main_name];
        provider_names.extend(config.llm.fallback_providers.iter().map(String::as_str));

        // 如果只有一个 provider（无 fallback），直接创建
        if provider_names.len() == 1 {
            return create_single_provider(config, provider_names[0], colored);
        }

        // 创建所有 provider，失败时记录 debug 日志
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

        // 如果只成功创建了一个，直接返回（避免不必要的包装）
        if providers.len() == 1 {
            return Ok(providers.into_iter().next().unwrap());
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
        // 使用第一个 provider 的能力
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
        spinner: Option<&Spinner>,
    ) -> Result<String> {
        let mut last_error = None;

        for (i, provider) in self.providers.iter().enumerate() {
            // 如果是 fallback（非第一个 provider），更新 spinner 显示
            if i > 0
                && let Some(s) = spinner
            {
                s.append_suffix(&rust_i18n::t!(
                    "provider.fallback_suffix",
                    provider = provider.name()
                ));
            }

            match provider
                .generate_commit_message(diff, context.clone(), spinner)
                .await
            {
                Ok(msg) => return Ok(msg),
                Err(e) => {
                    // 如果不是最后一个 provider，显示警告并继续
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
        spinner: Option<&Spinner>,
    ) -> Result<ReviewResult> {
        let mut last_error = None;

        for (i, provider) in self.providers.iter().enumerate() {
            // 如果是 fallback（非第一个 provider），更新 spinner 显示
            if i > 0
                && let Some(s) = spinner
            {
                s.append_suffix(&rust_i18n::t!(
                    "provider.fallback_suffix",
                    provider = provider.name()
                ));
            }

            match provider
                .review_code(diff, review_type.clone(), custom_prompt, spinner)
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

        // 尝试所有支持流式的 provider
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

        // 所有流式 provider 都失败了，fallback 到非流式模式
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
                // 如果非流式也失败了，优先返回流式的错误（更有意义）
                let error = last_error.map(|le| le.to_string()).unwrap_or(e.to_string());
                let _ = tx.send(StreamChunk::Error(error)).await;
            }
        }

        Ok(StreamHandle { receiver: rx })
    }
}
