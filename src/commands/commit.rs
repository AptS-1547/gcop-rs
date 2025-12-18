use std::sync::Arc;

use colored::Colorize;

use crate::cli::Cli;
use crate::config::AppConfig;
use crate::constants::commit::MAX_RETRIES;
use crate::error::{GcopError, Result};
use crate::git::{DiffStats, GitOperations, repository::GitRepository};
use crate::llm::{CommitContext, LLMProvider, provider::create_provider};
use crate::ui;

/// Commit 流程状态机
#[derive(Debug)]
enum CommitState {
    /// 需要生成/重新生成 message
    Generating {
        attempt: usize,
        feedbacks: Vec<String>,
    },
    /// 展示生成的 message 并等待用户操作
    WaitingForAction {
        message: String,
        attempt: usize,
        feedbacks: Vec<String>,
    },
    /// 用户接受，准备提交
    Accepted { message: String },
    /// 用户取消
    Cancelled,
}

/// 执行 commit 命令
///
/// # Arguments
/// * `cli` - CLI 参数
/// * `config` - 应用配置
/// * `no_edit` - 是否跳过编辑
/// * `yes` - 是否跳过确认
pub async fn run(cli: &Cli, config: &AppConfig, no_edit: bool, yes: bool) -> Result<()> {
    let colored = config.ui.colored;

    // 1. 初始化依赖
    let repo = GitRepository::open()?;
    let provider = create_provider(config, cli.provider.as_deref())?;

    // 2. 检查 staged changes
    if !repo.has_staged_changes()? {
        ui::error("No staged changes found. Use 'git add' first.", colored);
        return Err(GcopError::NoStagedChanges);
    }

    // 3. 获取 diff 和统计
    ui::step("1/4", "Analyzing staged changes...", colored);
    let diff = repo.get_staged_diff()?;
    let stats = repo.get_diff_stats(&diff)?;

    // 4. 显示预览（可选）
    if config.commit.show_diff_preview {
        println!("\n{}", ui::format_diff_stats(&stats, colored));
    }

    // 5. 状态机主循环
    let should_edit = config.commit.allow_edit && !no_edit;
    let mut state = CommitState::Generating {
        attempt: 0,
        feedbacks: vec![],
    };

    loop {
        state = match state {
            CommitState::Generating { attempt, feedbacks } => {
                // 检查重试上限
                if attempt >= MAX_RETRIES {
                    ui::warning(
                        &format!("Reached maximum retry limit ({})", MAX_RETRIES),
                        colored,
                    );
                    return Err(GcopError::Other("Too many retries".to_string()));
                }

                // 生成 message
                let message =
                    generate_message(&provider, &repo, &diff, &stats, config, &feedbacks, attempt)
                        .await?;

                // --yes 标志直接接受
                if yes {
                    CommitState::Accepted { message }
                } else {
                    // 显示生成的 message
                    display_message(&message, attempt, colored);
                    CommitState::WaitingForAction {
                        message,
                        attempt,
                        feedbacks,
                    }
                }
            }

            CommitState::WaitingForAction {
                message,
                attempt,
                feedbacks,
            } => {
                ui::step("3/4", "Choose next action...", colored);
                let action = ui::commit_action_menu(&message, should_edit, attempt, colored)?;

                match action {
                    ui::CommitAction::Accept => CommitState::Accepted { message },

                    ui::CommitAction::Edit => {
                        ui::step("3/4", "Opening editor...", colored);
                        match ui::edit_text(&message) {
                            Ok(edited) => {
                                display_edited_message(&edited, colored);
                                // 编辑后返回菜单
                                CommitState::WaitingForAction {
                                    message: edited,
                                    attempt,
                                    feedbacks,
                                }
                            }
                            Err(GcopError::UserCancelled) => {
                                ui::warning("Edit cancelled.", colored);
                                // 取消编辑返回菜单，保留原消息
                                CommitState::WaitingForAction {
                                    message,
                                    attempt,
                                    feedbacks,
                                }
                            }
                            Err(e) => return Err(e),
                        }
                    }

                    ui::CommitAction::Retry => CommitState::Generating {
                        attempt: attempt + 1,
                        feedbacks, // 保留已有 feedback
                    },

                    ui::CommitAction::RetryWithFeedback => {
                        let new_feedback = ui::get_retry_feedback()?;
                        let mut new_feedbacks = feedbacks;
                        if let Some(fb) = new_feedback {
                            new_feedbacks.push(fb);
                        } else {
                            ui::warning(
                                "No feedback provided, will retry with existing instructions.",
                                colored,
                            );
                        }
                        CommitState::Generating {
                            attempt: attempt + 1,
                            feedbacks: new_feedbacks,
                        }
                    }

                    ui::CommitAction::Quit => CommitState::Cancelled,
                }
            }

            CommitState::Accepted { message } => {
                // 执行 commit
                ui::step("4/4", "Creating commit...", colored);
                repo.commit(&message)?;

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
async fn generate_message(
    provider: &Arc<dyn LLMProvider>,
    repo: &GitRepository,
    diff: &str,
    stats: &DiffStats,
    config: &AppConfig,
    feedbacks: &[String],
    attempt: usize,
) -> Result<String> {
    let spinner = ui::Spinner::new(if attempt == 0 {
        "Generating commit message..."
    } else {
        "Regenerating commit message..."
    });

    let context = CommitContext {
        files_changed: stats.files_changed.clone(),
        insertions: stats.insertions,
        deletions: stats.deletions,
        branch_name: repo.get_current_branch()?,
        custom_prompt: config.commit.custom_prompt.clone(),
        user_feedback: feedbacks.to_vec(),
    };

    let message = provider
        .generate_commit_message(diff, Some(context))
        .await?;

    spinner.finish_and_clear();
    Ok(message)
}

/// 显示生成的 message
fn display_message(message: &str, attempt: usize, colored: bool) {
    let header = if attempt == 0 {
        "Generated commit message:".to_string()
    } else {
        format!("Regenerated commit message (attempt {}):", attempt + 1)
    };

    println!("\n{}", ui::info(&header, colored));
    if colored {
        println!("{}", message.yellow());
    } else {
        println!("{}", message);
    }
}

/// 显示编辑后的 message
fn display_edited_message(message: &str, colored: bool) {
    println!("\n{}", ui::info("Updated commit message:", colored));
    if colored {
        println!("{}", message.yellow());
    } else {
        println!("{}", message);
    }
}
