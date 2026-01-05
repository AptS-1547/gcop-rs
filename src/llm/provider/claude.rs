use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::base::{
    build_commit_prompt_with_log, build_endpoint, build_review_prompt_with_log, extract_api_key,
    get_max_tokens, get_temperature, process_commit_response, process_review_response,
    send_llm_request,
};
use super::streaming::process_claude_stream;
use super::utils::{CLAUDE_API_SUFFIX, DEFAULT_CLAUDE_BASE};
use crate::config::{NetworkConfig, ProviderConfig};
use crate::error::{GcopError, Result};
use crate::llm::{CommitContext, LLMProvider, ReviewResult, ReviewType, StreamHandle};

/// Claude API Provider
pub struct ClaudeProvider {
    name: String,
    client: Client,
    api_key: String,
    endpoint: String,
    model: String,
    max_tokens: u32,
    temperature: f32,
    max_retries: usize,
    retry_delay_ms: u64,
    max_retry_delay_ms: u64,
    colored: bool,
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
        provider_name: &str,
        network_config: &NetworkConfig,
        colored: bool,
    ) -> Result<Self> {
        let api_key = extract_api_key(config, "ANTHROPIC_API_KEY", "Claude")?;
        let endpoint = build_endpoint(config, DEFAULT_CLAUDE_BASE, CLAUDE_API_SUFFIX);
        let model = config.model.clone();
        let max_tokens = get_max_tokens(config);
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
            return Err(GcopError::LlmApi {
                status: status.as_u16(),
                message: format!("Claude: {}", body),
            });
        }

        // 在后台任务中处理流
        let colored = self.colored;
        tokio::spawn(async move {
            if let Err(e) = process_claude_stream(response, tx, colored).await {
                crate::ui::colors::error(
                    &format!("Claude stream processing error: {}", e),
                    colored,
                );
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

        tracing::debug!("Claude streaming prompt ({} chars)", prompt.len());

        self.call_api_streaming(&prompt).await
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
            api_key: Some("sk-ant-test".to_string()),
            model: "claude-3-haiku-20240307".to_string(),
            max_tokens: None,
            temperature: None,
            extra: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_claude_success_response_parsing() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/messages")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"content":[{"type":"text","text":"Hello"},{"type":"text","text":"Claude"}]}"#,
            )
            .create_async()
            .await;

        let provider = ClaudeProvider::new(
            &test_provider_config(server.url()),
            "claude",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        let result = provider.call_api("hi", None).await.unwrap();
        assert_eq!(result, "Hello\nClaude");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_claude_api_error_401() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/messages")
            .with_status(401)
            .with_body("Unauthorized")
            .create_async()
            .await;

        let provider = ClaudeProvider::new(
            &test_provider_config(server.url()),
            "claude",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        let err = provider.call_api("hi", None).await.unwrap_err();
        assert!(matches!(err, GcopError::LlmApi { status: 401, .. }));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_claude_api_error_429() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/messages")
            .with_status(429)
            .with_body("Too Many Requests")
            .create_async()
            .await;

        let provider = ClaudeProvider::new(
            &test_provider_config(server.url()),
            "claude",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        let err = provider.call_api("hi", None).await.unwrap_err();
        assert!(matches!(err, GcopError::LlmApi { status: 429, .. }));
        mock.assert_async().await;
    }
}
