//! Provider public abstractions and helper functions
//!
//! Extract the common logic of each Provider to reduce duplicate code.
//!
//! Module structure:
//! - `config` - configure extraction tool function
//! - `response` - response handling and JSON sanitization
//! - `retry` - HTTP request sending and retry logic
//! - `validation` - API validation helper function
//! - `ApiBackend` trait - each provider only needs to implement its unique part, and the common logic is provided by blanket impl

pub mod config;
pub mod response;
pub mod retry;
pub mod validation;

// Re-export commonly used functions to maintain backward compatibility
pub use config::*;
pub use response::*;
pub use retry::send_llm_request;
pub use validation::*;

use async_trait::async_trait;

use crate::error::{GcopError, Result};
use crate::llm::{
    CommitContext, LLMProvider, ProgressReporter, ReviewResult, ReviewType, StreamHandle,
};

/// Internal traits: Each provider only needs to implement its own unique part
///
/// `LLMProvider` is automatically provided to all `ApiBackend` implementers via blanket impl.
/// `FallbackProvider` does not implement this trait and directly implements `LLMProvider`.
#[async_trait]
pub(crate) trait ApiBackend: Send + Sync {
    /// Provider name
    fn name(&self) -> &str;

    /// Non-streaming API calls
    async fn call_api(
        &self,
        system: &str,
        user_message: &str,
        progress: Option<&dyn ProgressReporter>,
    ) -> Result<String>;

    /// Whether to support streaming response
    fn supports_streaming(&self) -> bool {
        false
    }

    /// Streaming API calls
    async fn call_api_streaming(&self, _system: &str, _user_message: &str) -> Result<StreamHandle> {
        Err(GcopError::Llm("Streaming not supported".into()))
    }

    /// Verify configuration
    async fn validate(&self) -> Result<()>;
}

#[async_trait]
impl<T: ApiBackend> LLMProvider for T {
    async fn generate_commit_message(
        &self,
        diff: &str,
        context: Option<CommitContext>,
        progress: Option<&dyn ProgressReporter>,
    ) -> Result<String> {
        let ctx = context.unwrap_or_default();
        let (system, user) = crate::llm::prompt::build_commit_prompt_split(
            diff,
            &ctx,
            ctx.custom_prompt.as_deref(),
            ctx.convention.as_ref(),
        );
        tracing::debug!(
            "Commit prompt split - system ({} chars), user ({} chars)",
            system.len(),
            user.len()
        );
        let response = self.call_api(&system, &user, progress).await?;
        Ok(process_commit_response(response))
    }

    async fn review_code(
        &self,
        diff: &str,
        review_type: ReviewType,
        custom_prompt: Option<&str>,
        progress: Option<&dyn ProgressReporter>,
    ) -> Result<ReviewResult> {
        let (system, user) =
            crate::llm::prompt::build_review_prompt_split(diff, &review_type, custom_prompt);
        tracing::debug!(
            "Review prompt split - system ({} chars), user ({} chars)",
            system.len(),
            user.len()
        );
        let response = self.call_api(&system, &user, progress).await?;
        process_review_response(&response)
    }

    fn name(&self) -> &str {
        ApiBackend::name(self)
    }

    async fn validate(&self) -> Result<()> {
        ApiBackend::validate(self).await
    }

    async fn query(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        progress: Option<&dyn ProgressReporter>,
    ) -> Result<String> {
        tracing::debug!(
            "Direct query - system ({} chars), user ({} chars)",
            system_prompt.len(),
            user_prompt.len()
        );
        self.call_api(system_prompt, user_prompt, progress).await
    }

    fn supports_streaming(&self) -> bool {
        ApiBackend::supports_streaming(self)
    }

    async fn generate_commit_message_streaming(
        &self,
        diff: &str,
        context: Option<CommitContext>,
    ) -> Result<StreamHandle> {
        if !ApiBackend::supports_streaming(self) {
            // Streaming is not supported, and the default fallback logic of the LLMProvider trait is used.
            let (tx, rx) = tokio::sync::mpsc::channel(32);
            let result = self.generate_commit_message(diff, context, None).await;
            match result {
                Ok(message) => {
                    let _ = tx.send(crate::llm::StreamChunk::Delta(message)).await;
                    let _ = tx.send(crate::llm::StreamChunk::Done).await;
                }
                Err(e) => {
                    let _ = tx.send(crate::llm::StreamChunk::Error(e.to_string())).await;
                }
            }
            return Ok(StreamHandle { receiver: rx });
        }

        let ctx = context.unwrap_or_default();
        let (system, user) = crate::llm::prompt::build_commit_prompt_split(
            diff,
            &ctx,
            ctx.custom_prompt.as_deref(),
            ctx.convention.as_ref(),
        );
        tracing::debug!(
            "Streaming - system ({} chars), user ({} chars)",
            system.len(),
            user.len()
        );
        self.call_api_streaming(&system, &user).await
    }
}
