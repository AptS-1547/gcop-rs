mod cli;
mod commands;
mod config;
mod error;
mod git;
mod llm;
mod ui;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands, ReviewTarget};
use tokio::runtime::Runtime;

fn main() -> Result<()> {
    // 初始化 tracing 日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    // 解析 CLI 参数
    let cli = Cli::parse();

    // 加载配置
    let config = config::load_config()?;

    // 创建 tokio 运行时
    let rt = Runtime::new()?;

    // 根据子命令路由
    rt.block_on(async {
        match cli.command {
            Commands::Commit { no_edit, yes } => {
                // 执行 commit 命令
                if let Err(e) = commands::commit::run(&cli, &config, no_edit, yes).await {
                    // 错误处理
                    match e {
                        error::GcopError::UserCancelled => {
                            // 用户取消不算错误，正常退出
                            std::process::exit(0);
                        }
                        error::GcopError::NoStagedChanges => {
                            ui::error(&e.to_string(), config.ui.colored);
                            std::process::exit(1);
                        }
                        _ => {
                            ui::error(&format!("Error: {}", e), config.ui.colored);
                            std::process::exit(1);
                        }
                    }
                }
                Ok(())
            }
            Commands::Review { target, format } => {
                println!("gcop review 命令");
                println!("  --format: {}", format);
                match target {
                    ReviewTarget::Changes => {
                        println!("  Target: 未提交的变更");
                    }
                    ReviewTarget::Commit { hash } => {
                        println!("  Target: Commit {}", hash);
                    }
                    ReviewTarget::Range { range } => {
                        println!("  Target: Range {}", range);
                    }
                    ReviewTarget::File { path } => {
                        println!("  Target: File/Dir {}", path);
                    }
                }
                println!("\n[Not implemented yet]");
                Ok(())
            }
        }
    })
}
