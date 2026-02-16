use std::process::Command;

use crate::error::{GcopError, Result};

/// Execute git commit
///
/// Use git CLI instead of git2 to support:
/// - GPG signature (commit.gpgsign, user.signingkey)
/// - Git hooks (pre-commit, commit-msg, etc.)
/// - All git config configurations
///
/// # Arguments
/// * `message` - Commit message
pub fn commit_changes(message: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["commit", "-m", message])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let error_msg = if stderr.trim().is_empty() {
            // Some git errors are output to stdout instead of stderr
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        } else {
            stderr.trim().to_string()
        };
        return Err(GcopError::GitCommand(error_msg));
    }

    Ok(())
}
