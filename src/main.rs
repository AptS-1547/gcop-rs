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
    // 0. Install rustls crypto provider
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("Failed to install rustls crypto provider"))?;

    // 1. Load configuration (load once, reuse globally)
    //    Save the Result and reuse it when successful. When it fails, follow the command to decide whether to report an error.
    let config_result = config::load_config();

    // Locale initialization uses default values ​​to ensure that it does not fail due to configuration corruption.
    let early_config = config_result.as_ref().cloned().unwrap_or_default();

    // 2. Initialize language (needs to be completed before CLI parsing, supports multi-language help text)
    init_locale(&early_config);

    // 3. Parse CLI parameters and inject internationalized help text
    let cli = parse_cli_localized()?;

    // Set log level based on verbose flag
    let log_level = if cli.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    // Initialize tracing log
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive(log_level.into()),
        )
        .init();

    // 4. The commit/review command requires complete configuration (provider, etc.), and an error will occur if the configuration is damaged.
    //    Other commands can use the fallback default value.
    let config = if matches!(
        &cli.command,
        Commands::Commit { .. } | Commands::Review { .. } | Commands::Hook { .. }
    ) {
        config_result?
    } else {
        early_config
    };

    // Create tokio runtime
    let rt = Runtime::new()?;

    // Route based on subcommand
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
                let options = commands::CommitOptions::from_cli(
                    &cli, no_edit, yes, dry_run, format, json, feedback,
                );
                let is_json = options.format.is_json();
                if let Err(e) = commands::commit::run(&options, &config).await {
                    if is_json {
                        // JSON errors are printed inside the commit command
                        std::process::exit(1);
                    }
                    match e {
                        error::GcopError::UserCancelled => std::process::exit(0),
                        error::GcopError::NoStagedChanges => std::process::exit(1),
                        _ => handle_command_error(&e, config.ui.colored),
                    }
                }
                Ok(())
            }
            Commands::Review {
                ref target,
                ref format,
                json,
            } => {
                let options = commands::ReviewOptions::from_cli(&cli, target, format, json);
                if let Err(e) = commands::review::run(&options, &config).await {
                    if options.format.is_json() {
                        // JSON errors are printed inside the review command
                        std::process::exit(1);
                    }
                    if matches!(e, error::GcopError::UserCancelled) {
                        std::process::exit(0);
                    }
                    handle_command_error(&e, config.ui.colored);
                }
                Ok(())
            }
            Commands::Init { force, project } => {
                if let Err(e) = commands::init::run(force, project, config.ui.colored) {
                    handle_command_error(&e, config.ui.colored);
                }
                Ok(())
            }
            Commands::Config { action } => {
                if let Err(e) = commands::config::run(action, config.ui.colored).await {
                    handle_command_error(&e, config.ui.colored);
                }
                Ok(())
            }
            Commands::Alias {
                force,
                list,
                remove,
            } => {
                if let Err(e) = commands::alias::run(force, list, remove, config.ui.colored) {
                    handle_command_error(&e, config.ui.colored);
                }
                Ok(())
            }
            Commands::Stats {
                ref format,
                json,
                ref author,
            } => {
                let options = commands::StatsOptions::from_cli(format, json, author.as_deref());
                if let Err(e) = commands::stats::run(&options, config.ui.colored) {
                    if options.format.is_json() {
                        // JSON errors have been printed inside the stats command
                        std::process::exit(1);
                    }
                    handle_command_error(&e, config.ui.colored);
                }
                Ok(())
            }
            Commands::Hook { ref action } => {
                match action {
                    cli::HookAction::Install { force } => {
                        if let Err(e) = commands::hook::install(*force) {
                            handle_command_error(&e, config.ui.colored);
                        }
                    }
                    cli::HookAction::Uninstall => {
                        if let Err(e) = commands::hook::uninstall() {
                            handle_command_error(&e, config.ui.colored);
                        }
                    }
                    cli::HookAction::Run {
                        commit_msg_file,
                        source,
                        sha: _,
                    } => {
                        commands::hook::run_hook_safe(
                            commit_msg_file,
                            source,
                            &config,
                            cli.verbose,
                            cli.provider.as_deref(),
                        )
                        .await;
                    }
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
                .mut_arg("project", |arg| {
                    arg.help(rust_i18n::t!("cli.init.project").to_string())
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
        })
        .mut_subcommand("hook", |cmd| {
            cmd.about(rust_i18n::t!("cli.hook").to_string())
                .mut_subcommand("install", |s| {
                    s.about(rust_i18n::t!("cli.hook.install").to_string())
                        .mut_arg("force", |arg| {
                            arg.help(rust_i18n::t!("cli.hook.install.force").to_string())
                        })
                })
                .mut_subcommand("uninstall", |s| {
                    s.about(rust_i18n::t!("cli.hook.uninstall").to_string())
                })
        });

    let matches = cmd.get_matches();
    Cli::from_arg_matches(&matches)
        .map_err(|e| anyhow::anyhow!("Failed to parse CLI arguments: {}", e))
}

/// Initialize locale from loaded config
///
/// Priority order:
/// 1. `config.ui.language` (already includes `GCOP__UI__LANGUAGE` override)
/// 2. System locale detection
/// 3. Fallback to English
fn init_locale(config: &config::AppConfig) {
    let locale = config
        .ui
        .language
        .clone()
        .or_else(detect_system_locale)
        .unwrap_or_else(|| "en".to_string());

    rust_i18n::set_locale(&locale);
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

/// Show error message + suggestions, then exit
fn handle_command_error(e: &error::GcopError, colored: bool) -> ! {
    ui::error(&e.localized_message(), colored);
    if let Some(suggestion) = e.localized_suggestion() {
        println!();
        println!("{}", ui::info(&suggestion, colored));
    }
    std::process::exit(1);
}
