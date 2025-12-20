use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::base::{build_endpoint, extract_extra_f32, parse_review_response, send_llm_request};
use super::utils::{DEFAULT_OLLAMA_BASE, OLLAMA_API_SUFFIX};
use crate::config::ProviderConfig;
use crate::error::Result;
use crate::llm::{CommitContext, LLMProvider, ReviewResult, ReviewType};

/// Ollama API Provider
pub struct OllamaProvider {
    client: Client,
    endpoint: String,
    model: String,
    temperature: Option<f32>,
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
    pub fn new(config: &ProviderConfig, _provider_name: &str) -> Result<Self> {
        // Ollama 本地部署，无需 API key
        let endpoint = build_endpoint(config, DEFAULT_OLLAMA_BASE, OLLAMA_API_SUFFIX);
        let model = config.model.clone();
        let temperature = extract_extra_f32(config, "temperature");

        Ok(Self {
            client: super::create_http_client()?,
            endpoint,
            model,
            temperature,
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
        let ctx = context.unwrap_or_default();
        let prompt =
            crate::llm::prompt::build_commit_prompt(diff, &ctx, ctx.custom_prompt.as_deref());

        tracing::debug!("Prompt ({} chars):\n{}", prompt.len(), prompt);

        let response = self.call_api(&prompt, spinner).await?;

        tracing::debug!("Generated commit message: {}", response);

        Ok(response)
    }

    async fn review_code(
        &self,
        diff: &str,
        review_type: ReviewType,
        custom_prompt: Option<&str>,
        spinner: Option<&crate::ui::Spinner>,
    ) -> Result<ReviewResult> {
        let prompt = crate::llm::prompt::build_review_prompt(diff, &review_type, custom_prompt);

        tracing::debug!("Review prompt ({} chars):\n{}", prompt.len(), prompt);

        let response = self.call_api(&prompt, spinner).await?;

        tracing::debug!("LLM review response: {}", response);

        parse_review_response(&response)
    }

    fn name(&self) -> &str {
        "ollama"
    }

    async fn validate(&self) -> Result<()> {
        // Ollama 本地部署，无需验证 API key
        Ok(())
    }
}
