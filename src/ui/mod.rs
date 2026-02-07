//! 用户界面工具
//!
//! 提供终端交互相关的工具函数和组件。
//!
//! # 模块
//! - [`colors`] - 彩色输出工具
//! - [`editor`] - 外部编辑器集成
//! - [`prompt`] - 交互式 prompt（确认、选择菜单等）
//! - [`spinner`] - 加载动画
//! - [`streaming`] - 流式文本输出（打字机效果）
//!
//! # 示例
//! ```ignore
//! use gcop_rs::ui::{Spinner, success};
//!
//! // 显示 spinner
//! let spinner = Spinner::new("Generating commit message...", true);
//! spinner.set_message("Almost done...");
//! // spinner 在 drop 时自动清理
//!
//! // 彩色输出
//! success("Commit successful!", true);
//! ```

pub mod colors;
pub mod editor;
pub mod prompt;
pub mod spinner;
pub mod streaming;

pub use colors::*;
pub use editor::*;
pub use prompt::{CommitAction, commit_action_menu, confirm, get_retry_feedback};
pub use spinner::*;
pub use streaming::*;
