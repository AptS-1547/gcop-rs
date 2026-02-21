//! SSE (Server-Sent Events) parsing module
//!
//! Used to parse streaming responses from APIs such as OpenAI/Claude/Gemini

pub mod claude;
pub mod gemini;
pub mod openai;

pub use claude::process_claude_stream;
pub use gemini::process_gemini_stream;
pub use openai::process_openai_stream;

/// Parse SSE lines and extract data content
pub(super) fn parse_sse_line(line: &str) -> Option<&str> {
    line.strip_prefix("data: ")
}
