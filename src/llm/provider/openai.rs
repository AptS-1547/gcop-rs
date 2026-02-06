use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::base::{
    build_endpoint, extract_api_key, get_max_tokens_optional, get_temperature,
    process_commit_response, process_review_response, send_llm_request,
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

    async fn call_api(
        &self,
        system: &str,
        user_message: &str,
        spinner: Option<&crate::ui::Spinner>,
    ) -> Result<String> {
        let request = OpenAIRequest {
            model: self.model.clone(),
            messages: vec![
                MessagePayload {
                    role: "system".to_string(),
                    content: system.to_string(),
                },
                MessagePayload {
                    role: "user".to_string(),
                    content: user_message.to_string(),
                },
            ],
            temperature: self.temperature,
            max_tokens: self.max_tokens,
        };

        tracing::debug!(
            "OpenAI API request: model={}, temperature={}, max_tokens={:?}, system_len={}, user_len={}",
            self.model,
            self.temperature,
            self.max_tokens,
            system.len(),
            user_message.len()
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
            .ok_or_else(|| GcopError::Llm(rust_i18n::t!("provider.openai_no_choices").to_string()))
    }

    /// 流式 API 调用
    async fn call_api_streaming(&self, system: &str, user_message: &str) -> Result<StreamHandle> {
        let (tx, rx) = mpsc::channel(64);

        let request = OpenAIStreamRequest {
            model: self.model.clone(),
            messages: vec![
                MessagePayload {
                    role: "system".to_string(),
                    content: system.to_string(),
                },
                MessagePayload {
                    role: "user".to_string(),
                    content: user_message.to_string(),
                },
            ],
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            stream: true,
        };

        tracing::debug!(
            "OpenAI Streaming API request: model={}, temperature={}, max_tokens={:?}, system_len={}, user_len={}",
            self.model,
            self.temperature,
            self.max_tokens,
            system.len(),
            user_message.len()
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
                crate::ui::colors::error(
                    &rust_i18n::t!("provider.stream_processing_error", error = e.to_string()),
                    colored,
                );
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
        let ctx = context.unwrap_or_default();
        let (system, user) =
            crate::llm::prompt::build_commit_prompt_split(diff, &ctx, ctx.custom_prompt.as_deref());
        tracing::debug!(
            "Commit prompt split - system ({} chars), user ({} chars)",
            system.len(),
            user.len()
        );
        let response = self.call_api(&system, &user, spinner).await?;
        Ok(process_commit_response(response))
    }

    async fn review_code(
        &self,
        diff: &str,
        review_type: ReviewType,
        custom_prompt: Option<&str>,
        spinner: Option<&crate::ui::Spinner>,
    ) -> Result<ReviewResult> {
        let (system, user) =
            crate::llm::prompt::build_review_prompt_split(diff, &review_type, custom_prompt);
        tracing::debug!(
            "Review prompt split - system ({} chars), user ({} chars)",
            system.len(),
            user.len()
        );
        let response = self.call_api(&system, &user, spinner).await?;
        process_review_response(&response)
    }

    fn name(&self) -> &str {
        &self.name
    }

    async fn validate(&self) -> Result<()> {
        if self.api_key.is_empty() {
            return Err(GcopError::Config(
                rust_i18n::t!("provider.api_key_empty").to_string(),
            ));
        }

        // Send minimal test request to validate API connection
        tracing::debug!("Validating OpenAI API connection...");

        let test_request = OpenAIRequest {
            model: self.model.clone(),
            messages: vec![MessagePayload {
                role: "user".to_string(),
                content: "test".to_string(),
            }],
            temperature: 1.0,
            max_tokens: Some(1), // Minimize API cost
        };

        // Direct request without retry (fast fail)
        let auth_header = format!("Bearer {}", self.api_key);
        let response = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("Authorization", &auth_header)
            .json(&test_request)
            .send()
            .await
            .map_err(GcopError::Network)?;

        // Check status code
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(GcopError::LlmApi {
                status: status.as_u16(),
                message: rust_i18n::t!(
                    "provider.api_validation_failed",
                    provider = "OpenAI",
                    body = body
                )
                .to_string(),
            });
        }

        tracing::debug!("OpenAI API connection validated successfully");
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
        let (system, user) =
            crate::llm::prompt::build_commit_prompt_split(diff, &ctx, ctx.custom_prompt.as_deref());

        tracing::debug!(
            "OpenAI streaming - system ({} chars), user ({} chars)",
            system.len(),
            user.len()
        );

        self.call_api_streaming(&system, &user).await
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
            api_key: Some("sk-test".to_string()),
            model: "gpt-4o-mini".to_string(),
            max_tokens: None,
            temperature: None,
            extra: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_openai_success_response_parsing() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"choices":[{"message":{"content":"Hello from OpenAI"}}]}"#)
            .create_async()
            .await;

        let provider = OpenAIProvider::new(
            &test_provider_config(server.url()),
            "openai",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        let result = provider.call_api("system", "hi", None).await.unwrap();
        assert_eq!(result, "Hello from OpenAI");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_openai_api_error_401() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .with_status(401)
            .with_body("Unauthorized")
            .create_async()
            .await;

        let provider = OpenAIProvider::new(
            &test_provider_config(server.url()),
            "openai",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        let err = provider.call_api("system", "hi", None).await.unwrap_err();
        assert!(matches!(err, GcopError::LlmApi { status: 401, .. }));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_openai_api_error_429() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .with_status(429)
            .with_body("Too Many Requests")
            .create_async()
            .await;

        let provider = OpenAIProvider::new(
            &test_provider_config(server.url()),
            "openai",
            &test_network_config_no_retry(),
            false,
        )
        .unwrap();

        let err = provider.call_api("system", "hi", None).await.unwrap_err();
        assert!(matches!(err, GcopError::LlmApi { status: 429, .. }));
        mock.assert_async().await;
    }
}
