use colored::Colorize;

use crate::git::DiffStats;

/// Show success message (green ✓)
pub fn success(msg: &str, colored: bool) {
    if colored {
        println!("{} {}", "✓".green().bold(), msg.green());
    } else {
        println!("✓ {}", msg);
    }
}

/// Show error message (red ✗)
pub fn error(msg: &str, colored: bool) {
    if colored {
        eprintln!("{} {}", "✗".red().bold(), msg.red());
    } else {
        eprintln!("✗ {}", msg);
    }
}

/// Show warning message (yellow ⚠)
pub fn warning(msg: &str, colored: bool) {
    if colored {
        println!("{} {}", "⚠".yellow().bold(), msg.yellow());
    } else {
        println!("⚠ {}", msg);
    }
}

/// Show information messages (blue ℹ)
pub fn info(msg: &str, colored: bool) -> String {
    if colored {
        format!("{} {}", "ℹ".blue().bold(), msg.blue())
    } else {
        format!("ℹ {}", msg)
    }
}

/// Show step prompts (gray)
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

/// Format diff statistics
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
        format!("{}, {}, {}", files_str, insertions_str, deletions_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Number of files singular and plural test ===

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

    // === Insertion/deletion quantity singular and plural test ===

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

    // === Boundary case: zero value ===

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

    // === Color output test ===

    #[test]
    fn test_format_diff_stats_colored() {
        let stats = DiffStats {
            files_changed: vec!["test.rs".to_string()],
            insertions: 10,
            deletions: 5,
        };
        let colored = format_diff_stats(&stats, true);
        let plain = format_diff_stats(&stats, false);

        // Both versions should contain the core information
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
        assert!(result.contains("0 files")); // The plural form is used for 0
    }
}
