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

    /// If the cleaned message differs from the raw streamed buffer,
    /// erase the streamed output and re-display the cleaned version.
    ///
    /// This handles the case where LLMs wrap commit messages in code fences
    /// (` ``` `). The fences get printed in real-time during streaming,
    /// but should not appear in the final displayed message.
    pub fn redisplay_if_cleaned(&self, cleaned: &str) {
        if cleaned == self.buffer {
            return;
        }

        let lines_to_erase = lines_to_erase_for(&self.buffer);

        // Erase raw output using ANSI escape sequences:
        //   \x1b[1A  = move cursor up 1 line
        //   \x1b[2K  = clear entire current line
        for _ in 0..lines_to_erase {
            print!("\x1b[1A\x1b[2K");
        }
        io::stdout().flush().ok();

        // Re-print the clean version
        if self.colored {
            println!("{}", cleaned.yellow());
        } else {
            println!("{}", cleaned);
        }
    }
}

/// Calculate how many terminal lines to erase for a raw streamed buffer.
///
/// Each `\n` in the buffer produced a visible line break, and `process()`
/// appended one more via `println!()`.
fn lines_to_erase_for(buffer: &str) -> usize {
    let newline_count = buffer.chars().filter(|&c| c == '\n').count();
    newline_count + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lines_to_erase_single_line() {
        // "feat: update" has no newlines; process() adds 1 println → erase 1 line
        assert_eq!(lines_to_erase_for("feat: update"), 1);
    }

    #[test]
    fn test_lines_to_erase_multiline() {
        // 2 newlines in content + 1 from println = 3
        assert_eq!(lines_to_erase_for("line1\nline2\nline3"), 3);
    }

    #[test]
    fn test_lines_to_erase_code_fenced() {
        // Simulates: ```\nfeat: msg\n- detail\n```
        // 3 newlines + 1 = 4
        let raw = "```\nfeat: msg\n- detail\n```";
        assert_eq!(lines_to_erase_for(raw), 4);
    }

    #[test]
    fn test_lines_to_erase_trailing_newline() {
        // "a\nb\n" has 2 newlines + 1 = 3
        assert_eq!(lines_to_erase_for("a\nb\n"), 3);
    }

    #[test]
    fn test_lines_to_erase_empty() {
        assert_eq!(lines_to_erase_for(""), 1);
    }

    #[test]
    fn test_redisplay_noop_when_unchanged() {
        let mut output = StreamingOutput::new(false);
        output.buffer = "feat: update".to_string();
        // Should not panic or produce output
        output.redisplay_if_cleaned("feat: update");
    }
}
