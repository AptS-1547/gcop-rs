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
    ui::step("1/5", "Analyzing staged changes...", colored);
    let diff = repo.get_staged_diff()?;
    let stats = repo.get_diff_stats(&diff)?;

    // 4. 显示预览（可选）
    if config.commit.show_diff_preview {
        println!("\n{}", ui::format_diff_stats(&stats, colored));
    }

    // 5. 生成 commit message
    let spinner = ui::Spinner::new("Generating commit message...");

    let context = CommitContext {
        files_changed: stats.files_changed.clone(),
        insertions: stats.insertions,
        deletions: stats.deletions,
        branch_name: repo.get_current_branch()?,
    };

    let mut message = provider
        .generate_commit_message(&diff, Some(context))
        .await?;

    spinner.finish_and_clear();

    // 6. 显示生成的消息
    println!("\n{}", ui::info("Generated commit message:", colored));
    println!("{}", message);

    // 7. 允许编辑（可选）
    let should_edit = config.commit.allow_edit && !no_edit;

    if should_edit {
        ui::step("3/5", "Opening editor...", colored);
        match ui::edit_text(&message) {
            Ok(edited) => {
                message = edited;
                println!("\n{}", ui::info("Updated commit message:", colored));
                println!("{}", message);
            }
            Err(GcopError::UserCancelled) => {
                ui::warning("Edit cancelled.", colored);
                return Err(GcopError::UserCancelled);
            }
            Err(e) => return Err(e),
        }
    }

    // 8. 确认提交（可选）
    let should_confirm = config.commit.confirm_before_commit && !yes;

    if should_confirm {
        let step_num = if should_edit { "4/5" } else { "3/5" };
        ui::step(step_num, "Confirming...", colored);

        if !ui::confirm("Proceed with commit?", true)? {
            ui::warning("Commit cancelled.", colored);
            return Err(GcopError::UserCancelled);
        }
    }

    // 9. 执行 commit
    let step_num = if should_edit && should_confirm {
        "5/5"
    } else if should_edit || should_confirm {
        "4/5"
    } else {
        "3/5"
    };
    ui::step(step_num, "Creating commit...", colored);

    repo.commit(&message)?;

    // 10. 显示成功
    println!();
    ui::success("Commit created successfully!", colored);
    if cli.verbose {
        println!("\n{}", message);
    }

    Ok(())
}
