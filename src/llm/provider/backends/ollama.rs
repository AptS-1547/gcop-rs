use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::super::base::{ApiBackend, build_endpoint, get_temperature_optional, send_llm_request};
use super::super::utils::{DEFAULT_OLLAMA_BASE, OLLAMA_API_SUFFIX};
use crate::config::{NetworkConfig, ProviderConfig};
use crate::error::{GcopError, Result};

/// Ollama API provider
///
/// Generate commit messages and code reviews using locally running Ollama models.
///
/// # Supported models
/// - `llama3.2` (recommended)
/// - `llama3.1`
/// - `codellama`
/// - `qwen2.5-coder`
/// - `deepseek-coder-v2`
/// - Other Ollama supported models
///
/// # Configuration example
/// ```toml
/// [llm]
/// default_provider = "ollama"
///
/// [llm.providers.ollama]
/// model = "llama3.2"
/// endpoint = "http://localhost:11434" # Optional, default value
/// temperature = 0.7 # optional
/// ```
///
/// # Configuration method
///
/// Set optional `endpoint` in `config.toml` (default `http://localhost:11434`).
/// Ollama runs natively and requires no API key.
/// Use the `GCOP_CI_ENDPOINT` environment variable in CI mode.
///
/// # Features
/// - Runs completely natively (no API key required)
/// - Support custom models
/// - Automatic retries (exponential backoff, default 3 times, configurable through `network.max_retries`)
/// - No streaming support (planned)
///
/// # Prerequisites for use
/// 1. Install Ollama: <https://ollama.ai>
/// 2. Pull model: `ollama pull llama3.2`
/// 3. Make sure the Ollama service is running: `ollama serve`
///
/// # Example
/// ```ignore
/// use gcop_rs::llm::{LLMProvider, provider::ollama::OllamaProvider};
/// use gcop_rs::config::{ProviderConfig, NetworkConfig};
///
/// # async fn example() -> anyhow::Result<()> {
/// let config = ProviderConfig {
///     model: "llama3.2".to_string(),
///     endpoint: Some("http://localhost:11434".to_string()),
///     ..Default::default()
/// };
/// let network_config = NetworkConfig::default();
/// let provider = OllamaProvider::new(&config, "ollama", &network_config, false)?;
///
/// // Generate commit message
/// let diff = "diff --git a/main.rs...";
/// let message = provider.generate_commit_message(diff, None, None).await?;
/// println!("Generated: {}", message);
/// # Ok(())
/// # }
/// ```
pub struct OllamaProvider {
    name: String,
    client: Client,
    endpoint: String,
    model: String,
    temperature: Option<f32>,
    max_retries: usize,
    retry_delay_ms: u64,
    max_retry_delay_ms: u64,
    #[allow(dead_code)] // Reserved for future streaming output support
    colored: bool,
}

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

#[derive(Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
    #[allow(dead_code)] // Reserved for integrity verification
    done: bool,
}

impl OllamaProvider {
    /// Builds an Ollama provider from runtime configuration.
    pub fn new(
        config: &ProviderConfig,
        provider_name: &str,
        network_config: &NetworkConfig,
        colored: bool,
    ) -> Result<Self> {
        // Ollama local deployment, no API key required
        let endpoint = build_endpoint(config, DEFAULT_OLLAMA_BASE, OLLAMA_API_SUFFIX);
        let model = config.model.clone();
        let temperature = get_temperature_optional(config);

        Ok(Self {
            name: provider_name.to_string(),
            client: super::super::create_http_client(network_config)?,
            endpoint,
            model,
            temperature,
            max_retries: network_config.max_retries,
            retry_delay_ms: network_config.retry_delay_ms,
            max_retry_delay_ms: network_config.max_retry_delay_ms,
            colored,
        })
    }
}

#[async_trait]
impl ApiBackend for OllamaProvider {
    fn name(&self) -> &str {
        &self.name
    }

    async fn call_api(
        &self,
        system: &str,
        user_message: &str,
        progress: Option<&dyn crate::llm::ProgressReporter>,
    ) -> Result<String> {
        let options = self.temperature.map(|temp| OllamaOptions {
            temperature: Some(temp),
        });

        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: user_message.to_string(),
            system: Some(system.to_string()),
            stream: false,
            options,
        };

        tracing::debug!(
            "Ollama API request: model={}, temperature={:?}, system_len={}, user_len={}",
            self.model,
            self.temperature,
            system.len(),
            user_message.len()
        );

        let response: OllamaResponse = send_llm_request(
            &self.client,
            &self.endpoint,
            &[], // Ollama does not require auth headers
            &request,
            "Ollama",
            progress,
            self.max_retries,
            self.retry_delay_ms,
            self.max_retry_delay_ms,
        )
        .await?;

        Ok(response.response)
    }

    async fn validate(&self) -> Result<()> {
        // Validate Ollama connection and model availability
        tracing::debug!("Validating Ollama connection...");

        // Ollama health check endpoint: /api/tags
        let health_endpoint = self.endpoint.replace("/api/generate", "/api/tags");

        let response = self
            .client
            .get(&health_endpoint)
            .send()
            .await
            .map_err(GcopError::Network)?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(GcopError::LlmApi {
                status: status.as_u16(),
                message: rust_i18n::t!(
                    "provider.api_validation_failed",
                    provider = "Ollama",
                    body = body
                )
                .to_string(),
            });
        }

        // Check if configured model exists
        #[derive(Deserialize)]
        struct TagsResponse {
            models: Vec<ModelInfo>,
        }

        #[derive(Deserialize)]
        struct ModelInfo {
            name: String,
        }

        let tags: TagsResponse = response.json().await.map_err(|e| {
            GcopError::Llm(
                rust_i18n::t!("provider.ollama_parse_tags_failed", error = e.to_string())
                    .to_string(),
            )
        })?;

        if !tags.models.iter().any(|m| m.name.starts_with(&self.model)) {
            return Err(GcopError::Config(
                rust_i18n::t!("provider.ollama_model_not_found", model = self.model).to_string(),
            ));
        }

        tracing::debug!("Ollama connection validated successfully");
        Ok(())
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
    async fn test_ollama_success_response_parsing() {
        ensure_crypto_provider();
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/generate")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"response":"Hello from Ollama","done":true}"#)
            .create_async()
            .await;

        let provider = OllamaProvider::new(
            &test_provider_config(server.url(), None, "llama3".to_string()),
            "ollama",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        let result = provider.call_api("system", "hi", None).await.unwrap();
        assert_eq!(result, "Hello from Ollama");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_ollama_api_error_401() {
        ensure_crypto_provider();
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/generate")
            .with_status(401)
            .with_body("Unauthorized")
            .create_async()
            .await;

        let provider = OllamaProvider::new(
            &test_provider_config(server.url(), None, "llama3".to_string()),
            "ollama",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        let err = provider.call_api("system", "hi", None).await.unwrap_err();
        assert!(matches!(err, GcopError::LlmApi { status: 401, .. }));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_ollama_api_error_429() {
        ensure_crypto_provider();
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/generate")
            .with_status(429)
            .with_body("Too Many Requests")
            .create_async()
            .await;

        let provider = OllamaProvider::new(
            &test_provider_config(server.url(), None, "llama3".to_string()),
            "ollama",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        let err = provider.call_api("system", "hi", None).await.unwrap_err();
        assert!(matches!(err, GcopError::LlmApi { status: 429, .. }));
        mock.assert_async().await;
    }
}
