use futures_util::StreamExt;
use reqwest::Response;
use serde::Deserialize;
use tokio::sync::mpsc;

use super::parse_sse_line;
use crate::error::{GcopError, Result};
use crate::llm::StreamChunk;
use crate::ui::colors;

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
