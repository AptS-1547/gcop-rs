use crate::error::{GcopError, Result};
use crate::ui;
use std::process::Command;

// 完整的 git alias 列表（10 个，基于原项目）
const GCOP_ALIASES: &[(&str, &str, &str)] = &[
    ("cop", "!gcop-rs", "Main entry point for gcop-rs"),
    (
        "gcommit",
        "!gcop-rs commit",
        "AI commit message and commit changes",
    ),
    ("c", "!gcop-rs commit", "Shorthand for 'git gcommit'"),
    (
        "ac",
        "!git add -A && gcop-rs commit -y",
        "Add all changes and commit with AI message",
    ),
    (
        "acp",
        "!git add -A && gcop-rs commit -y && git push",
        "Add all, commit with AI, and push",
    ),
    ("ghelp", "!gcop-rs --help", "Show gcop-rs help message"),
    (
        "gconfig",
        "!gcop-rs config edit",
        "Open config file in default editor",
    ),
    ("p", "!git push", "Push changes to remote repository"),
    (
        "pf",
        "!git push --force-with-lease",
        "Force push (safer with --force-with-lease)",
    ),
    (
        "undo",
        "!git reset HEAD~1",
        "Undo last commit, keep changes staged",
    ),
];

/// 管理 git aliases
pub fn run(force: bool, list: bool, remove: bool, colored: bool) -> Result<()> {
    if list {
        return list_aliases(colored);
    }

    if remove {
        return remove_aliases(force, colored);
    }

    // 默认：批量安装所有 alias
    install_all(force, colored)
}

/// 批量安装所有 git aliases（公开，供 init 调用）
pub fn install_all(force: bool, colored: bool) -> Result<()> {
    // 1. 检测 gcop-rs 命令
    if !is_gcop_in_path() {
        ui::error("'gcop-rs' command not found in PATH", colored);
        println!();
        println!("Install gcop-rs first:");
        println!("  sudo cp target/release/gcop-rs /usr/local/bin/gcop-rs");
        println!();
        println!("Or add to PATH:");
        println!("  export PATH=\"$HOME/.local/bin:$PATH\"");
        return Err(GcopError::Config("gcop-rs not in PATH".to_string()));
    }

    println!("Installing git aliases...");
    println!();

    let mut installed = 0;
    let mut skipped = 0;

    // 2. 逐个安装 alias
    for (name, command, description) in GCOP_ALIASES {
        match install_single_alias(name, command, description, force, colored) {
            Ok(true) => installed += 1,
            Ok(false) => skipped += 1,
            Err(_) => {}
        }
    }

    // 3. 显示摘要
    println!();
    if installed > 0 {
        ui::success(&format!("Installed {} aliases", installed), colored);
    }
    if skipped > 0 {
        println!("Skipped {} (already exists or conflicts)", skipped);
        if !force {
            println!();
            println!("Use --force to overwrite conflicts:");
            println!("  gcop-rs alias --force");
        }
    }

    println!();
    println!("Now you can use:");
    println!("  git c        # AI commit");
    println!("  git ac       # Add all and commit");
    println!("  git acp      # Add all, commit, and push");
    println!("  git gconfig  # Edit configuration");
    println!("  git p        # Push");
    println!("  git undo     # Undo last commit");

    Ok(())
}

/// 安装单个 alias
fn install_single_alias(
    name: &str,
    command: &str,
    description: &str,
    force: bool,
    _colored: bool,
) -> Result<bool> {
    let existing = get_git_alias(name)?;

    match existing {
        None => {
            add_git_alias(name, command)?;
            println!("  ✓  git {:10} → {}", name, description);
            Ok(true)
        }
        Some(existing_cmd) if existing_cmd == command => {
            println!("  ℹ  git {:10} → {} (already set)", name, description);
            Ok(false)
        }
        Some(existing_cmd) => {
            if force {
                add_git_alias(name, command)?;
                println!("  ⚠  git {:10} → {} (overwritten)", name, description);
                Ok(true)
            } else {
                println!("  ⊗  git {:10} - conflicts with: {}", name, existing_cmd);
                Ok(false)
            }
        }
    }
}

/// 添加 git alias
fn add_git_alias(name: &str, command: &str) -> Result<()> {
    let status = Command::new("git")
        .args(["config", "--global", &format!("alias.{}", name), command])
        .status()?;

    if !status.success() {
        return Err(GcopError::Other("git config failed".to_string()));
    }

    Ok(())
}

/// 列出所有可用的 aliases 及其状态
fn list_aliases(colored: bool) -> Result<()> {
    println!("Available git aliases for gcop-rs:");
    println!();

    for (name, command, description) in GCOP_ALIASES {
        let existing = get_git_alias(name)?;
        let status = match existing {
            Some(existing_cmd) if existing_cmd == *command => {
                if colored {
                    use colored::Colorize;
                    "✓ installed".green().to_string()
                } else {
                    "✓ installed".to_string()
                }
            }
            Some(existing_cmd) => {
                let msg = format!("⚠ conflicts: {}", existing_cmd);
                if colored {
                    use colored::Colorize;
                    msg.yellow().to_string()
                } else {
                    msg
                }
            }
            None => "  not installed".to_string(),
        };

        println!("  git {:10} → {:45} [{}]", name, description, status);
    }

    println!();
    println!("Run 'gcop-rs alias' to install all.");
    println!("Run 'gcop-rs alias --force' to overwrite conflicts.");

    Ok(())
}

/// 移除所有 gcop-related aliases
fn remove_aliases(force: bool, colored: bool) -> Result<()> {
    if !force {
        ui::warning("This will remove all gcop-related git aliases", colored);
        println!();
        println!("Aliases to be removed:");
        for (name, _, _) in GCOP_ALIASES {
            if get_git_alias(name)?.is_some() {
                println!("  - git {}", name);
            }
        }
        println!();
        println!("Use --force to confirm:");
        println!("  gcop-rs alias --remove --force");
        return Ok(());
    }

    println!("Removing git aliases...");
    println!();

    let mut removed = 0;

    for (name, _, _) in GCOP_ALIASES {
        if get_git_alias(name)?.is_some() {
            let status = Command::new("git")
                .args(["config", "--global", "--unset", &format!("alias.{}", name)])
                .status()?;

            if status.success() {
                println!("  ✓  Removed git {}", name);
                removed += 1;
            }
        }
    }

    println!();
    if removed > 0 {
        ui::success(&format!("Removed {} aliases", removed), colored);
    } else {
        println!("{}", ui::info("No aliases to remove", colored));
    }

    Ok(())
}

/// 检查 gcop-rs 命令是否在 PATH 中
fn is_gcop_in_path() -> bool {
    Command::new("which")
        .arg("gcop-rs")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// 获取 git alias 的值
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
