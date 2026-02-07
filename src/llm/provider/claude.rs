use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::base::{
    build_endpoint, extract_api_key, get_max_tokens, get_temperature, process_commit_response,
    process_review_response, send_llm_request,
};
use super::streaming::process_claude_stream;
use super::utils::{CLAUDE_API_SUFFIX, DEFAULT_CLAUDE_BASE};
use crate::config::{NetworkConfig, ProviderConfig};
use crate::error::{GcopError, Result};
use crate::llm::message::SystemBlock;
use crate::llm::{CommitContext, LLMProvider, ReviewResult, ReviewType, StreamHandle};

/// Claude API provider
///
/// 使用 Anthropic Claude API 生成 commit message 和代码审查。
///
/// # 支持的模型
/// - `claude-sonnet-4-5-20250929` (推荐，默认)
/// - `claude-opus-4-20241229`
/// - `claude-haiku-4-20250110`
///
/// # 配置示例
/// ```toml
/// [llm]
/// default_provider = "claude"
///
/// [llm.providers.claude]
/// api_key = "sk-ant-..."
/// model = "claude-sonnet-4-5-20250929"
/// base_url = "https://api.anthropic.com"  # 可选
/// max_tokens = 1000  # 可选
/// temperature = 0.7  # 可选
/// ```
///
/// # 特性
/// - 支持流式响应（SSE）
/// - 自动重试（3 次，指数退避）
/// - 支持 prompt caching（自动优化 API 成本）
/// - 自定义端点（支持代理或兼容 API）
///
/// # 环境变量
/// - `ANTHROPIC_API_KEY` - API key（优先级高于配置文件）
/// - `ANTHROPIC_BASE_URL` - 自定义端点（可选）
///
/// # 示例
/// ```ignore
/// use gcop_rs::llm::{LLMProvider, provider::claude::ClaudeProvider};
/// use gcop_rs::config::{ProviderConfig, NetworkConfig};
///
/// # async fn example() -> anyhow::Result<()> {
/// let config = ProviderConfig {
///     api_key: Some("sk-ant-...".to_string()),
///     model: "claude-sonnet-4-5-20250929".to_string(),
///     ..Default::default()
/// };
/// let network_config = NetworkConfig::default();
/// let provider = ClaudeProvider::new(&config, "claude", &network_config, false)?;
///
/// // 生成 commit message
/// let diff = "diff --git a/main.rs...";
/// let message = provider.generate_commit_message(diff, None, None).await?;
/// println!("Generated: {}", message);
/// # Ok(())
/// # }
/// ```
pub struct ClaudeProvider {
    name: String,
    client: Client,
    api_key: String,
    endpoint: String,
    model: String,
    max_tokens: u32,
    temperature: f32,
    max_retries: usize,
    retry_delay_ms: u64,
    max_retry_delay_ms: u64,
    colored: bool,
}

#[derive(Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    temperature: f32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    system: Vec<SystemBlock>,
    messages: Vec<MessagePayload>,
}

#[derive(Serialize)]
struct ClaudeStreamRequest {
    model: String,
    max_tokens: u32,
    temperature: f32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    system: Vec<SystemBlock>,
    messages: Vec<MessagePayload>,
    stream: bool,
}

#[derive(Serialize, Deserialize)]
struct MessagePayload {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

impl ClaudeProvider {
    pub fn new(
        config: &ProviderConfig,
        provider_name: &str,
        network_config: &NetworkConfig,
        colored: bool,
    ) -> Result<Self> {
        let api_key = extract_api_key(config, "ANTHROPIC_API_KEY", "Claude")?;
        let endpoint = build_endpoint(config, DEFAULT_CLAUDE_BASE, CLAUDE_API_SUFFIX);
        let model = config.model.clone();
        let max_tokens = get_max_tokens(config);
        let temperature = get_temperature(config);

        Ok(Self {
            name: provider_name.to_string(),
            client: super::create_http_client(network_config)?,
            api_key,
            endpoint,
            model,
            max_tokens,
            temperature,
            max_retries: network_config.max_retries,
            retry_delay_ms: network_config.retry_delay_ms,
            max_retry_delay_ms: network_config.max_retry_delay_ms,
            colored,
        })
    }

    async fn call_api(
        &self,
        system: &str,
        user_message: &str,
        spinner: Option<&crate::ui::Spinner>,
    ) -> Result<String> {
        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            system: vec![SystemBlock::cached(system)],
            messages: vec![MessagePayload {
                role: "user".to_string(),
                content: user_message.to_string(),
            }],
        };

        tracing::debug!(
            "Claude API request: model={}, max_tokens={}, temperature={}, system_len={}, user_len={}",
            self.model,
            self.max_tokens,
            self.temperature,
            system.len(),
            user_message.len()
        );

        let response: ClaudeResponse = send_llm_request(
            &self.client,
            &self.endpoint,
            &[
                ("x-api-key", self.api_key.as_str()),
                ("anthropic-version", "2023-06-01"),
            ],
            &request,
            "Claude",
            spinner,
            self.max_retries,
            self.retry_delay_ms,
            self.max_retry_delay_ms,
        )
        .await?;

        let text = response
            .content
            .into_iter()
            .filter(|block| block.content_type == "text")
            .map(|block| block.text)
            .collect::<Vec<_>>()
            .join("\n");

        Ok(text)
    }

    /// 流式 API 调用
    async fn call_api_streaming(&self, system: &str, user_message: &str) -> Result<StreamHandle> {
        let (tx, rx) = mpsc::channel(64);

        let request = ClaudeStreamRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            system: vec![SystemBlock::cached(system)],
            messages: vec![MessagePayload {
                role: "user".to_string(),
                content: user_message.to_string(),
            }],
            stream: true,
        };

        tracing::debug!(
            "Claude Streaming API request: model={}, max_tokens={}, temperature={}, system_len={}, user_len={}",
            self.model,
            self.max_tokens,
            self.temperature,
            system.len(),
            user_message.len()
        );

        let response = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request)
            .send()
            .await
            .map_err(GcopError::Network)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(GcopError::LlmApi {
                status: status.as_u16(),
                message: format!("Claude: {}", body),
            });
        }

        // 在后台任务中处理流
        let colored = self.colored;
        tokio::spawn(async move {
            if let Err(e) = process_claude_stream(response, tx, colored).await {
                crate::ui::colors::error(
                    &rust_i18n::t!("provider.stream_processing_error", error = e.to_string()),
                    colored,
                );
            }
        });

        Ok(StreamHandle { receiver: rx })
    }
}

#[async_trait]
impl LLMProvider for ClaudeProvider {
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
        &self.name
    }

    async fn validate(&self) -> Result<()> {
        if self.api_key.is_empty() {
            return Err(GcopError::Config(
                rust_i18n::t!("provider.api_key_empty").to_string(),
            ));
        }

        // Send minimal test request to validate API connection
        tracing::debug!("Validating Claude API connection...");

        let test_request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: 1, // Minimize API cost
            temperature: 1.0,
            system: vec![],
            messages: vec![MessagePayload {
                role: "user".to_string(),
                content: "test".to_string(),
            }],
        };

        // Direct request without retry (fast fail)
        let response = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&test_request)
            .send()
            .await
            .map_err(GcopError::Network)?;

        // Check status code
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(GcopError::LlmApi {
                status: status.as_u16(),
                message: rust_i18n::t!(
                    "provider.api_validation_failed",
                    provider = "Claude",
                    body = body
                )
                .to_string(),
            });
        }

        tracing::debug!("Claude API connection validated successfully");
        Ok(())
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    async fn generate_commit_message_streaming(
        &self,
        diff: &str,
        context: Option<CommitContext>,
    ) -> Result<StreamHandle> {
        let ctx = context.unwrap_or_default();
        let (system, user) =
            crate::llm::prompt::build_commit_prompt_split(diff, &ctx, ctx.custom_prompt.as_deref());

        tracing::debug!(
            "Claude streaming - system ({} chars), user ({} chars)",
            system.len(),
            user.len()
        );

        self.call_api_streaming(&system, &user).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use pretty_assertions::assert_eq;
    use std::collections::HashMap;

    use crate::config::{NetworkConfig, ProviderConfig};
    use crate::error::GcopError;

    fn test_network_config_no_retry() -> NetworkConfig {
        NetworkConfig {
            max_retries: 0,
            ..Default::default()
        }
    }

    fn test_provider_config(base_url: String) -> ProviderConfig {
        ProviderConfig {
            api_style: None,
            endpoint: Some(base_url),
            api_key: Some("sk-ant-test".to_string()),
            model: "claude-3-haiku-20240307".to_string(),
            max_tokens: None,
            temperature: None,
            extra: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_claude_success_response_parsing() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/messages")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"content":[{"type":"text","text":"Hello"},{"type":"text","text":"Claude"}]}"#,
            )
            .create_async()
            .await;

        let provider = ClaudeProvider::new(
            &test_provider_config(server.url()),
            "claude",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        let result = provider.call_api("system", "hi", None).await.unwrap();
        assert_eq!(result, "Hello\nClaude");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_claude_api_error_401() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/messages")
            .with_status(401)
            .with_body("Unauthorized")
            .create_async()
            .await;

        let provider = ClaudeProvider::new(
            &test_provider_config(server.url()),
            "claude",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        let err = provider.call_api("system", "hi", None).await.unwrap_err();
        assert!(matches!(err, GcopError::LlmApi { status: 401, .. }));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_claude_api_error_429() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/messages")
            .with_status(429)
            .with_body("Too Many Requests")
            .create_async()
            .await;

        let provider = ClaudeProvider::new(
            &test_provider_config(server.url()),
            "claude",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        let err = provider.call_api("system", "hi", None).await.unwrap_err();
        assert!(matches!(err, GcopError::LlmApi { status: 429, .. }));
        mock.assert_async().await;
    }
}
