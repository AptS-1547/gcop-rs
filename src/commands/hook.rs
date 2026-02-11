use std::fs;

use crate::commands::smart_truncate_diff;
use crate::config::AppConfig;
use crate::error::{GcopError, Result};
use crate::git::repository::GitRepository;
use crate::git::{find_git_root, GitOperations};
use crate::llm::provider::create_provider;
use crate::llm::CommitContext;

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
                rust_i18n::t!(
                    "hook.existing_hook",
                    path = hook_path.display().to_string()
                )
            );
            return Ok(());
        }

        eprintln!(
            "{}",
            rust_i18n::t!(
                "hook.overwriting",
                path = hook_path.display().to_string()
            )
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
        rust_i18n::t!(
            "hook.installed",
            path = hook_path.display().to_string()
        )
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

    let hook_path = git_root.join(".git").join("hooks").join("prepare-commit-msg");

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
        rust_i18n::t!(
            "hook.uninstalled",
            path = hook_path.display().to_string()
        )
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
/// * `config` - Application configuration
/// * `verbose` - Whether verbose mode is enabled
/// * `provider_override` - Optional provider name override
pub async fn run_hook_safe(
    commit_msg_file: &str,
    source: &str,
    config: &AppConfig,
    verbose: bool,
    provider_override: Option<&str>,
) {
    if let Err(e) =
        run_hook_inner(commit_msg_file, source, config, verbose, provider_override).await
    {
        eprintln!("gcop-rs: {}", e.localized_message());
    }
}

/// Internal hook logic that generates a commit message and writes it to the
/// commit message file.
///
/// Skips generation when the commit source indicates the message was already
/// provided (message, merge, commit, squash).
async fn run_hook_inner(
    commit_msg_file: &str,
    source: &str,
    config: &AppConfig,
    _verbose: bool,
    provider_override: Option<&str>,
) -> Result<()> {
    // Skip when git already has a message source
    match source {
        "message" | "merge" | "commit" | "squash" => return Ok(()),
        _ => {}
    }

    // Open repository
    let repo = GitRepository::open(Some(&config.file))?;

    // Check for staged changes
    if !repo.has_staged_changes()? {
        return Ok(());
    }

    // Get diff and stats
    let diff = repo.get_staged_diff()?;
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
    };

    // Create LLM provider
    let provider = create_provider(config, provider_override)?;

    // Print status to stderr (stdout must not be used in hooks)
    eprintln!("gcop-rs: {}", rust_i18n::t!("hook.generating"));

    // Generate commit message
    let message = provider
        .generate_commit_message(&diff, Some(context), None)
        .await?;

    // Write generated message to the commit message file
    fs::write(commit_msg_file, &message)?;

    // Print success to stderr
    eprintln!("gcop-rs: {}", rust_i18n::t!("hook.generated_success"));

    Ok(())
}
