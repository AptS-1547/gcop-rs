use colored::Colorize;

use crate::cli::Cli;
use crate::config::AppConfig;
use crate::error::{GcopError, Result};
use crate::git::{GitOperations, repository::GitRepository};
use crate::llm::{CommitContext, provider::create_provider};
use crate::ui;

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

    // 5. 生成 commit message（支持重试循环）
    let should_edit = config.commit.allow_edit && !no_edit;
    let mut message = String::new();
    let mut user_feedback: Option<String> = None;
    let mut retry_count = 0;
    let mut should_regenerate = true;
    let mut should_display = true;
    const MAX_RETRIES: usize = 10;

    loop {
        // 只在需要时生成或重新生成 commit message
        if should_regenerate {
            let spinner = ui::Spinner::new(if retry_count == 0 {
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
                user_feedback: user_feedback.clone(),
            };

            message = provider
                .generate_commit_message(&diff, Some(context))
                .await?;

            spinner.finish_and_clear();

            should_regenerate = false;
        }

        // 只在需要时显示消息
        if should_display {
            if retry_count == 0 {
                println!("\n{}", ui::info("Generated commit message:", colored));
            } else {
                println!(
                    "\n{}",
                    ui::info(
                        &format!("Regenerated commit message (attempt {}):", retry_count + 1),
                        colored
                    )
                );
            }
            if colored {
                println!("{}", message.yellow());
            } else {
                println!("{}", message);
            }

            should_display = false;
        }

        // 如果有 --yes 标志，跳过菜单直接接受
        if yes {
            break;
        }

        // 显示交互式菜单
        ui::step("3/4", "Choose next action...", colored);

        let action = ui::commit_action_menu(&message, should_edit, retry_count, colored)?;

        match action {
            ui::CommitAction::Accept => {
                // 接受，跳出循环
                break;
            }
            ui::CommitAction::Edit => {
                // 手动编辑
                ui::step("3/4", "Opening editor...", colored);
                match ui::edit_text(&message) {
                    Ok(edited) => {
                        message = edited;
                        println!("\n{}", ui::info("Updated commit message:", colored));
                        if colored {
                            println!("{}", message.yellow());
                        } else {
                            println!("{}", message);
                        }
                        continue;
                    }
                    Err(GcopError::UserCancelled) => {
                        ui::warning("Edit cancelled.", colored);
                        return Err(GcopError::UserCancelled);
                    }
                    Err(e) => return Err(e),
                }
            }
            ui::CommitAction::Retry => {
                // 重试，清空反馈
                user_feedback = None;
                retry_count += 1;
                should_regenerate = true;
                should_display = true;

                if retry_count >= MAX_RETRIES {
                    ui::warning(
                        &format!("Reached maximum retry limit ({})", MAX_RETRIES),
                        colored,
                    );
                    return Err(GcopError::Other("Too many retries".to_string()));
                }

                continue;
            }
            ui::CommitAction::RetryWithFeedback => {
                // 重试并获取反馈
                match ui::get_retry_feedback()? {
                    Some(feedback) => {
                        user_feedback = Some(feedback);
                    }
                    None => {
                        ui::warning(
                            "No feedback provided, will retry without additional instructions.",
                            colored,
                        );
                        user_feedback = None;
                    }
                }

                retry_count += 1;
                should_regenerate = true;
                should_display = true;

                if retry_count >= MAX_RETRIES {
                    ui::warning(
                        &format!("Reached maximum retry limit ({})", MAX_RETRIES),
                        colored,
                    );
                    return Err(GcopError::Other("Too many retries".to_string()));
                }

                continue;
            }
            ui::CommitAction::Quit => {
                ui::warning("Commit cancelled by user.", colored);
                return Err(GcopError::UserCancelled);
            }
        }
    }

    // 7. 执行 commit
    ui::step("4/4", "Creating commit...", colored);

    repo.commit(&message)?;

    // 8. 显示成功
    println!();
    ui::success("Commit created successfully!", colored);
    if cli.verbose {
        println!("\n{}", message);
    }

    Ok(())
}
