use colored::Colorize;
use dialoguer::{Confirm, Input, Select};

use crate::constants::ui::MAX_FEEDBACK_LENGTH;
use crate::error::{GcopError, Result};

/// 用户对 commit message 的操作选择
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitAction {
    Accept,            // 接受当前 message
    Edit,              // 打开编辑器手动修改
    Retry,             // 重新生成
    RetryWithFeedback, // 重新生成并附带反馈
    Quit,              // 退出
}

/// 显示 commit message 选项菜单
///
/// # Arguments
/// * `_message` - 当前生成的 commit message（暂未使用）
/// * `allow_edit` - 是否允许手动编辑（由配置和 --no-edit 控制）
/// * `retry_count` - 已重试次数（用于显示提示）
///
/// # Returns
/// * `Ok(CommitAction)` - 用户选择的操作
/// * `Err(GcopError::UserCancelled)` - 用户按 Ctrl+C
pub fn commit_action_menu(
    _message: &str,
    allow_edit: bool,
    retry_count: usize,
    colored: bool,
) -> Result<CommitAction> {
    // 构建选项列表
    let mut options = Vec::new();

    if colored {
        // 彩色版本
        options.push(format!(
            "{} {}",
            "✓".green().bold(),
            "Accept - Use this commit message".green()
        ));

        if allow_edit {
            options.push(format!(
                "{} {}",
                "✎".yellow().bold(),
                "Edit - Manually edit the message".yellow()
            ));
        }

        options.push(format!(
            "{} {}",
            "↻".blue().bold(),
            "Retry - Regenerate".blue()
        ));

        options.push(format!(
            "{} {}",
            "↻+".blue().bold(),
            "Retry with feedback - Regenerate with instructions".blue()
        ));

        options.push(format!(
            "{} {}",
            "✕".red().bold(),
            "Quit - Cancel commit".red()
        ));
    } else {
        // 纯文本版本（保持原样）
        options.push("✓ Accept - Use this commit message".to_string());

        if allow_edit {
            options.push("✎ Edit - Manually edit the message".to_string());
        }

        options.push("↻ Retry - Regenerate".to_string());
        options.push("↻+ Retry with feedback - Regenerate with instructions".to_string());
        options.push("✕ Quit - Cancel commit".to_string());
    }

    // 根据重试次数调整提示文字
    let prompt = if colored {
        if retry_count == 0 {
            format!("{}", "Choose next action:".cyan().bold())
        } else {
            format!("{}", "Not satisfied? Choose again:".cyan().bold())
        }
    } else if retry_count == 0 {
        "Choose next action:".to_string()
    } else {
        "Not satisfied? Choose again:".to_string()
    };

    let selection = Select::new()
        .with_prompt(prompt)
        .items(&options)
        .default(0) // 默认选择 Accept
        .interact()
        .map_err(|_| GcopError::UserCancelled)?;

    // 映射选择到枚举（需要考虑 allow_edit 的影响）
    let action = if allow_edit {
        match selection {
            0 => CommitAction::Accept,
            1 => CommitAction::Edit,
            2 => CommitAction::Retry,
            3 => CommitAction::RetryWithFeedback,
            4 => CommitAction::Quit,
            _ => unreachable!(),
        }
    } else {
        match selection {
            0 => CommitAction::Accept,
            1 => CommitAction::Retry,
            2 => CommitAction::RetryWithFeedback,
            3 => CommitAction::Quit,
            _ => unreachable!(),
        }
    };

    Ok(action)
}

/// 获取用户对重试的反馈
///
/// # Returns
/// * `Ok(Some(String))` - 用户输入的反馈
/// * `Ok(None)` - 用户未输入或取消
/// * `Err(_)` - 发生错误
pub fn get_retry_feedback() -> Result<Option<String>> {
    println!(
        "\nProvide instructions for regeneration (e.g., \"use Chinese\", \"be more concise\", \"include more details\")"
    );

    let feedback: String = Input::new()
        .with_prompt("Instructions")
        .allow_empty(true)
        .interact_text()
        .map_err(|_| GcopError::UserCancelled)?;

    let trimmed = feedback.trim();

    // 限制长度，防止 prompt 过长
    if trimmed.len() > MAX_FEEDBACK_LENGTH {
        let truncated = &trimmed[..MAX_FEEDBACK_LENGTH];
        println!(
            "⚠ Feedback too long, truncated to {} characters",
            MAX_FEEDBACK_LENGTH
        );
        Ok(Some(truncated.to_string()))
    } else if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed.to_string()))
    }
}

/// 交互式确认提示
///
/// # Arguments
/// * `message` - 提示信息
/// * `default` - 默认值（true = Yes, false = No）
///
/// # Returns
/// * `Ok(true)` - 用户选择 Yes
/// * `Ok(false)` - 用户选择 No
/// * `Err(_)` - 发生错误
pub fn confirm(message: &str, default: bool) -> Result<bool> {
    let result = Confirm::new()
        .with_prompt(message)
        .default(default)
        .interact()?;

    Ok(result)
}
