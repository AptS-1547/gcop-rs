use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::base::{
    build_commit_prompt_with_log, build_endpoint, build_review_prompt_with_log,
    get_temperature_optional, process_commit_response, process_review_response, send_llm_request,
};
use super::utils::{DEFAULT_OLLAMA_BASE, OLLAMA_API_SUFFIX};
use crate::config::{NetworkConfig, ProviderConfig};
use crate::error::Result;
use crate::llm::{CommitContext, LLMProvider, ReviewResult, ReviewType};

/// Ollama API Provider
pub struct OllamaProvider {
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
        _provider_name: &str,
        network_config: &NetworkConfig,
        colored: bool,
    ) -> Result<Self> {
        // Ollama 本地部署，无需 API key
        let endpoint = build_endpoint(config, DEFAULT_OLLAMA_BASE, OLLAMA_API_SUFFIX);
        let model = config.model.clone();
        let temperature = get_temperature_optional(config);

        Ok(Self {
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
        "ollama"
    }

    async fn validate(&self) -> Result<()> {
        // Ollama 本地部署，无需验证 API key
        Ok(())
    }
}
