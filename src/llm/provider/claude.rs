use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

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
    pub fn new(config: &ProviderConfig) -> Result<Self> {
        // 从配置或环境变量获取 API key
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
            .ok_or_else(|| {
                GcopError::Config(
                    "Claude API key not found. Set ANTHROPIC_API_KEY or configure in config.toml"
                        .to_string(),
                )
            })?;

        let endpoint = config
            .endpoint
            .clone()
            .unwrap_or_else(|| "https://api.anthropic.com/v1/messages".to_string());

        let model = config.model.clone();

        let max_tokens = config
            .extra
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(2000) as u32;

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
        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            messages: vec![MessagePayload {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        let response = self
            .client
            .post(&self.endpoint)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(GcopError::LLM(format!(
                "Claude API error ({}): {}",
                status, error_text
            )));
        }

        let response_body: ClaudeResponse = response.json().await?;

        let text = response_body
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
    ) -> Result<String> {
        let ctx = context.unwrap_or_else(|| CommitContext {
            files_changed: vec![],
            insertions: 0,
            deletions: 0,
            branch_name: None,
        });

        let prompt = crate::llm::prompt::build_commit_prompt(diff, &ctx);
        self.call_api(&prompt).await
    }

    async fn review_code(&self, diff: &str, review_type: ReviewType) -> Result<ReviewResult> {
        let prompt = crate::llm::prompt::build_review_prompt(diff, &review_type);
        let response = self.call_api(&prompt).await?;

        // 解析 JSON 响应
        let result: ReviewResult = serde_json::from_str(&response)
            .map_err(|e| GcopError::LLM(format!("Failed to parse review result: {}", e)))?;

        Ok(result)
    }

    fn name(&self) -> &str {
        "claude"
    }

    async fn validate(&self) -> Result<()> {
        // 简单验证：检查 API key 是否存在
        if self.api_key.is_empty() {
            return Err(GcopError::Config("API key is empty".to_string()));
        }
        Ok(())
    }
}
