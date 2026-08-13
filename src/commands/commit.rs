use std::io::IsTerminal;
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
use crate::llm::provider::base::response::process_commit_response_with_options;
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
    let (diff, truncated) = smart_truncate_diff(
        &diff,
        config.llm.max_diff_size,
        &config.file.lockfile_patterns,
    );
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
        let live_render = should_live_render(
            config.ui.streaming,
            provider.supports_streaming(),
            options.yes,
            std::io::stderr().is_terminal(),
        );
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
            live_render,
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
    let (diff, _truncated) = smart_truncate_diff(
        &diff,
        config.llm.max_diff_size,
        &config.file.lockfile_patterns,
    );
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

    // Live-render decision is fully derived here so generate_message stays
    // a pure executor of the choice. By the time we reach this function the
    // state machine has already excluded --json and --split, so the only
    // remaining gates are config.ui.streaming, provider capability,
    // --yes, and the actual TTY state of stderr.
    let live_render = should_live_render(
        config.ui.streaming,
        provider.supports_streaming(),
        options.yes,
        std::io::stderr().is_terminal(),
    );

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
        live_render,
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
            match ui::edit_commit_message(message) {
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

/// Pure predicate: should the commit flow live-render LLM deltas to the
/// terminal (typewriter effect) versus silently buffer them behind a spinner?
///
/// This decision is **independent** of HTTP transport — that is decided
/// per call site by `LLMConfig::stream_transport` + `supports_streaming()`.
/// The live-render path uses `StreamingOutput::process`, which `print!`s
/// deltas with ANSI control sequences; in non-interactive shells those
/// sequences become visual noise (CI logs, the Claude Code Bash tool, etc.).
///
/// All inputs are taken as plain values so the function is trivially
/// unit-testable without constructing real `AppConfig`/`Provider`/`stdio`
/// instances.
fn should_live_render(
    ui_streaming: bool,
    provider_supports_streaming: bool,
    yes: bool,
    is_terminal: bool,
) -> bool {
    ui_streaming && provider_supports_streaming && !yes && is_terminal
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
    live_render: bool,
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

    let colored = config.ui.colored;

    if live_render {
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
        let message = process_commit_response_with_options(message, provider.strip_thinking());

        // If code fences were stripped, erase raw output and redisplay clean version
        output.redisplay_if_cleaned(&message);

        Ok((message, true)) // Already shown
    } else {
        // Non-streaming UI mode: use spinner with cancel hint and elapsed time.
        // HTTP transport still streams when `config.llm.stream_transport` is on
        // and the provider supports SSE — this is what protects long-running
        // LLM calls from CDN first-byte timeouts (e.g. Cloudflare 524).
        let spinner_message = if attempt == 0 {
            rust_i18n::t!("spinner.generating").to_string()
        } else {
            rust_i18n::t!("spinner.regenerating").to_string()
        };
        let mut spinner = ui::Spinner::new_with_cancel_hint(&spinner_message, colored);
        spinner.start_time_display();

        let message = provider
            .send_prompt_collect(&system, &user, Some(&spinner))
            .await?;

        spinner.finish_and_clear();
        let message = process_commit_response_with_options(message, provider.strip_thinking());
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

/// Generate commit message for JSON output mode.
///
/// The function name preserves history but the HTTP transport is now SSE
/// when the provider supports streaming (which itself already accounts for
/// `LLMConfig::stream_transport`). Only the UI never renders deltas — the
/// response is buffered into a single `String` and returned to the caller
/// intact for JSON output.
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

    let message = provider.send_prompt_collect(&system, &user, None).await?;
    Ok(process_commit_response_with_options(
        message,
        provider.strip_thinking(),
    ))
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

    // === should_live_render predicate ========================================

    #[test]
    fn test_live_render_in_interactive_default() {
        // ui.streaming=true, supports_streaming=true, yes=false, tty=true → live render
        assert!(should_live_render(true, true, false, true));
    }

    #[test]
    fn test_live_render_blocked_by_yes() {
        // -y suppresses live render even on interactive terminal
        assert!(!should_live_render(true, true, true, true));
    }

    #[test]
    fn test_live_render_blocked_by_non_tty() {
        // stderr is not a terminal (CI logs, Bash tool, pipe) → silent buffering
        assert!(!should_live_render(true, true, false, false));
    }

    #[test]
    fn test_live_render_blocked_by_ui_streaming_false() {
        // User opted out via config.ui.streaming=false
        assert!(!should_live_render(false, true, false, true));
    }

    #[test]
    fn test_live_render_blocked_by_provider_no_support() {
        // Ollama-style backend without SSE support
        assert!(!should_live_render(true, false, false, true));
    }

    #[test]
    fn test_live_render_blocked_when_all_off() {
        assert!(!should_live_render(false, false, true, false));
    }

    #[test]
    fn test_live_render_requires_all_four_signals() {
        // Any single false gate suppresses live render — exhaustive check.
        let combos = [
            (false, true, false, true),
            (true, false, false, true),
            (true, true, true, true),
            (true, true, false, false),
        ];
        for (s, p, y, t) in combos {
            assert!(
                !should_live_render(s, p, y, t),
                "should_live_render({}, {}, {}, {}) should be false but was true",
                s,
                p,
                y,
                t
            );
        }
        assert!(should_live_render(true, true, false, true));
    }

    // === End-to-end HTTP transport assertion =================================
    //
    // The original user bug: `gcop-rs commit -y --json` (non-TTY) was sending
    // HTTP requests with `stream: null/absent`, causing CDN first-byte timeouts
    // (Cloudflare 524). After the fix, `generate_message_no_streaming` (the
    // function backing `handle_json_mode` and the JSON-output path) must route
    // through the SSE transport when `stream_transport=true`, even though it
    // collects the full string before returning.

    /// JSON-mode path with `stream_transport = true` must use SSE transport.
    #[tokio::test]
    async fn test_generate_message_no_streaming_uses_sse_when_stream_transport_on() {
        use std::sync::Arc;

        use crate::git::DiffStats;
        use crate::llm::LLMProvider;
        use crate::llm::provider::backends::claude::ClaudeProvider;
        use crate::llm::provider::test_utils::ensure_crypto_provider;

        ensure_crypto_provider();
        let mut server = mockito::Server::new_async().await;
        // Demands stream:true in body; if generate_message_no_streaming
        // routes through send_prompt (the old path), this mock will not match
        // and the request will fail. SSE body is returned on success.
        let mock = server
            .mock("POST", "/v1/messages")
            .match_body(mockito::Matcher::PartialJson(serde_json::json!({
                "stream": true,
            })))
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(concat!(
                "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"feat: ship\"}}\n\n",
                "data: {\"type\":\"message_stop\"}\n\n",
            ))
            .create_async()
            .await;

        let provider_cfg = crate::llm::provider::test_utils::test_provider_config(
            server.url(),
            Some("sk-ant-test".into()),
            "claude-3-haiku-20240307".into(),
        );
        let net_cfg = crate::llm::provider::test_utils::test_network_config_no_retry();
        let provider: Arc<dyn LLMProvider> =
            Arc::new(ClaudeProvider::new(&provider_cfg, "claude", &net_cfg, false, true).unwrap());

        let stats = DiffStats {
            files_changed: vec!["a.rs".into()],
            insertions: 1,
            deletions: 0,
        };

        let result = generate_message_no_streaming(
            &provider,
            "diff --git a/a.rs b/a.rs\n+1 line",
            &stats,
            &[],
            false,
            &None,
            &None,
            &None,
            &None,
        )
        .await
        .expect("generate_message_no_streaming should succeed");
        assert_eq!(result, "feat: ship");
        mock.assert_async().await;
    }

    /// Escape hatch: when the provider is constructed with
    /// `stream_transport_enabled = false` (because the user set
    /// `[llm].stream_transport = false`), `supports_streaming()` returns
    /// false and `generate_message_no_streaming` falls through to the
    /// legacy non-streaming HTTP body. Mirrors the regression guard in
    /// the backend tests.
    #[tokio::test]
    async fn test_generate_message_no_streaming_falls_back_when_stream_transport_off() {
        use std::sync::Arc;

        use crate::git::DiffStats;
        use crate::llm::LLMProvider;
        use crate::llm::provider::backends::claude::ClaudeProvider;
        use crate::llm::provider::test_utils::ensure_crypto_provider;

        ensure_crypto_provider();
        let mut server = mockito::Server::new_async().await;
        // Streaming-only mock — must NOT match when stream_transport=false.
        let stream_mock = server
            .mock("POST", "/v1/messages")
            .match_body(mockito::Matcher::PartialJson(
                serde_json::json!({"stream": true}),
            ))
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body("data: {\"type\":\"message_stop\"}\n\n")
            .expect(0)
            .create_async()
            .await;
        // Permissive fallback for the legacy synchronous POST.
        let _fallback = server
            .mock("POST", "/v1/messages")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"content":[{"type":"text","text":"feat: ship"}]}"#)
            .create_async()
            .await;

        let provider_cfg = crate::llm::provider::test_utils::test_provider_config(
            server.url(),
            Some("sk-ant-test".into()),
            "claude-3-haiku-20240307".into(),
        );
        let net_cfg = crate::llm::provider::test_utils::test_network_config_no_retry();
        // stream_transport_enabled = false → supports_streaming() == false →
        // generate_message_no_streaming falls through to send_prompt (legacy
        // non-streaming HTTP body).
        let provider: Arc<dyn LLMProvider> =
            Arc::new(ClaudeProvider::new(&provider_cfg, "claude", &net_cfg, false, false).unwrap());

        let stats = DiffStats {
            files_changed: vec!["a.rs".into()],
            insertions: 1,
            deletions: 0,
        };

        let result = generate_message_no_streaming(
            &provider,
            "diff",
            &stats,
            &[],
            false,
            &None,
            &None,
            &None,
            &None,
        )
        .await
        .expect("should succeed via legacy path");
        assert_eq!(result, "feat: ship");
        stream_mock.assert_async().await; // expect(0) verified
    }
}
