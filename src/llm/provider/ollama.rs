use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::base::{
    build_commit_prompt_with_log, build_endpoint, build_review_prompt_with_log,
    get_temperature_optional, process_commit_response, process_review_response, send_llm_request,
};
use super::utils::{DEFAULT_OLLAMA_BASE, OLLAMA_API_SUFFIX};
use crate::config::{NetworkConfig, ProviderConfig};
use crate::error::{GcopError, Result};
use crate::llm::{CommitContext, LLMProvider, ReviewResult, ReviewType};

/// Ollama API Provider
pub struct OllamaProvider {
    name: String,
    client: Client,
    endpoint: String,
    model: String,
    temperature: Option<f32>,
    max_retries: usize,
    retry_delay_ms: u64,
    max_retry_delay_ms: u64,
    #[allow(dead_code)] // 保留用于未来流式输出支持
    colored: bool,
}

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
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
    #[allow(dead_code)] // 保留用于完整性验证
    done: bool,
}

impl OllamaProvider {
    pub fn new(
        config: &ProviderConfig,
        provider_name: &str,
        network_config: &NetworkConfig,
        colored: bool,
    ) -> Result<Self> {
        // Ollama 本地部署，无需 API key
        let endpoint = build_endpoint(config, DEFAULT_OLLAMA_BASE, OLLAMA_API_SUFFIX);
        let model = config.model.clone();
        let temperature = get_temperature_optional(config);

        Ok(Self {
            name: provider_name.to_string(),
            client: super::create_http_client(network_config)?,
            endpoint,
            model,
            temperature,
            max_retries: network_config.max_retries,
            retry_delay_ms: network_config.retry_delay_ms,
            max_retry_delay_ms: network_config.max_retry_delay_ms,
            colored,
        })
    }

    async fn call_api(&self, prompt: &str, spinner: Option<&crate::ui::Spinner>) -> Result<String> {
        let options = self.temperature.map(|temp| OllamaOptions {
            temperature: Some(temp),
        });

        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
            options,
        };

        tracing::debug!(
            "Ollama API request: model={}, temperature={:?}",
            self.model,
            self.temperature
        );

        let response: OllamaResponse = send_llm_request(
            &self.client,
            &self.endpoint,
            &[], // Ollama 无需 auth headers
            &request,
            "Ollama",
            spinner,
            self.max_retries,
            self.retry_delay_ms,
            self.max_retry_delay_ms,
        )
        .await?;

        Ok(response.response)
    }
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    async fn generate_commit_message(
        &self,
        diff: &str,
        context: Option<CommitContext>,
        spinner: Option<&crate::ui::Spinner>,
    ) -> Result<String> {
        let prompt = build_commit_prompt_with_log(diff, context);
        let response = self.call_api(&prompt, spinner).await?;
        Ok(process_commit_response(response))
    }

    async fn review_code(
        &self,
        diff: &str,
        review_type: ReviewType,
        custom_prompt: Option<&str>,
        spinner: Option<&crate::ui::Spinner>,
    ) -> Result<ReviewResult> {
        let prompt = build_review_prompt_with_log(diff, &review_type, custom_prompt);
        let response = self.call_api(&prompt, spinner).await?;
        process_review_response(&response)
    }

    fn name(&self) -> &str {
        &self.name
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
                message: format!("Ollama validation failed: {}", body),
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
            GcopError::Llm(format!("Failed to parse Ollama tags response: {}", e))
        })?;

        if !tags.models.iter().any(|m| m.name.starts_with(&self.model)) {
            return Err(GcopError::Config(format!(
                "Model '{}' not found in Ollama. Run 'ollama pull {}' first.",
                self.model, self.model
            )));
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
            api_key: None,
            model: "llama3".to_string(),
            max_tokens: None,
            temperature: None,
            extra: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_ollama_success_response_parsing() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/generate")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"response":"Hello from Ollama","done":true}"#)
            .create_async()
            .await;

        let provider = OllamaProvider::new(
            &test_provider_config(server.url()),
            "ollama",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        let result = provider.call_api("hi", None).await.unwrap();
        assert_eq!(result, "Hello from Ollama");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_ollama_api_error_401() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/generate")
            .with_status(401)
            .with_body("Unauthorized")
            .create_async()
            .await;

        let provider = OllamaProvider::new(
            &test_provider_config(server.url()),
            "ollama",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        let err = provider.call_api("hi", None).await.unwrap_err();
        assert!(matches!(err, GcopError::LlmApi { status: 401, .. }));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_ollama_api_error_429() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/generate")
            .with_status(429)
            .with_body("Too Many Requests")
            .create_async()
            .await;

        let provider = OllamaProvider::new(
            &test_provider_config(server.url()),
            "ollama",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        let err = provider.call_api("hi", None).await.unwrap_err();
        assert!(matches!(err, GcopError::LlmApi { status: 429, .. }));
        mock.assert_async().await;
    }
}
