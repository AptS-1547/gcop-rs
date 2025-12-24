use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::base::{
    build_endpoint, extract_api_key, get_max_tokens, get_temperature, parse_review_response,
    send_llm_request,
};
use super::streaming::process_claude_stream;
use super::utils::{CLAUDE_API_SUFFIX, DEFAULT_CLAUDE_BASE};
use crate::config::{NetworkConfig, ProviderConfig};
use crate::error::{GcopError, Result};
use crate::llm::{CommitContext, LLMProvider, ReviewResult, ReviewType, StreamHandle};

/// Claude API Provider
pub struct ClaudeProvider {
    client: Client,
    api_key: String,
    endpoint: String,
    model: String,
    max_tokens: u32,
    temperature: f32,
    max_retries: usize,
    retry_delay_ms: u64,
    max_retry_delay_ms: u64,
}

#[derive(Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    temperature: f32,
    messages: Vec<MessagePayload>,
}

#[derive(Serialize)]
struct ClaudeStreamRequest {
    model: String,
    max_tokens: u32,
    temperature: f32,
    messages: Vec<MessagePayload>,
    stream: bool,
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
    pub fn new(
        config: &ProviderConfig,
        _provider_name: &str,
        network_config: &NetworkConfig,
    ) -> Result<Self> {
        let api_key = extract_api_key(config, "ANTHROPIC_API_KEY", "Claude")?;
        let endpoint = build_endpoint(config, DEFAULT_CLAUDE_BASE, CLAUDE_API_SUFFIX);
        let model = config.model.clone();
        let max_tokens = get_max_tokens(config);
        let temperature = get_temperature(config);

        Ok(Self {
            client: super::create_http_client(network_config)?,
            api_key,
            endpoint,
            model,
            max_tokens,
            temperature,
            max_retries: network_config.max_retries,
            retry_delay_ms: network_config.retry_delay_ms,
            max_retry_delay_ms: network_config.max_retry_delay_ms,
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
            self.max_retries,
            self.retry_delay_ms,
            self.max_retry_delay_ms,
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

    /// 流式 API 调用
    async fn call_api_streaming(&self, prompt: &str) -> Result<StreamHandle> {
        let (tx, rx) = mpsc::channel(64);

        let request = ClaudeStreamRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            messages: vec![MessagePayload {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            stream: true,
        };

        tracing::debug!(
            "Claude Streaming API request: model={}, max_tokens={}, temperature={}",
            self.model,
            self.max_tokens,
            self.temperature
        );

        let response = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request)
            .send()
            .await
            .map_err(GcopError::Network)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(GcopError::Llm(format!(
                "Claude API error ({}): {}",
                status, body
            )));
        }

        // 在后台任务中处理流
        tokio::spawn(async move {
            if let Err(e) = process_claude_stream(response, tx).await {
                tracing::error!("Claude stream processing error: {}", e);
            }
        });

        Ok(StreamHandle { receiver: rx })
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

    fn supports_streaming(&self) -> bool {
        true
    }

    async fn generate_commit_message_streaming(
        &self,
        diff: &str,
        context: Option<CommitContext>,
    ) -> Result<StreamHandle> {
        let ctx = context.unwrap_or_default();
        let prompt =
            crate::llm::prompt::build_commit_prompt(diff, &ctx, ctx.custom_prompt.as_deref());

        tracing::debug!("Claude streaming prompt ({} chars)", prompt.len());

        self.call_api_streaming(&prompt).await
    }
}
