use std::sync::Arc;

use colored::Colorize;
use serde::Serialize;

use super::options::CommitOptions;
use super::smart_truncate_diff;
use crate::commands::commit_state_machine::{CommitState, GenerationResult, UserAction};
use crate::commands::json::{self, JsonOutput};
use crate::config::AppConfig;
use crate::error::{GcopError, Result};
use crate::git::{DiffStats, GitOperations, repository::GitRepository};
use crate::llm::provider::base::response::process_commit_response;
use crate::llm::{CommitContext, LLMProvider, ScopeInfo, provider::create_provider};
use crate::ui;

/// The data part of the Commit command
#[derive(Debug, Serialize)]
pub struct CommitData {
    /// Final commit message produced by the command.
    pub message: String,
    /// Diff statistics included in JSON output.
    pub diff_stats: DiffStatsJson,
    /// Whether `git commit` was executed (`false` for dry-run/json-only flows).
    pub committed: bool,
}

/// Serializable diff statistics payload used by command JSON output.
#[derive(Debug, Serialize)]
pub struct DiffStatsJson {
    /// Files changed in the staged diff.
    pub files_changed: Vec<String>,
    /// Number of inserted lines.
    pub insertions: usize,
    /// Number of deleted lines.
    pub deletions: usize,
    /// Total changed lines (`insertions + deletions`).
    pub total_changes: usize,
}

impl From<&DiffStats> for DiffStatsJson {
    fn from(stats: &DiffStats) -> Self {
        Self {
            files_changed: stats.files_changed.clone(),
            insertions: stats.insertions,
            deletions: stats.deletions,
            total_changes: stats.insertions + stats.deletions,
        }
    }
}

/// Execute commit command
///
/// # Arguments
/// * `options` - Commit command options
/// * `config` - application configuration
pub async fn run(options: &CommitOptions<'_>, config: &AppConfig) -> Result<()> {
    let repo = GitRepository::open(None)?;
    let provider = create_provider(config, options.provider_override)?;

    run_with_deps(options, config, &repo as &dyn GitOperations, &provider).await
}

/// Execute commit command (testable version, accepts trait objects)
#[allow(dead_code)] // for testing
async fn run_with_deps(
    options: &CommitOptions<'_>,
    config: &AppConfig,
    repo: &dyn GitOperations,
    provider: &Arc<dyn LLMProvider>,
) -> Result<()> {
    let colored = options.effective_colored(config);

    // Merge command line parameters into one feedback (easy to use without quotes)
    // e.g. `gcop-rs commit use Chinese` -> "use Chinese"
    let initial_feedbacks = if options.feedback.is_empty() {
        vec![]
    } else {
        vec![options.feedback.join(" ")]
    };

    // Split mode: separate flow
    if options.split {
        if options.amend {
            ui::error(&rust_i18n::t!("commit.amend_split_conflict"), colored);
            return Err(GcopError::InvalidInput(
                "Cannot use --amend with --split".to_string(),
            ));
        }
        return crate::commands::split::run_split_flow(options, config, repo, provider).await;
    }

    // Amend: require at least one existing commit
    if options.amend && repo.is_empty()? {
        ui::error(&rust_i18n::t!("commit.amend_no_commits"), colored);
        return Err(GcopError::InvalidInput(
            "Cannot amend: repository has no commits".to_string(),
        ));
    }

    // JSON Schema: Standalone Process
    if options.format.is_json() {
        return handle_json_mode(options, config, repo, provider, &initial_feedbacks).await;
    }

    // Get diff based on mode (normal vs amend)
    if !options.amend && !repo.has_staged_changes()? {
        ui::error(&rust_i18n::t!("commit.no_staged_changes"), colored);
        return Err(GcopError::NoStagedChanges);
    }
    let diff = get_diff(repo, options.amend)?;

    // Get diff statistics
    let stats = repo.get_diff_stats(&diff)?;

    // Truncate overly large diffs to prevent tokens from exceeding the limit
    let (diff, truncated) = smart_truncate_diff(&diff, config.llm.max_diff_size);
    if truncated {
        ui::warning(&rust_i18n::t!("diff.truncated"), colored);
    }

    // Workspace scope detection
    let scope_info = compute_scope_info(&stats.files_changed, config);

    ui::step(
        &rust_i18n::t!("commit.step1"),
        &rust_i18n::t!(
            "commit.analyzed",
            files = stats.files_changed.len(),
            changes = stats.insertions + stats.deletions
        ),
        colored,
    );

    if config.commit.show_diff_preview {
        println!("\n{}", ui::format_diff_stats(&stats, colored));
    }

    // dry_run mode: only generate without submitting
    if options.dry_run {
        let branch_name = repo.get_current_branch()?;
        let custom_prompt = config.commit.custom_prompt.clone();
        let (message, already_displayed) = generate_message(
            provider,
            &diff,
            &stats,
            config,
            &initial_feedbacks,
            0,
            options.verbose,
            &branch_name,
            &custom_prompt,
            &scope_info,
        )
        .await?;
        if !already_displayed {
            display_message(&message, 0, config.ui.colored);
        }
        return Ok(());
    }

    // Interactive mode: state machine main loop
    let should_edit = config.commit.allow_edit && !options.no_edit;
    let max_retries = config.commit.max_retries;

    // Extract the unchanged context in the loop (branch_name, custom_prompt will not change with retry)
    let branch_name = repo.get_current_branch()?;
    let custom_prompt = config.commit.custom_prompt.clone();

    let mut state = CommitState::Generating {
        attempt: 0,
        feedbacks: initial_feedbacks,
    };

    loop {
        state = match state {
            CommitState::Generating { attempt, feedbacks } => {
                handle_generating(
                    attempt,
                    feedbacks,
                    max_retries,
                    colored,
                    options,
                    config,
                    provider,
                    &diff,
                    &stats,
                    &branch_name,
                    &custom_prompt,
                    &scope_info,
                )
                .await?
            }

            CommitState::WaitingForAction {
                ref message,
                attempt,
                ref feedbacks,
            } => handle_waiting_for_action(message, attempt, feedbacks, should_edit, colored)?,

            CommitState::Accepted { ref message } => {
                ui::step(
                    &rust_i18n::t!("commit.step4"),
                    &rust_i18n::t!("commit.creating"),
                    colored,
                );
                if options.amend {
                    repo.commit_amend(message)?;
                } else {
                    repo.commit(message)?;
                }
                println!();
                if options.amend {
                    ui::success(&rust_i18n::t!("commit.amend_success"), colored);
                } else {
                    ui::success(&rust_i18n::t!("commit.success"), colored);
                }
                if options.verbose {
                    println!("\n{}", message);
                }
                return Ok(());
            }

            CommitState::Cancelled => {
                ui::warning(&rust_i18n::t!("commit.cancelled"), colored);
                return Err(GcopError::UserCancelled);
            }
        };
    }
}

/// Full execution flow for JSON output mode.
async fn handle_json_mode(
    options: &CommitOptions<'_>,
    config: &AppConfig,
    repo: &dyn GitOperations,
    provider: &Arc<dyn LLMProvider>,
    initial_feedbacks: &[String],
) -> Result<()> {
    if !options.amend && !repo.has_staged_changes()? {
        json::output_json_error::<CommitData>(&GcopError::NoStagedChanges)?;
        return Err(GcopError::NoStagedChanges);
    }
    let diff = get_diff(repo, options.amend)?;
    let stats = repo.get_diff_stats(&diff)?;
    let (diff, _truncated) = smart_truncate_diff(&diff, config.llm.max_diff_size);
    let branch_name = repo.get_current_branch()?;
    let custom_prompt = config.commit.custom_prompt.clone();
    let scope_info = compute_scope_info(&stats.files_changed, config);

    match generate_message_no_streaming(
        provider,
        &diff,
        &stats,
        initial_feedbacks,
        options.verbose,
        &branch_name,
        &custom_prompt,
        &config.commit.convention,
        &scope_info,
    )
    .await
    {
        Ok(message) => output_json_success(&message, &stats, false),
        Err(e) => {
            json::output_json_error::<CommitData>(&e)?;
            Err(e)
        }
    }
}

/// Handles the `Generating` state.
#[allow(clippy::too_many_arguments)]
async fn handle_generating(
    attempt: usize,
    feedbacks: Vec<String>,
    max_retries: usize,
    colored: bool,
    options: &CommitOptions<'_>,
    config: &AppConfig,
    provider: &Arc<dyn LLMProvider>,
    diff: &str,
    stats: &DiffStats,
    branch_name: &Option<String>,
    custom_prompt: &Option<String>,
    scope_info: &Option<ScopeInfo>,
) -> Result<CommitState> {
    // Check retry limit
    let gen_state = CommitState::Generating {
        attempt,
        feedbacks: feedbacks.clone(),
    };

    if gen_state.is_at_max_retries(max_retries) {
        ui::warning(
            &rust_i18n::t!("commit.max_retries", count = max_retries),
            colored,
        );
        return gen_state.handle_generation(GenerationResult::MaxRetriesExceeded, options.yes);
    }

    // Generate message.
    let (message, already_displayed) = generate_message(
        provider,
        diff,
        stats,
        config,
        &feedbacks,
        attempt,
        options.verbose,
        branch_name,
        custom_prompt,
        scope_info,
    )
    .await?;

    // Use state-machine transition for generation result.
    let gen_state = CommitState::Generating { attempt, feedbacks };
    let result = GenerationResult::Success(message.clone());
    let next_state = gen_state.handle_generation(result, options.yes)?;

    // Show generated message unless it was auto-accepted or already streamed.
    if !options.yes && !already_displayed {
        display_message(&message, attempt, colored);
    }

    Ok(next_state)
}

/// Handles the `WaitingForAction` state.
fn handle_waiting_for_action(
    message: &str,
    attempt: usize,
    feedbacks: &[String],
    should_edit: bool,
    colored: bool,
) -> Result<CommitState> {
    ui::step(
        &rust_i18n::t!("commit.step3"),
        &rust_i18n::t!("commit.choose_action"),
        colored,
    );
    let ui_action = ui::commit_action_menu(message, should_edit, attempt, colored)?;

    // Map UI action to state-machine action and apply editor flow when needed.
    let user_action = match ui_action {
        ui::CommitAction::Accept => UserAction::Accept,

        ui::CommitAction::Edit => {
            ui::step(
                &rust_i18n::t!("commit.step3"),
                &rust_i18n::t!("commit.opening_editor"),
                colored,
            );
            match ui::edit_text(message) {
                Ok(edited) => {
                    display_edited_message(&edited, colored);
                    UserAction::Edit {
                        new_message: edited,
                    }
                }
                Err(GcopError::UserCancelled) => {
                    ui::warning(&rust_i18n::t!("commit.edit_cancelled"), colored);
                    UserAction::EditCancelled
                }
                Err(e) => return Err(e),
            }
        }

        ui::CommitAction::Retry => UserAction::Retry,

        ui::CommitAction::RetryWithFeedback => {
            let new_feedback = ui::get_retry_feedback(colored)?;
            if new_feedback.is_none() {
                ui::warning(&rust_i18n::t!("commit.feedback.empty"), colored);
            }
            UserAction::RetryWithFeedback {
                feedback: new_feedback,
            }
        }

        ui::CommitAction::Quit => UserAction::Quit,
    };

    let waiting_state = CommitState::WaitingForAction {
        message: message.to_string(),
        attempt,
        feedbacks: feedbacks.to_vec(),
    };
    Ok(waiting_state.handle_action(user_action))
}

/// Generates a commit message.
///
/// Returns `(message, already_displayed)`.
#[allow(clippy::too_many_arguments)] // There are many parameters but reasonable
async fn generate_message(
    provider: &Arc<dyn LLMProvider>,
    diff: &str,
    stats: &DiffStats,
    config: &AppConfig,
    feedbacks: &[String],
    attempt: usize,
    verbose: bool,
    branch_name: &Option<String>,
    custom_prompt: &Option<String>,
    scope_info: &Option<ScopeInfo>,
) -> Result<(String, bool)> {
    let context = CommitContext {
        files_changed: stats.files_changed.clone(),
        insertions: stats.insertions,
        deletions: stats.deletions,
        branch_name: branch_name.clone(),
        custom_prompt: custom_prompt.clone(),
        user_feedback: feedbacks.to_vec(),
        convention: config.commit.convention.clone(),
        scope_info: scope_info.clone(),
    };

    // Build prompt once
    let (system, user) = crate::llm::prompt::build_commit_prompt_split(
        diff,
        &context,
        context.custom_prompt.as_deref(),
        context.convention.as_ref(),
    );

    // Show prompts in verbose mode.
    if verbose {
        print_verbose_prompt(&system, &user, false, true);
    }

    // Decide whether to use streaming mode.
    let use_streaming = config.ui.streaming && provider.supports_streaming();
    let colored = config.ui.colored;

    if use_streaming {
        // Streaming mode: print header, then stream response chunks.
        let step_msg = if attempt == 0 {
            rust_i18n::t!("spinner.generating_streaming")
        } else {
            rust_i18n::t!("spinner.regenerating_streaming")
        };
        ui::step(&rust_i18n::t!("commit.step2"), &step_msg, colored);
        println!("\n{}", ui::info(&format_message_header(attempt), colored));

        let stream_handle = provider.send_prompt_streaming(&system, &user).await?;

        let mut output = ui::StreamingOutput::new(colored);
        let message = output.process(stream_handle.receiver).await?;
        let message = process_commit_response(message);

        // If code fences were stripped, erase raw output and redisplay clean version
        output.redisplay_if_cleaned(&message);

        Ok((message, true)) // Already shown
    } else {
        // Non-streaming mode: use spinner with cancel hint and elapsed time.
        let spinner_message = if attempt == 0 {
            rust_i18n::t!("spinner.generating").to_string()
        } else {
            rust_i18n::t!("spinner.regenerating").to_string()
        };
        let mut spinner = ui::Spinner::new_with_cancel_hint(&spinner_message, colored);
        spinner.start_time_display();

        let message = provider.send_prompt(&system, &user, Some(&spinner)).await?;

        spinner.finish_and_clear();
        let message = process_commit_response(message);
        Ok((message, false)) // Not shown yet
    }
}

/// Formats the message header (pure function, easy to test).
fn format_message_header(attempt: usize) -> String {
    if attempt == 0 {
        rust_i18n::t!("commit.generated").to_string()
    } else {
        rust_i18n::t!("commit.regenerated", attempt = attempt + 1).to_string()
    }
}

/// Formats the edited-message header (pure function, easy to test).
fn format_edited_header() -> String {
    rust_i18n::t!("commit.updated").to_string()
}

/// Displays the generated message.
fn display_message(message: &str, attempt: usize, colored: bool) {
    let header = format_message_header(attempt);

    println!("\n{}", ui::info(&header, colored));
    if colored {
        println!("{}", message.yellow());
    } else {
        println!("{}", message);
    }
}

/// Show the edited message
fn display_edited_message(message: &str, colored: bool) {
    println!("\n{}", ui::info(&format_edited_header(), colored));
    if colored {
        println!("{}", message.yellow());
    } else {
        println!("{}", message);
    }
}

/// Generate commit message (non-streaming version, for JSON output mode)
#[allow(clippy::too_many_arguments)]
async fn generate_message_no_streaming(
    provider: &Arc<dyn LLMProvider>,
    diff: &str,
    stats: &DiffStats,
    feedbacks: &[String],
    verbose: bool,
    branch_name: &Option<String>,
    custom_prompt: &Option<String>,
    convention: &Option<crate::config::CommitConvention>,
    scope_info: &Option<ScopeInfo>,
) -> Result<String> {
    let context = CommitContext {
        files_changed: stats.files_changed.clone(),
        insertions: stats.insertions,
        deletions: stats.deletions,
        branch_name: branch_name.clone(),
        custom_prompt: custom_prompt.clone(),
        user_feedback: feedbacks.to_vec(),
        convention: convention.clone(),
        scope_info: scope_info.clone(),
    };

    // Build prompt
    let (system, user) = crate::llm::prompt::build_commit_prompt_split(
        diff,
        &context,
        context.custom_prompt.as_deref(),
        context.convention.as_ref(),
    );

    // Display prompt in verbose mode
    if verbose {
        // JSON mode: output to stderr (stdout reserved for JSON), no color
        print_verbose_prompt(&system, &user, true, false);
    }

    // Use the non-streaming API directly
    provider.send_prompt(&system, &user, None).await
}

/// JSON format successfully output
fn output_json_success(message: &str, stats: &DiffStats, committed: bool) -> Result<()> {
    let output = JsonOutput {
        success: true,
        data: Some(CommitData {
            message: message.to_string(),
            diff_stats: stats.into(),
            committed,
        }),
        error: None,
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Display prompt details in verbose mode.
///
/// `to_stderr`: use stderr (for JSON mode where stdout is reserved)
/// `colored`: apply color formatting
fn print_verbose_prompt(system: &str, user: &str, to_stderr: bool, colored: bool) {
    macro_rules! vprintln {
        ($($arg:tt)*) => {
            if to_stderr {
                eprintln!($($arg)*);
            } else {
                println!($($arg)*);
            }
        };
    }

    if colored {
        vprintln!(
            "\n{}",
            rust_i18n::t!("commit.verbose.generated_prompt")
                .cyan()
                .bold()
        );
        vprintln!("{}", rust_i18n::t!("commit.verbose.system_prompt").cyan());
        vprintln!("{}", system);
        vprintln!("{}", rust_i18n::t!("commit.verbose.user_message").cyan());
        vprintln!("{}", user);
        vprintln!(
            "{}\n",
            rust_i18n::t!("commit.verbose.divider").cyan().bold()
        );
    } else {
        vprintln!("\n{}", rust_i18n::t!("commit.verbose.generated_prompt"));
        vprintln!("{}", rust_i18n::t!("commit.verbose.system_prompt"));
        vprintln!("{}", system);
        vprintln!("{}", rust_i18n::t!("commit.verbose.user_message"));
        vprintln!("{}", user);
        vprintln!("{}\n", rust_i18n::t!("commit.verbose.divider"));
    }
}

/// Public wrapper for `compute_scope_info` (used by split module).
pub(crate) fn compute_scope_info_pub(
    files_changed: &[String],
    config: &AppConfig,
) -> Option<ScopeInfo> {
    compute_scope_info(files_changed, config)
}

/// Calculate workspace scope information
///
/// Detect workspace configuration from git root and infer the scope of changed files.
/// Supports manual configuration override automatic detection. Returns None (non-fatal) if detection fails.
fn compute_scope_info(files_changed: &[String], config: &AppConfig) -> Option<ScopeInfo> {
    if !config.workspace.enabled {
        return None;
    }

    let root = crate::git::find_git_root()?;

    // Build WorkspaceInfo: Manual configuration takes precedence, otherwise automatic detection
    let workspace_info = if let Some(ref manual_members) = config.workspace.members {
        crate::workspace::WorkspaceInfo {
            workspace_types: vec![],
            members: manual_members
                .iter()
                .map(|p| crate::workspace::WorkspaceMember {
                    prefix: crate::workspace::glob_pattern_to_prefix(p),
                    pattern: p.clone(),
                })
                .collect(),
            root,
        }
    } else {
        crate::workspace::detect_workspace(&root)?
    };

    // Output detection results
    if !workspace_info.workspace_types.is_empty() {
        let type_str = workspace_info
            .workspace_types
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        tracing::debug!(
            "{}",
            rust_i18n::t!(
                "workspace.detected",
                "type" = type_str,
                count = workspace_info.members.len()
            )
        );
    }

    let scope = crate::workspace::scope::infer_scope(files_changed, &workspace_info, None);

    // Apply scope_mappings remapping
    let suggested = scope.suggested_scope.map(|s| {
        config
            .workspace
            .scope_mappings
            .get(&s)
            .cloned()
            .unwrap_or(s)
    });

    if let Some(ref s) = suggested {
        tracing::debug!("{}", rust_i18n::t!("workspace.scope_suggestion", scope = s));
    }

    Some(ScopeInfo {
        workspace_types: workspace_info
            .workspace_types
            .iter()
            .map(|t| t.to_string())
            .collect(),
        packages: scope.packages,
        suggested_scope: suggested,
        has_root_changes: !scope.root_files.is_empty(),
    })
}

/// Get diff based on commit mode.
///
/// - Amend: HEAD commit diff, optionally combined with new staged changes.
/// - Normal: staged diff (caller must check `has_staged_changes` before calling).
fn get_diff(repo: &dyn GitOperations, amend: bool) -> Result<String> {
    if amend {
        let commit_diff = repo.get_commit_diff("HEAD")?;
        if repo.has_staged_changes()? {
            let staged_diff = repo.get_staged_diff()?;
            Ok(format!("{}\n{}", commit_diff, staged_diff))
        } else {
            Ok(commit_diff)
        }
    } else {
        repo.get_staged_diff()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    // === format_message_header test ===

    #[test]
    fn test_format_message_header_first_attempt() {
        let header = format_message_header(0);
        assert_eq!(header, "Generated commit message:");
    }

    #[test]
    fn test_format_message_header_second_attempt() {
        let header = format_message_header(1);
        assert_eq!(header, "Regenerated commit message (attempt 2):");
    }

    #[test]
    fn test_format_message_header_third_attempt() {
        let header = format_message_header(2);
        assert_eq!(header, "Regenerated commit message (attempt 3):");
    }

    // === format_edited_header test ===

    #[test]
    fn test_format_edited_header() {
        let header = format_edited_header();
        assert_eq!(header, "Updated commit message:");
    }
}
