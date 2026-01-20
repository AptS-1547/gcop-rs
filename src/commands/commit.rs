use std::sync::Arc;

use colored::Colorize;
use serde::Serialize;

use crate::cli::Cli;
use crate::commands::commit_state_machine::{CommitState, GenerationResult, UserAction};
use crate::commands::json::ErrorJson;
use crate::config::AppConfig;
use crate::error::{GcopError, Result};
use crate::git::{DiffStats, GitOperations, repository::GitRepository};
use crate::llm::{CommitContext, LLMProvider, provider::create_provider};
use crate::ui;

/// JSON 输出格式（统一结构）
#[derive(Debug, Serialize)]
pub struct CommitJsonOutput {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<CommitData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorJson>,
}

/// Commit 命令的数据部分
#[derive(Debug, Serialize)]
pub struct CommitData {
    pub message: String,
    pub diff_stats: DiffStatsJson,
    pub committed: bool,
}

#[derive(Debug, Serialize)]
pub struct DiffStatsJson {
    pub files_changed: Vec<String>,
    pub insertions: usize,
    pub deletions: usize,
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

/// 执行 commit 命令
///
/// # Arguments
/// * `cli` - CLI 参数
/// * `config` - 应用配置
/// * `no_edit` - 是否跳过编辑
/// * `yes` - 是否跳过确认
/// * `dry_run` - 是否只输出 commit message 而不提交
/// * `format` - 输出格式 ("text" | "json")
/// * `feedback` - 初始反馈/指令
pub async fn run(
    cli: &Cli,
    config: &AppConfig,
    no_edit: bool,
    yes: bool,
    dry_run: bool,
    format: &str,
    feedback: Vec<String>,
) -> Result<()> {
    let repo = GitRepository::open(None)?;
    let provider = create_provider(config, cli.provider.as_deref())?;

    run_with_deps(
        cli,
        config,
        no_edit,
        yes,
        dry_run,
        format,
        feedback,
        &repo as &dyn GitOperations,
        &provider,
    )
    .await
}

/// 执行 commit 命令（可测试版本，接受 trait 对象）
#[allow(dead_code)] // 供测试使用
#[allow(clippy::too_many_arguments)] // 参数较多但合理
async fn run_with_deps(
    cli: &Cli,
    config: &AppConfig,
    no_edit: bool,
    yes: bool,
    dry_run: bool,
    format: &str,
    feedback: Vec<String>,
    repo: &dyn GitOperations,
    provider: &Arc<dyn LLMProvider>,
) -> Result<()> {
    let is_json = format == "json";
    // JSON 模式禁用彩色输出
    let colored = if is_json { false } else { config.ui.colored };

    // 将命令行参数合并为一条反馈（便于不加引号时使用）
    // e.g. `gcop-rs commit use Chinese` -> "use Chinese"
    let initial_feedbacks = if feedback.is_empty() {
        vec![]
    } else {
        vec![feedback.join(" ")]
    };

    // 2. 检查 staged changes
    if !repo.has_staged_changes()? {
        if is_json {
            output_json_error(&GcopError::NoStagedChanges)?;
            return Err(GcopError::NoStagedChanges);
        }
        ui::error("No staged changes found. Use 'git add' first.", colored);
        return Err(GcopError::NoStagedChanges);
    }

    // 3. 获取 diff 和统计
    let diff = repo.get_staged_diff()?;
    let stats = repo.get_diff_stats(&diff)?;

    // JSON 模式跳过 UI 输出
    if !is_json {
        ui::step(
            "1/4",
            &format!(
                "Analyzed {} file(s), {} change(s)",
                stats.files_changed.len(),
                stats.insertions + stats.deletions
            ),
            colored,
        );

        // 4. 显示预览（可选）
        if config.commit.show_diff_preview {
            println!("\n{}", ui::format_diff_stats(&stats, colored));
        }
    }

    // JSON 模式：生成 message 并输出 JSON（隐式 dry_run）
    if is_json {
        // JSON 模式禁用流式输出
        let result = generate_message_no_streaming(
            provider,
            repo,
            &diff,
            &stats,
            config,
            &initial_feedbacks,
            cli.verbose,
        )
        .await;

        match result {
            Ok(message) => {
                output_json_success(&message, &stats, false)?;
            }
            Err(e) => {
                output_json_error(&e)?;
                return Err(e);
            }
        }
        return Ok(());
    }

    // dry_run 模式：只生成并输出 commit message
    if dry_run {
        let (message, already_displayed) = generate_message(
            provider,
            repo,
            &diff,
            &stats,
            config,
            &initial_feedbacks,
            0,
            cli.verbose,
        )
        .await?;
        if !already_displayed {
            display_message(&message, 0, config.ui.colored);
        }
        return Ok(());
    }

    // 5. 状态机主循环
    let should_edit = config.commit.allow_edit && !no_edit;
    let max_retries = config.commit.max_retries;

    let mut state = CommitState::Generating {
        attempt: 0,
        feedbacks: initial_feedbacks,
    };

    loop {
        state = match state {
            CommitState::Generating { attempt, feedbacks } => {
                // 使用状态机方法检查重试上限
                let gen_state = CommitState::Generating {
                    attempt,
                    feedbacks: feedbacks.clone(),
                };

                if gen_state.is_at_max_retries(max_retries) {
                    ui::warning(
                        &format!("Reached maximum retry limit ({})", max_retries),
                        colored,
                    );
                    // 使用 MaxRetriesExceeded 变体，直接触发错误
                    gen_state.handle_generation(GenerationResult::MaxRetriesExceeded, yes)?;
                    unreachable!("MaxRetriesExceeded should return error");
                }

                // 生成 message
                let (message, already_displayed) = generate_message(
                    provider,
                    repo,
                    &diff,
                    &stats,
                    config,
                    &feedbacks,
                    attempt,
                    cli.verbose,
                )
                .await?;

                // 使用状态机方法处理生成结果
                let gen_state = CommitState::Generating { attempt, feedbacks };
                let result = GenerationResult::Success(message.clone());
                let next_state = gen_state.handle_generation(result, yes)?;

                // 显示生成的消息（除非 --yes 直接接受，或流式模式已经显示过）
                if !yes && !already_displayed {
                    display_message(&message, attempt, colored);
                }

                next_state
            }

            CommitState::WaitingForAction {
                ref message,
                attempt,
                ref feedbacks,
            } => {
                ui::step("3/4", "Choose next action...", colored);
                let ui_action = ui::commit_action_menu(message, should_edit, attempt, colored)?;

                // 映射 UI action 到状态机 action，处理编辑逻辑
                let user_action = match ui_action {
                    ui::CommitAction::Accept => UserAction::Accept,

                    ui::CommitAction::Edit => {
                        ui::step("3/4", "Opening editor...", colored);
                        match ui::edit_text(message) {
                            Ok(edited) => {
                                display_edited_message(&edited, colored);
                                UserAction::Edit {
                                    new_message: edited,
                                }
                            }
                            Err(GcopError::UserCancelled) => {
                                ui::warning("Edit cancelled.", colored);
                                UserAction::EditCancelled
                            }
                            Err(e) => return Err(e),
                        }
                    }

                    ui::CommitAction::Retry => UserAction::Retry,

                    ui::CommitAction::RetryWithFeedback => {
                        let new_feedback = ui::get_retry_feedback(colored)?;
                        if new_feedback.is_none() {
                            ui::warning(
                                "No feedback provided, will retry with existing instructions.",
                                colored,
                            );
                        }
                        UserAction::RetryWithFeedback {
                            feedback: new_feedback,
                        }
                    }

                    ui::CommitAction::Quit => UserAction::Quit,
                };

                // 克隆 WaitingForAction 状态以调用 handle_action
                let waiting_state = CommitState::WaitingForAction {
                    message: message.clone(),
                    attempt,
                    feedbacks: feedbacks.clone(),
                };
                waiting_state.handle_action(user_action)
            }

            CommitState::Accepted { ref message } => {
                // 执行 commit
                ui::step("4/4", "Creating commit...", colored);
                repo.commit(message)?;

                println!();
                ui::success("Commit created successfully!", colored);
                if cli.verbose {
                    println!("\n{}", message);
                }
                return Ok(());
            }

            CommitState::Cancelled => {
                ui::warning("Commit cancelled by user.", colored);
                return Err(GcopError::UserCancelled);
            }
        };
    }
}

/// 生成 commit message
///
/// 返回 (message, already_displayed) - 流式模式下 message 已经显示过了
#[allow(clippy::too_many_arguments)] // 参数较多但合理
async fn generate_message(
    provider: &Arc<dyn LLMProvider>,
    repo: &dyn GitOperations,
    diff: &str,
    stats: &DiffStats,
    config: &AppConfig,
    feedbacks: &[String],
    attempt: usize,
    verbose: bool,
) -> Result<(String, bool)> {
    let context = CommitContext {
        files_changed: stats.files_changed.clone(),
        insertions: stats.insertions,
        deletions: stats.deletions,
        branch_name: repo.get_current_branch()?,
        custom_prompt: config.commit.custom_prompt.clone(),
        user_feedback: feedbacks.to_vec(),
    };

    // verbose 模式下显示 prompt
    if verbose {
        let (system, user) = crate::llm::prompt::build_commit_prompt_split(
            diff,
            &context,
            context.custom_prompt.as_deref(),
        );
        println!("\n{}", "=== Verbose: Generated Prompt ===".cyan().bold());
        println!("{}", "--- System Prompt ---".cyan());
        println!("{}", system);
        println!("{}", "--- User Message ---".cyan());
        println!("{}", user);
        println!("{}\n", "=================================".cyan().bold());
    }

    // 判断是否使用流式模式
    let use_streaming = config.ui.streaming && provider.supports_streaming();
    let colored = config.ui.colored;

    if use_streaming {
        // 流式模式：先显示标题，再流式输出
        let step_msg = if attempt == 0 {
            "Generating commit message (streaming)... (Ctrl+C to cancel)"
        } else {
            "Regenerating commit message (streaming)... (Ctrl+C to cancel)"
        };
        ui::step("2/4", step_msg, colored);
        println!("\n{}", ui::info(&format_message_header(attempt), colored));

        let stream_handle = provider
            .generate_commit_message_streaming(diff, Some(context))
            .await?;

        let mut output = ui::StreamingOutput::new(colored);
        let message = output.process(stream_handle.receiver).await?;

        Ok((message, true)) // 已经显示过了
    } else {
        // 非流式模式：使用 Spinner（带取消提示和时间显示）
        let mut spinner = ui::Spinner::new_with_cancel_hint(
            if attempt == 0 {
                "Generating commit message..."
            } else {
                "Regenerating commit message..."
            },
            colored,
        );
        spinner.start_time_display();

        let message = provider
            .generate_commit_message(diff, Some(context), Some(&spinner))
            .await?;

        spinner.finish_and_clear();
        Ok((message, false)) // 还没显示
    }
}

/// 格式化消息头部（纯函数，便于测试）
fn format_message_header(attempt: usize) -> String {
    if attempt == 0 {
        "Generated commit message:".to_string()
    } else {
        format!("Regenerated commit message (attempt {}):", attempt + 1)
    }
}

/// 格式化编辑后消息头部（纯函数，便于测试）
fn format_edited_header() -> &'static str {
    "Updated commit message:"
}

/// 显示生成的 message
fn display_message(message: &str, attempt: usize, colored: bool) {
    let header = format_message_header(attempt);

    println!("\n{}", ui::info(&header, colored));
    if colored {
        println!("{}", message.yellow());
    } else {
        println!("{}", message);
    }
}

/// 显示编辑后的 message
fn display_edited_message(message: &str, colored: bool) {
    println!("\n{}", ui::info(format_edited_header(), colored));
    if colored {
        println!("{}", message.yellow());
    } else {
        println!("{}", message);
    }
}

/// 生成 commit message（非流式版本，用于 JSON 输出模式）
async fn generate_message_no_streaming(
    provider: &Arc<dyn LLMProvider>,
    repo: &dyn GitOperations,
    diff: &str,
    stats: &DiffStats,
    config: &AppConfig,
    feedbacks: &[String],
    verbose: bool,
) -> Result<String> {
    let context = CommitContext {
        files_changed: stats.files_changed.clone(),
        insertions: stats.insertions,
        deletions: stats.deletions,
        branch_name: repo.get_current_branch()?,
        custom_prompt: config.commit.custom_prompt.clone(),
        user_feedback: feedbacks.to_vec(),
    };

    // verbose 模式下显示 prompt
    if verbose {
        let (system, user) = crate::llm::prompt::build_commit_prompt_split(
            diff,
            &context,
            context.custom_prompt.as_deref(),
        );
        eprintln!("\n=== Verbose: Generated Prompt ===");
        eprintln!("--- System Prompt ---");
        eprintln!("{}", system);
        eprintln!("--- User Message ---");
        eprintln!("{}", user);
        eprintln!("=================================\n");
    }

    // 直接使用非流式 API
    provider
        .generate_commit_message(diff, Some(context), None)
        .await
}

/// JSON 格式成功输出
fn output_json_success(message: &str, stats: &DiffStats, committed: bool) -> Result<()> {
    let output = CommitJsonOutput {
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

/// JSON 格式错误输出
fn output_json_error(err: &GcopError) -> Result<()> {
    let output = CommitJsonOutput {
        success: false,
        data: None,
        error: Some(ErrorJson::from_error(err)),
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    // === format_message_header 测试 ===

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

    // === format_edited_header 测试 ===

    #[test]
    fn test_format_edited_header() {
        let header = format_edited_header();
        assert_eq!(header, "Updated commit message:");
    }
}
