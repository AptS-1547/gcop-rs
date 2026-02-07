#[macro_use]
extern crate rust_i18n;

// Re-export all library modules
use gcop_rs::*;

use anyhow::Result;
use clap::{CommandFactory, FromArgMatches};
use cli::{Cli, Commands};
use tokio::runtime::Runtime;

// Initialize i18n for binary crate
// This ensures translations are available in main.rs context
i18n!("locales", fallback = "en");

fn main() -> Result<()> {
    // 在解析 CLI 之前初始化语言（支持多语言 help text）
    init_locale_early();

    // 解析 CLI 参数并注入国际化 help text
    let cli = parse_cli_localized()?;

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
                // 使用 CommitOptions 聚合参数
                let options = commands::CommitOptions::from_cli(
                    &cli, no_edit, yes, dry_run, format, json, feedback,
                );
                let is_json = options.format.is_json();
                // 执行 commit 命令
                if let Err(e) = commands::commit::run(&options, &config).await {
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
                            ui::error(&e.localized_message(), config.ui.colored);
                            if let Some(suggestion) = e.localized_suggestion() {
                                println!();
                                println!("{}", ui::info(&suggestion, config.ui.colored));
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
                // 使用 ReviewOptions 聚合参数
                let options = commands::ReviewOptions::from_cli(&cli, target, format, json);
                let is_json = options.format.is_json();
                // 执行 review 命令
                if let Err(e) = commands::review::run(&options, &config).await {
                    // JSON 模式下输出 JSON 错误
                    if is_json {
                        let _ = commands::json::output_json_error::<llm::ReviewResult>(&e);
                        std::process::exit(1);
                    }
                    // 错误处理
                    match e {
                        error::GcopError::UserCancelled => {
                            std::process::exit(0);
                        }
                        _ => {
                            ui::error(&e.localized_message(), config.ui.colored);
                            if let Some(suggestion) = e.localized_suggestion() {
                                println!();
                                println!("{}", ui::info(&suggestion, config.ui.colored));
                            }
                            std::process::exit(1);
                        }
                    }
                }
                Ok(())
            }
            Commands::Init { force } => {
                if let Err(e) = commands::init::run(force, config.ui.colored) {
                    ui::error(&e.localized_message(), config.ui.colored);
                    if let Some(suggestion) = e.localized_suggestion() {
                        println!();
                        println!("{}", ui::info(&suggestion, config.ui.colored));
                    }
                    std::process::exit(1);
                }
                Ok(())
            }
            Commands::Config { action } => {
                if let Err(e) = commands::config::run(action, config.ui.colored).await {
                    ui::error(&e.localized_message(), config.ui.colored);
                    if let Some(suggestion) = e.localized_suggestion() {
                        println!();
                        println!("{}", ui::info(&suggestion, config.ui.colored));
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
                    ui::error(&e.localized_message(), config.ui.colored);
                    if let Some(suggestion) = e.localized_suggestion() {
                        println!();
                        println!("{}", ui::info(&suggestion, config.ui.colored));
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
                // 使用 StatsOptions 聚合参数
                let options = commands::StatsOptions::from_cli(format, json, author.as_deref());
                let is_json = options.format.is_json();
                if let Err(e) = commands::stats::run(&options, config.ui.colored) {
                    // JSON 模式下输出 JSON 错误
                    if is_json {
                        let _ = commands::json::output_json_error::<commands::stats::RepoStats>(&e);
                        std::process::exit(1);
                    }
                    ui::error(&e.localized_message(), config.ui.colored);
                    if let Some(suggestion) = e.localized_suggestion() {
                        println!();
                        println!("{}", ui::info(&suggestion, config.ui.colored));
                    }
                    std::process::exit(1);
                }
                Ok(())
            }
        }
    })
}

/// Parse CLI arguments with localized help text
///
/// Uses clap's derive + runtime override pattern:
/// 1. Get Command from derive macro (type-safe parsing)
/// 2. Override help text at runtime with rust_i18n::t!()
/// 3. Parse and reconstruct the Cli struct
fn parse_cli_localized() -> Result<Cli> {
    let cmd = Cli::command()
        .about(rust_i18n::t!("cli.about").to_string())
        .mut_arg("verbose", |arg| {
            arg.help(rust_i18n::t!("cli.verbose").to_string())
        })
        .mut_arg("provider", |arg| {
            arg.help(rust_i18n::t!("cli.provider").to_string())
        })
        .mut_subcommand("commit", |cmd| {
            cmd.about(rust_i18n::t!("cli.commit").to_string())
                .mut_arg("no_edit", |arg| {
                    arg.help(rust_i18n::t!("cli.commit.no_edit").to_string())
                })
                .mut_arg("yes", |arg| {
                    arg.help(rust_i18n::t!("cli.commit.yes").to_string())
                })
                .mut_arg("dry_run", |arg| {
                    arg.help(rust_i18n::t!("cli.commit.dry_run").to_string())
                })
                .mut_arg("format", |arg| {
                    arg.help(rust_i18n::t!("cli.commit.format").to_string())
                })
                .mut_arg("json", |arg| {
                    arg.help(rust_i18n::t!("cli.commit.json").to_string())
                })
                .mut_arg("feedback", |arg| {
                    arg.help(rust_i18n::t!("cli.commit.feedback").to_string())
                })
        })
        .mut_subcommand("review", |cmd| {
            cmd.about(rust_i18n::t!("cli.review").to_string())
                .mut_arg("format", |arg| {
                    arg.help(rust_i18n::t!("cli.review.format").to_string())
                })
                .mut_arg("json", |arg| {
                    arg.help(rust_i18n::t!("cli.review.json").to_string())
                })
                .mut_subcommand("changes", |s| {
                    s.about(rust_i18n::t!("cli.review.changes").to_string())
                })
                .mut_subcommand("commit", |s| {
                    s.about(rust_i18n::t!("cli.review.commit").to_string())
                        .mut_arg("hash", |arg| {
                            arg.help(rust_i18n::t!("cli.review.commit.hash").to_string())
                        })
                })
                .mut_subcommand("range", |s| {
                    s.about(rust_i18n::t!("cli.review.range").to_string())
                        .mut_arg("range", |arg| {
                            arg.help(rust_i18n::t!("cli.review.range.range").to_string())
                        })
                })
                .mut_subcommand("file", |s| {
                    s.about(rust_i18n::t!("cli.review.file").to_string())
                        .mut_arg("path", |arg| {
                            arg.help(rust_i18n::t!("cli.review.file.path").to_string())
                        })
                })
        })
        .mut_subcommand("init", |cmd| {
            cmd.about(rust_i18n::t!("cli.init").to_string())
                .mut_arg("force", |arg| {
                    arg.help(rust_i18n::t!("cli.init.force").to_string())
                })
        })
        .mut_subcommand("config", |cmd| {
            cmd.about(rust_i18n::t!("cli.config").to_string())
                .mut_subcommand("edit", |s| {
                    s.about(rust_i18n::t!("cli.config.edit").to_string())
                })
                .mut_subcommand("validate", |s| {
                    s.about(rust_i18n::t!("cli.config.validate").to_string())
                })
        })
        .mut_subcommand("alias", |cmd| {
            cmd.about(rust_i18n::t!("cli.alias").to_string())
                .mut_arg("force", |arg| {
                    arg.help(rust_i18n::t!("cli.alias.force").to_string())
                })
                .mut_arg("list", |arg| {
                    arg.help(rust_i18n::t!("cli.alias.list").to_string())
                })
                .mut_arg("remove", |arg| {
                    arg.help(rust_i18n::t!("cli.alias.remove").to_string())
                })
        })
        .mut_subcommand("stats", |cmd| {
            cmd.about(rust_i18n::t!("cli.stats").to_string())
                .mut_arg("format", |arg| {
                    arg.help(rust_i18n::t!("cli.stats.format").to_string())
                })
                .mut_arg("json", |arg| {
                    arg.help(rust_i18n::t!("cli.stats.json").to_string())
                })
                .mut_arg("author", |arg| {
                    arg.help(rust_i18n::t!("cli.stats.author").to_string())
                })
        });

    let matches = cmd.get_matches();
    Cli::from_arg_matches(&matches)
        .map_err(|e| anyhow::anyhow!("Failed to parse CLI arguments: {}", e))
}

/// Initialize locale early in the startup process
///
/// Priority order:
/// 1. Environment variable GCOP_UI_LANGUAGE (highest priority)
/// 2. Configuration file ui.language
/// 3. System locale detection
/// 4. Fallback to English
fn init_locale_early() {
    let locale = std::env::var("GCOP_UI_LANGUAGE")
        .ok()
        .or_else(|| get_language_from_config().ok())
        .or_else(detect_system_locale)
        .unwrap_or_else(|| "en".to_string());

    rust_i18n::set_locale(&locale);
}

/// Attempt to read language setting from config file
///
/// This is a lightweight read that only parses the ui.language field
/// without loading the entire configuration or validating providers.
fn get_language_from_config() -> Result<String> {
    use directories::ProjectDirs;

    // Get config path (same logic as config::get_config_path)
    let config_path = ProjectDirs::from("", "", "gcop")
        .map(|dirs| dirs.config_dir().join("config.toml"))
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;

    if !config_path.exists() {
        return Err(anyhow::anyhow!("Config file not found"));
    }

    let content = std::fs::read_to_string(&config_path)?;
    let config: toml::Value = toml::from_str(&content)?;

    // Extract ui.language if present
    if let Some(language) = config
        .get("ui")
        .and_then(|ui| ui.get("language"))
        .and_then(|lang| lang.as_str())
    {
        Ok(language.to_string())
    } else {
        Err(anyhow::anyhow!("ui.language not found in config"))
    }
}

/// Detect system locale using sys-locale crate
///
/// Returns locale in BCP 47 format (e.g., "en", "zh-CN", "ja-JP")
fn detect_system_locale() -> Option<String> {
    sys_locale::get_locale().map(|locale| {
        // Normalize locale format: "zh_CN" -> "zh-CN"
        locale.replace('_', "-")
    })
}
