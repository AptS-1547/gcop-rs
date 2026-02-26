use std::fs;

use crate::commands::smart_truncate_diff;
use crate::config::AppConfig;
use crate::error::{GcopError, Result};
use crate::git::repository::GitRepository;
use crate::git::{GitOperations, find_git_root};
use crate::llm::CommitContext;
use crate::llm::provider::base::response::process_commit_response;
use crate::llm::provider::create_provider;

/// Hook marker used to identify hooks installed by gcop-rs
const HOOK_MARKER: &str = "gcop-rs hook run";

/// Shell script content for the prepare-commit-msg hook
const HOOK_SCRIPT: &str = r#"#!/bin/sh
# gcop-rs prepare-commit-msg hook
# Installed by: gcop-rs hook install
# To remove: gcop-rs hook uninstall
if ! command -v gcop-rs >/dev/null 2>&1; then
    exit 0
fi
gcop-rs hook run "$1" "$2" "$3"
"#;

/// Install the prepare-commit-msg hook into the current git repository.
///
/// If the hook already exists and was installed by gcop-rs, prints an info message.
/// If the hook already exists but was NOT installed by gcop-rs, requires `--force`
/// to overwrite.
///
/// # Arguments
/// * `force` - If true, overwrite an existing non-gcop-rs hook
pub fn install(force: bool) -> Result<()> {
    let git_root = find_git_root().ok_or_else(|| {
        GcopError::Git(crate::error::GitErrorWrapper(git2::Error::from_str(
            "Not in a git repository",
        )))
    })?;

    let hooks_dir = git_root.join(".git").join("hooks");
    fs::create_dir_all(&hooks_dir)?;

    let hook_path = hooks_dir.join("prepare-commit-msg");

    if hook_path.exists() {
        let content = fs::read_to_string(&hook_path)?;

        if content.contains(HOOK_MARKER) {
            eprintln!(
                "{}",
                rust_i18n::t!(
                    "hook.already_installed",
                    path = hook_path.display().to_string()
                )
            );
            return Ok(());
        }

        if !force {
            eprintln!(
                "{}",
                rust_i18n::t!("hook.existing_hook", path = hook_path.display().to_string())
            );
            return Ok(());
        }

        eprintln!(
            "{}",
            rust_i18n::t!("hook.overwriting", path = hook_path.display().to_string())
        );
    }

    fs::write(&hook_path, HOOK_SCRIPT)?;

    // Set executable permission on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o755);
        fs::set_permissions(&hook_path, perms)?;
    }

    eprintln!(
        "{}",
        rust_i18n::t!("hook.installed", path = hook_path.display().to_string())
    );

    Ok(())
}

/// Uninstall the prepare-commit-msg hook from the current git repository.
///
/// Only removes the hook if it was installed by gcop-rs (contains the marker).
/// If the hook was not installed by gcop-rs, prints a warning and does nothing.
pub fn uninstall() -> Result<()> {
    let git_root = find_git_root().ok_or_else(|| {
        GcopError::Git(crate::error::GitErrorWrapper(git2::Error::from_str(
            "Not in a git repository",
        )))
    })?;

    let hook_path = git_root
        .join(".git")
        .join("hooks")
        .join("prepare-commit-msg");

    if !hook_path.exists() {
        eprintln!("{}", rust_i18n::t!("hook.no_hook_found"));
        return Ok(());
    }

    let content = fs::read_to_string(&hook_path)?;
    if !content.contains(HOOK_MARKER) {
        eprintln!("{}", rust_i18n::t!("hook.not_installed_by_gcop"));
        return Ok(());
    }

    fs::remove_file(&hook_path)?;

    eprintln!(
        "{}",
        rust_i18n::t!("hook.uninstalled", path = hook_path.display().to_string())
    );

    Ok(())
}

/// Safe wrapper for `run_hook_inner` that catches and prints errors to stderr.
///
/// This function is called from the CLI when `gcop-rs hook run` is invoked
/// by the prepare-commit-msg hook script. Errors are printed but do not
/// cause git commit to fail (exit code 0).
///
/// # Arguments
/// * `commit_msg_file` - Path to the file containing the commit message (from git)
/// * `source` - The commit source (message, merge, commit, squash, or empty)
/// * `sha` - Commit SHA (non-empty for amend, provided by git as $3)
/// * `config` - Application configuration
/// * `verbose` - Whether verbose mode is enabled
/// * `provider_override` - Optional provider name override
pub async fn run_hook_safe(
    commit_msg_file: &str,
    source: &str,
    sha: &str,
    config: &AppConfig,
    verbose: bool,
    provider_override: Option<&str>,
) {
    if let Err(e) = run_hook_inner(
        commit_msg_file,
        source,
        sha,
        config,
        verbose,
        provider_override,
    )
    .await
    {
        eprintln!("gcop-rs: {}", e.localized_message());
    }
}

/// Result of analyzing hook source and sha parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HookMode {
    /// Skip hook execution (message already provided by git)
    Skip,
    /// Normal commit: generate message from staged diff
    Normal,
    /// Amend commit: generate message from the original commit's diff
    Amend,
}

/// Determines the hook mode based on `source` and `sha` parameters from git.
///
/// Git's `prepare-commit-msg` hook receives up to 3 arguments:
/// - `$1`: path to the commit message file
/// - `$2` (source): `"message"`, `"merge"`, `"commit"`, `"squash"`, or `""` (empty)
/// - `$3` (sha): commit SHA (non-empty only for `--amend`)
///
/// | source     | sha       | mode   | rationale                                  |
/// |------------|-----------|--------|--------------------------------------------|
/// | `message`  | *         | Skip   | user already provided `-m` / `-C` / `-c`   |
/// | `merge`    | *         | Skip   | merge commit message auto-generated        |
/// | `squash`   | *         | Skip   | squash merge message auto-generated        |
/// | `commit`   | empty     | Skip   | non-amend reuse (e.g. `git commit -C`)     |
/// | `commit`   | non-empty | Amend  | `--amend` with known target SHA            |
/// | `""` / _   | *         | Normal | regular `git commit`                       |
fn determine_hook_mode(source: &str, sha: &str) -> HookMode {
    match source {
        "message" | "merge" | "squash" => HookMode::Skip,
        "commit" if sha.is_empty() => HookMode::Skip,
        "commit" => HookMode::Amend,
        _ => HookMode::Normal,
    }
}

/// Internal hook logic that generates a commit message and writes it to the
/// commit message file.
///
/// Skips generation when the commit source indicates the message was already
/// provided (message, merge, squash). For `source == "commit"` (amend), skips
/// only when `sha` is empty (e.g. `git commit -C`); when `sha` is non-empty,
/// generates a new message based on the amend target's diff.
async fn run_hook_inner(
    commit_msg_file: &str,
    source: &str,
    sha: &str,
    config: &AppConfig,
    _verbose: bool,
    provider_override: Option<&str>,
) -> Result<()> {
    let mode = determine_hook_mode(source, sha);
    if mode == HookMode::Skip {
        return Ok(());
    }

    let is_amend = mode == HookMode::Amend;

    // Open repository
    let repo = GitRepository::open(Some(&config.file))?;

    // Get diff based on scenario
    let diff = if is_amend {
        // Amend scenario: get the original commit's diff
        let commit_diff = repo.get_commit_diff(sha)?;
        if repo.has_staged_changes()? {
            // Amend with additional staged changes: combine both diffs
            let staged_diff = repo.get_staged_diff()?;
            format!("{}\n{}", commit_diff, staged_diff)
        } else {
            // Amend without new staged changes (pure message rewrite)
            commit_diff
        }
    } else {
        // Normal commit: require staged changes
        if !repo.has_staged_changes()? {
            return Ok(());
        }
        repo.get_staged_diff()?
    };

    let stats = repo.get_diff_stats(&diff)?;

    // Truncate diff to fit LLM token limit
    let (diff, _) = smart_truncate_diff(&diff, config.llm.max_diff_size);

    // Get current branch name
    let branch_name = repo.get_current_branch()?;

    // Build commit context
    let context = CommitContext {
        files_changed: stats.files_changed,
        insertions: stats.insertions,
        deletions: stats.deletions,
        branch_name,
        custom_prompt: config.commit.custom_prompt.clone(),
        user_feedback: vec![],
        convention: config.commit.convention.clone(),
        scope_info: None, // Hook mode does not currently support workspace scope
    };

    // Build prompt
    let (system, user) = crate::llm::prompt::build_commit_prompt_split(
        &diff,
        &context,
        context.custom_prompt.as_deref(),
        context.convention.as_ref(),
    );

    // Create LLM provider
    let provider = create_provider(config, provider_override)?;

    // Print status to stderr (stdout must not be used in hooks)
    if is_amend {
        eprintln!("gcop-rs: {}", rust_i18n::t!("hook.generating_amend"));
    } else {
        eprintln!("gcop-rs: {}", rust_i18n::t!("hook.generating"));
    }

    // Generate commit message
    let message = provider.send_prompt(&system, &user, None).await?;
    let message = process_commit_response(message);

    // Write generated message to the commit message file
    fs::write(commit_msg_file, &message)?;

    // Print success to stderr
    eprintln!("gcop-rs: {}", rust_i18n::t!("hook.generated_success"));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // === determine_hook_mode tests ===

    #[test]
    fn test_source_message_skips() {
        assert_eq!(determine_hook_mode("message", ""), HookMode::Skip);
        assert_eq!(determine_hook_mode("message", "abc123"), HookMode::Skip);
    }

    #[test]
    fn test_source_merge_skips() {
        assert_eq!(determine_hook_mode("merge", ""), HookMode::Skip);
        assert_eq!(determine_hook_mode("merge", "abc123"), HookMode::Skip);
    }

    #[test]
    fn test_source_squash_skips() {
        assert_eq!(determine_hook_mode("squash", ""), HookMode::Skip);
        assert_eq!(determine_hook_mode("squash", "abc123"), HookMode::Skip);
    }

    #[test]
    fn test_source_commit_empty_sha_skips() {
        // git commit -C / -c without amend: source is "commit" but sha is empty
        assert_eq!(determine_hook_mode("commit", ""), HookMode::Skip);
    }

    #[test]
    fn test_source_commit_with_sha_is_amend() {
        // git commit --amend: source is "commit" and sha is the HEAD commit hash
        assert_eq!(
            determine_hook_mode("commit", "abc123def456"),
            HookMode::Amend
        );
        assert_eq!(
            determine_hook_mode("commit", "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2"),
            HookMode::Amend
        );
    }

    #[test]
    fn test_empty_source_is_normal() {
        // Regular git commit: source is empty string
        assert_eq!(determine_hook_mode("", ""), HookMode::Normal);
    }

    #[test]
    fn test_unknown_source_is_normal() {
        // Any unrecognized source falls through to normal
        assert_eq!(determine_hook_mode("template", ""), HookMode::Normal);
        assert_eq!(determine_hook_mode("unknown", ""), HookMode::Normal);
    }
}
