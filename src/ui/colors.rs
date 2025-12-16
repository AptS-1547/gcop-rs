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
    let files_str = if stats.files_changed.len() == 1 {
        "1 file"
    } else {
        &format!("{} files", stats.files_changed.len())
    };

    let insertions_str = if stats.insertions == 1 {
        "1 insertion(+)"
    } else {
        &format!("{} insertions(+)", stats.insertions)
    };

    let deletions_str = if stats.deletions == 1 {
        "1 deletion(-)"
    } else {
        &format!("{} deletions(-)", stats.deletions)
    };

    if colored {
        format!(
            "{} changed, {}, {}",
            files_str.bold(),
            insertions_str.green(),
            deletions_str.red()
        )
    } else {
        format!(
            "{} changed, {}, {}",
            files_str, insertions_str, deletions_str
        )
    }
}
