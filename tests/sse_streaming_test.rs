use bytes::Bytes;
use gcop_rs::error::{GcopError, Result};
use gcop_rs::llm::StreamChunk;
use gcop_rs::llm::provider::streaming::{
    process_claude_stream, process_gemini_stream, process_openai_responses_stream,
    process_openai_stream,
};
use reqwest::{Body, Response};
use serde_json::{Value, json};
use tokio::sync::mpsc;

const TEXT: &str = "  你好 🐱 café  ";

#[derive(Clone, Copy, Debug)]
enum Protocol {
    Claude,
    Chat,
    Responses,
    Gemini,
}

const PROTOCOLS: [Protocol; 4] = [
    Protocol::Claude,
    Protocol::Chat,
    Protocol::Responses,
    Protocol::Gemini,
];

impl Protocol {
    fn delta(self) -> Value {
        match self {
            Self::Claude => json!({
                "type": "content_block_delta", "index": 0,
                "delta": {"type": "text_delta", "text": TEXT},
            }),
            Self::Chat => json!({
                "choices": [{"delta": {"content": TEXT}, "finish_reason": null}],
            }),
            Self::Responses => json!({"type": "response.output_text.delta", "delta": TEXT}),
            Self::Gemini => json!({"candidates": [{"content": {"parts": [{"text": TEXT}]}}]}),
        }
    }

    fn terminal(self) -> String {
        match self {
            Self::Claude => json!({"type": "message_stop"}).to_string(),
            Self::Chat => "[DONE]".to_string(),
            Self::Responses => json!({"type": "response.completed"}).to_string(),
            Self::Gemini => json!({"candidates": [{"finishReason": "STOP"}]}).to_string(),
        }
    }

    async fn process(self, response: Response) -> (Result<()>, Vec<StreamChunk>) {
        let (tx, mut rx) = mpsc::channel(16);
        let result = match self {
            Self::Claude => process_claude_stream(response, tx, false).await,
            Self::Chat => process_openai_stream(response, tx, false).await,
            Self::Responses => process_openai_responses_stream(response, tx, false).await,
            Self::Gemini => process_gemini_stream(response, tx, false).await,
        };
        let mut chunks = Vec::new();
        while let Some(chunk) = rx.recv().await {
            chunks.push(chunk);
        }
        (result, chunks)
    }
}

fn response(wire: &[u8], chunk_size: usize) -> Response {
    let chunks: Vec<std::io::Result<Bytes>> = wire
        .chunks(chunk_size)
        .map(|chunk| Ok(Bytes::copy_from_slice(chunk)))
        .collect();
    http::Response::builder()
        .status(200)
        .header("content-type", "text/event-stream")
        .body(Body::wrap_stream(futures_util::stream::iter(chunks)))
        .unwrap()
        .into()
}

fn frame(data: &str, space: &str, newline: &str) -> String {
    let mut wire = format!(
        ": keepalive{newline}event:{space}message{newline}id: 7{newline}retry: 1000{newline}"
    );
    for line in data.lines() {
        wire.push_str(&format!("data:{space}{line}{newline}"));
    }
    wire.push_str(newline);
    wire
}

fn assert_text(chunks: &[StreamChunk]) {
    assert!(matches!(&chunks[0], StreamChunk::Delta(text) if text == TEXT));
}

#[tokio::test]
async fn sse_formats_preserve_text_and_completion_for_every_protocol() {
    for protocol in PROTOCOLS {
        for space in ["", " "] {
            for newline in ["\n", "\r\n", "\r"] {
                for multiline in [false, true] {
                    let delta = if multiline {
                        serde_json::to_string_pretty(&protocol.delta()).unwrap()
                    } else {
                        protocol.delta().to_string()
                    };
                    let wire = format!(
                        "\u{feff}{}{}",
                        frame(&delta, space, newline),
                        frame(&protocol.terminal(), space, newline),
                    );
                    for chunk_size in [1, 7, 4096] {
                        let (result, chunks) = protocol
                            .process(response(wire.as_bytes(), chunk_size))
                            .await;
                        assert!(
                            result.is_ok(),
                            "{protocol:?}, space={space:?}, newline={newline:?}, multiline={multiline}, chunk_size={chunk_size}: {result:?}"
                        );
                        assert_eq!(chunks.len(), 2, "{protocol:?}: {chunks:?}");
                        assert_text(&chunks);
                        assert!(matches!(chunks[1], StreamChunk::Done));
                    }
                }
            }
        }
    }
}

#[tokio::test]
async fn sse_missing_or_unterminated_terminal_stays_truncated() {
    for protocol in PROTOCOLS {
        for terminal in [String::new(), format!("data:{}\n", protocol.terminal())] {
            let wire = format!(
                "{}{}",
                frame(&protocol.delta().to_string(), "", "\n"),
                terminal
            );
            let (result, chunks) = protocol.process(response(wire.as_bytes(), 1)).await;
            assert!(matches!(result, Err(GcopError::LlmStreamTruncated { .. })));
            assert_eq!(chunks.len(), 1, "{protocol:?}: {chunks:?}");
            assert_text(&chunks);
        }
    }
}

#[tokio::test]
async fn sse_comments_and_metadata_cannot_complete_a_stream() {
    for protocol in PROTOCOLS {
        let terminal = protocol.terminal();
        let wire = format!(
            "{}:data:{terminal}\nunknown:{terminal}\nid: {terminal}\nretry: 10\n\n",
            frame(&protocol.delta().to_string(), "", "\n"),
        );
        let (result, chunks) = protocol.process(response(wire.as_bytes(), 1)).await;
        assert!(matches!(result, Err(GcopError::LlmStreamTruncated { .. })));
        assert_eq!(chunks.len(), 1, "{protocol:?}: {chunks:?}");
        assert_text(&chunks);
    }
}

#[tokio::test]
async fn sse_invalid_utf8_fails_without_done() {
    for protocol in PROTOCOLS {
        let mut wire = frame(&protocol.delta().to_string(), "", "\n").into_bytes();
        wire.extend_from_slice(b"data:\xff\n\n");
        wire.extend_from_slice(frame(&protocol.terminal(), "", "\n").as_bytes());
        let (result, chunks) = protocol.process(response(&wire, 1)).await;
        assert!(matches!(result, Err(GcopError::Llm(message)) if message.contains("SSE")));
        assert_eq!(chunks.len(), 1, "{protocol:?}: {chunks:?}");
        assert_text(&chunks);
    }
}

#[tokio::test]
async fn sse_transport_errors_keep_the_network_error_type() {
    for protocol in PROTOCOLS {
        let chunks = vec![
            Ok(Bytes::from(frame(&protocol.delta().to_string(), "", "\n"))),
            Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionReset,
                "connection reset during SSE delivery",
            )),
        ];
        let response = http::Response::builder()
            .status(200)
            .body(Body::wrap_stream(futures_util::stream::iter(chunks)))
            .unwrap()
            .into();
        let (result, chunks) = protocol.process(response).await;
        assert!(matches!(result, Err(GcopError::Network(_))));
        assert_eq!(chunks.len(), 1, "{protocol:?}: {chunks:?}");
        assert_text(&chunks);
    }
}
