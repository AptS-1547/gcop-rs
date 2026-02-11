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
//! - [`hook`] - Git hook 管理（prepare-commit-msg）
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
//!   ├── commands/stats.rs
//!   └── commands/hook.rs
//!        └── options.rs (CommitOptions, ReviewOptions, etc.)
//! ```

pub mod alias;
pub mod commit;
pub mod commit_state_machine;
pub mod config;
pub mod format;
pub mod hook;
pub mod init;
pub mod json;
pub mod options;
pub mod review;
pub mod stats;

// Re-export for external use (tests, lib users)
#[allow(unused_imports)]
pub use format::OutputFormat;
pub use options::{CommitOptions, ReviewOptions, StatsOptions};

use crate::git::diff::{FileDiff, split_diff_by_file};
use std::fmt::Write;

/// 自动生成文件的匹配模式
const AUTO_GENERATED_SUFFIXES: &[&str] = &[".lock", ".min.js", ".min.css"];

/// 自动生成文件的精确文件名匹配（basename）
const AUTO_GENERATED_BASENAMES: &[&str] = &["package-lock.json", "pnpm-lock.yaml", "go.sum"];

/// 自动生成文件的子串匹配
const AUTO_GENERATED_SUBSTRINGS: &[&str] = &[".generated."];

/// 检查文件名是否匹配自动生成文件模式
fn is_auto_generated(filename: &str) -> bool {
    let basename = filename.rsplit('/').next().unwrap_or(filename);

    if AUTO_GENERATED_BASENAMES.contains(&basename) {
        return true;
    }
    if AUTO_GENERATED_SUFFIXES
        .iter()
        .any(|&s| filename.ends_with(s))
    {
        return true;
    }
    if AUTO_GENERATED_SUBSTRINGS
        .iter()
        .any(|&s| filename.contains(s))
    {
        return true;
    }
    false
}

/// 按文件粒度智能截断 diff，防止 LLM token 超限
///
/// 替代旧的字节级一刀切截断。所有文件至少保留统计信息，
/// 重要文件保留完整 diff，自动生成文件和超预算文件降级为仅统计。
///
/// 返回 (格式化后的 diff 文本, 是否有文件被降级)。
pub(crate) fn smart_truncate_diff(diff: &str, max_size: usize) -> (String, bool) {
    let files = split_diff_by_file(diff);

    if files.is_empty() {
        return (diff.to_string(), false);
    }

    // 快速路径：总大小在预算内且无自动生成文件需要降级
    if diff.len() <= max_size {
        return (diff.to_string(), false);
    }

    // 分类：auto-generated vs normal
    let mut full_files: Vec<&FileDiff> = Vec::new();
    let mut summary_files: Vec<(&FileDiff, &str)> = Vec::new(); // (file, reason)

    // auto-generated 直接降级
    let mut normal_files: Vec<&FileDiff> = Vec::new();
    for file in &files {
        if is_auto_generated(&file.filename) {
            summary_files.push((file, "auto-generated"));
        } else {
            normal_files.push(file);
        }
    }

    // normal 文件按 content 大小从小到大排序（小文件优先保留完整 diff）
    normal_files.sort_by_key(|f| f.content.len());

    // 贪心装箱
    let mut budget_used = 0usize;
    for file in &normal_files {
        if budget_used + file.content.len() <= max_size {
            budget_used += file.content.len();
            full_files.push(file);
        } else {
            summary_files.push((file, "budget exceeded"));
        }
    }

    let was_truncated = !summary_files.is_empty();

    // 计算总统计
    let total_files = files.len();
    let total_ins: usize = files.iter().map(|f| f.insertions).sum();
    let total_del: usize = files.iter().map(|f| f.deletions).sum();

    // 格式化输出
    let mut output = String::new();
    let _ = writeln!(
        output,
        "Changed files ({} files, +{} -{}):\n",
        total_files, total_ins, total_del
    );

    if !full_files.is_empty() {
        let _ = writeln!(output, "## Full diff ({} files):\n", full_files.len());
        // 按原始顺序输出 full diff
        for file in &files {
            if full_files.iter().any(|f| std::ptr::eq(*f, file)) {
                let _ = writeln!(output, "{}", file.content);
            }
        }
    }

    if !summary_files.is_empty() {
        let _ = writeln!(output, "\n## Summary only ({} files):", summary_files.len());
        for (file, reason) in &summary_files {
            let _ = writeln!(
                output,
                "- {} (+{} -{}) [{}]",
                file.filename, file.insertions, file.deletions, reason
            );
        }
    }

    (output, was_truncated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_auto_generated_lock_files() {
        assert!(is_auto_generated("Cargo.lock"));
        assert!(is_auto_generated("yarn.lock"));
        assert!(is_auto_generated("poetry.lock"));
        assert!(is_auto_generated("package-lock.json"));
        assert!(is_auto_generated("pnpm-lock.yaml"));
        assert!(is_auto_generated("go.sum"));
    }

    #[test]
    fn test_is_auto_generated_generated_files() {
        assert!(is_auto_generated("foo.generated.ts"));
        assert!(is_auto_generated("src/api.generated.rs"));
        assert!(is_auto_generated("bundle.min.js"));
        assert!(is_auto_generated("styles.min.css"));
    }

    #[test]
    fn test_is_auto_generated_normal_files() {
        assert!(!is_auto_generated("src/main.rs"));
        assert!(!is_auto_generated("README.md"));
        assert!(!is_auto_generated("Cargo.toml"));
        assert!(!is_auto_generated("src/locksmith.rs")); // 包含 "lock" 但不以 .lock 结尾
    }

    #[test]
    fn test_smart_truncate_no_truncation() {
        let diff = "diff --git a/src/main.rs b/src/main.rs\n\
                     --- a/src/main.rs\n\
                     +++ b/src/main.rs\n\
                     +hello";
        // 预算足够大
        let (result, truncated) = smart_truncate_diff(diff, 10000);
        assert!(!truncated);
        assert_eq!(result, diff);
    }

    #[test]
    fn test_smart_truncate_auto_generated_demoted() {
        let diff = "diff --git a/src/main.rs b/src/main.rs\n\
                     --- a/src/main.rs\n\
                     +++ b/src/main.rs\n\
                     +hello\n\
                     diff --git a/Cargo.lock b/Cargo.lock\n\
                     --- a/Cargo.lock\n\
                     +++ b/Cargo.lock\n\
                     +lots of lock content";
        // 预算足够放下所有内容，但因为总大小 > max_size 才会触发智能截断
        // 设置一个刚好不够的预算
        let (result, truncated) = smart_truncate_diff(diff, diff.len() - 1);
        assert!(truncated);
        assert!(result.contains("## Full diff"));
        assert!(result.contains("src/main.rs"));
        assert!(result.contains("## Summary only"));
        assert!(result.contains("Cargo.lock"));
        assert!(result.contains("[auto-generated]"));
    }

    #[test]
    fn test_smart_truncate_budget_overflow() {
        // 创建一个小文件和一个大文件
        let small_diff = "diff --git a/small.rs b/small.rs\n--- a/small.rs\n+++ b/small.rs\n+x";
        let big_content = "+".repeat(500);
        let big_diff = format!(
            "diff --git a/big.rs b/big.rs\n--- a/big.rs\n+++ b/big.rs\n{}",
            big_content
        );
        let diff = format!("{}\n{}", small_diff, big_diff);

        // 预算只够放小文件
        let (result, truncated) = smart_truncate_diff(&diff, small_diff.len() + 100);
        assert!(truncated);
        assert!(result.contains("## Full diff"));
        assert!(result.contains("small.rs"));
        assert!(result.contains("## Summary only"));
        assert!(result.contains("big.rs"));
        assert!(result.contains("[budget exceeded]"));
    }

    #[test]
    fn test_smart_truncate_all_files_too_large() {
        let big1 = format!(
            "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n{}",
            "+".repeat(500)
        );
        let big2 = format!(
            "diff --git a/b.rs b/b.rs\n--- a/b.rs\n+++ b/b.rs\n{}",
            "+".repeat(500)
        );
        let diff = format!("{}\n{}", big1, big2);

        // 预算极小，两个文件都放不下
        let (result, truncated) = smart_truncate_diff(&diff, 10);
        assert!(truncated);
        assert!(result.contains("## Summary only (2 files)"));
        assert!(result.contains("a.rs"));
        assert!(result.contains("b.rs"));
    }

    #[test]
    fn test_smart_truncate_empty_diff() {
        let (result, truncated) = smart_truncate_diff("", 1000);
        assert!(!truncated);
        assert_eq!(result, "");
    }

    #[test]
    fn test_smart_truncate_preserves_file_boundary() {
        // 创建两个文件，预算只够放一个
        let file_a = "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n+line1\n+line2";
        let file_b = "diff --git a/b.rs b/b.rs\n--- a/b.rs\n+++ b/b.rs\n+line3";
        let diff = format!("{}\n{}", file_a, file_b);
        // 预算只够放 file_b（较小的那个），不够放两个
        let (result, truncated) = smart_truncate_diff(&diff, file_a.len());
        assert!(truncated);
        // full diff 中的文件内容应该完整（不会被切到一半）
        if result.contains("+line1") {
            // 如果 a.rs 在 full diff 中，line2 也必须在
            assert!(result.contains("+line2"));
        }
        // b.rs 较小，应该在 full diff 中
        assert!(result.contains("## Full diff"));
        assert!(result.contains("## Summary only"));
    }
}
