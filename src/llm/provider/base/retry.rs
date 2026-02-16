//! HTTP request sending and retry logic
//!
//! Provides a general LLM API request sending function, including retry, 429 current limiting processing and exponential backoff

use reqwest::Client;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::time::{Duration, SystemTime};

use crate::error::{GcopError, Result};

/// Determine whether the error should be retried
fn is_retryable_error(error: &GcopError) -> bool {
    matches!(
        error,
        GcopError::LlmTimeout { .. }
            | GcopError::LlmConnectionFailed { .. }
            | GcopError::Network(_)
    )
}

/// Determine whether an HTTP status code should trigger a retry.
///
/// Retryable: 408, 500, 502, 503, 504
/// Note: 429 is handled separately with Retry-After header support.
fn is_retryable_status(status: u16) -> bool {
    matches!(status, 408 | 500 | 502 | 503 | 504)
}

/// Parse Retry-After header value
///
/// Two formats are supported:
/// - Number of seconds: `120`
/// - HTTP Date: `Wed, 21 Oct 2015 07:28:00 GMT`
///
/// Return value:
/// - `Some(secs)`: parsed successfully, returns the number of seconds to wait (returns 0 if the date is earlier than the current time)
/// - `None`: The format is invalid and cannot be parsed
fn parse_retry_after(value: &str) -> Option<u64> {
    // First try parsing into seconds
    if let Ok(secs) = value.parse::<u64>() {
        return Some(secs);
    }

    // Try parsing to HTTP date again
    if let Ok(date) = httpdate::parse_http_date(value) {
        let now = SystemTime::now();
        // If the date is before the current time, return 0 (retry immediately)
        return Some(date.duration_since(now).map(|d| d.as_secs()).unwrap_or(0));
    }

    None
}

/// Attempt to send an HTTP request (only handles network layer errors)
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

        // Map network errors to structured error types
        if e.is_timeout() {
            GcopError::LlmTimeout {
                provider: provider_name.to_string(),
                detail: error_details,
            }
        } else if e.is_connect() {
            GcopError::LlmConnectionFailed {
                provider: provider_name.to_string(),
                detail: error_details,
            }
        } else {
            GcopError::Network(e)
        }
    })
}

/// Generic function for sending LLM API requests (with retry mechanism)
///
/// # Arguments
/// * `client` - HTTP client
/// * `endpoint` - API endpoint
/// * `headers` - additional request headers
/// * `request_body` - request body
/// * `provider_name` - Provider name (used for log and error messages)
/// * `spinner` - optional progress reporter (used to show retry progress)
/// * `max_retries` - Maximum number of retries
/// * `retry_delay_ms` - initial retry delay (milliseconds)
/// * `max_retry_delay_ms` - Maximum retry delay (milliseconds)
#[allow(clippy::too_many_arguments)]
pub async fn send_llm_request<Req, Resp>(
    client: &Client,
    endpoint: &str,
    headers: &[(&str, &str)],
    request_body: &Req,
    provider_name: &str,
    progress: Option<&dyn crate::llm::ProgressReporter>,
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

        // Try sending a request
        let response =
            match try_send_request(client, endpoint, headers, request_body, provider_name).await {
                Ok(resp) => resp,
                Err(e) => {
                    // Network error: Determine whether you should try again
                    if !is_retryable_error(&e) || attempt > max_retries {
                        return Err(e);
                    }

                    // Update spinner to show retry progress
                    if let Some(p) = progress {
                        p.append_suffix(&rust_i18n::t!(
                            "provider.retrying_suffix",
                            attempt = attempt,
                            max = max_retries
                        ));
                    }

                    // Network errors using exponential backoff
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

        // 429 Current limiting: parse Retry-After and try again
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

            // Check if there are still retries
            if attempt > max_retries {
                return Err(GcopError::LlmApi {
                    status: 429,
                    message: format!("{}: {}", provider_name, body),
                });
            }

            // Update spinner to show retry progress
            if let Some(p) = progress {
                p.append_suffix(&rust_i18n::t!(
                    "provider.retrying_suffix",
                    attempt = attempt,
                    max = max_retries
                ));
            }

            // Calculate delay: use Retry-After first, otherwise use exponential backoff
            let delay = if let Some(secs) = retry_after {
                let retry_after_ms = secs.saturating_mul(1000);
                if retry_after_ms > max_retry_delay_ms {
                    // Retry-After exceeds the limit and returns an error directly.
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

        // Read response body
        let response_text = response.text().await?;

        tracing::debug!("{} API response status: {}", provider_name, status);
        tracing::debug!("{} API response body: {}", provider_name, response_text);

        // Retryable server errors (5xx, 408) -- retry with exponential backoff
        if !status.is_success() && is_retryable_status(status.as_u16()) {
            if attempt > max_retries {
                return Err(GcopError::LlmApi {
                    status: status.as_u16(),
                    message: format!("{}: {}", provider_name, response_text),
                });
            }

            if let Some(p) = progress {
                p.append_suffix(&rust_i18n::t!(
                    "provider.retrying_suffix",
                    attempt = attempt,
                    max = max_retries
                ));
            }

            let delay = calculate_exponential_backoff(attempt, retry_delay_ms, max_retry_delay_ms);
            tracing::debug!(
                "{} API server error {} (attempt {}/{}). Retrying in {:.1}s...",
                provider_name,
                status.as_u16(),
                attempt,
                max_retries + 1,
                delay.as_secs_f64()
            );
            tokio::time::sleep(delay).await;
            continue;
        }

        // Non-retryable error status codes (4xx except 408/429)
        if !status.is_success() {
            return Err(GcopError::LlmApi {
                status: status.as_u16(),
                message: format!("{}: {}", provider_name, response_text),
            });
        }

        // Success: parsed JSON
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

/// Calculate exponential backoff delay
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

    // === is_retryable_error tests ===

    #[test]
    fn test_is_retryable_timeout() {
        let err = GcopError::LlmTimeout {
            provider: "OpenAI".to_string(),
            detail: "read timed out".to_string(),
        };
        assert!(is_retryable_error(&err));
    }

    #[test]
    fn test_is_retryable_connection_failed() {
        let err = GcopError::LlmConnectionFailed {
            provider: "Claude".to_string(),
            detail: "DNS resolution error".to_string(),
        };
        assert!(is_retryable_error(&err));
    }

    #[test]
    fn test_is_retryable_llm_not_retryable() {
        let err = GcopError::Llm("API error: no candidates".to_string());
        assert!(!is_retryable_error(&err));
    }

    #[test]
    fn test_is_retryable_config_not_retryable() {
        let err = GcopError::Config("Missing API key".to_string());
        assert!(!is_retryable_error(&err));
    }

    #[test]
    fn test_is_retryable_llm_api_not_retryable() {
        // LlmApi errors are handled by is_retryable_status, not is_retryable_error
        let err = GcopError::LlmApi {
            status: 500,
            message: "Internal Server Error".to_string(),
        };
        assert!(!is_retryable_error(&err));
    }

    // === is_retryable_status tests ===

    #[test]
    fn test_retryable_status_5xx() {
        assert!(is_retryable_status(500));
        assert!(is_retryable_status(502));
        assert!(is_retryable_status(503));
        assert!(is_retryable_status(504));
    }

    #[test]
    fn test_retryable_status_408() {
        assert!(is_retryable_status(408));
    }

    #[test]
    fn test_non_retryable_status_429() {
        // 429 is handled separately with Retry-After support
        assert!(!is_retryable_status(429));
    }

    #[test]
    fn test_non_retryable_status_4xx() {
        assert!(!is_retryable_status(400));
        assert!(!is_retryable_status(401));
        assert!(!is_retryable_status(403));
        assert!(!is_retryable_status(404));
        assert!(!is_retryable_status(422));
    }

    #[test]
    fn test_non_retryable_status_501() {
        // 501 Not Implemented -- not transient
        assert!(!is_retryable_status(501));
    }

    #[test]
    fn test_non_retryable_status_2xx() {
        assert!(!is_retryable_status(200));
        assert!(!is_retryable_status(201));
    }
}
