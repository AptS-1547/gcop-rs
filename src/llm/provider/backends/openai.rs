use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::super::base::{
    ApiBackend, build_endpoint, extract_api_key, get_max_tokens_optional, get_temperature,
    send_llm_request, send_llm_request_streaming, validate_api_key, validate_http_endpoint,
};
use super::super::streaming::process_openai_stream;
use super::super::utils::{DEFAULT_OPENAI_BASE, OPENAI_API_SUFFIX};
use crate::config::{NetworkConfig, ProviderConfig};
use crate::error::{GcopError, Result};
use crate::llm::StreamHandle;

/// OpenAI API provider
///
/// Use the OpenAI API (or a compatible API) to generate commit messages and code reviews.
///
/// # Supported models
/// - **OpenAI Official**:
///   - `gpt-4` (recommended)
///   - `gpt-4-turbo`
///   - `gpt-3.5-turbo`
/// - **Compatible with API** (such as Azure OpenAI, OpenRouter, etc.)
///
/// # Configuration example
/// ```toml
/// [llm]
/// default_provider = "openai"
///
/// [llm.providers.openai]
/// api_key = "sk-..."
/// model = "gpt-4"
/// endpoint = "https://api.openai.com" # optional
/// max_tokens = 1000 # optional
/// temperature = 0.7 # optional
/// ```
///
/// # Configuration method
///
/// Set `api_key` and optional `endpoint` in `config.toml`.
/// Use the `GCOP_CI_API_KEY` and `GCOP_CI_ENDPOINT` environment variables in CI mode.
///
/// # Features
/// - Supports streaming responses (SSE)
/// - Automatic retries (exponential backoff, default 3 times, configurable through `network.max_retries`)
/// - Third-party services compatible with OpenAI API
/// - Custom endpoint (supports proxy or Azure OpenAI)
///
/// #Azure OpenAI Example
/// ```toml
/// [llm.providers.openai]
/// api_key = "your-azure-key"
/// model = "gpt-4"
/// endpoint = "https://your-resource.openai.azure.com/v1/chat/completions"
/// ```
///
/// # Example
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
/// // Generate commit message
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

#[derive(Clone, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<MessagePayload>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Clone, Serialize, Deserialize)]
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
    /// Builds an OpenAI-compatible provider from runtime configuration.
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
            client: super::super::create_http_client(network_config)?,
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
        progress: Option<&dyn crate::llm::ProgressReporter>,
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
            progress,
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

        let response = send_llm_request_streaming(
            &self.client,
            &self.endpoint,
            &[("Authorization", auth_header.as_str())],
            &request,
            "OpenAI",
            None,
            self.max_retries,
            self.retry_delay_ms,
            self.max_retry_delay_ms,
        )
        .await?;

        use super::super::base::spawn_stream_with_retry;

        let colored = self.colored;
        let client = self.client.clone();
        let endpoint = self.endpoint.clone();
        let api_key = self.api_key.clone();
        let retry_delay_ms = self.retry_delay_ms;
        let max_retry_delay_ms = self.max_retry_delay_ms;
        let request = request.clone();

        spawn_stream_with_retry(
            response,
            tx,
            colored,
            "OpenAI",
            self.max_retries,
            retry_delay_ms,
            max_retry_delay_ms,
            process_openai_stream,
            move || {
                let client = client.clone();
                let endpoint = endpoint.clone();
                let api_key = api_key.clone();
                let request = request.clone();
                async move {
                    let auth_header = format!("Bearer {}", api_key);
                    send_llm_request_streaming(
                        &client,
                        &endpoint,
                        &[("Authorization", auth_header.as_str())],
                        &request,
                        "OpenAI",
                        None,
                        0,
                        retry_delay_ms,
                        max_retry_delay_ms,
                    )
                    .await
                }
            },
        );

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
