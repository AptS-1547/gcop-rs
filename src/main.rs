mod cli;
mod commands;
mod config;
mod error;
mod git;
mod llm;
mod ui;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use tokio::runtime::Runtime;

fn main() -> Result<()> {
    // 先解析 CLI 参数
    let cli = Cli::parse();

    // 根据 verbose 标志设置日志级别
    let log_level = if cli.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    // 初始化 tracing 日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive(log_level.into()),
        )
        .init();

    // 判断是否需要加载配置
    // config/init/alias 命令不需要完整配置，可以在配置损坏时运行
    let needs_config = matches!(
        &cli.command,
        Commands::Commit { .. } | Commands::Review { .. }
    );

    // 加载配置（管理命令使用默认配置，允许在配置损坏时运行）
    let config = if needs_config {
        config::load_config()?
    } else {
        config::load_config().unwrap_or_default()
    };

    // 创建 tokio 运行时
    let rt = Runtime::new()?;

    // 根据子命令路由
    rt.block_on(async {
        match cli.command {
            Commands::Commit {
                no_edit,
                yes,
                dry_run,
                ref format,
                json,
                ref feedback,
            } => {
                let effective_format = if json { "json" } else { format.as_str() };
                let is_json = effective_format == "json";
                // 执行 commit 命令
                if let Err(e) = commands::commit::run(
                    &cli,
                    &config,
                    no_edit,
                    yes,
                    dry_run,
                    effective_format,
                    feedback.clone(),
                )
                .await
                {
                    // JSON 模式下，错误已经输出过 JSON 了，直接退出
                    if is_json {
                        std::process::exit(1);
                    }
                    // 错误处理
                    match e {
                        error::GcopError::UserCancelled => {
                            // 用户取消不算错误，正常退出
                            std::process::exit(0);
                        }
                        error::GcopError::NoStagedChanges => {
                            // NoStagedChanges 错误已经在 commit.rs 中输出过了
                            std::process::exit(1);
                        }
                        _ => {
                            ui::error(&format!("Error: {}", e), config.ui.colored);
                            if let Some(suggestion) = e.suggestion() {
                                println!();
                                println!(
                                    "{}",
                                    ui::info(&format!("Tip: {}", suggestion), config.ui.colored)
                                );
                            }
                            std::process::exit(1);
                        }
                    }
                }
                Ok(())
            }
            Commands::Review {
                ref target,
                ref format,
                json,
            } => {
                let effective_format = if json { "json" } else { format.as_str() };
                let is_json = effective_format == "json";
                // 执行 review 命令
                if let Err(e) = commands::review::run(&cli, &config, target, effective_format).await
                {
                    // JSON 模式下输出 JSON 错误
                    if is_json {
                        let _ = commands::review::output_json_error(&e);
                        std::process::exit(1);
                    }
                    // 错误处理
                    match e {
                        error::GcopError::UserCancelled => {
                            std::process::exit(0);
                        }
                        _ => {
                            ui::error(&format!("Error: {}", e), config.ui.colored);
                            if let Some(suggestion) = e.suggestion() {
                                println!();
                                println!(
                                    "{}",
                                    ui::info(&format!("Tip: {}", suggestion), config.ui.colored)
                                );
                            }
                            std::process::exit(1);
                        }
                    }
                }
                Ok(())
            }
            Commands::Init { force } => {
                if let Err(e) = commands::init::run(force, config.ui.colored) {
                    ui::error(&format!("Error: {}", e), config.ui.colored);
                    if let Some(suggestion) = e.suggestion() {
                        println!();
                        println!(
                            "{}",
                            ui::info(&format!("Tip: {}", suggestion), config.ui.colored)
                        );
                    }
                    std::process::exit(1);
                }
                Ok(())
            }
            Commands::Config { action } => {
                if let Err(e) = commands::config::run(action, config.ui.colored).await {
                    ui::error(&format!("Error: {}", e), config.ui.colored);
                    if let Some(suggestion) = e.suggestion() {
                        println!();
                        println!(
                            "{}",
                            ui::info(&format!("Tip: {}", suggestion), config.ui.colored)
                        );
                    }
                    std::process::exit(1);
                }
                Ok(())
            }
            Commands::Alias {
                force,
                list,
                remove,
            } => {
                if let Err(e) = commands::alias::run(force, list, remove, config.ui.colored) {
                    ui::error(&format!("Error: {}", e), config.ui.colored);
                    if let Some(suggestion) = e.suggestion() {
                        println!();
                        println!(
                            "{}",
                            ui::info(&format!("Tip: {}", suggestion), config.ui.colored)
                        );
                    }
                    std::process::exit(1);
                }
                Ok(())
            }
            Commands::Stats {
                ref format,
                json,
                ref author,
            } => {
                let effective_format = if json { "json" } else { format.as_str() };
                let is_json = effective_format == "json";
                if let Err(e) =
                    commands::stats::run(effective_format, author.as_deref(), config.ui.colored)
                {
                    // JSON 模式下输出 JSON 错误
                    if is_json {
                        let _ = commands::stats::output_json_error(&e);
                        std::process::exit(1);
                    }
                    ui::error(&format!("Error: {}", e), config.ui.colored);
                    if let Some(suggestion) = e.suggestion() {
                        println!();
                        println!(
                            "{}",
                            ui::info(&format!("Tip: {}", suggestion), config.ui.colored)
                        );
                    }
                    std::process::exit(1);
                }
                Ok(())
            }
        }
    })
}
