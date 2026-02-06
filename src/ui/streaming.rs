//! 流式输出 UI 组件
//!
//! 用于实时显示 LLM 流式响应（类似 ChatGPT 打字效果）

use std::io::{self, Write};

use colored::Colorize;
use tokio::sync::mpsc;

use crate::error::{GcopError, Result};
use crate::llm::StreamChunk;

/// 流式文本输出器
pub struct StreamingOutput {
    buffer: String,
    colored: bool,
}

impl StreamingOutput {
    pub fn new(colored: bool) -> Self {
        Self {
            buffer: String::new(),
            colored,
        }
    }

    /// 处理流式响应，实时输出到终端
    ///
    /// 返回完整的响应文本
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
                    println!(); // 换行
                    // 显示错误提示
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
