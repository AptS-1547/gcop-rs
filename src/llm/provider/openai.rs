use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::base::{
    build_endpoint, default_temperature, extract_api_key, extract_extra_f32_or, extract_extra_u32,
    parse_review_response, send_llm_request,
};
use super::utils::{DEFAULT_OPENAI_BASE, OPENAI_API_SUFFIX};
use crate::config::ProviderConfig;
use crate::error::{GcopError, Result};
use crate::llm::{CommitContext, LLMProvider, ReviewResult, ReviewType};

/// OpenAI API Provider
pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    endpoint: String,
    model: String,
    max_tokens: Option<u32>,
    temperature: f32,
}

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<MessagePayload>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
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
    pub fn new(config: &ProviderConfig, _provider_name: &str) -> Result<Self> {
        let api_key = extract_api_key(config, "OPENAI_API_KEY", "OpenAI")?;
        let endpoint = build_endpoint(config, DEFAULT_OPENAI_BASE, OPENAI_API_SUFFIX);
        let model = config.model.clone();
        let max_tokens = extract_extra_u32(config, "max_tokens");
        let temperature = extract_extra_f32_or(config, "temperature", default_temperature());

        Ok(Self {
            client: super::create_http_client()?,
            api_key,
            endpoint,
            model,
            max_tokens,
            temperature,
        })
    }

    async fn call_api(&self, prompt: &str, spinner: Option<&crate::ui::Spinner>) -> Result<String> {
        let request = OpenAIRequest {
            model: self.model.clone(),
            messages: vec![MessagePayload {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            temperature: self.temperature,
            max_tokens: self.max_tokens,
        };

        tracing::debug!(
            "OpenAI API request: model={}, temperature={}, max_tokens={:?}",
            self.model,
            self.temperature,
            self.max_tokens
        );

        let auth_header = format!("Bearer {}", self.api_key);
        let response: OpenAIResponse = send_llm_request(
            &self.client,
            &self.endpoint,
            &[("Authorization", auth_header.as_str())],
            &request,
            "OpenAI",
            spinner,
        )
        .await?;

        response
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .ok_or_else(|| GcopError::Llm("OpenAI response contains no choices".to_string()))
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
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
        "openai"
    }

    async fn validate(&self) -> Result<()> {
        if self.api_key.is_empty() {
            return Err(GcopError::Config("API key is empty".to_string()));
        }
        Ok(())
    }
}
