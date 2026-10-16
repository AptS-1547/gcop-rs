//! SSE (Server-Sent Events) parsing module
//!
//! Used to parse streaming responses from APIs such as OpenAI/Claude/Gemini

use futures_util::{Stream, StreamExt};
use reqwest::Response;
use sse_stream::SseByteStream;

use crate::error::{GcopError, Result};

pub mod claude;
pub mod gemini;
pub mod openai;

pub use claude::process_claude_stream;
pub use gemini::process_gemini_stream;
pub use openai::{process_openai_responses_stream, process_openai_stream};

/// Decode complete SSE data events while preserving HTTP transport errors.
fn sse_data(response: Response) -> impl Stream<Item = Result<String>> {
    SseByteStream::new(response.bytes_stream()).filter_map(|event| {
        std::future::ready(match event {
            Ok(event) => event.data.map(Ok),
            Err(sse_stream::Error::Body(error)) => {
                Some(Err(match error.downcast::<reqwest::Error>() {
                    Ok(error) => GcopError::Network(*error),
                    Err(error) => GcopError::Llm(error.to_string()),
                }))
            }
            Err(error) => Some(Err(GcopError::Llm(
                rust_i18n::t!("provider.stream.invalid_sse", error = error.to_string())
                    .into_owned(),
            ))),
        })
    })
}
