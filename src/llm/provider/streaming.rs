//! SSE (Server-Sent Events) 解析模块
//!
//! 用于解析 OpenAI/Claude 等 API 的流式响应

use futures_util::StreamExt;
use reqwest::Response;
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::error::{GcopError, Result};
use crate::llm::StreamChunk;
use crate::ui::colors;

/// OpenAI 流式响应的 delta 结构
#[derive(Debug, serde::Deserialize)]
pub struct OpenAIDelta {
    pub choices: Vec<OpenAIDeltaChoice>,
}

#[derive(Debug, serde::Deserialize)]
pub struct OpenAIDeltaChoice {
    pub delta: OpenAIDeltaContent,
    pub finish_reason: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct OpenAIDeltaContent {
    pub content: Option<String>,
}

/// 解析 SSE 行，提取 data 内容
fn parse_sse_line(line: &str) -> Option<&str> {
    line.strip_prefix("data: ")
}

/// 处理 OpenAI 流式响应
///
/// SSE 格式:
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

        // 按行处理
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

                // 解析 JSON
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

    // 流结束但没有收到 [DONE]
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
// Claude SSE 解析
// ============================================================================

/// Claude SSE 事件类型
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ClaudeSSEEvent {
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { delta: ClaudeTextDelta },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(other)]
    Other,
}

/// Claude 文本增量
#[derive(Debug, Deserialize)]
pub struct ClaudeTextDelta {
    #[serde(rename = "type")]
    pub delta_type: String,
    #[serde(default)]
    pub text: String,
}

/// 处理 Claude 流式响应
///
/// Claude SSE 格式:
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

        // Claude SSE 使用双换行分隔事件块
        while let Some(pos) = buffer.find("\n\n") {
            let event_block = buffer[..pos].to_string();
            buffer = buffer[pos + 2..].to_string();

            // 查找 data: 行
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
                            // 忽略其他事件类型
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

    // 流结束但没有收到 message_stop
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

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_parse_sse_line() {
        assert_eq!(parse_sse_line("data: hello"), Some("hello"));
        assert_eq!(parse_sse_line("data: [DONE]"), Some("[DONE]"));

        // 不符合 "data: " 前缀的行应返回 None
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
}
