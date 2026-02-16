use colored::Colorize;
use dialoguer::{Confirm, Input, Select};

use crate::error::{GcopError, Result};

/// Maximum length of user feedback
const MAX_FEEDBACK_LENGTH: usize = 200;

/// User's operation selection for commit message
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitAction {
    /// Accept the current generated message.
    Accept,
    /// Open the editor and manually modify the message.
    Edit,
    /// Regenerate without additional feedback.
    Retry,
    /// Regenerate and include user feedback.
    RetryWithFeedback,
    /// Exit without committing.
    Quit,
}

/// Show commit message options menu
///
/// # Arguments
/// * `_message` - currently generated commit message (not used yet)
/// * `allow_edit` - whether manual editing is allowed (controlled by configuration and --no-edit)
/// * `retry_count` - number of retries (used to display prompts)
///
/// # Returns
/// * `Ok(CommitAction)` - the action selected by the user
/// * `Err(GcopError::UserCancelled)` - user pressed Ctrl+C
pub fn commit_action_menu(
    _message: &str,
    allow_edit: bool,
    retry_count: usize,
    colored: bool,
) -> Result<CommitAction> {
    use rust_i18n::t;

    // Build options list
    let mut options = Vec::new();

    if colored {
        // Color version
        options.push(format!(
            "{} {}",
            "✓".green().bold(),
            t!("commit.menu.actions.accept").green()
        ));

        if allow_edit {
            options.push(format!(
                "{} {}",
                "✎".yellow().bold(),
                t!("commit.menu.actions.edit").yellow()
            ));
        }

        options.push(format!(
            "{} {}",
            "↻".blue().bold(),
            t!("commit.menu.actions.retry").blue()
        ));

        options.push(format!(
            "{} {}",
            "↻+".blue().bold(),
            t!("commit.menu.actions.retry_feedback").blue()
        ));

        options.push(format!(
            "{} {}",
            "✕".red().bold(),
            t!("commit.menu.actions.quit").red()
        ));
    } else {
        // Plain text version
        options.push(format!("✓ {}", t!("commit.menu.actions.accept")));

        if allow_edit {
            options.push(format!("✎ {}", t!("commit.menu.actions.edit")));
        }

        options.push(format!("↻ {}", t!("commit.menu.actions.retry")));
        options.push(format!("↻+ {}", t!("commit.menu.actions.retry_feedback")));
        options.push(format!("✕ {}", t!("commit.menu.actions.quit")));
    }

    // Adjust the prompt text based on the number of retries
    let prompt = if colored {
        if retry_count == 0 {
            format!(
                "{} {}",
                t!("commit.menu.choose_action").cyan().bold(),
                t!("messages.esc_to_quit").dimmed()
            )
        } else {
            format!(
                "{} {}",
                t!("commit.menu.not_satisfied").cyan().bold(),
                t!("messages.esc_to_quit").dimmed()
            )
        }
    } else if retry_count == 0 {
        format!(
            "{} {}",
            t!("commit.menu.choose_action"),
            t!("messages.esc_to_quit")
        )
    } else {
        format!(
            "{} {}",
            t!("commit.menu.not_satisfied"),
            t!("messages.esc_to_quit")
        )
    };

    let selection = Select::new()
        .with_prompt(prompt)
        .items(&options)
        .default(0) // Accept is selected by default
        .interact_opt()
        .map_err(|_| GcopError::UserCancelled)?;

    // ESC or 'q' key to cancel
    let selection = match selection {
        Some(idx) => idx,
        None => {
            // User presses ESC or 'q' to cancel
            return Ok(CommitAction::Quit);
        }
    };

    // Mapping selections to enumerations (need to consider the impact of allow_edit)
    let action = if allow_edit {
        match selection {
            0 => CommitAction::Accept,
            1 => CommitAction::Edit,
            2 => CommitAction::Retry,
            3 => CommitAction::RetryWithFeedback,
            4 => CommitAction::Quit,
            _ => {
                tracing::error!("Unexpected selection (allow_edit=true): {}", selection);
                CommitAction::Quit
            }
        }
    } else {
        match selection {
            0 => CommitAction::Accept,
            1 => CommitAction::Retry,
            2 => CommitAction::RetryWithFeedback,
            3 => CommitAction::Quit,
            _ => {
                tracing::error!("Unexpected selection (allow_edit=false): {}", selection);
                CommitAction::Quit
            }
        }
    };

    Ok(action)
}

/// Get user feedback on retries
///
/// # Returns
/// * `Ok(Some(String))` - user-entered feedback
/// * `Ok(None)` - user did not enter or canceled
/// * `Err(_)` - An error occurred
pub fn get_retry_feedback(colored: bool) -> Result<Option<String>> {
    use rust_i18n::t;

    let hint = t!("commit.feedback.hint");
    if colored {
        println!("\n{}", hint.cyan());
    } else {
        println!("\n{}", hint);
    }

    let feedback: String = Input::new()
        .with_prompt(t!("commit.feedback.prompt").to_string())
        .allow_empty(true)
        .interact_text()
        .map_err(|_| GcopError::UserCancelled)?;

    let trimmed = feedback.trim();

    // Limit the length to prevent prompt from being too long
    if trimmed.len() > MAX_FEEDBACK_LENGTH {
        let truncated = &trimmed[..MAX_FEEDBACK_LENGTH];
        if colored {
            println!(
                "{} {}",
                "⚠".yellow(),
                t!("commit.feedback.too_long", length = MAX_FEEDBACK_LENGTH).yellow()
            );
        } else {
            println!(
                "{}",
                t!("commit.feedback.too_long", length = MAX_FEEDBACK_LENGTH)
            );
        }
        Ok(Some(truncated.to_string()))
    } else if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed.to_string()))
    }
}

/// Interactive confirmation prompt
///
/// # Arguments
/// * `message` - prompt message
/// * `default` - default value (true = Yes, false = No)
///
/// # Returns
/// * `Ok(true)` - user selected Yes
/// * `Ok(false)` - user selected No
/// * `Err(_)` - An error occurred
pub fn confirm(message: &str, default: bool) -> Result<bool> {
    let result = Confirm::new()
        .with_prompt(message)
        .default(default)
        .interact()?;

    Ok(result)
}
