//! Streaming output UI components
//!
//! Used to display LLM streaming responses in real time (similar to ChatGPT typing effect)

use std::io::{self, Write};

use colored::Colorize;
use tokio::sync::mpsc;

use crate::error::{GcopError, Result};
use crate::llm::StreamChunk;

/// Streaming text output
pub struct StreamingOutput {
    buffer: String,
    colored: bool,
}

impl StreamingOutput {
    /// Creates a streaming renderer with optional colored output.
    pub fn new(colored: bool) -> Self {
        Self {
            buffer: String::new(),
            colored,
        }
    }

    /// Process streaming responses and output to the terminal in real time
    ///
    /// Return the complete response text
    pub async fn process(&mut self, mut receiver: mpsc::Receiver<StreamChunk>) -> Result<String> {
        while let Some(chunk) = receiver.recv().await {
            match chunk {
                StreamChunk::Delta(text) => {
                    self.buffer.push_str(&text);
                    if self.colored {
                        print!("{}", text.yellow());
                    } else {
                        print!("{}", text);
                    }
                    io::stdout().flush().ok();
                }
                StreamChunk::Done => {
                    break;
                }
                StreamChunk::Error(e) => {
                    println!(); // newline
                    // Show error message
                    if self.colored {
                        eprintln!(
                            "{} {}",
                            "✗".red(),
                            rust_i18n::t!("stream.error", error = e.as_str()).red()
                        );
                    } else {
                        eprintln!("✗ {}", rust_i18n::t!("stream.error", error = e.as_str()));
                    }
                    return Err(GcopError::Llm(e));
                }
            }
        }

        println!();
        Ok(self.buffer.clone())
    }
}
