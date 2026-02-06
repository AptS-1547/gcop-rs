use colored::Colorize;

use crate::git::DiffStats;

/// 显示成功消息（绿色 ✓）
pub fn success(msg: &str, colored: bool) {
    if colored {
        println!("{} {}", "✓".green().bold(), msg.green());
    } else {
        println!("✓ {}", msg);
    }
}

/// 显示错误消息（红色 ✗）
pub fn error(msg: &str, colored: bool) {
    if colored {
        eprintln!("{} {}", "✗".red().bold(), msg.red());
    } else {
        eprintln!("✗ {}", msg);
    }
}

/// 显示警告消息（黄色 ⚠）
pub fn warning(msg: &str, colored: bool) {
    if colored {
        println!("{} {}", "⚠".yellow().bold(), msg.yellow());
    } else {
        println!("⚠ {}", msg);
    }
}

/// 显示信息消息（蓝色 ℹ）
pub fn info(msg: &str, colored: bool) -> String {
    if colored {
        format!("{} {}", "ℹ".blue().bold(), msg.blue())
    } else {
        format!("ℹ {}", msg)
    }
}

/// 显示步骤提示（灰色）
pub fn step(step: &str, msg: &str, colored: bool) {
    if colored {
        println!(
            "{} {}",
            format!("[{}]", step).bright_black().bold(),
            msg.bright_black()
        );
    } else {
        println!("[{}] {}", step, msg);
    }
}

/// 格式化 diff 统计信息
pub fn format_diff_stats(stats: &DiffStats, colored: bool) -> String {
    use rust_i18n::t;

    let count = stats.files_changed.len();
    let files_str = if count == 1 {
        t!("diff.files_changed", count = 1)
    } else {
        t!("diff.files_changed_plural", count = count)
    };

    let insertions_str = if stats.insertions == 1 {
        t!("diff.insertions", count = 1)
    } else {
        t!("diff.insertions_plural", count = stats.insertions)
    };

    let deletions_str = if stats.deletions == 1 {
        t!("diff.deletions", count = 1)
    } else {
        t!("diff.deletions_plural", count = stats.deletions)
    };

    if colored {
        format!(
            "{}, {}, {}",
            files_str.bold(),
            insertions_str.green(),
            deletions_str.red()
        )
    } else {
        format!(
            "{}, {}, {}",
            files_str, insertions_str, deletions_str
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === 文件数量单复数测试 ===

    #[test]
    fn test_format_diff_stats_single_file() {
        let stats = DiffStats {
            files_changed: vec!["main.rs".to_string()],
            insertions: 5,
            deletions: 3,
        };
        let result = format_diff_stats(&stats, false);
        assert!(result.contains("1 file"));
        assert!(!result.contains("1 files"));
    }

    #[test]
    fn test_format_diff_stats_multiple_files() {
        let stats = DiffStats {
            files_changed: vec!["a.rs".to_string(), "b.rs".to_string(), "c.rs".to_string()],
            insertions: 10,
            deletions: 5,
        };
        let result = format_diff_stats(&stats, false);
        assert!(result.contains("3 files"));
    }

    // === 插入/删除数量单复数测试 ===

    #[test]
    fn test_format_diff_stats_single_insertion() {
        let stats = DiffStats {
            files_changed: vec!["test.rs".to_string()],
            insertions: 1,
            deletions: 5,
        };
        let result = format_diff_stats(&stats, false);
        assert!(result.contains("1 insertion(+)"));
        assert!(!result.contains("1 insertions"));
    }

    #[test]
    fn test_format_diff_stats_single_deletion() {
        let stats = DiffStats {
            files_changed: vec!["test.rs".to_string()],
            insertions: 5,
            deletions: 1,
        };
        let result = format_diff_stats(&stats, false);
        assert!(result.contains("1 deletion(-)"));
        assert!(!result.contains("1 deletions"));
    }

    // === 边界情况：零值 ===

    #[test]
    fn test_format_diff_stats_zero_insertions() {
        let stats = DiffStats {
            files_changed: vec!["deleted.rs".to_string()],
            insertions: 0,
            deletions: 50,
        };
        let result = format_diff_stats(&stats, false);
        assert!(result.contains("0 insertions(+)"));
        assert!(result.contains("50 deletions(-)"));
    }

    #[test]
    fn test_format_diff_stats_zero_deletions() {
        let stats = DiffStats {
            files_changed: vec!["new.rs".to_string()],
            insertions: 100,
            deletions: 0,
        };
        let result = format_diff_stats(&stats, false);
        assert!(result.contains("100 insertions(+)"));
        assert!(result.contains("0 deletions(-)"));
    }

    // === 彩色输出测试 ===

    #[test]
    fn test_format_diff_stats_colored() {
        let stats = DiffStats {
            files_changed: vec!["test.rs".to_string()],
            insertions: 10,
            deletions: 5,
        };
        let colored = format_diff_stats(&stats, true);
        let plain = format_diff_stats(&stats, false);

        // 两个版本都应该包含核心信息
        assert!(colored.contains("1 file"));
        assert!(colored.contains("10 insertions(+)"));
        assert!(colored.contains("5 deletions(-)"));

        assert!(plain.contains("1 file"));
        assert!(plain.contains("10 insertions(+)"));
        assert!(plain.contains("5 deletions(-)"));
    }

    #[test]
    fn test_format_diff_stats_empty_files() {
        let stats = DiffStats {
            files_changed: vec![],
            insertions: 0,
            deletions: 0,
        };
        let result = format_diff_stats(&stats, false);
        assert!(result.contains("0 files")); // 复数形式用于0
    }
}
