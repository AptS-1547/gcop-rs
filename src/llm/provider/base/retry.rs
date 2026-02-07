//! HTTP 请求发送与重试逻辑
//!
//! 提供通用的 LLM API 请求发送函数，包含重试、429 限流处理和指数退避

use reqwest::Client;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::time::{Duration, SystemTime};

use crate::error::{GcopError, Result};

/// 判断错误是否应该重试（当前仅对连接失败重试）
fn is_retryable_error(error: &GcopError) -> bool {
    match error {
        // 连接失败 -> 重试（大小写不敏感）
        GcopError::Llm(msg) => msg.to_lowercase().contains("connection failed"),

        // 其他错误 -> 不重试
        _ => false,
    }
}

/// 解析 Retry-After header 值
///
/// 支持两种格式：
/// - 秒数：`120`
/// - HTTP 日期：`Wed, 21 Oct 2015 07:28:00 GMT`
///
/// 返回值：
/// - `Some(secs)`: 解析成功，返回等待秒数（日期早于当前时间时返回 0）
/// - `None`: 格式无效，无法解析
fn parse_retry_after(value: &str) -> Option<u64> {
    // 先尝试解析为秒数
    if let Ok(secs) = value.parse::<u64>() {
        return Some(secs);
    }

    // 再尝试解析为 HTTP 日期
    if let Ok(date) = httpdate::parse_http_date(value) {
        let now = SystemTime::now();
        // 如果日期早于当前时间，返回 0（立即重试）
        return Some(date.duration_since(now).map(|d| d.as_secs()).unwrap_or(0));
    }

    None
}

/// 尝试发送一次 HTTP 请求（只处理网络层错误）
async fn try_send_request<Req: Serialize>(
    client: &Client,
    endpoint: &str,
    headers: &[(&str, &str)],
    request_body: &Req,
    provider_name: &str,
) -> Result<reqwest::Response> {
    let mut req = client
        .post(endpoint)
        .header("Content-Type", "application/json");

    for (key, value) in headers {
        req = req.header(*key, *value);
    }

    tracing::debug!("Sending request to: {}", endpoint);

    req.json(request_body).send().await.map_err(|e| {
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
            GcopError::Llm(
                rust_i18n::t!(
                    "provider.api_request_timeout",
                    provider = provider_name,
                    detail = error_details.as_str()
                )
                .to_string(),
            )
        } else if e.is_connect() {
            GcopError::Llm(
                rust_i18n::t!(
                    "provider.api_connection_failed",
                    provider = provider_name,
                    detail = error_details.as_str()
                )
                .to_string(),
            )
        } else {
            GcopError::Network(e)
        }
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
/// * `max_retry_delay_ms` - 最大重试延迟（毫秒）
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
    max_retry_delay_ms: u64,
) -> Result<Resp>
where
    Req: Serialize,
    Resp: DeserializeOwned,
{
    let mut attempt = 0;

    loop {
        attempt += 1;

        // 尝试发送请求
        let response =
            match try_send_request(client, endpoint, headers, request_body, provider_name).await {
                Ok(resp) => resp,
                Err(e) => {
                    // 网络错误：判断是否应该重试
                    if !is_retryable_error(&e) || attempt > max_retries {
                        return Err(e);
                    }

                    // 更新 spinner 显示重试进度
                    if let Some(s) = spinner {
                        s.append_suffix(&rust_i18n::t!(
                            "provider.retrying_suffix",
                            attempt = attempt,
                            max = max_retries
                        ));
                    }

                    // 网络错误使用指数退避
                    let delay =
                        calculate_exponential_backoff(attempt, retry_delay_ms, max_retry_delay_ms);
                    tracing::debug!(
                        "{} API network error (attempt {}/{}): {}. Retrying in {:.1}s...",
                        provider_name,
                        attempt,
                        max_retries + 1,
                        e,
                        delay.as_secs_f64()
                    );
                    tokio::time::sleep(delay).await;
                    continue;
                }
            };

        let status = response.status();

        // 429 限流：解析 Retry-After 并重试
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| {
                    let result = parse_retry_after(v);
                    if result.is_none() {
                        eprintln!(
                            "{}",
                            rust_i18n::t!("provider.warning.invalid_retry_after", value = v)
                        );
                    }
                    result
                });

            let body = response.text().await.unwrap_or_else(|e| {
                eprintln!(
                    "{}",
                    rust_i18n::t!(
                        "provider.warning.read_429_body_failed",
                        error = e.to_string()
                    )
                );
                format!("<body read error: {}>", e)
            });

            tracing::debug!(
                "{} API rate limited (429), Retry-After: {:?}",
                provider_name,
                retry_after
            );

            // 检查是否还有重试次数
            if attempt > max_retries {
                return Err(GcopError::LlmApi {
                    status: 429,
                    message: format!("{}: {}", provider_name, body),
                });
            }

            // 更新 spinner 显示重试进度
            if let Some(s) = spinner {
                s.append_suffix(&rust_i18n::t!(
                    "provider.retrying_suffix",
                    attempt = attempt,
                    max = max_retries
                ));
            }

            // 计算延迟：优先使用 Retry-After，否则使用指数退避
            let delay = if let Some(secs) = retry_after {
                let retry_after_ms = secs.saturating_mul(1000);
                if retry_after_ms > max_retry_delay_ms {
                    // Retry-After 超过限制，直接返回错误
                    eprintln!(
                        "{}",
                        rust_i18n::t!(
                            "provider.warning.retry_after_exceeds_max",
                            seconds = secs,
                            max_ms = max_retry_delay_ms
                        )
                    );
                    return Err(GcopError::Llm(
                        rust_i18n::t!("provider.rate_limited_exceeds_limit", seconds = secs)
                            .to_string(),
                    ));
                }
                tracing::debug!("Using Retry-After header: {} seconds", secs);
                Duration::from_secs(secs)
            } else {
                calculate_exponential_backoff(attempt, retry_delay_ms, max_retry_delay_ms)
            };

            tracing::debug!(
                "{} API rate limited (attempt {}/{}). Retrying in {:.1}s...",
                provider_name,
                attempt,
                max_retries + 1,
                delay.as_secs_f64()
            );
            tokio::time::sleep(delay).await;
            continue;
        }

        // 读取响应 body
        let response_text = response.text().await?;

        tracing::debug!("{} API response status: {}", provider_name, status);
        tracing::debug!("{} API response body: {}", provider_name, response_text);

        // 其他错误状态码
        if !status.is_success() {
            return Err(GcopError::LlmApi {
                status: status.as_u16(),
                message: format!("{}: {}", provider_name, response_text),
            });
        }

        // 成功：解析 JSON
        if attempt > 1 {
            tracing::debug!(
                "{} API request succeeded after {} attempts",
                provider_name,
                attempt
            );
        }

        return serde_json::from_str(&response_text).map_err(|e| {
            GcopError::Llm(
                rust_i18n::t!(
                    "provider.parse_response_failed",
                    provider = provider_name,
                    error = e.to_string(),
                    response = response_text.as_str()
                )
                .to_string(),
            )
        });
    }
}

/// 计算指数退避延迟
fn calculate_exponential_backoff(
    attempt: usize,
    retry_delay_ms: u64,
    max_retry_delay_ms: u64,
) -> Duration {
    const MIN_RETRY_DELAY_MS: u64 = 100;
    let multiplier = 1u64.checked_shl((attempt - 1) as u32).unwrap_or(u64::MAX);
    let delay_ms = retry_delay_ms
        .saturating_mul(multiplier)
        .min(max_retry_delay_ms)
        .max(MIN_RETRY_DELAY_MS);
    Duration::from_millis(delay_ms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::GcopError;

    // === is_retryable_error 测试 ===

    #[test]
    fn test_is_retryable_connection_failed() {
        let err = GcopError::Llm("connection failed: timeout".to_string());
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

    #[test]
    fn test_is_retryable_mixed_case() {
        // 测试各种大小写变体都能匹配
        let cases = vec![
            "Connection Failed",
            "CONNECTION FAILED",
            "connection failed",
            "API connection failed: timeout",
        ];

        for msg in cases {
            let err = GcopError::Llm(msg.to_string());
            assert!(is_retryable_error(&err), "Should retry for: {}", msg);
        }
    }
}
