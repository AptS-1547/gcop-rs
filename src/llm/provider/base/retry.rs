//! HTTP request sending and retry logic
//!
//! Provides a general LLM API request sending function, including retry, 429 current limiting processing and exponential backoff

use reqwest::Client;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::time::{Duration, SystemTime};

use crate::error::{GcopError, Result};

/// Determine whether the error should be retried
pub(crate) fn is_retryable_error(error: &GcopError) -> bool {
    matches!(
        error,
        GcopError::LlmTimeout { .. }
            | GcopError::LlmConnectionFailed { .. }
            | GcopError::LlmStreamTruncated { .. }
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
    let response = execute_with_retry(
        client,
        endpoint,
        headers,
        request_body,
        provider_name,
        progress,
        max_retries,
        retry_delay_ms,
        max_retry_delay_ms,
    )
    .await?;

    let response_text = response.text().await?;

    tracing::debug!("{} API response body: {}", provider_name, response_text);

    serde_json::from_str(&response_text).map_err(|e| {
        GcopError::Llm(
            rust_i18n::t!(
                "provider.parse_response_failed",
                provider = provider_name,
                error = e.to_string(),
                response = response_text.as_str()
            )
            .to_string(),
        )
    })
}

/// Generic function for sending LLM API streaming requests (with retry mechanism)
///
/// Like `send_llm_request` but returns the raw `reqwest::Response` on success
/// instead of parsing the body as JSON, so the caller can stream the response.
///
/// Handles the same retry cases as `send_llm_request`:
/// - Network errors (timeout, connection failure): exponential backoff
/// - 429 Too Many Requests: Retry-After header or exponential backoff
/// - Retryable server errors (408, 500, 502, 503, 504): exponential backoff
///
/// # Arguments
/// * `client` - HTTP client
/// * `endpoint` - API endpoint
/// * `headers` - additional request headers
/// * `request_body` - request body
/// * `provider_name` - Provider name (used for log and error messages)
/// * `progress` - optional progress reporter
/// * `max_retries` - Maximum number of retries
/// * `retry_delay_ms` - initial retry delay (milliseconds)
/// * `max_retry_delay_ms` - Maximum retry delay (milliseconds)
#[allow(clippy::too_many_arguments)]
pub async fn send_llm_request_streaming<Req: Serialize>(
    client: &Client,
    endpoint: &str,
    headers: &[(&str, &str)],
    request_body: &Req,
    provider_name: &str,
    progress: Option<&dyn crate::llm::ProgressReporter>,
    max_retries: usize,
    retry_delay_ms: u64,
    max_retry_delay_ms: u64,
) -> Result<reqwest::Response> {
    execute_with_retry(
        client,
        endpoint,
        headers,
        request_body,
        provider_name,
        progress,
        max_retries,
        retry_delay_ms,
        max_retry_delay_ms,
    )
    .await
}

/// Core retry loop: handles network errors, 429, and retryable 5xx.
/// Returns the successful `reqwest::Response` without reading its body.
///
/// Both `send_llm_request` and `send_llm_request_streaming` delegate here;
/// they differ only in what they do with the response on success.
#[allow(clippy::too_many_arguments)]
async fn execute_with_retry<Req: Serialize>(
    client: &Client,
    endpoint: &str,
    headers: &[(&str, &str)],
    request_body: &Req,
    provider_name: &str,
    progress: Option<&dyn crate::llm::ProgressReporter>,
    max_retries: usize,
    retry_delay_ms: u64,
    max_retry_delay_ms: u64,
) -> Result<reqwest::Response> {
    let mut attempt = 0;

    loop {
        attempt += 1;

        let response =
            match try_send_request(client, endpoint, headers, request_body, provider_name).await {
                Ok(resp) => resp,
                Err(e) => {
                    if !is_retryable_error(&e) || attempt > max_retries {
                        return Err(e);
                    }

                    if let Some(p) = progress {
                        let reason = match &e {
                            GcopError::LlmTimeout { .. } => "timeout",
                            GcopError::LlmConnectionFailed { .. } => "connection failed",
                            _ => "network error",
                        };
                        p.append_suffix(&rust_i18n::t!(
                            "provider.retrying_reason_suffix",
                            attempt = attempt,
                            max = max_retries,
                            reason = reason
                        ));
                    }

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

        // 429 rate limiting: parse Retry-After and retry
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| {
                    let result = parse_retry_after(v);
                    if result.is_none() {
                        tracing::warn!("Invalid Retry-After header value: {}", v);
                    }
                    result
                });

            let body = response.text().await.unwrap_or_else(|e| {
                tracing::warn!("Failed to read 429 response body: {}", e);
                format!("<body read error: {}>", e)
            });

            tracing::debug!(
                "{} API rate limited (429), Retry-After: {:?}",
                provider_name,
                retry_after
            );

            if attempt > max_retries {
                return Err(GcopError::LlmApi {
                    status: 429,
                    message: format!("{}: {}", provider_name, body),
                });
            }

            if let Some(p) = progress {
                p.append_suffix(&rust_i18n::t!(
                    "provider.retrying_reason_suffix",
                    attempt = attempt,
                    max = max_retries,
                    reason = "429 rate limited"
                ));
            }

            let delay = if let Some(secs) = retry_after {
                let retry_after_ms = secs.saturating_mul(1000);
                if retry_after_ms > max_retry_delay_ms {
                    tracing::warn!(
                        "Retry-After ({} seconds) exceeds max retry delay ({}ms)",
                        secs,
                        max_retry_delay_ms
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

        // Retryable server errors (5xx, 408) -- retry with exponential backoff
        if !status.is_success() && is_retryable_status(status.as_u16()) {
            let response_text = response.text().await.unwrap_or_else(|e| {
                tracing::warn!("Failed to read error response body: {}", e);
                format!("<body read error: {}>", e)
            });

            if attempt > max_retries {
                return Err(GcopError::LlmApi {
                    status: status.as_u16(),
                    message: format!("{}: {}", provider_name, response_text),
                });
            }

            if let Some(p) = progress {
                p.append_suffix(&rust_i18n::t!(
                    "provider.retrying_reason_suffix",
                    attempt = attempt,
                    max = max_retries,
                    reason = status.as_u16().to_string()
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
            let response_text = response.text().await.unwrap_or_default();
            return Err(GcopError::LlmApi {
                status: status.as_u16(),
                message: format!("{}: {}", provider_name, response_text),
            });
        }

        // Success: return raw response; caller decides how to consume the body
        if attempt > 1 {
            tracing::debug!(
                "{} API request succeeded after {} attempts",
                provider_name,
                attempt
            );
        }

        return Ok(response);
    }
}

/// Spawn a background task that consumes a streaming response with retry on truncation.
///
/// This is the stream-level retry counterpart to `execute_with_retry` (which only
/// handles HTTP-level retries).  When the stream processor returns a retryable error
/// (e.g. `LlmStreamTruncated`), this function re-sends the HTTP request and starts
/// a fresh stream, sending `StreamChunk::Retry` so the UI can clear its buffer.
///
/// # Type parameters
/// * `ProcessFut` – the async stream-processing function: `(Response, Sender, bool) -> Result<()>`
/// * `ResendFut`  – the async function that re-sends the HTTP request:  `() -> Result<Response>`
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_stream_with_retry<ProcessFn, ProcessFut, ResendFn, ResendFut>(
    initial_response: reqwest::Response,
    tx: tokio::sync::mpsc::Sender<crate::llm::StreamChunk>,
    colored: bool,
    provider_name: &'static str,
    max_retries: usize,
    retry_delay_ms: u64,
    max_retry_delay_ms: u64,
    process_stream: ProcessFn,
    resend_request: ResendFn,
) where
    ProcessFn: Fn(
            reqwest::Response,
            tokio::sync::mpsc::Sender<crate::llm::StreamChunk>,
            bool,
        ) -> ProcessFut
        + Send
        + 'static,
    ProcessFut: std::future::Future<Output = crate::error::Result<()>> + Send,
    ResendFn: Fn() -> ResendFut + Send + 'static,
    ResendFut: std::future::Future<Output = crate::error::Result<reqwest::Response>> + Send,
{
    use crate::llm::StreamChunk;

    tokio::spawn(async move {
        let mut current_response = initial_response;
        let mut stream_attempt = 0usize;

        loop {
            let error_tx = tx.clone();
            match process_stream(current_response, tx.clone(), colored).await {
                Ok(()) => return,
                Err(e) => {
                    stream_attempt += 1;
                    if !is_retryable_error(&e) || stream_attempt > max_retries {
                        crate::ui::colors::error(
                            &rust_i18n::t!(
                                "provider.stream_processing_error",
                                error = e.to_string()
                            ),
                            colored,
                        );
                        let _ = error_tx.send(StreamChunk::Error(e.to_string())).await;
                        return;
                    }

                    let delay = calculate_exponential_backoff(
                        stream_attempt,
                        retry_delay_ms,
                        max_retry_delay_ms,
                    );
                    tracing::warn!(
                        "{} stream truncated (attempt {}/{}). Retrying in {:.1}s...",
                        provider_name,
                        stream_attempt,
                        max_retries,
                        delay.as_secs_f64()
                    );
                    tokio::time::sleep(delay).await;

                    match resend_request().await {
                        Ok(resp) => {
                            let _ = tx.send(StreamChunk::Retry).await;
                            current_response = resp;
                            continue;
                        }
                        Err(retry_err) => {
                            crate::ui::colors::error(
                                &rust_i18n::t!(
                                    "provider.stream_processing_error",
                                    error = retry_err.to_string()
                                ),
                                colored,
                            );
                            let _ = error_tx
                                .send(StreamChunk::Error(retry_err.to_string()))
                                .await;
                            return;
                        }
                    }
                }
            }
        }
    });
}

/// Calculate exponential backoff delay
pub(crate) fn calculate_exponential_backoff(
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

    // === parse_retry_after tests ===

    #[test]
    fn test_parse_retry_after_integer_seconds() {
        assert_eq!(parse_retry_after("60"), Some(60));
        assert_eq!(parse_retry_after("0"), Some(0));
        assert_eq!(parse_retry_after("120"), Some(120));
    }

    #[test]
    fn test_parse_retry_after_http_date_future() {
        // Use httpdate to generate a correctly formatted future HTTP date
        let future = SystemTime::now() + Duration::from_secs(60);
        let date_str = httpdate::fmt_http_date(future);
        let result = parse_retry_after(&date_str);
        assert!(result.is_some());
        assert!(result.unwrap() > 0);
    }

    #[test]
    fn test_parse_retry_after_http_date_past() {
        // A date in the past should return Some(0) (retry immediately)
        let result = parse_retry_after("Mon, 01 Jan 2001 00:00:00 GMT");
        assert_eq!(result, Some(0));
    }

    #[test]
    fn test_parse_retry_after_invalid_returns_none() {
        assert_eq!(parse_retry_after("not-a-date"), None);
        assert_eq!(parse_retry_after(""), None);
        assert_eq!(parse_retry_after("abc"), None);
        assert_eq!(parse_retry_after("-1"), None);
    }

    // === calculate_exponential_backoff tests ===

    #[test]
    fn test_backoff_first_attempt_uses_base_delay() {
        // attempt=1: multiplier=1, so delay = retry_delay_ms
        let d = calculate_exponential_backoff(1, 500, 60_000);
        assert_eq!(d, Duration::from_millis(500));
    }

    #[test]
    fn test_backoff_doubles_each_attempt() {
        let d1 = calculate_exponential_backoff(1, 500, 60_000);
        let d2 = calculate_exponential_backoff(2, 500, 60_000);
        let d3 = calculate_exponential_backoff(3, 500, 60_000);
        assert_eq!(d1, Duration::from_millis(500));
        assert_eq!(d2, Duration::from_millis(1000));
        assert_eq!(d3, Duration::from_millis(2000));
    }

    #[test]
    fn test_backoff_capped_at_max_delay() {
        // Large attempt number should be capped at max_retry_delay_ms
        let d = calculate_exponential_backoff(20, 1000, 5_000);
        assert_eq!(d, Duration::from_millis(5_000));
    }

    #[test]
    fn test_backoff_minimum_floor_100ms() {
        // retry_delay_ms=0 should floor to MIN_RETRY_DELAY_MS (100ms)
        let d = calculate_exponential_backoff(1, 0, 60_000);
        assert_eq!(d, Duration::from_millis(100));
    }

    #[test]
    fn test_backoff_overflow_protection() {
        // Very large attempt (e.g., 100) with checked_shl overflowing to u64::MAX
        // should still be capped at max_retry_delay_ms
        let d = calculate_exponential_backoff(100, 1000, 30_000);
        assert_eq!(d, Duration::from_millis(30_000));
    }

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
    fn test_is_retryable_stream_truncated() {
        let err = GcopError::LlmStreamTruncated {
            provider: "Claude".to_string(),
            detail: "no message_stop received".to_string(),
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

    // === send_llm_request_streaming tests ===

    fn make_client() -> Client {
        crate::llm::provider::test_utils::ensure_crypto_provider();
        Client::new()
    }

    #[tokio::test]
    async fn test_streaming_200_returns_ok_response() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/stream")
            .with_status(200)
            .with_body("data: hello\n\n")
            .create_async()
            .await;

        let client = make_client();
        let endpoint = format!("{}/stream", server.url());
        let result = send_llm_request_streaming(
            &client,
            &endpoint,
            &[],
            &serde_json::json!({}),
            "Test",
            None,
            0,
            0,
            1000,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().status(), 200);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_streaming_401_returns_llm_api_error() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/stream")
            .with_status(401)
            .with_body("Unauthorized")
            .create_async()
            .await;

        let client = make_client();
        let endpoint = format!("{}/stream", server.url());
        let err = send_llm_request_streaming(
            &client,
            &endpoint,
            &[],
            &serde_json::json!({}),
            "Test",
            None,
            0,
            0,
            1000,
        )
        .await
        .unwrap_err();

        assert!(matches!(err, GcopError::LlmApi { status: 401, .. }));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_streaming_429_no_retries_returns_error() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/stream")
            .with_status(429)
            .with_body("rate limited")
            .create_async()
            .await;

        let client = make_client();
        let endpoint = format!("{}/stream", server.url());
        let err = send_llm_request_streaming(
            &client,
            &endpoint,
            &[],
            &serde_json::json!({}),
            "Test",
            None,
            0,
            0,
            1000,
        )
        .await
        .unwrap_err();

        assert!(matches!(err, GcopError::LlmApi { status: 429, .. }));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_streaming_500_no_retries_returns_error() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/stream")
            .with_status(500)
            .with_body("internal error")
            .create_async()
            .await;

        let client = make_client();
        let endpoint = format!("{}/stream", server.url());
        let err = send_llm_request_streaming(
            &client,
            &endpoint,
            &[],
            &serde_json::json!({}),
            "Test",
            None,
            0,
            0,
            1000,
        )
        .await
        .unwrap_err();

        assert!(matches!(err, GcopError::LlmApi { status: 500, .. }));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_streaming_429_retry_after_zero_then_success() {
        let mut server = mockito::Server::new_async().await;
        // FIFO: created first → matched first
        let mock_429 = server
            .mock("POST", "/stream")
            .with_status(429)
            .with_header("Retry-After", "0")
            .with_body("rate limited")
            .expect(1)
            .create_async()
            .await;
        // Created second → matched after mock_429 is exhausted
        let mock_200 = server
            .mock("POST", "/stream")
            .with_status(200)
            .with_body("ok")
            .expect(1)
            .create_async()
            .await;

        let client = make_client();
        let endpoint = format!("{}/stream", server.url());
        let result = send_llm_request_streaming(
            &client,
            &endpoint,
            &[],
            &serde_json::json!({}),
            "Test",
            None,
            1,
            0,
            60_000,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().status(), 200);
        mock_429.assert_async().await;
        mock_200.assert_async().await;
    }

    #[tokio::test]
    async fn test_streaming_500_retry_then_success() {
        let mut server = mockito::Server::new_async().await;
        // FIFO: created first → matched first
        let mock_500 = server
            .mock("POST", "/stream")
            .with_status(500)
            .with_body("error")
            .expect(1)
            .create_async()
            .await;
        // Created second → matched after mock_500 is exhausted
        let mock_200 = server
            .mock("POST", "/stream")
            .with_status(200)
            .with_body("ok")
            .expect(1)
            .create_async()
            .await;

        let client = make_client();
        let endpoint = format!("{}/stream", server.url());
        let result = send_llm_request_streaming(
            &client,
            &endpoint,
            &[],
            &serde_json::json!({}),
            "Test",
            None,
            1,
            0,
            60_000,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().status(), 200);
        mock_500.assert_async().await;
        mock_200.assert_async().await;
    }

    #[tokio::test]
    async fn test_streaming_429_retry_after_exceeds_max_delay() {
        let mut server = mockito::Server::new_async().await;
        // Retry-After: 2 = 2000ms; max_retry_delay_ms = 1000ms → exceeds limit
        let mock = server
            .mock("POST", "/stream")
            .with_status(429)
            .with_header("Retry-After", "2")
            .with_body("rate limited")
            .create_async()
            .await;

        let client = make_client();
        let endpoint = format!("{}/stream", server.url());
        let err = send_llm_request_streaming(
            &client,
            &endpoint,
            &[],
            &serde_json::json!({}),
            "Test",
            None,
            1,
            0,
            1000, // max_retry_delay_ms = 1000ms < 2000ms (Retry-After)
        )
        .await
        .unwrap_err();

        assert!(matches!(err, GcopError::Llm(_)));
        mock.assert_async().await;
    }
}
