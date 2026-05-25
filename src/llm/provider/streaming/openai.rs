use futures_util::StreamExt;
use reqwest::Response;
use tokio::sync::mpsc;

use super::parse_sse_line;
use crate::error::{GcopError, Result};
use crate::llm::StreamChunk;
use crate::ui::colors;

const OPENAI_PROVIDER_NAME: &str = "OpenAI";

/// delta structure of OpenAI streaming response
#[derive(Debug, serde::Deserialize)]
struct OpenAIDelta {
    pub choices: Vec<OpenAIDeltaChoice>,
}

#[derive(Debug, serde::Deserialize)]
struct OpenAIDeltaChoice {
    pub delta: OpenAIDeltaContent,
    pub finish_reason: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct OpenAIDeltaContent {
    pub content: Option<String>,
}

/// Event structure of OpenAI Responses API streaming response.
#[derive(Debug, serde::Deserialize)]
struct OpenAIResponsesEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub delta: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub response: Option<OpenAIResponsesStreamResponse>,
}

#[derive(Debug, serde::Deserialize)]
struct OpenAIResponsesStreamResponse {
    #[serde(default)]
    pub error: Option<OpenAIResponsesStreamError>,
    #[serde(default)]
    pub incomplete_details: Option<OpenAIResponsesIncompleteDetails>,
}

#[derive(Debug, serde::Deserialize)]
struct OpenAIResponsesStreamError {
    #[serde(default)]
    pub code: Option<String>,
    pub message: String,
}

#[derive(Debug, serde::Deserialize)]
struct OpenAIResponsesIncompleteDetails {
    pub reason: String,
}

/// Handling OpenAI streaming responses
///
/// SSE format:
/// ```text
/// data: {"id":"...","choices":[{"delta":{"content":"Hello"}}]}
///
/// data: {"id":"...","choices":[{"delta":{"content":" world"}}]}
///
/// data: [DONE]
/// ```
pub async fn process_openai_stream(
    response: Response,
    tx: mpsc::Sender<StreamChunk>,
    colored: bool,
) -> Result<()> {
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut parse_errors = 0usize;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(GcopError::Network)?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        // Process by row
        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim().to_string();
            buffer = buffer[pos + 1..].to_string();

            if line.is_empty() {
                continue;
            }

            if let Some(data) = parse_sse_line(&line) {
                if data == "[DONE]" {
                    if parse_errors > 0 {
                        colors::warning(
                            &rust_i18n::t!(
                                "provider.stream.openai_parse_errors",
                                count = parse_errors
                            ),
                            colored,
                        );
                    }
                    let _ = tx.send(StreamChunk::Done).await;
                    return Ok(());
                }

                // Parse JSON
                match serde_json::from_str::<OpenAIDelta>(data) {
                    Ok(delta) => {
                        if let Some(choice) = delta.choices.first() {
                            if let Some(content) = &choice.delta.content
                                && !content.is_empty()
                            {
                                let _ = tx.send(StreamChunk::Delta(content.clone())).await;
                            }
                            if choice.finish_reason.is_some() {
                                if parse_errors > 0 {
                                    colors::warning(
                                        &rust_i18n::t!(
                                            "provider.stream.openai_parse_errors",
                                            count = parse_errors
                                        ),
                                        colored,
                                    );
                                }
                                let _ = tx.send(StreamChunk::Done).await;
                                return Ok(());
                            }
                        }
                    }
                    Err(e) => {
                        parse_errors += 1;
                        tracing::warn!("Failed to parse SSE data: {}, line: {}", e, data);
                    }
                }
            }
        }
    }

    // Stream ended without [DONE] received
    if parse_errors > 0 {
        // All received lines failed to parse — treat as error
        return Err(GcopError::LlmStreamTruncated {
            provider: "OpenAI".to_string(),
            detail: rust_i18n::t!("provider.stream.openai_parse_errors", count = parse_errors)
                .to_string(),
        });
    }
    let _ = tx.send(StreamChunk::Done).await;
    Ok(())
}

/// Handling OpenAI Responses API streaming responses.
///
/// Responses API emits typed events such as `response.output_text.delta`,
/// `response.output_text.done`, `response.completed`, `response.failed`, and
/// `response.incomplete`.
pub async fn process_openai_responses_stream(
    response: Response,
    tx: mpsc::Sender<StreamChunk>,
    colored: bool,
) -> Result<()> {
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut parse_errors = 0usize;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(GcopError::Network)?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim().to_string();
            buffer = buffer[pos + 1..].to_string();

            if line.is_empty() {
                continue;
            }

            if let Some(data) = parse_sse_line(&line) {
                if data == "[DONE]" {
                    warn_openai_responses_parse_errors(parse_errors, colored);
                    let _ = tx.send(StreamChunk::Done).await;
                    return Ok(());
                }

                match serde_json::from_str::<OpenAIResponsesEvent>(data) {
                    Ok(event) => match event.event_type.as_str() {
                        "response.output_text.delta" => {
                            if let Some(delta) = event.delta
                                && !delta.is_empty()
                            {
                                let _ = tx.send(StreamChunk::Delta(delta)).await;
                            }
                        }
                        "response.output_text.done"
                            // Delta events already emitted the text. `text` is a finalized copy.
                            if event.text.is_none() => {
                                tracing::debug!(
                                    "OpenAI Responses stream output_text.done without text"
                                );
                            }
                        "response.completed" => {
                            warn_openai_responses_parse_errors(parse_errors, colored);
                            let _ = tx.send(StreamChunk::Done).await;
                            return Ok(());
                        }
                        "response.failed" => {
                            return Err(openai_responses_event_error(
                                event.response,
                                "response failed",
                            ));
                        }
                        "response.incomplete" => {
                            return Err(openai_responses_incomplete_error(event.response));
                        }
                        _ => {
                            // Ignore lifecycle and non-text events.
                        }
                    },
                    Err(e) => {
                        parse_errors += 1;
                        tracing::warn!(
                            "Failed to parse OpenAI Responses SSE data: {}, line: {}",
                            e,
                            data
                        );
                    }
                }
            }
        }
    }

    if parse_errors > 0 {
        return Err(GcopError::LlmStreamTruncated {
            provider: OPENAI_PROVIDER_NAME.to_string(),
            detail: rust_i18n::t!("provider.stream.openai_parse_errors", count = parse_errors)
                .to_string(),
        });
    }

    let _ = tx.send(StreamChunk::Done).await;
    Ok(())
}

fn warn_openai_responses_parse_errors(parse_errors: usize, colored: bool) {
    if parse_errors > 0 {
        colors::warning(
            &rust_i18n::t!("provider.stream.openai_parse_errors", count = parse_errors),
            colored,
        );
    }
}

fn openai_responses_event_error(
    response: Option<OpenAIResponsesStreamResponse>,
    fallback: &str,
) -> GcopError {
    let detail = response
        .and_then(|response| response.error)
        .map(|error| {
            let code = error.code.unwrap_or_else(|| "unknown".to_string());
            format!("{}: {}", code, error.message)
        })
        .unwrap_or_else(|| fallback.to_string());

    GcopError::Llm(format!("OpenAI Responses API error: {}", detail))
}

fn openai_responses_incomplete_error(response: Option<OpenAIResponsesStreamResponse>) -> GcopError {
    let reason = response
        .and_then(|response| response.incomplete_details)
        .map(|details| details.reason)
        .unwrap_or_else(|| "unknown".to_string());

    GcopError::Llm(format!(
        "OpenAI Responses API response incomplete: {}",
        reason
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use tokio::sync::mpsc;

    use crate::error::GcopError;

    fn sse_response(body: &str) -> Response {
        http::Response::builder()
            .status(200)
            .body(bytes::Bytes::from(body.to_string()))
            .unwrap()
            .into()
    }

    async fn drain(mut rx: mpsc::Receiver<StreamChunk>) -> Vec<StreamChunk> {
        let mut out = Vec::new();
        while let Some(c) = rx.recv().await {
            out.push(c);
        }
        out
    }

    fn delta_text(chunk: &StreamChunk) -> &str {
        match chunk {
            StreamChunk::Delta(text) => text.as_str(),
            other => panic!("Expected Delta, got {:?}", other),
        }
    }

    fn assert_done(chunk: &StreamChunk) {
        assert!(
            matches!(chunk, StreamChunk::Done),
            "Expected Done, got {:?}",
            chunk
        );
    }

    #[test]
    fn test_parse_sse_line() {
        use super::super::parse_sse_line;
        assert_eq!(parse_sse_line("data: hello"), Some("hello"));
        assert_eq!(parse_sse_line("data: [DONE]"), Some("[DONE]"));

        // Rows that do not match the "data: " prefix should return None
        assert_eq!(parse_sse_line("event: message_start"), None);
        assert_eq!(parse_sse_line("data:").is_some(), false);
    }

    #[test]
    fn test_openai_delta_parse() {
        let json = r#"{"choices":[{"delta":{"content":"Hello"},"finish_reason":null}]}"#;
        let delta: OpenAIDelta = serde_json::from_str(json).unwrap();
        assert_eq!(delta.choices.len(), 1);
        assert_eq!(delta.choices[0].delta.content.as_deref(), Some("Hello"));
        assert_eq!(delta.choices[0].finish_reason, None);
    }

    #[tokio::test]
    async fn test_openai_normal_completion_with_done() {
        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"},\"finish_reason\":null}]}\n",
            "data: [DONE]\n",
        );
        let (tx, rx) = mpsc::channel(16);
        let result = process_openai_stream(sse_response(body), tx, false).await;

        assert!(result.is_ok());
        let chunks = drain(rx).await;
        assert_eq!(chunks.len(), 2);
        assert_eq!(delta_text(&chunks[0]), "Hello");
        assert_done(&chunks[1]);
    }

    #[tokio::test]
    async fn test_openai_normal_completion_via_finish_reason() {
        // finish_reason present → treated as end of stream (no [DONE] required)
        let body = "data: {\"choices\":[{\"delta\":{\"content\":\"World\"},\"finish_reason\":\"stop\"}]}\n";
        let (tx, rx) = mpsc::channel(16);
        let result = process_openai_stream(sse_response(body), tx, false).await;

        assert!(result.is_ok());
        let chunks = drain(rx).await;
        assert_eq!(chunks.len(), 2);
        assert_eq!(delta_text(&chunks[0]), "World");
        assert_done(&chunks[1]);
    }

    /// All lines fail to parse AND no [DONE] → LlmStreamTruncated.
    #[tokio::test]
    async fn test_openai_truncated_all_parse_errors() {
        let body = "data: bad-json\ndata: also-bad\n";
        let (tx, rx) = mpsc::channel(16);
        let result = process_openai_stream(sse_response(body), tx, false).await;

        assert!(
            matches!(result, Err(GcopError::LlmStreamTruncated { ref provider, .. }) if provider == "OpenAI"),
            "Expected LlmStreamTruncated, got {:?}",
            result
        );
        let chunks = drain(rx).await;
        assert!(chunks.is_empty());
    }

    /// Stream ends without [DONE] but with zero parse errors → silent recovery:
    /// sends Done and returns Ok. This is the current intentional behaviour.
    #[tokio::test]
    async fn test_openai_clean_truncation_sends_done() {
        let body =
            "data: {\"choices\":[{\"delta\":{\"content\":\"partial\"},\"finish_reason\":null}]}\n";
        let (tx, rx) = mpsc::channel(16);
        let result = process_openai_stream(sse_response(body), tx, false).await;

        assert!(
            result.is_ok(),
            "Expected Ok for clean truncation, got {:?}",
            result
        );
        let chunks = drain(rx).await;
        // Delta was emitted, then Done was sent as silent recovery
        assert_eq!(delta_text(&chunks[0]), "partial");
        assert_done(chunks.last().unwrap());
    }

    #[test]
    fn test_openai_responses_event_parse() {
        let json = r#"{"type":"response.output_text.delta","item_id":"msg_123","output_index":0,"content_index":0,"delta":"Hi","sequence_number":1}"#;
        let event: OpenAIResponsesEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type, "response.output_text.delta");
        assert_eq!(event.delta.as_deref(), Some("Hi"));
    }

    #[tokio::test]
    async fn test_openai_responses_normal_completion() {
        let body = concat!(
            "data: {\"type\":\"response.output_text.delta\",\"item_id\":\"msg_123\",\"output_index\":0,\"content_index\":0,\"delta\":\"Hello\",\"sequence_number\":1}\n",
            "data: {\"type\":\"response.output_text.delta\",\"item_id\":\"msg_123\",\"output_index\":0,\"content_index\":0,\"delta\":\" world\",\"sequence_number\":2}\n",
            "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_123\",\"status\":\"completed\"},\"sequence_number\":3}\n",
        );
        let (tx, rx) = mpsc::channel(16);
        let result = process_openai_responses_stream(sse_response(body), tx, false).await;

        assert!(result.is_ok());
        let chunks = drain(rx).await;
        assert_eq!(chunks.len(), 3);
        assert_eq!(delta_text(&chunks[0]), "Hello");
        assert_eq!(delta_text(&chunks[1]), " world");
        assert_done(&chunks[2]);
    }

    #[tokio::test]
    async fn test_openai_responses_failed_event() {
        let body = "data: {\"type\":\"response.failed\",\"response\":{\"error\":{\"code\":\"server_error\",\"message\":\"failed\"}},\"sequence_number\":1}\n";
        let (tx, rx) = mpsc::channel(16);
        let result = process_openai_responses_stream(sse_response(body), tx, false).await;

        assert!(
            matches!(result, Err(GcopError::Llm(ref message)) if message.contains("server_error: failed")),
            "Expected Llm error, got {:?}",
            result
        );
        let chunks = drain(rx).await;
        assert!(chunks.is_empty());
    }

    #[tokio::test]
    async fn test_openai_responses_incomplete_event() {
        let body = "data: {\"type\":\"response.incomplete\",\"response\":{\"incomplete_details\":{\"reason\":\"max_output_tokens\"}},\"sequence_number\":1}\n";
        let (tx, rx) = mpsc::channel(16);
        let result = process_openai_responses_stream(sse_response(body), tx, false).await;

        assert!(
            matches!(result, Err(GcopError::Llm(ref message)) if message.contains("max_output_tokens")),
            "Expected Llm error, got {:?}",
            result
        );
        let chunks = drain(rx).await;
        assert!(chunks.is_empty());
    }
}
