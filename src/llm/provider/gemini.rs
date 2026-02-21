use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::base::{
    ApiBackend, extract_api_key, get_max_tokens_optional, get_temperature, send_llm_request,
    send_llm_request_streaming, validate_api_key, validate_http_endpoint,
};
use super::streaming::process_gemini_stream;
use super::utils::DEFAULT_GEMINI_BASE;
use crate::config::{NetworkConfig, ProviderConfig};
use crate::error::{GcopError, Result};
use crate::llm::{StreamChunk, StreamHandle};

/// Google Gemini API provider
///
/// Generate commit messages and code reviews using the Google Gemini API.
///
/// # Configuration example
/// ```toml
/// [llm]
/// default_provider = "gemini"
///
/// [llm.providers.gemini]
/// api_key = "AIza..."
/// model = "gemini-3-flash-preview"
/// endpoint = "https://generativelanguage.googleapis.com" # Optional
/// max_tokens = 2000 # optional
/// temperature = 0.3 # optional
/// ```
///
/// # Features
/// - Supports streaming responses (SSE)
/// - Automatic retry (exponential backoff)
/// - Custom endpoints
pub struct GeminiProvider {
    name: String,
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
    max_output_tokens: Option<u32>,
    temperature: f32,
    max_retries: usize,
    retry_delay_ms: u64,
    max_retry_delay_ms: u64,
    colored: bool,
}

// ============================================================================
// Request/response structure
// ============================================================================

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiContent>,
    contents: Vec<GeminiContent>,
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct GeminiContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiCandidate {
    content: Option<GeminiResponseContent>,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct GeminiResponseContent {
    #[serde(default)]
    parts: Option<Vec<GeminiResponsePart>>,
}

#[derive(Deserialize)]
struct GeminiResponsePart {
    text: String,
}

// ============================================================================
// accomplish
// ============================================================================

impl GeminiProvider {
    /// Builds a Gemini provider from runtime configuration.
    pub fn new(
        config: &ProviderConfig,
        provider_name: &str,
        network_config: &NetworkConfig,
        colored: bool,
    ) -> Result<Self> {
        let api_key = extract_api_key(config, "Gemini")?;
        let base_url = config
            .endpoint
            .as_deref()
            .unwrap_or(DEFAULT_GEMINI_BASE)
            .trim_end_matches('/')
            .to_string();
        let model = config.model.clone();
        let max_output_tokens = get_max_tokens_optional(config);
        let temperature = get_temperature(config);

        Ok(Self {
            name: provider_name.to_string(),
            client: super::create_http_client(network_config)?,
            api_key,
            base_url,
            model,
            max_output_tokens,
            temperature,
            max_retries: network_config.max_retries,
            retry_delay_ms: network_config.retry_delay_ms,
            max_retry_delay_ms: network_config.max_retry_delay_ms,
            colored,
        })
    }

    /// Non-streaming endpoint: /v1beta/models/{model}:generateContent
    fn generate_content_url(&self) -> String {
        format!(
            "{}/v1beta/models/{}:generateContent",
            self.base_url, self.model
        )
    }

    /// Streaming endpoint: /v1beta/models/{model}:streamGenerateContent?alt=sse
    fn stream_generate_content_url(&self) -> String {
        format!(
            "{}/v1beta/models/{}:streamGenerateContent?alt=sse",
            self.base_url, self.model
        )
    }

    fn build_request(&self, system: &str, user_message: &str) -> GeminiRequest {
        GeminiRequest {
            system_instruction: Some(GeminiContent {
                role: None,
                parts: vec![GeminiPart {
                    text: system.to_string(),
                }],
            }),
            contents: vec![GeminiContent {
                role: Some("user".to_string()),
                parts: vec![GeminiPart {
                    text: user_message.to_string(),
                }],
            }],
            generation_config: GenerationConfig {
                temperature: self.temperature,
                max_output_tokens: self.max_output_tokens,
            },
        }
    }
}

#[async_trait]
impl ApiBackend for GeminiProvider {
    fn name(&self) -> &str {
        &self.name
    }

    async fn call_api(
        &self,
        system: &str,
        user_message: &str,
        progress: Option<&dyn crate::llm::ProgressReporter>,
    ) -> Result<String> {
        let request = self.build_request(system, user_message);

        tracing::debug!(
            "Gemini API request: model={}, temperature={}, max_output_tokens={:?}, system_len={}, user_len={}",
            self.model,
            self.temperature,
            self.max_output_tokens,
            system.len(),
            user_message.len()
        );

        let endpoint = self.generate_content_url();
        let response: GeminiResponse = send_llm_request(
            &self.client,
            &endpoint,
            &[("x-goog-api-key", self.api_key.as_str())],
            &request,
            "Gemini",
            progress,
            self.max_retries,
            self.retry_delay_ms,
            self.max_retry_delay_ms,
        )
        .await?;

        let candidate = response
            .candidates
            .and_then(|c| c.into_iter().next())
            .ok_or_else(|| {
                GcopError::Llm(rust_i18n::t!("provider.gemini_no_candidates").to_string())
            })?;

        // Check the reasons for abnormal end (SAFETY, RECITATION, etc.)
        if let Some(reason) = &candidate.finish_reason {
            match reason.as_str() {
                "STOP" => {}
                "MAX_TOKENS" => {
                    tracing::warn!("Gemini response truncated (MAX_TOKENS)");
                }
                _ => {
                    tracing::warn!("Gemini response finished with reason: {}", reason);
                    return Err(GcopError::LlmContentBlocked {
                        provider: "Gemini".to_string(),
                        reason: reason.clone(),
                    });
                }
            }
        }

        candidate
            .content
            .and_then(|c| c.parts)
            .and_then(|parts| parts.into_iter().next())
            .map(|p| p.text)
            .ok_or_else(|| {
                GcopError::Llm(rust_i18n::t!("provider.gemini_no_candidates").to_string())
            })
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    async fn call_api_streaming(&self, system: &str, user_message: &str) -> Result<StreamHandle> {
        let (tx, rx) = mpsc::channel(64);

        let request = self.build_request(system, user_message);
        let endpoint = self.stream_generate_content_url();

        tracing::debug!(
            "Gemini Streaming API request: model={}, temperature={}, max_output_tokens={:?}, system_len={}, user_len={}",
            self.model,
            self.temperature,
            self.max_output_tokens,
            system.len(),
            user_message.len()
        );

        let response = send_llm_request_streaming(
            &self.client,
            &endpoint,
            &[("x-goog-api-key", self.api_key.as_str())],
            &request,
            "Gemini",
            None,
            self.max_retries,
            self.retry_delay_ms,
            self.max_retry_delay_ms,
        )
        .await?;

        let colored = self.colored;
        tokio::spawn(async move {
            let error_tx = tx.clone();
            if let Err(e) = process_gemini_stream(response, tx, colored).await {
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

        let test_request = GeminiRequest {
            system_instruction: None,
            contents: vec![GeminiContent {
                role: Some("user".to_string()),
                parts: vec![GeminiPart {
                    text: "test".to_string(),
                }],
            }],
            generation_config: GenerationConfig {
                temperature: 1.0,
                max_output_tokens: Some(1), // Minimize API cost
            },
        };
        let endpoint = self.generate_content_url();

        validate_http_endpoint(
            &self.client,
            &endpoint,
            &[("x-goog-api-key", self.api_key.as_str())],
            &test_request,
            "Gemini",
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
    async fn test_gemini_success_response_parsing() {
        ensure_crypto_provider();
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1beta/models/gemini-3-flash-preview:generateContent")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"candidates":[{"content":{"parts":[{"text":"Hello from Gemini"}],"role":"model"}}]}"#,
            )
            .create_async()
            .await;

        let provider = GeminiProvider::new(
            &test_provider_config(
                server.url(),
                Some("AIza-test".to_string()),
                "gemini-3-flash-preview".to_string(),
            ),
            "gemini",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        let result = provider.call_api("system", "hi", None).await.unwrap();
        assert_eq!(result, "Hello from Gemini");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_gemini_api_error_401() {
        ensure_crypto_provider();
        let mut server = Server::new_async().await;
        let mock = server
            .mock(
                "POST",
                "/v1beta/models/gemini-3-flash-preview:generateContent",
            )
            .with_status(401)
            .with_body("Unauthorized")
            .create_async()
            .await;

        let provider = GeminiProvider::new(
            &test_provider_config(
                server.url(),
                Some("AIza-test".to_string()),
                "gemini-3-flash-preview".to_string(),
            ),
            "gemini",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        let err = provider.call_api("system", "hi", None).await.unwrap_err();
        assert!(matches!(err, GcopError::LlmApi { status: 401, .. }));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_gemini_api_error_429() {
        ensure_crypto_provider();
        let mut server = Server::new_async().await;
        let mock = server
            .mock(
                "POST",
                "/v1beta/models/gemini-3-flash-preview:generateContent",
            )
            .with_status(429)
            .with_body("Too Many Requests")
            .create_async()
            .await;

        let provider = GeminiProvider::new(
            &test_provider_config(
                server.url(),
                Some("AIza-test".to_string()),
                "gemini-3-flash-preview".to_string(),
            ),
            "gemini",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        let err = provider.call_api("system", "hi", None).await.unwrap_err();
        assert!(matches!(err, GcopError::LlmApi { status: 429, .. }));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_gemini_safety_blocked_response() {
        ensure_crypto_provider();
        let mut server = Server::new_async().await;
        let mock = server
            .mock(
                "POST",
                "/v1beta/models/gemini-3-flash-preview:generateContent",
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"candidates":[{"finishReason":"SAFETY"}]}"#)
            .create_async()
            .await;

        let provider = GeminiProvider::new(
            &test_provider_config(
                server.url(),
                Some("AIza-test".to_string()),
                "gemini-3-flash-preview".to_string(),
            ),
            "gemini",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        let err = provider.call_api("system", "hi", None).await.unwrap_err();
        match &err {
            GcopError::LlmContentBlocked { provider, reason } => {
                assert_eq!(provider, "Gemini");
                assert_eq!(reason, "SAFETY");
            }
            _ => panic!("Expected GcopError::LlmContentBlocked, got: {:?}", err),
        }
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_gemini_no_content_response() {
        ensure_crypto_provider();
        let mut server = Server::new_async().await;
        let mock = server
            .mock(
                "POST",
                "/v1beta/models/gemini-3-flash-preview:generateContent",
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"candidates":[{"content":{"parts":[]},"finishReason":"STOP"}]}"#)
            .create_async()
            .await;

        let provider = GeminiProvider::new(
            &test_provider_config(
                server.url(),
                Some("AIza-test".to_string()),
                "gemini-3-flash-preview".to_string(),
            ),
            "gemini",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        let err = provider.call_api("system", "hi", None).await.unwrap_err();
        assert!(matches!(err, GcopError::Llm(_)));
        mock.assert_async().await;
    }
}
