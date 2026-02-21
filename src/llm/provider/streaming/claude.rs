use futures_util::StreamExt;
use reqwest::Response;
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::error::{GcopError, Result};
use crate::llm::StreamChunk;
use crate::ui::colors;

/// Claude SSE event type
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClaudeSSEEvent {
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { delta: ClaudeTextDelta },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(other)]
    Other,
}

/// Claude text increment
#[derive(Debug, Deserialize)]
struct ClaudeTextDelta {
    #[serde(rename = "type")]
    pub delta_type: String,
    #[serde(default)]
    pub text: String,
}

/// Handling Claude streaming responses
///
/// Claude SSE format:
/// ```text
/// event: message_start
/// data: {"type":"message_start","message":{"id":"..."}}
///
/// event: content_block_delta
/// data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}
///
/// event: message_stop
/// data: {"type":"message_stop"}
/// ```
pub async fn process_claude_stream(
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

        // Claude SSE uses double newlines to delimit event blocks
        while let Some(pos) = buffer.find("\n\n") {
            let event_block = buffer[..pos].to_string();
            buffer = buffer[pos + 2..].to_string();

            // Find data: rows
            for line in event_block.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    match serde_json::from_str::<ClaudeSSEEvent>(data) {
                        Ok(ClaudeSSEEvent::ContentBlockDelta { delta }) => {
                            if delta.delta_type == "text_delta" && !delta.text.is_empty() {
                                let _ = tx.send(StreamChunk::Delta(delta.text)).await;
                            }
                        }
                        Ok(ClaudeSSEEvent::MessageStop) => {
                            if parse_errors > 0 {
                                colors::warning(
                                    &rust_i18n::t!(
                                        "provider.stream.claude_parse_errors",
                                        count = parse_errors
                                    ),
                                    colored,
                                );
                            }
                            let _ = tx.send(StreamChunk::Done).await;
                            return Ok(());
                        }
                        Ok(ClaudeSSEEvent::Other) => {
                            // Ignore other event types
                        }
                        Err(e) => {
                            parse_errors += 1;
                            tracing::warn!(
                                "Failed to parse Claude SSE data: {}, line: {}",
                                e,
                                data
                            );
                        }
                    }
                }
            }
        }
    }

    // Stream ended but message_stop was not received — treat as error
    let detail = if parse_errors > 0 {
        rust_i18n::t!(
            "provider.stream.claude_ended_with_errors",
            count = parse_errors
        )
        .to_string()
    } else {
        rust_i18n::t!("provider.stream.claude_ended_without_stop").to_string()
    };
    Err(GcopError::LlmStreamTruncated {
        provider: "Claude".to_string(),
        detail,
    })
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
    fn test_claude_sse_event_parse() {
        let delta_json =
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi"}}"#;
        let event: ClaudeSSEEvent = serde_json::from_str(delta_json).unwrap();
        match event {
            ClaudeSSEEvent::ContentBlockDelta { delta } => {
                assert_eq!(delta.delta_type, "text_delta");
                assert_eq!(delta.text, "Hi");
            }
            _ => panic!("unexpected event: {:?}", event),
        }

        let stop_json = r#"{"type":"message_stop"}"#;
        let event: ClaudeSSEEvent = serde_json::from_str(stop_json).unwrap();
        assert!(matches!(event, ClaudeSSEEvent::MessageStop));
    }

    #[tokio::test]
    async fn test_claude_normal_completion() {
        let body = concat!(
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}\n\n",
            "data: {\"type\":\"message_stop\"}\n\n",
        );
        let (tx, rx) = mpsc::channel(16);
        let result = process_claude_stream(sse_response(body), tx, false).await;

        assert!(result.is_ok());
        let chunks = drain(rx).await;
        assert_eq!(chunks.len(), 2);
        assert_eq!(delta_text(&chunks[0]), "Hello");
        assert_done(&chunks[1]);
    }

    #[tokio::test]
    async fn test_claude_multiple_deltas_then_stop() {
        let body = concat!(
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}\n\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\" world\"}}\n\n",
            "data: {\"type\":\"message_stop\"}\n\n",
        );
        let (tx, rx) = mpsc::channel(16);
        let result = process_claude_stream(sse_response(body), tx, false).await;

        assert!(result.is_ok());
        let chunks = drain(rx).await;
        assert_eq!(chunks.len(), 3);
        assert_eq!(delta_text(&chunks[0]), "Hello");
        assert_eq!(delta_text(&chunks[1]), " world");
        assert_done(&chunks[2]);
    }

    /// Stream ends after valid deltas but WITHOUT message_stop → LlmStreamTruncated.
    #[tokio::test]
    async fn test_claude_truncated_without_stop() {
        let body = "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"partial\"}}\n\n";
        let (tx, rx) = mpsc::channel(16);
        let result = process_claude_stream(sse_response(body), tx, false).await;

        assert!(
            matches!(result, Err(GcopError::LlmStreamTruncated { ref provider, .. }) if provider == "Claude"),
            "Expected LlmStreamTruncated, got {:?}",
            result
        );
        // Delta was delivered before the stream ended
        let chunks = drain(rx).await;
        assert_eq!(chunks.len(), 1);
        assert_eq!(delta_text(&chunks[0]), "partial");
    }

    /// Completely empty body → LlmStreamTruncated (no message_stop, no content).
    #[tokio::test]
    async fn test_claude_empty_stream_truncated() {
        let (tx, rx) = mpsc::channel(16);
        let result = process_claude_stream(sse_response(""), tx, false).await;

        assert!(
            matches!(result, Err(GcopError::LlmStreamTruncated { ref provider, .. }) if provider == "Claude"),
        );
        let chunks = drain(rx).await;
        assert!(chunks.is_empty());
    }

    /// Stream with only unparseable JSON (all parse errors) + no message_stop →
    /// LlmStreamTruncated whose detail mentions errors.
    #[tokio::test]
    async fn test_claude_truncated_all_parse_errors() {
        let body = "data: not-valid-json\n\ndata: also-broken\n\n";
        let (tx, rx) = mpsc::channel(16);
        let result = process_claude_stream(sse_response(body), tx, false).await;

        match result {
            Err(GcopError::LlmStreamTruncated { provider, detail }) => {
                assert_eq!(provider, "Claude");
                // detail should reference the error count, not the "ended_without_stop" key
                assert!(!detail.is_empty());
            }
            other => panic!("Expected LlmStreamTruncated, got {:?}", other),
        }
        let chunks = drain(rx).await;
        assert!(
            chunks.is_empty(),
            "No deltas expected from all-error stream"
        );
    }
}
