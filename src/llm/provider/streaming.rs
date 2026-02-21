//! SSE (Server-Sent Events) parsing module
//!
//! Used to parse streaming responses from APIs such as OpenAI/Claude/Gemini

use futures_util::StreamExt;
use reqwest::Response;
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::error::{GcopError, Result};
use crate::llm::StreamChunk;
use crate::ui::colors;

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

/// Parse SSE lines and extract data content
fn parse_sse_line(line: &str) -> Option<&str> {
    line.strip_prefix("data: ")
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

// ============================================================================
// Claude SSE Analysis
// ============================================================================

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

// ============================================================================
// Gemini SSE Analysis
// ============================================================================

/// Gemini streaming response block
#[derive(Debug, Deserialize)]
struct GeminiStreamChunk {
    pub candidates: Option<Vec<GeminiStreamCandidate>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiStreamCandidate {
    pub content: Option<GeminiStreamContent>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiStreamContent {
    pub parts: Option<Vec<GeminiStreamPart>>,
}

#[derive(Debug, Deserialize)]
struct GeminiStreamPart {
    pub text: Option<String>,
}

/// Handling Gemini streaming responses
///
/// Gemini SSE format (use `?alt=sse`):
/// ```text
/// data: {"candidates":[{"content":{"parts":[{"text":"Hello"}],"role":"model"}}]}
///
/// data: {"candidates":[{"content":{"parts":[{"text":" world"}],"role":"model"},"finishReason":"STOP"}]}
/// ```
pub async fn process_gemini_stream(
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
                match serde_json::from_str::<GeminiStreamChunk>(data) {
                    Ok(chunk) => {
                        if let Some(candidates) = &chunk.candidates
                            && let Some(candidate) = candidates.first()
                        {
                            // Extract text
                            if let Some(content) = &candidate.content
                                && let Some(parts) = &content.parts
                            {
                                for part in parts {
                                    if let Some(text) = &part.text
                                        && !text.is_empty()
                                    {
                                        let _ = tx.send(StreamChunk::Delta(text.clone())).await;
                                    }
                                }
                            }

                            // Check if it is finished (any finishReason indicates the end of the stream)
                            if let Some(reason) = &candidate.finish_reason {
                                if reason != "STOP" && reason != "MAX_TOKENS" {
                                    // SAFETY / RECITATION / OTHER: return Err, consistent with
                                    // non-streaming path (gemini.rs:234-239)
                                    tracing::warn!(
                                        "Gemini stream ended with non-STOP reason: {}",
                                        reason
                                    );
                                    return Err(GcopError::LlmContentBlocked {
                                        provider: "Gemini".to_string(),
                                        reason: reason.clone(),
                                    });
                                }
                                if reason == "MAX_TOKENS" {
                                    tracing::warn!("Gemini stream truncated (MAX_TOKENS)");
                                    colors::warning(
                                        &rust_i18n::t!(
                                            "provider.stream.gemini_finish_reason_warning",
                                            reason = reason.as_str()
                                        ),
                                        colored,
                                    );
                                }
                                if parse_errors > 0 {
                                    colors::warning(
                                        &rust_i18n::t!(
                                            "provider.stream.gemini_parse_errors",
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
                        tracing::warn!("Failed to parse Gemini SSE data: {}, line: {}", e, data);
                    }
                }
            }
        }
    }

    // The stream ended without receiving finishReason: STOP
    if parse_errors > 0 {
        colors::warning(
            &rust_i18n::t!("provider.stream.gemini_parse_errors", count = parse_errors),
            colored,
        );
    }
    let _ = tx.send(StreamChunk::Done).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    /// Build a fake 200 OK streaming response from a raw SSE body string.
    fn sse_response(body: &str) -> Response {
        http::Response::builder()
            .status(200)
            .body(bytes::Bytes::from(body.to_string()))
            .unwrap()
            .into()
    }

    /// Drain all chunks from the receiver after the process function returns.
    /// Safe to call because tx is moved into the process function and dropped on return.
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

    // =========================================================================
    // parse_sse_line
    // =========================================================================

    #[test]
    fn test_parse_sse_line() {
        assert_eq!(parse_sse_line("data: hello"), Some("hello"));
        assert_eq!(parse_sse_line("data: [DONE]"), Some("[DONE]"));

        // Rows that do not match the "data: " prefix should return None
        assert_eq!(parse_sse_line("event: message_start"), None);
        assert_eq!(parse_sse_line("data:").is_some(), false);
    }

    // =========================================================================
    // SSE structure deserialization
    // =========================================================================

    #[test]
    fn test_openai_delta_parse() {
        let json = r#"{"choices":[{"delta":{"content":"Hello"},"finish_reason":null}]}"#;
        let delta: OpenAIDelta = serde_json::from_str(json).unwrap();
        assert_eq!(delta.choices.len(), 1);
        assert_eq!(delta.choices[0].delta.content.as_deref(), Some("Hello"));
        assert_eq!(delta.choices[0].finish_reason, None);
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

    #[test]
    fn test_gemini_stream_chunk_parse() {
        let json = r#"{"candidates":[{"content":{"parts":[{"text":"Hello"}],"role":"model"}}]}"#;
        let chunk: GeminiStreamChunk = serde_json::from_str(json).unwrap();
        let candidates = chunk.candidates.unwrap();
        assert_eq!(candidates.len(), 1);
        let text = candidates[0]
            .content
            .as_ref()
            .unwrap()
            .parts
            .as_ref()
            .unwrap()[0]
            .text
            .as_deref();
        assert_eq!(text, Some("Hello"));
        assert_eq!(candidates[0].finish_reason, None);
    }

    #[test]
    fn test_gemini_stream_chunk_with_finish_reason() {
        let json = r#"{"candidates":[{"content":{"parts":[{"text":"!"}],"role":"model"},"finishReason":"STOP"}]}"#;
        let chunk: GeminiStreamChunk = serde_json::from_str(json).unwrap();
        let candidates = chunk.candidates.unwrap();
        assert_eq!(candidates[0].finish_reason.as_deref(), Some("STOP"));
    }

    #[test]
    fn test_gemini_stream_chunk_with_safety_finish_reason() {
        let json = r#"{"candidates":[{"finishReason":"SAFETY"}]}"#;
        let chunk: GeminiStreamChunk = serde_json::from_str(json).unwrap();
        let candidates = chunk.candidates.unwrap();
        assert_eq!(candidates[0].finish_reason.as_deref(), Some("SAFETY"));
        assert!(candidates[0].content.is_none());
    }

    #[test]
    fn test_gemini_stream_chunk_with_max_tokens_finish_reason() {
        let json = r#"{"candidates":[{"content":{"parts":[{"text":"partial"}],"role":"model"},"finishReason":"MAX_TOKENS"}]}"#;
        let chunk: GeminiStreamChunk = serde_json::from_str(json).unwrap();
        let candidates = chunk.candidates.unwrap();
        assert_eq!(candidates[0].finish_reason.as_deref(), Some("MAX_TOKENS"));
        let text = candidates[0]
            .content
            .as_ref()
            .unwrap()
            .parts
            .as_ref()
            .unwrap()[0]
            .text
            .as_deref();
        assert_eq!(text, Some("partial"));
    }

    // =========================================================================
    // process_claude_stream — full stream processing
    // =========================================================================

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

    // =========================================================================
    // process_openai_stream — full stream processing
    // =========================================================================

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

    // =========================================================================
    // process_gemini_stream — full stream processing
    // =========================================================================

    #[tokio::test]
    async fn test_gemini_normal_stop() {
        let body = concat!(
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"Hello\"}],\"role\":\"model\"}}]}\n",
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"!\"}],\"role\":\"model\"},\"finishReason\":\"STOP\"}]}\n",
        );
        let (tx, rx) = mpsc::channel(16);
        let result = process_gemini_stream(sse_response(body), tx, false).await;

        assert!(result.is_ok());
        let chunks = drain(rx).await;
        assert_eq!(chunks.len(), 3);
        assert_eq!(delta_text(&chunks[0]), "Hello");
        assert_eq!(delta_text(&chunks[1]), "!");
        assert_done(&chunks[2]);
    }

    #[tokio::test]
    async fn test_gemini_content_blocked_safety() {
        let body = "data: {\"candidates\":[{\"finishReason\":\"SAFETY\"}]}\n";
        let (tx, rx) = mpsc::channel(16);
        let result = process_gemini_stream(sse_response(body), tx, false).await;

        match result {
            Err(GcopError::LlmContentBlocked { provider, reason }) => {
                assert_eq!(provider, "Gemini");
                assert_eq!(reason, "SAFETY");
            }
            other => panic!("Expected LlmContentBlocked(SAFETY), got {:?}", other),
        }
        // No Done chunk should have been sent
        let chunks = drain(rx).await;
        assert!(chunks.is_empty());
    }

    #[tokio::test]
    async fn test_gemini_content_blocked_recitation() {
        let body = "data: {\"candidates\":[{\"finishReason\":\"RECITATION\"}]}\n";
        let (tx, _rx) = mpsc::channel(16);
        let result = process_gemini_stream(sse_response(body), tx, false).await;

        assert!(
            matches!(result, Err(GcopError::LlmContentBlocked { ref reason, .. }) if reason == "RECITATION"),
        );
    }

    /// MAX_TOKENS: partial output is sent, then Done. Returns Ok (not an error).
    #[tokio::test]
    async fn test_gemini_max_tokens_sends_done() {
        let body = "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"partial\"}],\"role\":\"model\"},\"finishReason\":\"MAX_TOKENS\"}]}\n";
        let (tx, rx) = mpsc::channel(16);
        let result = process_gemini_stream(sse_response(body), tx, false).await;

        assert!(
            result.is_ok(),
            "MAX_TOKENS should not be an error, got {:?}",
            result
        );
        let chunks = drain(rx).await;
        assert_eq!(delta_text(&chunks[0]), "partial");
        assert_done(chunks.last().unwrap());
    }

    /// Gemini stream ends without any finishReason → not treated as an error.
    /// Unlike Claude, Gemini silently sends Done.
    #[tokio::test]
    async fn test_gemini_no_finish_reason_sends_done() {
        let body = "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"incomplete\"}],\"role\":\"model\"}}]}\n";
        let (tx, rx) = mpsc::channel(16);
        let result = process_gemini_stream(sse_response(body), tx, false).await;

        assert!(
            result.is_ok(),
            "Gemini should silently recover, got {:?}",
            result
        );
        let chunks = drain(rx).await;
        assert_eq!(delta_text(&chunks[0]), "incomplete");
        assert_done(chunks.last().unwrap());
    }
}
