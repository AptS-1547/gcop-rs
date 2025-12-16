use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

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
        // API key 读取顺序：
        // 1. 配置文件中的 api_key（优先）
        // 2. OPENAI_API_KEY 环境变量（fallback）
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| {
                GcopError::Config(
                    "OpenAI API key not found. Set api_key in config.toml or OPENAI_API_KEY environment variable".to_string()
                )
            })?;

        let endpoint = config
            .endpoint
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1/chat/completions".to_string());

        let model = config.model.clone();

        let max_tokens = config
            .extra
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        let temperature = config
            .extra
            .get("temperature")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.3) as f32;

        Ok(Self {
            client: Client::new(),
            api_key,
            endpoint,
            model,
            max_tokens,
            temperature,
        })
    }

    async fn call_api(&self, prompt: &str) -> Result<String> {
        let request = OpenAIRequest {
            model: self.model.clone(),
            messages: vec![MessagePayload {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            temperature: self.temperature,
            max_tokens: self.max_tokens,
        };

        // Debug 日志
        tracing::debug!(
            "OpenAI API request: model={}, temperature={}, max_tokens={:?}",
            self.model,
            self.temperature,
            self.max_tokens
        );

        let response = self
            .client
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        let response_text = response.text().await?;

        tracing::debug!("OpenAI API response status: {}", status);
        tracing::debug!("OpenAI API response body: {}", response_text);

        if !status.is_success() {
            return Err(GcopError::LLM(format!(
                "OpenAI API error ({}): {}",
                status, response_text
            )));
        }

        // 解析响应
        let response_body: OpenAIResponse = serde_json::from_str(&response_text).map_err(|e| {
            GcopError::LLM(format!(
                "Failed to parse OpenAI response: {}. Raw response: {}",
                e, response_text
            ))
        })?;

        let text = response_body
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .unwrap_or_default();

        Ok(text)
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
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
        "openai"
    }

    async fn validate(&self) -> Result<()> {
        if self.api_key.is_empty() {
            return Err(GcopError::Config("API key is empty".to_string()));
        }
        Ok(())
    }
}
