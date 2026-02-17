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
use crate::llm::{LLMProvider, ProgressReporter, ReviewResult, ReviewType, StreamHandle};

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

/// Blanket impl: every `ApiBackend` automatically becomes an `LLMProvider`.
///
/// `send_prompt` delegates to `call_api`.
/// `send_prompt_streaming` delegates to `call_api_streaming` (with non-streaming fallback).
/// `generate_commit_message` and `generate_commit_message_streaming` use the trait defaults
/// (build prompt → `send_prompt` / `send_prompt_streaming`).
#[async_trait]
impl<T: ApiBackend> LLMProvider for T {
    async fn send_prompt(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        progress: Option<&dyn ProgressReporter>,
    ) -> Result<String> {
        tracing::debug!(
            "send_prompt - system ({} chars), user ({} chars)",
            system_prompt.len(),
            user_prompt.len()
        );
        self.call_api(system_prompt, user_prompt, progress).await
    }

    async fn send_prompt_streaming(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<StreamHandle> {
        if ApiBackend::supports_streaming(self) {
            tracing::debug!(
                "Streaming - system ({} chars), user ({} chars)",
                system_prompt.len(),
                user_prompt.len()
            );
            self.call_api_streaming(system_prompt, user_prompt).await
        } else {
            // Fallback to non-streaming, emit full response as single chunk.
            let (tx, rx) = tokio::sync::mpsc::channel(32);
            let result = self.send_prompt(system_prompt, user_prompt, None).await;
            match result {
                Ok(message) => {
                    let _ = tx.send(crate::llm::StreamChunk::Delta(message)).await;
                    let _ = tx.send(crate::llm::StreamChunk::Done).await;
                }
                Err(e) => {
                    let _ = tx.send(crate::llm::StreamChunk::Error(e.to_string())).await;
                }
            }
            Ok(StreamHandle { receiver: rx })
        }
    }

    // generate_commit_message: uses trait default (build prompt → send_prompt)

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

    fn supports_streaming(&self) -> bool {
        ApiBackend::supports_streaming(self)
    }
}
