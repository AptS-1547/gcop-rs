//! Provider 公共抽象和辅助函数
//!
//! 提取各 Provider 的通用逻辑，减少重复代码。
//!
//! 模块结构：
//! - `config` - 配置提取工具函数
//! - `response` - 响应处理和 JSON 清理
//! - `retry` - HTTP 请求发送与重试逻辑
//! - `validation` - API 验证辅助函数
//! - `ApiBackend` trait - 各 provider 只需实现独有部分，通用逻辑由 blanket impl 提供

pub mod config;
pub mod response;
pub mod retry;
pub mod validation;

// 重新导出常用函数，保持向后兼容
pub use config::*;
pub use response::*;
pub use retry::send_llm_request;
pub use validation::*;

use async_trait::async_trait;

use crate::error::{GcopError, Result};
use crate::llm::{CommitContext, LLMProvider, ReviewResult, ReviewType, StreamHandle};

/// 内部 trait：每个 provider 只需实现自己独有的部分
///
/// 通过 blanket impl 自动为所有 `ApiBackend` 实现者提供 `LLMProvider`。
/// `FallbackProvider` 不实现此 trait，直接实现 `LLMProvider`。
#[async_trait]
pub(crate) trait ApiBackend: Send + Sync {
    /// Provider 名称
    fn name(&self) -> &str;

    /// 非流式 API 调用
    async fn call_api(
        &self,
        system: &str,
        user_message: &str,
        spinner: Option<&crate::ui::Spinner>,
    ) -> Result<String>;

    /// 是否支持流式响应
    fn supports_streaming(&self) -> bool {
        false
    }

    /// 流式 API 调用
    async fn call_api_streaming(&self, _system: &str, _user_message: &str) -> Result<StreamHandle> {
        Err(GcopError::Llm("Streaming not supported".into()))
    }

    /// 验证配置
    async fn validate(&self) -> Result<()>;
}

#[async_trait]
impl<T: ApiBackend> LLMProvider for T {
    async fn generate_commit_message(
        &self,
        diff: &str,
        context: Option<CommitContext>,
        spinner: Option<&crate::ui::Spinner>,
    ) -> Result<String> {
        let ctx = context.unwrap_or_default();
        let (system, user) =
            crate::llm::prompt::build_commit_prompt_split(diff, &ctx, ctx.custom_prompt.as_deref());
        tracing::debug!(
            "Commit prompt split - system ({} chars), user ({} chars)",
            system.len(),
            user.len()
        );
        let response = self.call_api(&system, &user, spinner).await?;
        Ok(process_commit_response(response))
    }

    async fn review_code(
        &self,
        diff: &str,
        review_type: ReviewType,
        custom_prompt: Option<&str>,
        spinner: Option<&crate::ui::Spinner>,
    ) -> Result<ReviewResult> {
        let (system, user) =
            crate::llm::prompt::build_review_prompt_split(diff, &review_type, custom_prompt);
        tracing::debug!(
            "Review prompt split - system ({} chars), user ({} chars)",
            system.len(),
            user.len()
        );
        let response = self.call_api(&system, &user, spinner).await?;
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

    async fn generate_commit_message_streaming(
        &self,
        diff: &str,
        context: Option<CommitContext>,
    ) -> Result<StreamHandle> {
        if !ApiBackend::supports_streaming(self) {
            // 不支持流式，使用 LLMProvider trait 的默认 fallback 逻辑
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
        let (system, user) =
            crate::llm::prompt::build_commit_prompt_split(diff, &ctx, ctx.custom_prompt.as_deref());
        tracing::debug!(
            "Streaming - system ({} chars), user ({} chars)",
            system.len(),
            user.len()
        );
        self.call_api_streaming(&system, &user).await
    }
}
