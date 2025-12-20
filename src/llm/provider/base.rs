//! Provider 公共抽象和辅助函数
//!
//! 提取各 Provider 的通用逻辑，减少重复代码

use reqwest::Client;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::time::Duration;

use crate::config::ProviderConfig;
use crate::error::{GcopError, Result};
use crate::llm::ReviewResult;

use super::utils::complete_endpoint;

/// 默认 max_tokens
const DEFAULT_MAX_TOKENS: u32 = 2000;

/// 默认 temperature
const DEFAULT_TEMPERATURE: f32 = 0.3;

/// 错误预览最大长度
const ERROR_PREVIEW_LENGTH: usize = 500;

/// 判断错误是否应该重试
fn is_retryable_error(error: &GcopError) -> bool {
    match error {
        // 连接失败 -> 重试
        GcopError::Llm(msg) if msg.contains("connection failed") => true,

        // 429 限流 -> 重试
        GcopError::Llm(msg) if msg.contains("429") => true,

        // 其他错误 -> 不重试
        _ => false,
    }
}

/// 尝试发送一次 LLM API 请求（不包含重试逻辑）
async fn try_send_request<Req, Resp>(
    client: &Client,
    endpoint: &str,
    headers: &[(&str, &str)],
    request_body: &Req,
    provider_name: &str,
) -> Result<Resp>
where
    Req: Serialize,
    Resp: DeserializeOwned,
{
    let mut req = client
        .post(endpoint)
        .header("Content-Type", "application/json");

    for (key, value) in headers {
        req = req.header(*key, *value);
    }

    tracing::debug!("Sending request to: {}", endpoint);

    let response = req.json(request_body).send().await.map_err(|e| {
        let error_details = format!("{}", e);
        let mut error_type = "unknown";

        if e.is_timeout() {
            error_type = "timeout";
        } else if e.is_connect() {
            error_type = "connection failed";
        } else if e.is_request() {
            error_type = "request error";
        } else if e.is_body() {
            error_type = "body error";
        } else if e.is_decode() {
            error_type = "decode error";
        }

        tracing::debug!(
            "{} API request failed [{}]: {}",
            provider_name,
            error_type,
            error_details
        );

        // 为不同类型的网络错误提供更详细的错误信息
        if e.is_timeout() {
            GcopError::Llm(format!(
                "{} API request timeout: {}. The request took too long to complete.",
                provider_name, error_details
            ))
        } else if e.is_connect() {
            GcopError::Llm(format!(
                "{} API connection failed: {}. Check network connectivity or API endpoint.",
                provider_name, error_details
            ))
        } else {
            GcopError::Network(e)
        }
    })?;

    let status = response.status();
    let response_text = response.text().await?;

    tracing::debug!("{} API response status: {}", provider_name, status);
    tracing::debug!("{} API response body: {}", provider_name, response_text);

    if !status.is_success() {
        return Err(GcopError::Llm(format!(
            "{} API error ({}): {}",
            provider_name, status, response_text
        )));
    }

    serde_json::from_str(&response_text).map_err(|e| {
        GcopError::Llm(format!(
            "Failed to parse {} response: {}. Raw response: {}",
            provider_name, e, response_text
        ))
    })
}

/// 发送 LLM API 请求的通用函数（带重试机制）
///
/// # Arguments
/// * `client` - HTTP 客户端
/// * `endpoint` - API 端点
/// * `headers` - 额外的请求头
/// * `request_body` - 请求体
/// * `provider_name` - Provider 名称（用于日志和错误信息）
/// * `spinner` - 可选的进度 spinner（用于显示重试进度）
/// * `max_retries` - 最大重试次数
/// * `retry_delay_ms` - 初始重试延迟（毫秒）
#[allow(clippy::too_many_arguments)]
pub async fn send_llm_request<Req, Resp>(
    client: &Client,
    endpoint: &str,
    headers: &[(&str, &str)],
    request_body: &Req,
    provider_name: &str,
    spinner: Option<&crate::ui::Spinner>,
    max_retries: usize,
    retry_delay_ms: u64,
) -> Result<Resp>
where
    Req: Serialize,
    Resp: DeserializeOwned,
{
    let mut attempt = 0;

    loop {
        attempt += 1;

        // 尝试发送请求
        match try_send_request(client, endpoint, headers, request_body, provider_name).await {
            Ok(resp) => {
                if attempt > 1 {
                    tracing::info!(
                        "{} API request succeeded after {} attempts",
                        provider_name,
                        attempt
                    );
                }
                return Ok(resp);
            }
            Err(e) => {
                // 判断是否应该重试
                let should_retry = is_retryable_error(&e);

                if !should_retry {
                    tracing::debug!(
                        "{} API request failed with non-retryable error",
                        provider_name
                    );
                    return Err(e);
                }

                // 检查是否还有重试次数
                if attempt > max_retries {
                    tracing::debug!(
                        "{} API request failed after {} attempts",
                        provider_name,
                        attempt
                    );
                    return Err(e);
                }

                // 更新 spinner 显示重试进度
                if let Some(s) = spinner {
                    s.append_suffix(&format!("(Retrying {}/{})", attempt, max_retries));
                }

                // 计算指数退避延迟：1s, 2s, 4s
                let delay_ms = retry_delay_ms * (1 << (attempt - 1));
                let delay = Duration::from_millis(delay_ms);

                tracing::debug!(
                    "{} API request failed (attempt {}/{}): {}. Retrying in {:.1}s...",
                    provider_name,
                    attempt,
                    max_retries + 1,
                    e,
                    delay.as_secs_f64()
                );

                tokio::time::sleep(delay).await;
            }
        }
    }
}

/// 提取 API key（配置优先，环境变量 fallback）
///
/// # Arguments
/// * `config` - Provider 配置
/// * `env_var` - 环境变量名
/// * `provider_name` - Provider 名称（用于错误提示）
pub fn extract_api_key(
    config: &ProviderConfig,
    env_var: &str,
    provider_name: &str,
) -> Result<String> {
    config
        .api_key
        .clone()
        .or_else(|| std::env::var(env_var).ok())
        .ok_or_else(|| {
            GcopError::Config(format!(
                "{} API key not found. Set api_key in config.toml or {} environment variable",
                provider_name, env_var
            ))
        })
}

/// 构建完整 endpoint
///
/// # Arguments
/// * `config` - Provider 配置
/// * `default_base` - 默认 base URL
/// * `suffix` - API 路径后缀
pub fn build_endpoint(config: &ProviderConfig, default_base: &str, suffix: &str) -> String {
    config
        .endpoint
        .as_ref()
        .map(|e| complete_endpoint(e, suffix))
        .unwrap_or_else(|| format!("{}{}", default_base, suffix))
}

/// 提取 extra 配置中的 u32 值
pub fn extract_extra_u32(config: &ProviderConfig, key: &str) -> Option<u32> {
    config
        .extra
        .get(key)
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
}

/// 提取 extra 配置中的 f32 值
pub fn extract_extra_f32(config: &ProviderConfig, key: &str) -> Option<f32> {
    config
        .extra
        .get(key)
        .and_then(|v| v.as_f64())
        .map(|v| v as f32)
}

/// 从配置中获取 max_tokens（优先显式字段，fallback 到 extra，最后使用默认值）
pub fn get_max_tokens(config: &ProviderConfig) -> u32 {
    config
        .max_tokens
        .or_else(|| extract_extra_u32(config, "max_tokens"))
        .unwrap_or(DEFAULT_MAX_TOKENS)
}

/// 从配置中获取 max_tokens（可选，用于 OpenAI 等不强制要求的场景）
pub fn get_max_tokens_optional(config: &ProviderConfig) -> Option<u32> {
    config
        .max_tokens
        .or_else(|| extract_extra_u32(config, "max_tokens"))
}

/// 从配置中获取 temperature（优先显式字段，fallback 到 extra，最后使用默认值）
pub fn get_temperature(config: &ProviderConfig) -> f32 {
    config
        .temperature
        .or_else(|| extract_extra_f32(config, "temperature"))
        .unwrap_or(DEFAULT_TEMPERATURE)
}

/// 从配置中获取 temperature（可选）
pub fn get_temperature_optional(config: &ProviderConfig) -> Option<f32> {
    config
        .temperature
        .or_else(|| extract_extra_f32(config, "temperature"))
}

/// 清理 JSON 响应（移除 markdown 代码块标记）
pub fn clean_json_response(response: &str) -> &str {
    let trimmed = response.trim();

    // 提取 { 到 } 之间的内容
    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}'))
        && start < end
    {
        return &trimmed[start..=end];
    }

    // Backup: 回退到移除 markdown 代码块标记
    let without_prefix = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```JSON"))
        .or_else(|| trimmed.strip_prefix("```"))
        .map(|s| s.trim_start()) // 移除前缀后的换行符
        .unwrap_or(trimmed);

    without_prefix
        .strip_suffix("```")
        .map(|s| s.trim_end()) // 移除后缀前的换行符
        .unwrap_or(without_prefix)
        .trim()
}

/// 截断字符串用于错误预览
pub fn truncate_for_preview(s: &str) -> String {
    if s.len() > ERROR_PREVIEW_LENGTH {
        format!("{}...", &s[..ERROR_PREVIEW_LENGTH])
    } else {
        s.to_string()
    }
}

/// 解析 review 响应 JSON
pub fn parse_review_response(response: &str) -> Result<ReviewResult> {
    let cleaned = clean_json_response(response);
    serde_json::from_str(cleaned).map_err(|e| {
        let preview = truncate_for_preview(response);
        GcopError::Llm(format!(
            "Failed to parse review result: {}. Response preview: {}",
            e, preview
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::IssueSeverity;
    use pretty_assertions::assert_eq;

    // === clean_json_response 测试 ===

    #[test]
    fn test_clean_json_plain() {
        let input = r#"{"key": "value"}"#;
        assert_eq!(clean_json_response(input), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_clean_json_markdown_lowercase() {
        let input = "```json\n{\"key\": \"value\"}\n```";
        assert_eq!(clean_json_response(input), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_clean_json_markdown_uppercase() {
        let input = "```JSON\n{\"key\": \"value\"}\n```";
        assert_eq!(clean_json_response(input), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_clean_json_markdown_no_lang() {
        let input = "```\n{\"key\": \"value\"}\n```";
        assert_eq!(clean_json_response(input), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_clean_json_with_prefix_text() {
        let input = "Here is the result:\n{\"key\": \"value\"}";
        assert_eq!(clean_json_response(input), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_clean_json_with_suffix_text() {
        let input = "{\"key\": \"value\"}\nHope this helps!";
        assert_eq!(clean_json_response(input), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_clean_json_with_both_prefix_suffix() {
        let input = "Result:\n{\"key\": \"value\"}\nDone.";
        assert_eq!(clean_json_response(input), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_clean_json_nested_braces() {
        let input = r#"{"outer": {"inner": "value"}}"#;
        assert_eq!(
            clean_json_response(input),
            r#"{"outer": {"inner": "value"}}"#
        );
    }

    #[test]
    fn test_clean_json_empty_string() {
        assert_eq!(clean_json_response(""), "");
    }

    #[test]
    fn test_clean_json_no_braces() {
        let input = "Just some text without JSON";
        assert_eq!(clean_json_response(input), "Just some text without JSON");
    }

    // === is_retryable_error 测试 ===

    #[test]
    fn test_is_retryable_connection_failed() {
        let err = GcopError::Llm("connection failed: timeout".to_string());
        assert!(is_retryable_error(&err));
    }

    #[test]
    fn test_is_retryable_429_rate_limit() {
        let err = GcopError::Llm("API error (429): Rate limit exceeded".to_string());
        assert!(is_retryable_error(&err));
    }

    #[test]
    fn test_is_retryable_other_errors() {
        let err = GcopError::Llm("API error (500): Internal server error".to_string());
        assert!(!is_retryable_error(&err));

        let err = GcopError::Config("Missing API key".to_string());
        assert!(!is_retryable_error(&err));
    }

    #[test]
    fn test_is_retryable_401_no_retry() {
        let err = GcopError::Llm("API error (401): Unauthorized".to_string());
        assert!(!is_retryable_error(&err));
    }

    // === truncate_for_preview 测试 ===

    #[test]
    fn test_truncate_short_string() {
        let short = "This is a short string";
        assert_eq!(truncate_for_preview(short), short);
    }

    #[test]
    fn test_truncate_long_string() {
        let long = "a".repeat(600);
        let result = truncate_for_preview(&long);

        assert!(result.len() < long.len());
        assert!(result.ends_with("..."));
        assert_eq!(result.len(), ERROR_PREVIEW_LENGTH + 3); // 500 + "..."
    }

    // === parse_review_response 测试 ===

    #[test]
    fn test_parse_review_valid_json() {
        let json = r#"{
            "summary": "Good code",
            "issues": [
                {
                    "severity": "warning",
                    "description": "Consider adding comments"
                }
            ],
            "suggestions": ["Add tests"]
        }"#;

        let result = parse_review_response(json).unwrap();
        assert_eq!(result.summary, "Good code");
        assert_eq!(result.issues.len(), 1);
        assert!(matches!(result.issues[0].severity, IssueSeverity::Warning));
        assert_eq!(result.suggestions.len(), 1);
    }

    #[test]
    fn test_parse_review_with_markdown() {
        let json = r#"```json
{
    "summary": "Clean code",
    "issues": [],
    "suggestions": []
}
```"#;

        let result = parse_review_response(json).unwrap();
        assert_eq!(result.summary, "Clean code");
        assert!(result.issues.is_empty());
    }

    #[test]
    fn test_parse_review_invalid_json() {
        let invalid = "This is not valid JSON";
        let result = parse_review_response(invalid);

        assert!(result.is_err());
        if let Err(GcopError::Llm(msg)) = result {
            assert!(msg.contains("Failed to parse review result"));
        }
    }

    #[test]
    fn test_parse_review_empty_issues() {
        let json = r#"{
            "summary": "Perfect!",
            "issues": [],
            "suggestions": ["Keep up the good work"]
        }"#;

        let result = parse_review_response(json).unwrap();
        assert!(result.issues.is_empty());
        assert_eq!(result.suggestions.len(), 1);
    }

    // === 额外的边界测试 ===

    #[test]
    fn test_clean_json_with_whitespace() {
        let input = "   \n  {\"key\": \"value\"}  \n   ";
        assert_eq!(clean_json_response(input), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_clean_json_complex_nested() {
        let input = r#"Here's the review:
{
    "summary": "Test",
    "issues": [{"severity": "info", "description": "ok"}],
    "suggestions": []
}
Let me know if you need more."#;

        let result = clean_json_response(input);
        // 应该能正确解析
        let parsed: serde_json::Value = serde_json::from_str(result).unwrap();
        assert_eq!(parsed["summary"], "Test");
    }

    #[test]
    fn test_parse_review_with_file_and_line() {
        let json = r#"{
            "summary": "Found issue",
            "issues": [
                {
                    "severity": "critical",
                    "description": "Memory leak",
                    "file": "main.rs",
                    "line": 42
                }
            ],
            "suggestions": []
        }"#;

        let result = parse_review_response(json).unwrap();
        assert_eq!(result.issues[0].file, Some("main.rs".to_string()));
        assert_eq!(result.issues[0].line, Some(42));
    }

    #[test]
    fn test_is_retryable_mixed_case() {
        // 确保大小写匹配
        let err = GcopError::Llm("Connection Failed".to_string());
        // 当前实现是小写匹配，所以这应该不重试
        assert!(!is_retryable_error(&err));
    }
}
