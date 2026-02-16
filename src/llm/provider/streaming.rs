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
        colors::warning(
            &rust_i18n::t!("provider.stream.openai_parse_errors", count = parse_errors),
            colored,
        );
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

    // Stream ended but message_stop was not received
    if parse_errors > 0 {
        colors::warning(
            &rust_i18n::t!(
                "provider.stream.claude_ended_with_errors",
                count = parse_errors
            ),
            colored,
        );
    } else {
        colors::warning(
            &rust_i18n::t!("provider.stream.claude_ended_without_stop"),
            colored,
        );
    }
    let _ = tx.send(StreamChunk::Done).await;
    Ok(())
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
                                if reason != "STOP" {
                                    tracing::warn!(
                                        "Gemini stream ended with non-STOP reason: {}",
                                        reason
                                    );
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

    #[test]
    fn test_parse_sse_line() {
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
}
