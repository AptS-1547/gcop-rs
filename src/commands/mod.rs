//! Command implementations.
//!
//! Contains implementations of all gcop-rs CLI commands.
//!
//! # Modules
//! - `commit` - Commit message generation flow.
//! - `review` - Code review.
//! - `config` - Configuration management.
//! - `alias` - Git alias management.
//! - `init` - Project initialization.
//! - `stats` - Repository statistics.
//! - `hook` - Git hook management (`prepare-commit-msg`).
//! - `commit_state_machine` - Commit workflow state machine.
//! - `format` - Output format definition.
//! - `options` - Command option structs.
//! - `json` - JSON output helpers.
//!
//! # Architecture
//! ```text
//! CLI (cli.rs)
//!   ├── commands/commit.rs ─> commit_state_machine.rs
//!   ├── commands/review.rs
//!   ├── commands/config.rs
//!   ├── commands/stats.rs
//!   └── shared command options (commands/options.rs)
//! ```

/// Git alias management commands.
pub mod alias;
/// Commit generation command flow.
pub mod commit;
/// Commit workflow state machine.
pub mod commit_state_machine;
/// Configuration edit/validation commands.
pub mod config;
/// Output format types and parsing helpers.
pub mod format;
/// Git hook install/uninstall command.
pub mod hook;
/// Configuration initialization commands.
pub mod init;
/// Shared JSON output helpers.
pub mod json;
/// Shared command option structs.
pub mod options;
/// Code review command flow.
pub mod review;
/// Atomic split commit logic.
pub mod split;
/// Repository statistics command flow.
pub mod stats;

// Re-export for external use (tests, library users).
#[allow(unused_imports)]
pub use format::OutputFormat;
pub use options::{CommitOptions, ReviewOptions, StatsOptions};

use crate::git::diff::{FileDiff, split_diff_by_file};
use std::fmt::Write;

/// Filename suffixes that are typically auto-generated artifacts.
const AUTO_GENERATED_SUFFIXES: &[&str] = &[".lock", ".min.js", ".min.css"];

/// Exact auto-generated basenames.
const AUTO_GENERATED_BASENAMES: &[&str] = &["package-lock.json", "pnpm-lock.yaml", "go.sum"];

/// Substrings that usually indicate generated files.
const AUTO_GENERATED_SUBSTRINGS: &[&str] = &[".generated."];

/// Returns `true` if `filename` matches an auto-generated file pattern.
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

/// Truncates diffs at file granularity to reduce LLM token usage.
///
/// Replaces previous byte-level truncation. Every file keeps at least summary stats.
/// Important files keep full patches, while generated or over-budget files are downgraded to summary-only entries.
///
/// Returns `(formatted_diff, had_downgraded_files)`.
pub(crate) fn smart_truncate_diff(diff: &str, max_size: usize) -> (String, bool) {
    let files = split_diff_by_file(diff);

    if files.is_empty() {
        return (diff.to_string(), false);
    }

    // Fast path: total diff size is within budget.
    if diff.len() <= max_size {
        return (diff.to_string(), false);
    }

    // Classify files into auto-generated and regular files.
    let mut full_files: Vec<&FileDiff> = Vec::new();
    let mut summary_files: Vec<(&FileDiff, &str)> = Vec::new(); // (file, reason)

    // Auto-generated files are always downgraded to summary-only mode.
    let mut normal_files: Vec<&FileDiff> = Vec::new();
    for file in &files {
        if is_auto_generated(&file.filename) {
            summary_files.push((file, "auto-generated"));
        } else {
            normal_files.push(file);
        }
    }

    // Sort normal files by ascending patch size (small files are kept first).
    normal_files.sort_by_key(|f| f.content.len());

    // Greedy packing into remaining budget.
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

    // Calculate total statistics
    let total_files = files.len();
    let total_ins: usize = files.iter().map(|f| f.insertions).sum();
    let total_del: usize = files.iter().map(|f| f.deletions).sum();

    // Formatted output
    let mut output = String::new();
    let _ = writeln!(
        output,
        "Changed files ({} files, +{} -{}):\n",
        total_files, total_ins, total_del
    );

    if !full_files.is_empty() {
        let _ = writeln!(output, "## Full diff ({} files):\n", full_files.len());
        // Output full diff in original order
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
        assert!(!is_auto_generated("src/locksmith.rs")); // Contains "lock" but does not end with .lock
    }

    #[test]
    fn test_smart_truncate_no_truncation() {
        let diff = "diff --git a/src/main.rs b/src/main.rs\n\
                     --- a/src/main.rs\n\
                     +++ b/src/main.rs\n\
                     +hello";
        // budget is big enough
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
        // The budget is enough to fit everything, but smart truncation is triggered because the total size > max_size
        // Set a budget that’s just enough
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
        // Create a small file and a large file
        let small_diff = "diff --git a/small.rs b/small.rs\n--- a/small.rs\n+++ b/small.rs\n+x";
        let big_content = "+".repeat(500);
        let big_diff = format!(
            "diff --git a/big.rs b/big.rs\n--- a/big.rs\n+++ b/big.rs\n{}",
            big_content
        );
        let diff = format!("{}\n{}", small_diff, big_diff);

        // The budget is only enough for small files
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

        // The budget is extremely small and there is no room for both files.
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
        // Create two files, budget only enough for one
        let file_a = "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n+line1\n+line2";
        let file_b = "diff --git a/b.rs b/b.rs\n--- a/b.rs\n+++ b/b.rs\n+line3";
        let diff = format!("{}\n{}", file_a, file_b);
        // The budget is only enough for file_b (the smaller one), not enough for two
        let (result, truncated) = smart_truncate_diff(&diff, file_a.len());
        assert!(truncated);
        // The file content in full diff should be complete (not cut in half)
        if result.contains("+line1") {
            // If a.rs is in full diff, line2 must also be in
            assert!(result.contains("+line2"));
        }
        // b.rs is smaller and should be in full diff
        assert!(result.contains("## Full diff"));
        assert!(result.contains("## Summary only"));
    }
}
