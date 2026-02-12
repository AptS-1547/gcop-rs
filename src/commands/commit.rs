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
use crate::llm::{CommitContext, LLMProvider, ScopeInfo, provider::create_provider};
use crate::ui;

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
/// * `options` - Commit 命令选项
/// * `config` - 应用配置
pub async fn run(options: &CommitOptions<'_>, config: &AppConfig) -> Result<()> {
    let repo = GitRepository::open(None)?;
    let provider = create_provider(config, options.provider_override)?;

    run_with_deps(options, config, &repo as &dyn GitOperations, &provider).await
}

/// 执行 commit 命令（可测试版本，接受 trait 对象）
#[allow(dead_code)] // 供测试使用
async fn run_with_deps(
    options: &CommitOptions<'_>,
    config: &AppConfig,
    repo: &dyn GitOperations,
    provider: &Arc<dyn LLMProvider>,
) -> Result<()> {
    let colored = options.effective_colored(config);

    // 合并命令行参数为一条反馈（便于不加引号时使用）
    // e.g. `gcop-rs commit use Chinese` -> "use Chinese"
    let initial_feedbacks = if options.feedback.is_empty() {
        vec![]
    } else {
        vec![options.feedback.join(" ")]
    };

    // JSON 模式：独立流程
    if options.format.is_json() {
        return handle_json_mode(options, config, repo, provider, &initial_feedbacks).await;
    }

    // 检查 staged changes
    if !repo.has_staged_changes()? {
        ui::error(&rust_i18n::t!("commit.no_staged_changes"), colored);
        return Err(GcopError::NoStagedChanges);
    }

    // 获取 diff 和统计
    let diff = repo.get_staged_diff()?;
    let stats = repo.get_diff_stats(&diff)?;

    // 截断过大的 diff，防止 token 超限
    let (diff, truncated) = smart_truncate_diff(&diff, config.llm.max_diff_size);
    if truncated {
        ui::warning(&rust_i18n::t!("diff.truncated"), colored);
    }

    // Workspace scope 检测
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

    // dry_run 模式：只生成不提交
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

    // 交互模式：状态机主循环
    let should_edit = config.commit.allow_edit && !options.no_edit;
    let max_retries = config.commit.max_retries;

    // 提取循环中不变的上下文（branch_name、custom_prompt 不会随 retry 变化）
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
                repo.commit(message)?;
                println!();
                ui::success(&rust_i18n::t!("commit.success"), colored);
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

/// JSON 模式的完整处理流程
async fn handle_json_mode(
    options: &CommitOptions<'_>,
    config: &AppConfig,
    repo: &dyn GitOperations,
    provider: &Arc<dyn LLMProvider>,
    initial_feedbacks: &[String],
) -> Result<()> {
    if !repo.has_staged_changes()? {
        json::output_json_error::<CommitData>(&GcopError::NoStagedChanges)?;
        return Err(GcopError::NoStagedChanges);
    }

    let diff = repo.get_staged_diff()?;
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

/// 处理 Generating 状态
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
    // 检查重试上限
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

    // 生成 message
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

    // 使用状态机方法处理生成结果
    let gen_state = CommitState::Generating { attempt, feedbacks };
    let result = GenerationResult::Success(message.clone());
    let next_state = gen_state.handle_generation(result, options.yes)?;

    // 显示生成的消息（除非 --yes 直接接受，或流式模式已经显示过）
    if !options.yes && !already_displayed {
        display_message(&message, attempt, colored);
    }

    Ok(next_state)
}

/// 处理 WaitingForAction 状态
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

    // 映射 UI action 到状态机 action，处理编辑逻辑
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

/// 生成 commit message
///
/// 返回 (message, already_displayed) - 流式模式下 message 已经显示过了
#[allow(clippy::too_many_arguments)] // 参数较多但合理
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

    // verbose 模式下显示 prompt
    if verbose {
        let (system, user) = crate::llm::prompt::build_commit_prompt_split(
            diff,
            &context,
            context.custom_prompt.as_deref(),
            context.convention.as_ref(),
        );
        println!(
            "\n{}",
            rust_i18n::t!("commit.verbose.generated_prompt")
                .cyan()
                .bold()
        );
        println!("{}", rust_i18n::t!("commit.verbose.system_prompt").cyan());
        println!("{}", system);
        println!("{}", rust_i18n::t!("commit.verbose.user_message").cyan());
        println!("{}", user);
        println!(
            "{}\n",
            rust_i18n::t!("commit.verbose.divider").cyan().bold()
        );
    }

    // 判断是否使用流式模式
    let use_streaming = config.ui.streaming && provider.supports_streaming();
    let colored = config.ui.colored;

    if use_streaming {
        // 流式模式：先显示标题，再流式输出
        let step_msg = if attempt == 0 {
            rust_i18n::t!("spinner.generating_streaming")
        } else {
            rust_i18n::t!("spinner.regenerating_streaming")
        };
        ui::step(&rust_i18n::t!("commit.step2"), &step_msg, colored);
        println!("\n{}", ui::info(&format_message_header(attempt), colored));

        let stream_handle = provider
            .generate_commit_message_streaming(diff, Some(context))
            .await?;

        let mut output = ui::StreamingOutput::new(colored);
        let message = output.process(stream_handle.receiver).await?;

        Ok((message, true)) // 已经显示过了
    } else {
        // 非流式模式：使用 Spinner（带取消提示和时间显示）
        let spinner_message = if attempt == 0 {
            rust_i18n::t!("spinner.generating").to_string()
        } else {
            rust_i18n::t!("spinner.regenerating").to_string()
        };
        let mut spinner = ui::Spinner::new_with_cancel_hint(&spinner_message, colored);
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
        rust_i18n::t!("commit.generated").to_string()
    } else {
        rust_i18n::t!("commit.regenerated", attempt = attempt + 1).to_string()
    }
}

/// 格式化编辑后消息头部（纯函数，便于测试）
fn format_edited_header() -> String {
    rust_i18n::t!("commit.updated").to_string()
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
    println!("\n{}", ui::info(&format_edited_header(), colored));
    if colored {
        println!("{}", message.yellow());
    } else {
        println!("{}", message);
    }
}

/// 生成 commit message（非流式版本，用于 JSON 输出模式）
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

    // verbose 模式下显示 prompt
    if verbose {
        let (system, user) = crate::llm::prompt::build_commit_prompt_split(
            diff,
            &context,
            context.custom_prompt.as_deref(),
            context.convention.as_ref(),
        );
        eprintln!("\n{}", rust_i18n::t!("commit.verbose.generated_prompt"));
        eprintln!("{}", rust_i18n::t!("commit.verbose.system_prompt"));
        eprintln!("{}", system);
        eprintln!("{}", rust_i18n::t!("commit.verbose.user_message"));
        eprintln!("{}", user);
        eprintln!("{}\n", rust_i18n::t!("commit.verbose.divider"));
    }

    // 直接使用非流式 API
    provider
        .generate_commit_message(diff, Some(context), None)
        .await
}

/// JSON 格式成功输出
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

/// 计算 workspace scope 信息
///
/// 从 git root 检测 workspace 配置，推断 changed files 的 scope。
/// 支持手动配置覆盖自动检测。检测失败时返回 None（非致命）。
fn compute_scope_info(files_changed: &[String], config: &AppConfig) -> Option<ScopeInfo> {
    if !config.workspace.enabled {
        return None;
    }

    let root = crate::git::find_git_root()?;

    // 构建 WorkspaceInfo：手动配置优先，否则自动检测
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

    // 输出检测结果
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

    // 应用 scope_mappings 重映射
    let suggested = scope.suggested_scope.map(|s| {
        config
            .workspace
            .scope_mappings
            .get(&s)
            .cloned()
            .unwrap_or(s)
    });

    if let Some(ref s) = suggested {
        tracing::debug!(
            "{}",
            rust_i18n::t!("workspace.scope_suggestion", scope = s)
        );
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
