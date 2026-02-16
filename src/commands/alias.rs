use colored::Colorize;

use crate::error::{GcopError, Result};
use crate::ui;
use std::process::Command;
use which::which;

// Complete list of git aliases (14, based on original project + review)
const GCOP_ALIASES: &[(&str, &str, &str)] = &[
    ("cop", "!gcop-rs", "alias.desc.cop"),
    ("gcommit", "!gcop-rs commit", "alias.desc.gcommit"),
    ("c", "!gcop-rs commit", "alias.desc.c"),
    ("r", "!gcop-rs review", "alias.desc.r"),
    ("s", "!gcop-rs stats", "alias.desc.s"),
    ("ac", "!git add -A && gcop-rs commit", "alias.desc.ac"),
    ("cp", "!gcop-rs commit && git push", "alias.desc.cp"),
    (
        "acp",
        "!git add -A && gcop-rs commit && git push",
        "alias.desc.acp",
    ),
    ("amend", "!git commit --amend", "alias.desc.amend"),
    ("ghelp", "!gcop-rs --help", "alias.desc.ghelp"),
    ("gconfig", "!gcop-rs config edit", "alias.desc.gconfig"),
    ("p", "!git push", "alias.desc.p"),
    ("pf", "!git push --force-with-lease", "alias.desc.pf"),
    ("undo", "!git reset --soft HEAD^", "alias.desc.undo"),
];

/// Managing git aliases
pub fn run(force: bool, list: bool, remove: bool, colored: bool) -> Result<()> {
    if list {
        return list_aliases(colored);
    }

    if remove {
        return remove_aliases(force, colored);
    }

    // Default: Install all aliases in batches
    install_all(force, colored)
}

/// Install all git aliases in batches (public, for init calls)
pub fn install_all(force: bool, colored: bool) -> Result<()> {
    // 1. Detect gcop-rs command
    if !is_gcop_in_path() {
        ui::error(&rust_i18n::t!("alias.not_found"), colored);
        println!();
        println!(
            "{}",
            ui::info(&rust_i18n::t!("alias.install_first"), colored)
        );
        println!("{}", rust_i18n::t!("alias.install_cmd"));
        println!();
        println!("{}", ui::info(&rust_i18n::t!("alias.read_guide"), colored));
        println!("{}", rust_i18n::t!("alias.guide_url"));
        return Err(GcopError::Config("gcop-rs not in PATH".to_string()));
    }

    ui::step("1/2", &rust_i18n::t!("alias.installing"), colored);
    println!();

    let mut installed = 0;
    let mut skipped = 0;
    let mut failed: Vec<String> = Vec::new();

    // 2. Install alias one by one
    for (name, command, description) in GCOP_ALIASES {
        match install_single_alias(name, command, description, force, colored) {
            Ok(true) => installed += 1,
            Ok(false) => skipped += 1,
            Err(e) => {
                failed.push(format!("{}: {}", name, e));
            }
        }
    }

    // 3. Show summary
    println!();
    if installed > 0 {
        ui::success(
            &rust_i18n::t!("alias.installed", count = installed),
            colored,
        );
    }
    if !failed.is_empty() {
        for msg in &failed {
            ui::error(msg, colored);
        }
    }
    if skipped > 0 {
        println!(
            "{}",
            ui::info(&rust_i18n::t!("alias.skipped", count = skipped), colored)
        );
        if !force {
            println!();
            println!("{}", ui::info(&rust_i18n::t!("alias.use_force"), colored));
            println!("{}", rust_i18n::t!("alias.force_cmd"));
        }
    }

    println!();
    println!("\n{}", ui::info(&rust_i18n::t!("alias.now_use"), colored));
    println!("{}", rust_i18n::t!("alias.use_c"));
    println!("{}", rust_i18n::t!("alias.use_r"));
    println!("{}", rust_i18n::t!("alias.use_s"));
    println!("{}", rust_i18n::t!("alias.use_ac"));
    println!("{}", rust_i18n::t!("alias.use_cp"));
    println!("{}", rust_i18n::t!("alias.use_acp"));
    println!("{}", rust_i18n::t!("alias.use_gconfig"));
    println!("{}", rust_i18n::t!("alias.use_p"));
    println!("{}", rust_i18n::t!("alias.use_undo"));

    Ok(())
}

/// Install a single alias
fn install_single_alias(
    name: &str,
    command: &str,
    description: &str,
    force: bool,
    colored: bool,
) -> Result<bool> {
    let description = rust_i18n::t!(description).to_string();
    let existing = get_git_alias(name)?;

    match existing {
        None => {
            add_git_alias(name, command)?;
            if colored {
                println!(
                    "  {}  git {:10} → {}",
                    "✓".green().bold(),
                    name.bold(),
                    description
                );
            } else {
                println!("  ✓  git {:10} → {}", name, description);
            }
            Ok(true)
        }
        Some(existing_cmd) if existing_cmd == command => {
            if colored {
                println!(
                    "  {}  git {:10} → {} {}",
                    "ℹ".blue().bold(),
                    name.bold(),
                    description,
                    rust_i18n::t!("alias.already_set").dimmed()
                );
            } else {
                println!(
                    "  ℹ  git {:10} → {} {}",
                    name,
                    description,
                    rust_i18n::t!("alias.already_set")
                );
            }
            Ok(false)
        }
        Some(existing_cmd) => {
            if force {
                add_git_alias(name, command)?;
                if colored {
                    println!(
                        "  {}  git {:10} → {} {}",
                        "⚠".yellow().bold(),
                        name.bold(),
                        description,
                        rust_i18n::t!("alias.overwritten").yellow()
                    );
                } else {
                    println!(
                        "  ⚠  git {:10} → {} {}",
                        name,
                        description,
                        rust_i18n::t!("alias.overwritten")
                    );
                }
                Ok(true)
            } else {
                if colored {
                    println!(
                        "  {}  git {:10} - {}",
                        "⊗".red().bold(),
                        name.bold(),
                        rust_i18n::t!("alias.conflicts", cmd = existing_cmd).dimmed()
                    );
                } else {
                    println!(
                        "  ⊗  git {:10} - {}",
                        name,
                        rust_i18n::t!("alias.conflicts", cmd = existing_cmd)
                    );
                }
                Ok(false)
            }
        }
    }
}

/// Add git alias
fn add_git_alias(name: &str, command: &str) -> Result<()> {
    let status = Command::new("git")
        .args(["config", "--global", &format!("alias.{}", name), command])
        .status()?;

    if !status.success() {
        return Err(GcopError::GitCommand(
            rust_i18n::t!("alias.config_failed").to_string(),
        ));
    }

    Ok(())
}

/// List all available aliases and their status
fn list_aliases(colored: bool) -> Result<()> {
    println!("{}", ui::info(&rust_i18n::t!("alias.available"), colored));
    println!();

    for (name, command, description) in GCOP_ALIASES {
        let description = rust_i18n::t!(*description).to_string();
        let existing = get_git_alias(name)?;
        let status = match existing {
            Some(existing_cmd) if existing_cmd == *command => {
                if colored {
                    rust_i18n::t!("alias.status_installed").green().to_string()
                } else {
                    rust_i18n::t!("alias.status_installed").to_string()
                }
            }
            Some(existing_cmd) => {
                let msg = rust_i18n::t!("alias.status_conflicts", cmd = existing_cmd).to_string();
                if colored {
                    msg.yellow().to_string()
                } else {
                    msg
                }
            }
            None => {
                if colored {
                    rust_i18n::t!("alias.status_not_installed")
                        .dimmed()
                        .to_string()
                } else {
                    rust_i18n::t!("alias.status_not_installed").to_string()
                }
            }
        };

        if colored {
            println!("  git {:10} → {:45} [{}]", name.bold(), description, status);
        } else {
            println!("  git {:10} → {:45} [{}]", name, description, status);
        }
    }

    println!();
    println!("{}", ui::info(&rust_i18n::t!("alias.run_install"), colored));
    println!("{}", ui::info(&rust_i18n::t!("alias.run_force"), colored));

    Ok(())
}

/// Remove all gcop-related aliases
fn remove_aliases(force: bool, colored: bool) -> Result<()> {
    if !force {
        ui::warning(&rust_i18n::t!("alias.remove_warning"), colored);
        println!();
        println!("{}", ui::info(&rust_i18n::t!("alias.to_remove"), colored));
        for (name, _, _) in GCOP_ALIASES {
            if get_git_alias(name)?.is_some() {
                if colored {
                    println!("  - git {}", name.bold());
                } else {
                    println!("  - git {}", name);
                }
            }
        }
        println!();
        println!(
            "{}",
            ui::info(&rust_i18n::t!("alias.confirm_force"), colored)
        );
        println!("{}", rust_i18n::t!("alias.confirm_cmd"));
        return Ok(());
    }

    ui::step("1/1", &rust_i18n::t!("alias.removing"), colored);
    println!();

    let mut removed = 0;

    for (name, _, _) in GCOP_ALIASES {
        if get_git_alias(name)?.is_some() {
            let status = Command::new("git")
                .args(["config", "--global", "--unset", &format!("alias.{}", name)])
                .status()?;

            if status.success() {
                if colored {
                    println!(
                        "  {}  {}",
                        "✓".green().bold(),
                        rust_i18n::t!("alias.removed_single", name = name).bold()
                    );
                } else {
                    println!(
                        "  ✓  {}",
                        rust_i18n::t!("alias.removed_single", name = name)
                    );
                }
                removed += 1;
            }
        }
    }

    println!();
    if removed > 0 {
        ui::success(&rust_i18n::t!("alias.removed", count = removed), colored);
    } else {
        println!("{}", ui::info(&rust_i18n::t!("alias.no_remove"), colored));
    }

    Ok(())
}

/// Check if gcop-rs command is in PATH
fn is_gcop_in_path() -> bool {
    which("gcop-rs").is_ok()
}

/// Get the value of git alias
fn get_git_alias(name: &str) -> Result<Option<String>> {
    let output = Command::new("git")
        .args(["config", "--global", &format!("alias.{}", name)])
        .output()?;

    if output.status.success() {
        let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(Some(value))
    } else {
        Ok(None)
    }
}
