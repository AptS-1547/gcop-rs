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

        // Debug 模式下输出请求内容
        tracing::debug!(
            "Claude API request: model={}, max_tokens={}, temperature={}",
            self.model,
            self.max_tokens,
            self.temperature
        );

        let response = self
            .client
            .post(&self.endpoint)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        let status = response.status();

        // 先获取文本响应
        let response_text = response.text().await?;

        // Debug 模式下输出原始响应
        tracing::debug!("Claude API response status: {}", status);
        tracing::debug!("Claude API response body: {}", response_text);

        if !status.is_success() {
            return Err(GcopError::LLM(format!(
                "Claude API error ({}): {}",
                status, response_text
            )));
        }

        // 解析 JSON
        let response_body: ClaudeResponse = serde_json::from_str(&response_text).map_err(|e| {
            GcopError::LLM(format!(
                "Failed to parse Claude response: {}. Raw response: {}",
                e, response_text
            ))
        })?;

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

        // Debug 模式下输出 prompt 长度
        tracing::debug!(
            "Commit message generation prompt length: {} chars",
            prompt.len()
        );

        let response = self.call_api(&prompt).await?;

        // Debug 模式下输出生成的消息
        tracing::debug!("Generated commit message: {}", response);

        Ok(response)
    }

    async fn review_code(&self, diff: &str, review_type: ReviewType) -> Result<ReviewResult> {
        let prompt = crate::llm::prompt::build_review_prompt(diff, &review_type);
        let response = self.call_api(&prompt).await?;

        // Debug 模式下输出 LLM 返回的原始文本
        tracing::debug!("LLM review response: {}", response);

        // 清理响应：移除可能的 markdown 代码块标记
        let cleaned_response = response
            .trim()
            .strip_prefix("```json")
            .unwrap_or(&response)
            .strip_prefix("```")
            .unwrap_or(&response)
            .strip_suffix("```")
            .unwrap_or(&response)
            .trim();

        // 解析 JSON 响应
        let result: ReviewResult = serde_json::from_str(cleaned_response).map_err(|e| {
            // 在错误中包含原始响应的前 500 字符
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
