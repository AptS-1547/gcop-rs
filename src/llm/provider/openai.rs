use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::base::{
    build_commit_prompt_with_log, build_endpoint, build_review_prompt_with_log, extract_api_key,
    get_max_tokens_optional, get_temperature, process_commit_response, process_review_response,
    send_llm_request,
};
use super::streaming::process_openai_stream;
use super::utils::{DEFAULT_OPENAI_BASE, OPENAI_API_SUFFIX};
use crate::config::{NetworkConfig, ProviderConfig};
use crate::error::{GcopError, Result};
use crate::llm::{CommitContext, LLMProvider, ReviewResult, ReviewType, StreamHandle};

/// OpenAI API Provider
pub struct OpenAIProvider {
    name: String,
    client: Client,
    api_key: String,
    endpoint: String,
    model: String,
    max_tokens: Option<u32>,
    temperature: f32,
    max_retries: usize,
    retry_delay_ms: u64,
    max_retry_delay_ms: u64,
    colored: bool,
}

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<MessagePayload>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

/// 流式请求结构体
#[derive(Serialize)]
struct OpenAIStreamRequest {
    model: String,
    messages: Vec<MessagePayload>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    stream: bool,
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
    pub fn new(
        config: &ProviderConfig,
        provider_name: &str,
        network_config: &NetworkConfig,
        colored: bool,
    ) -> Result<Self> {
        let api_key = extract_api_key(config, "OPENAI_API_KEY", "OpenAI")?;
        let endpoint = build_endpoint(config, DEFAULT_OPENAI_BASE, OPENAI_API_SUFFIX);
        let model = config.model.clone();
        let max_tokens = get_max_tokens_optional(config);
        let temperature = get_temperature(config);

        Ok(Self {
            name: provider_name.to_string(),
            client: super::create_http_client(network_config)?,
            api_key,
            endpoint,
            model,
            max_tokens,
            temperature,
            max_retries: network_config.max_retries,
            retry_delay_ms: network_config.retry_delay_ms,
            max_retry_delay_ms: network_config.max_retry_delay_ms,
            colored,
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
            .ok_or_else(|| GcopError::Llm("OpenAI response contains no choices".to_string()))
    }

    /// 流式 API 调用
    async fn call_api_streaming(&self, prompt: &str) -> Result<StreamHandle> {
        let (tx, rx) = mpsc::channel(64);

        let request = OpenAIStreamRequest {
            model: self.model.clone(),
            messages: vec![MessagePayload {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            stream: true,
        };

        tracing::debug!(
            "OpenAI Streaming API request: model={}, temperature={}, max_tokens={:?}",
            self.model,
            self.temperature,
            self.max_tokens
        );

        let auth_header = format!("Bearer {}", self.api_key);

        let response = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("Authorization", &auth_header)
            .json(&request)
            .send()
            .await
            .map_err(GcopError::Network)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(GcopError::LlmApi {
                status: status.as_u16(),
                message: format!("OpenAI: {}", body),
            });
        }

        // 在后台任务中处理流
        // tx 会在任务结束时自动 drop，从而关闭 channel
        let colored = self.colored;
        tokio::spawn(async move {
            if let Err(e) = process_openai_stream(response, tx, colored).await {
                crate::ui::colors::error(&format!("Stream processing error: {}", e), colored);
            }
            // tx 在这里被 drop，channel 关闭
        });

        Ok(StreamHandle { receiver: rx })
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

        tracing::debug!("Streaming prompt ({} chars)", prompt.len());

        self.call_api_streaming(&prompt).await
    }
}
