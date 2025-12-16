use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::utils::{DEFAULT_OLLAMA_BASE, OLLAMA_API_SUFFIX, complete_endpoint};
use crate::config::ProviderConfig;
use crate::error::{GcopError, Result};
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
        // provider_name 参数保留用于统一接口
        let endpoint = config
            .endpoint
            .as_ref()
            .map(|e| complete_endpoint(e, OLLAMA_API_SUFFIX))
            .unwrap_or_else(|| format!("{}{}", DEFAULT_OLLAMA_BASE, OLLAMA_API_SUFFIX));

        let model = config.model.clone();

        let temperature = config
            .extra
            .get("temperature")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32);

        Ok(Self {
            client: super::create_http_client()?,
            endpoint,
            model,
            temperature,
        })
    }

    async fn call_api(&self, prompt: &str) -> Result<String> {
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

        let response = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        let response_text = response.text().await?;

        tracing::debug!("Ollama API response status: {}", status);
        tracing::debug!("Ollama API response body: {}", response_text);

        if !status.is_success() {
            return Err(GcopError::LLM(format!(
                "Ollama API error ({}): {}",
                status, response_text
            )));
        }

        let response_body: OllamaResponse = serde_json::from_str(&response_text).map_err(|e| {
            GcopError::LLM(format!(
                "Failed to parse Ollama response: {}. Raw response: {}",
                e, response_text
            ))
        })?;

        Ok(response_body.response)
    }
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    async fn generate_commit_message(
        &self,
        diff: &str,
        context: Option<CommitContext>,
    ) -> Result<String> {
        let ctx = context.unwrap_or_else(|| CommitContext {
            files_changed: vec![],
            insertions: 0,
            deletions: 0,
            branch_name: None,
            custom_prompt: None,
        });

        let prompt =
            crate::llm::prompt::build_commit_prompt(diff, &ctx, ctx.custom_prompt.as_deref());

        tracing::debug!(
            "Commit message generation prompt length: {} chars",
            prompt.len()
        );

        let response = self.call_api(&prompt).await?;

        tracing::debug!("Generated commit message: {}", response);

        Ok(response)
    }

    async fn review_code(
        &self,
        diff: &str,
        review_type: ReviewType,
        custom_prompt: Option<&str>,
    ) -> Result<ReviewResult> {
        let prompt = crate::llm::prompt::build_review_prompt(diff, &review_type, custom_prompt);
        let response = self.call_api(&prompt).await?;

        tracing::debug!("LLM review response: {}", response);

        let result: ReviewResult = serde_json::from_str(&response).map_err(|e| {
            let preview = if response.len() > 500 {
                format!("{}...", &response[..500])
            } else {
                response.clone()
            };

            GcopError::LLM(format!(
                "Failed to parse review result: {}. Response preview: {}",
                e, preview
            ))
        })?;

        Ok(result)
    }

    fn name(&self) -> &str {
        "ollama"
    }

    async fn validate(&self) -> Result<()> {
        // Ollama 本地部署，无需验证 API key
        Ok(())
    }
}
