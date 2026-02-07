//! 命令实现
//!
//! 包含所有 gcop-rs CLI 命令的实现。
//!
//! # 模块
//! - [`commit`] - Commit message 生成
//! - [`review`] - 代码审查
//! - [`config`] - 配置管理
//! - [`alias`] - Git alias 管理
//! - [`init`] - 项目初始化
//! - [`stats`] - 仓库统计
//! - [`commit_state_machine`] - Commit 流程状态机
//! - [`format`] - 输出格式定义
//! - [`options`] - 命令选项结构体
//! - [`json`] - JSON 输出工具
//!
//! # 架构
//! ```text
//! CLI (cli.rs)
//!   ├── commands/commit.rs ─> commit_state_machine.rs
//!   ├── commands/review.rs
//!   ├── commands/config.rs
//!   └── commands/stats.rs
//!        └── options.rs (CommitOptions, ReviewOptions, etc.)
//! ```

pub mod alias;
pub mod commit;
pub mod commit_state_machine;
pub mod config;
pub mod format;
pub mod init;
pub mod json;
pub mod options;
pub mod review;
pub mod stats;

// Re-export for external use (tests, lib users)
#[allow(unused_imports)]
pub use format::OutputFormat;
pub use options::{CommitOptions, ReviewOptions, StatsOptions};

/// 截断过大的 diff，防止 LLM token 超限
///
/// 在 char boundary 处截断，避免多字节字符 panic。
/// 返回 (截断后的 diff, 是否发生了截断)。
pub(crate) fn truncate_diff(diff: &str, max_size: usize) -> (String, bool) {
    if diff.len() <= max_size {
        return (diff.to_string(), false);
    }
    let boundary = diff
        .char_indices()
        .map(|(i, _)| i)
        .take_while(|&i| i <= max_size)
        .last()
        .unwrap_or(0);
    (
        format!(
            "{}\n\n... [truncated, {} bytes total]",
            &diff[..boundary],
            diff.len()
        ),
        true,
    )
}
