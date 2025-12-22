//! SSE (Server-Sent Events) 解析模块
//!
//! 用于解析 OpenAI/Claude 等 API 的流式响应

use futures::StreamExt;
use reqwest::Response;
use tokio::sync::mpsc;

use crate::error::{GcopError, Result};
use crate::llm::StreamChunk;

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
) -> Result<()> {
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk_result) = stream.next().await {
        let chunk: bytes::Bytes = chunk_result.map_err(GcopError::Network)?;
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
                                let _ = tx.send(StreamChunk::Done).await;
                                return Ok(());
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse SSE data: {}, line: {}", e, data);
                    }
                }
            }
        }
    }

    // 流结束但没有收到 [DONE]
    let _ = tx.send(StreamChunk::Done).await;
    Ok(())
}
