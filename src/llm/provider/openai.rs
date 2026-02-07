use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::base::{
    ApiBackend, build_endpoint, extract_api_key, get_max_tokens_optional, get_temperature,
    send_llm_request, validate_api_key, validate_http_endpoint,
};
use super::streaming::process_openai_stream;
use super::utils::{DEFAULT_OPENAI_BASE, OPENAI_API_SUFFIX};
use crate::config::{NetworkConfig, ProviderConfig};
use crate::error::{GcopError, Result};
use crate::llm::{StreamChunk, StreamHandle};

/// OpenAI API provider
///
/// 使用 OpenAI API（或兼容的 API）生成 commit message 和代码审查。
///
/// # 支持的模型
/// - **OpenAI 官方**：
///   - `gpt-4` (推荐)
///   - `gpt-4-turbo`
///   - `gpt-3.5-turbo`
/// - **兼容 API**（如 Azure OpenAI, OpenRouter 等）
///
/// # 配置示例
/// ```toml
/// [llm]
/// default_provider = "openai"
///
/// [llm.providers.openai]
/// api_key = "sk-..."
/// model = "gpt-4"
/// endpoint = "https://api.openai.com"  # 可选
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
/// - 兼容 OpenAI API 的第三方服务
/// - 自定义端点（支持代理或 Azure OpenAI）
///
/// # Azure OpenAI 示例
/// ```toml
/// [llm.providers.openai]
/// api_key = "your-azure-key"
/// model = "gpt-4"
/// endpoint = "https://your-resource.openai.azure.com/v1/chat/completions"
/// ```
///
/// # 示例
/// ```ignore
/// use gcop_rs::llm::{LLMProvider, provider::openai::OpenAIProvider};
/// use gcop_rs::config::{ProviderConfig, NetworkConfig};
///
/// # async fn example() -> anyhow::Result<()> {
/// let config = ProviderConfig {
///     api_key: Some("sk-...".to_string()),
///     model: "gpt-4".to_string(),
///     ..Default::default()
/// };
/// let network_config = NetworkConfig::default();
/// let provider = OpenAIProvider::new(&config, "openai", &network_config, false)?;
///
/// // 生成 commit message
/// let diff = "diff --git a/main.rs...";
/// let message = provider.generate_commit_message(diff, None, None).await?;
/// println!("Generated: {}", message);
/// # Ok(())
/// # }
/// ```
pub struct OpenAIProvider {
    name: String,
    client: Client,
    api_key: String,
    endpoint: String,
    model: String,
    max_tokens: Option<u32>,
    temperature: f32,
    max_retries: usize,
    retry_delay_ms: u64,
    max_retry_delay_ms: u64,
    colored: bool,
}

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<MessagePayload>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Serialize, Deserialize)]
struct MessagePayload {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: MessageContent,
}

#[derive(Deserialize)]
struct MessageContent {
    content: String,
}

impl OpenAIProvider {
    pub fn new(
        config: &ProviderConfig,
        provider_name: &str,
        network_config: &NetworkConfig,
        colored: bool,
    ) -> Result<Self> {
        let api_key = extract_api_key(config, "OpenAI")?;
        let endpoint = build_endpoint(config, DEFAULT_OPENAI_BASE, OPENAI_API_SUFFIX);
        let model = config.model.clone();
        let max_tokens = get_max_tokens_optional(config);
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
impl ApiBackend for OpenAIProvider {
    fn name(&self) -> &str {
        &self.name
    }

    async fn call_api(
        &self,
        system: &str,
        user_message: &str,
        spinner: Option<&crate::ui::Spinner>,
    ) -> Result<String> {
        let request = OpenAIRequest {
            model: self.model.clone(),
            messages: vec![
                MessagePayload {
                    role: "system".to_string(),
                    content: system.to_string(),
                },
                MessagePayload {
                    role: "user".to_string(),
                    content: user_message.to_string(),
                },
            ],
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            stream: None,
        };

        tracing::debug!(
            "OpenAI API request: model={}, temperature={}, max_tokens={:?}, system_len={}, user_len={}",
            self.model,
            self.temperature,
            self.max_tokens,
            system.len(),
            user_message.len()
        );

        let auth_header = format!("Bearer {}", self.api_key);
        let response: OpenAIResponse = send_llm_request(
            &self.client,
            &self.endpoint,
            &[("Authorization", auth_header.as_str())],
            &request,
            "OpenAI",
            spinner,
            self.max_retries,
            self.retry_delay_ms,
            self.max_retry_delay_ms,
        )
        .await?;

        response
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .ok_or_else(|| GcopError::Llm(rust_i18n::t!("provider.openai_no_choices").to_string()))
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    async fn call_api_streaming(&self, system: &str, user_message: &str) -> Result<StreamHandle> {
        let (tx, rx) = mpsc::channel(64);

        let request = OpenAIRequest {
            model: self.model.clone(),
            messages: vec![
                MessagePayload {
                    role: "system".to_string(),
                    content: system.to_string(),
                },
                MessagePayload {
                    role: "user".to_string(),
                    content: user_message.to_string(),
                },
            ],
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            stream: Some(true),
        };

        tracing::debug!(
            "OpenAI Streaming API request: model={}, temperature={}, max_tokens={:?}, system_len={}, user_len={}",
            self.model,
            self.temperature,
            self.max_tokens,
            system.len(),
            user_message.len()
        );

        let auth_header = format!("Bearer {}", self.api_key);

        let response = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("Authorization", &auth_header)
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
        // tx 会在任务结束时自动 drop，从而关闭 channel
        let colored = self.colored;
        tokio::spawn(async move {
            let error_tx = tx.clone();
            if let Err(e) = process_openai_stream(response, tx, colored).await {
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

        let test_request = OpenAIRequest {
            model: self.model.clone(),
            messages: vec![MessagePayload {
                role: "user".to_string(),
                content: "test".to_string(),
            }],
            temperature: 1.0,
            max_tokens: Some(1), // Minimize API cost
            stream: None,
        };

        let auth_header = format!("Bearer {}", self.api_key);
        validate_http_endpoint(
            &self.client,
            &self.endpoint,
            &[("Authorization", auth_header.as_str())],
            &test_request,
            "OpenAI",
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
    async fn test_openai_success_response_parsing() {
        ensure_crypto_provider();
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"choices":[{"message":{"content":"Hello from OpenAI"}}]}"#)
            .create_async()
            .await;

        let provider = OpenAIProvider::new(
            &test_provider_config(
                server.url(),
                Some("sk-test".to_string()),
                "gpt-4o-mini".to_string(),
            ),
            "openai",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        let result = provider.call_api("system", "hi", None).await.unwrap();
        assert_eq!(result, "Hello from OpenAI");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_openai_api_error_401() {
        ensure_crypto_provider();
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .with_status(401)
            .with_body("Unauthorized")
            .create_async()
            .await;

        let provider = OpenAIProvider::new(
            &test_provider_config(
                server.url(),
                Some("sk-test".to_string()),
                "gpt-4o-mini".to_string(),
            ),
            "openai",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        let err = provider.call_api("system", "hi", None).await.unwrap_err();
        assert!(matches!(err, GcopError::LlmApi { status: 401, .. }));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_openai_api_error_429() {
        ensure_crypto_provider();
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .with_status(429)
            .with_body("Too Many Requests")
            .create_async()
            .await;

        let provider = OpenAIProvider::new(
            &test_provider_config(
                server.url(),
                Some("sk-test".to_string()),
                "gpt-4o-mini".to_string(),
            ),
            "openai",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        let err = provider.call_api("system", "hi", None).await.unwrap_err();
        assert!(matches!(err, GcopError::LlmApi { status: 429, .. }));
        mock.assert_async().await;
    }
}
