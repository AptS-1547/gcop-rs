use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::super::base::{
    ApiBackend, build_endpoint, extract_api_key, extract_extra_bool, get_max_tokens_optional,
    get_temperature, send_llm_request, send_llm_request_streaming, validate_api_key,
    validate_http_endpoint,
};
use super::super::streaming::{process_openai_responses_stream, process_openai_stream};
use super::super::utils::{DEFAULT_OPENAI_BASE, OPENAI_API_SUFFIX, OPENAI_RESPONSES_API_SUFFIX};
use crate::config::{ApiStyle, NetworkConfig, ProviderConfig};
use crate::error::{GcopError, Result};
use crate::llm::StreamHandle;

/// OpenAI API provider
///
/// Use the OpenAI API (or a compatible API) to generate commit messages and code reviews.
///
/// # Model compatibility
/// `gcop-rs` does not hardcode an OpenAI model allowlist.
/// Any Chat Completions compatible model can be used, including third-party
/// OpenAI-compatible services.
///
/// # Configuration example
/// ```toml
/// [llm]
/// default_provider = "openai"
///
/// [llm.providers.openai]
/// api_key = "sk-..."
/// model = "gpt-4o-mini"
/// endpoint = "https://api.openai.com" # optional base URL or full request path
/// max_tokens = 1000 # optional
/// temperature = 0.7 # optional
/// ```
///
/// # Configuration method
///
/// Set `api_key` and optional `endpoint` in `config.toml`.
/// `endpoint` may be either a base URL (for example `https://api.openai.com`)
/// or a full chat-completions path.
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
/// model = "gpt-4o-mini"
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
///     model: "gpt-4o-mini".to_string(),
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
    chat_endpoint: String,
    responses_endpoint: String,
    api_mode: OpenAIApiMode,
    model: String,
    max_tokens: Option<u32>,
    temperature: f32,
    max_retries: usize,
    retry_delay_ms: u64,
    max_retry_delay_ms: u64,
    colored: bool,
    strip_thinking: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenAIApiMode {
    ChatCompletions,
    Responses,
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

#[derive(Clone, Serialize)]
struct OpenAIResponsesRequest {
    model: String,
    instructions: String,
    input: String,
    temperature: f32,
    tool_choice: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Deserialize)]
struct OpenAIResponsesResponse {
    #[serde(default)]
    output: Vec<ResponseOutputItem>,
    #[serde(default)]
    output_text: Option<String>,
    #[serde(default)]
    error: Option<ResponseError>,
    #[serde(default)]
    incomplete_details: Option<ResponseIncompleteDetails>,
}

#[derive(Deserialize)]
struct ResponseOutputItem {
    #[serde(default)]
    content: Vec<ResponseContentItem>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ResponseContentItem {
    #[serde(rename = "output_text")]
    OutputText { text: String },
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct ResponseError {
    code: Option<String>,
    message: String,
}

#[derive(Deserialize)]
struct ResponseIncompleteDetails {
    reason: String,
}

impl OpenAIResponsesResponse {
    fn into_text(self) -> Result<String> {
        if let Some(error) = self.error {
            let code = error.code.unwrap_or_else(|| "unknown".to_string());
            return Err(GcopError::Llm(format!(
                "OpenAI Responses API error ({}): {}",
                code, error.message
            )));
        }

        if let Some(details) = self.incomplete_details {
            tracing::warn!(
                "OpenAI Responses API returned incomplete response: {}",
                details.reason
            );
        }

        if let Some(output_text) = self.output_text
            && !output_text.is_empty()
        {
            return Ok(output_text);
        }

        let text = self
            .output
            .into_iter()
            .flat_map(|item| item.content)
            .filter_map(|content| match content {
                ResponseContentItem::OutputText { text } => Some(text),
                ResponseContentItem::Other => None,
            })
            .collect::<Vec<_>>()
            .join("");

        if text.is_empty() {
            return Err(GcopError::Llm(
                rust_i18n::t!("provider.empty_response", provider = "OpenAI").to_string(),
            ));
        }

        Ok(text)
    }
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
        let api_mode = match config
            .api_style
            .or_else(|| provider_name.parse::<ApiStyle>().ok())
        {
            Some(ApiStyle::OpenAIResponse) => OpenAIApiMode::Responses,
            _ => OpenAIApiMode::ChatCompletions,
        };
        let chat_endpoint = build_endpoint(config, DEFAULT_OPENAI_BASE, OPENAI_API_SUFFIX);
        let responses_endpoint =
            build_endpoint(config, DEFAULT_OPENAI_BASE, OPENAI_RESPONSES_API_SUFFIX);
        let model = config.model.clone();
        let max_tokens = get_max_tokens_optional(config);
        let temperature = get_temperature(config);
        let strip_thinking = extract_extra_bool(config, "strip_thinking").unwrap_or(false);

        Ok(Self {
            name: provider_name.to_string(),
            client: super::super::create_http_client(network_config)?,
            api_key,
            chat_endpoint,
            responses_endpoint,
            api_mode,
            model,
            max_tokens,
            temperature,
            max_retries: network_config.max_retries,
            retry_delay_ms: network_config.retry_delay_ms,
            max_retry_delay_ms: network_config.max_retry_delay_ms,
            colored,
            strip_thinking,
        })
    }

    async fn call_chat_completions_api(
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
            "OpenAI Chat Completions API request: model={}, temperature={}, max_tokens={:?}, system_len={}, user_len={}",
            self.model,
            self.temperature,
            self.max_tokens,
            system.len(),
            user_message.len()
        );

        let auth_header = format!("Bearer {}", self.api_key);
        let response: OpenAIResponse = send_llm_request(
            &self.client,
            &self.chat_endpoint,
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

    async fn call_responses_api(
        &self,
        system: &str,
        user_message: &str,
        progress: Option<&dyn crate::llm::ProgressReporter>,
    ) -> Result<String> {
        let request = OpenAIResponsesRequest {
            model: self.model.clone(),
            instructions: system.to_string(),
            input: user_message.to_string(),
            temperature: self.temperature,
            max_output_tokens: self.max_tokens,
            tool_choice: "none",
            stream: None,
        };

        tracing::debug!(
            "OpenAI Responses API request: model={}, temperature={}, max_output_tokens={:?}, system_len={}, user_len={}",
            self.model,
            self.temperature,
            self.max_tokens,
            system.len(),
            user_message.len()
        );

        let auth_header = format!("Bearer {}", self.api_key);
        let response: OpenAIResponsesResponse = send_llm_request(
            &self.client,
            &self.responses_endpoint,
            &[("Authorization", auth_header.as_str())],
            &request,
            "OpenAI",
            progress,
            self.max_retries,
            self.retry_delay_ms,
            self.max_retry_delay_ms,
        )
        .await?;

        response.into_text()
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
        match self.api_mode {
            OpenAIApiMode::ChatCompletions => {
                self.call_chat_completions_api(system, user_message, progress)
                    .await
            }
            OpenAIApiMode::Responses => {
                self.call_responses_api(system, user_message, progress)
                    .await
            }
        }
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn strip_thinking(&self) -> bool {
        self.strip_thinking
    }

    async fn call_api_streaming(&self, system: &str, user_message: &str) -> Result<StreamHandle> {
        let (tx, rx) = mpsc::channel(64);

        match self.api_mode {
            OpenAIApiMode::ChatCompletions => {
                self.call_chat_completions_streaming(system, user_message, tx)
                    .await?;
            }
            OpenAIApiMode::Responses => {
                self.call_responses_streaming(system, user_message, tx)
                    .await?;
            }
        }

        Ok(StreamHandle { receiver: rx })
    }

    async fn validate(&self) -> Result<()> {
        validate_api_key(&self.api_key)?;

        match self.api_mode {
            OpenAIApiMode::ChatCompletions => {
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
                    &self.chat_endpoint,
                    &[("Authorization", auth_header.as_str())],
                    &test_request,
                    "OpenAI",
                )
                .await
            }
            OpenAIApiMode::Responses => {
                let test_request = OpenAIResponsesRequest {
                    model: self.model.clone(),
                    instructions: "Return one word.".to_string(),
                    input: "test".to_string(),
                    temperature: 1.0,
                    max_output_tokens: Some(16), // Responses API minimum
                    tool_choice: "none",
                    stream: None,
                };

                let auth_header = format!("Bearer {}", self.api_key);
                validate_http_endpoint(
                    &self.client,
                    &self.responses_endpoint,
                    &[("Authorization", auth_header.as_str())],
                    &test_request,
                    "OpenAI",
                )
                .await
            }
        }
    }
}

impl OpenAIProvider {
    async fn call_chat_completions_streaming(
        &self,
        system: &str,
        user_message: &str,
        tx: mpsc::Sender<crate::llm::StreamChunk>,
    ) -> Result<()> {
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
            &self.chat_endpoint,
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
        let endpoint = self.chat_endpoint.clone();
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

        Ok(())
    }

    async fn call_responses_streaming(
        &self,
        system: &str,
        user_message: &str,
        tx: mpsc::Sender<crate::llm::StreamChunk>,
    ) -> Result<()> {
        let request = OpenAIResponsesRequest {
            model: self.model.clone(),
            instructions: system.to_string(),
            input: user_message.to_string(),
            temperature: self.temperature,
            max_output_tokens: self.max_tokens,
            tool_choice: "none",
            stream: Some(true),
        };

        tracing::debug!(
            "OpenAI Responses Streaming API request: model={}, temperature={}, max_output_tokens={:?}, system_len={}, user_len={}",
            self.model,
            self.temperature,
            self.max_tokens,
            system.len(),
            user_message.len()
        );

        let auth_header = format!("Bearer {}", self.api_key);

        let response = send_llm_request_streaming(
            &self.client,
            &self.responses_endpoint,
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
        let endpoint = self.responses_endpoint.clone();
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
            process_openai_responses_stream,
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

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use pretty_assertions::assert_eq;
    use std::collections::HashMap;

    use crate::error::GcopError;
    use crate::llm::provider::test_utils::{
        ensure_crypto_provider, test_network_config_no_retry, test_provider_config,
    };

    fn responses_provider_config(base_url: String) -> ProviderConfig {
        ProviderConfig {
            api_style: Some(ApiStyle::OpenAIResponse),
            endpoint: Some(base_url),
            api_key: Some("sk-test".to_string()),
            model: "gpt-5-mini".to_string(),
            max_tokens: None,
            temperature: None,
            extra: HashMap::new(),
        }
    }

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

    #[tokio::test]
    async fn test_openai_responses_success_response_parsing() {
        ensure_crypto_provider();
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/responses")
            .match_body(mockito::Matcher::PartialJson(serde_json::json!({
                "model": "gpt-5-mini",
                "instructions": "system",
                "input": "hi",
                "temperature": 0.3,
                "tool_choice": "none"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"output":[{"type":"message","content":[{"type":"output_text","text":"Hello from Responses"}]}]}"#,
            )
            .create_async()
            .await;

        let provider = OpenAIProvider::new(
            &responses_provider_config(server.url()),
            "openai",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        let result = provider.call_api("system", "hi", None).await.unwrap();
        assert_eq!(result, "Hello from Responses");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_openai_responses_inferred_from_provider_name() {
        ensure_crypto_provider();
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/responses")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"output_text":"Inferred mode"}"#)
            .create_async()
            .await;

        let mut config = responses_provider_config(server.url());
        config.api_style = None;

        let provider = OpenAIProvider::new(
            &config,
            "openai-response",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        let result = provider.call_api("system", "hi", None).await.unwrap();
        assert_eq!(result, "Inferred mode");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_openai_responses_uses_output_text_convenience_field() {
        ensure_crypto_provider();
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/responses")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"output_text":"Aggregated text","output":[]}"#)
            .create_async()
            .await;

        let provider = OpenAIProvider::new(
            &responses_provider_config(server.url()),
            "openai",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        let result = provider.call_api("system", "hi", None).await.unwrap();
        assert_eq!(result, "Aggregated text");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_openai_responses_api_error_payload() {
        ensure_crypto_provider();
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/responses")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error":{"code":"server_error","message":"model failed"},"output":[]}"#)
            .create_async()
            .await;

        let provider = OpenAIProvider::new(
            &responses_provider_config(server.url()),
            "openai",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        let err = provider.call_api("system", "hi", None).await.unwrap_err();
        assert!(
            matches!(err, GcopError::Llm(ref message) if message.contains("server_error")),
            "Expected Llm error, got {:?}",
            err
        );
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_openai_responses_validate_uses_minimum_max_output_tokens() {
        ensure_crypto_provider();
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/responses")
            .match_body(mockito::Matcher::PartialJson(serde_json::json!({
                "max_output_tokens": 16,
                "tool_choice": "none"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"output":[{"type":"message","content":[{"type":"output_text","text":"ok"}]}]}"#,
            )
            .create_async()
            .await;

        let provider = OpenAIProvider::new(
            &responses_provider_config(server.url()),
            "openai",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        provider.validate().await.unwrap();
        mock.assert_async().await;
    }
}
