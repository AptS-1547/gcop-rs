use std::process::Command;

use crate::error::{GcopError, Result};

/// 执行 git commit
///
/// 使用 git CLI 而非 git2，以支持：
/// - GPG 签名 (commit.gpgsign, user.signingkey)
/// - Git hooks (pre-commit, commit-msg 等)
/// - 所有 git config 配置
///
/// # Arguments
/// * `message` - Commit 消息
pub fn commit_changes(message: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["commit", "-m", message])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let error_msg = if stderr.trim().is_empty() {
            // 有些 git 错误会输出到 stdout 而不是 stderr
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        } else {
            stderr.trim().to_string()
        };
        return Err(GcopError::GitCommand(error_msg));
    }

    Ok(())
}
