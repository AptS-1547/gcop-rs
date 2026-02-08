use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::base::{
    ApiBackend, build_endpoint, extract_api_key, get_max_tokens, get_temperature, send_llm_request,
    validate_api_key, validate_http_endpoint,
};
use super::streaming::process_claude_stream;
use super::utils::{CLAUDE_API_SUFFIX, DEFAULT_CLAUDE_BASE};
use crate::config::{NetworkConfig, ProviderConfig};
use crate::error::{GcopError, Result};
use crate::llm::message::SystemBlock;
use crate::llm::{StreamChunk, StreamHandle};

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
/// endpoint = "https://api.anthropic.com"  # 可选
/// max_tokens = 1000  # 可选
/// temperature = 0.7  # 可选
/// ```
///
/// # 配置方式
///
/// 在 `config.toml` 中设置 `api_key` 和可选的 `endpoint`。
/// CI 模式下使用 `GCOP_CI_API_KEY` 和 `GCOP_CI_ENDPOINT` 环境变量。
///
/// # 特性
/// - 支持流式响应（SSE）
/// - 自动重试（指数退避，默认 3 次，可通过 `network.max_retries` 配置）
/// - 支持 prompt caching（自动优化 API 成本）
/// - 自定义端点（支持代理或兼容 API）
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
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
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
        let api_key = extract_api_key(config, "Claude")?;
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
}

#[async_trait]
impl ApiBackend for ClaudeProvider {
    fn name(&self) -> &str {
        &self.name
    }

    async fn call_api(
        &self,
        system: &str,
        user_message: &str,
        progress: Option<&dyn crate::llm::ProgressReporter>,
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
            stream: None,
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
            progress,
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

        if text.is_empty() {
            return Err(crate::error::GcopError::Llm(
                rust_i18n::t!("provider.empty_response", provider = "Claude").to_string(),
            ));
        }

        Ok(text)
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    async fn call_api_streaming(&self, system: &str, user_message: &str) -> Result<StreamHandle> {
        let (tx, rx) = mpsc::channel(64);

        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            system: vec![SystemBlock::cached(system)],
            messages: vec![MessagePayload {
                role: "user".to_string(),
                content: user_message.to_string(),
            }],
            stream: Some(true),
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
                message: format!("{}: {}", self.name, body),
            });
        }

        // 在后台任务中处理流
        let colored = self.colored;
        tokio::spawn(async move {
            let error_tx = tx.clone();
            if let Err(e) = process_claude_stream(response, tx, colored).await {
                crate::ui::colors::error(
                    &rust_i18n::t!("provider.stream_processing_error", error = e.to_string()),
                    colored,
                );
                let _ = error_tx.send(StreamChunk::Error(e.to_string())).await;
            }
        });

        Ok(StreamHandle { receiver: rx })
    }

    async fn validate(&self) -> Result<()> {
        validate_api_key(&self.api_key)?;

        let test_request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: 1, // Minimize API cost
            temperature: 1.0,
            system: vec![],
            messages: vec![MessagePayload {
                role: "user".to_string(),
                content: "test".to_string(),
            }],
            stream: None,
        };

        validate_http_endpoint(
            &self.client,
            &self.endpoint,
            &[
                ("x-api-key", self.api_key.as_str()),
                ("anthropic-version", "2023-06-01"),
            ],
            &test_request,
            "Claude",
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use pretty_assertions::assert_eq;

    use crate::error::GcopError;
    use crate::llm::provider::test_utils::{
        ensure_crypto_provider, test_network_config_no_retry, test_provider_config,
    };

    #[tokio::test]
    async fn test_claude_success_response_parsing() {
        ensure_crypto_provider();
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
            &test_provider_config(
                server.url(),
                Some("sk-ant-test".to_string()),
                "claude-3-haiku-20240307".to_string(),
            ),
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
        ensure_crypto_provider();
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/messages")
            .with_status(401)
            .with_body("Unauthorized")
            .create_async()
            .await;

        let provider = ClaudeProvider::new(
            &test_provider_config(
                server.url(),
                Some("sk-ant-test".to_string()),
                "claude-3-haiku-20240307".to_string(),
            ),
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
        ensure_crypto_provider();
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/messages")
            .with_status(429)
            .with_body("Too Many Requests")
            .create_async()
            .await;

        let provider = ClaudeProvider::new(
            &test_provider_config(
                server.url(),
                Some("sk-ant-test".to_string()),
                "claude-3-haiku-20240307".to_string(),
            ),
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
