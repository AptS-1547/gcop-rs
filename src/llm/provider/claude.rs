use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::base::{
    build_endpoint, default_max_tokens, default_temperature, extract_api_key, extract_extra_f32_or,
    extract_extra_u32_or, parse_review_response, send_llm_request,
};
use super::utils::{CLAUDE_API_SUFFIX, DEFAULT_CLAUDE_BASE};
use crate::config::ProviderConfig;
use crate::error::{GcopError, Result};
use crate::llm::{CommitContext, LLMProvider, ReviewResult, ReviewType};

/// Claude API Provider
pub struct ClaudeProvider {
    client: Client,
    api_key: String,
    endpoint: String,
    model: String,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    temperature: f32,
    messages: Vec<MessagePayload>,
}

#[derive(Serialize, Deserialize)]
struct MessagePayload {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

impl ClaudeProvider {
    pub fn new(config: &ProviderConfig, _provider_name: &str) -> Result<Self> {
        let api_key = extract_api_key(config, "ANTHROPIC_API_KEY", "Claude")?;
        let endpoint = build_endpoint(config, DEFAULT_CLAUDE_BASE, CLAUDE_API_SUFFIX);
        let model = config.model.clone();
        let max_tokens = extract_extra_u32_or(config, "max_tokens", default_max_tokens());
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
        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            messages: vec![MessagePayload {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        tracing::debug!(
            "Claude API request: model={}, max_tokens={}, temperature={}",
            self.model,
            self.max_tokens,
            self.temperature
        );

        let response: ClaudeResponse = send_llm_request(
            &self.client,
            &self.endpoint,
            &[
                ("x-api-key", self.api_key.as_str()),
                ("anthropic-version", "2023-06-01"),
            ],
            &request,
            "Claude",
            spinner,
        )
        .await?;

        let text = response
            .content
            .into_iter()
            .filter(|block| block.content_type == "text")
            .map(|block| block.text)
            .collect::<Vec<_>>()
            .join("\n");

        Ok(text)
    }
}

#[async_trait]
impl LLMProvider for ClaudeProvider {
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
        "claude"
    }

    async fn validate(&self) -> Result<()> {
        if self.api_key.is_empty() {
            return Err(GcopError::Config("API key is empty".to_string()));
        }
        Ok(())
    }
}
