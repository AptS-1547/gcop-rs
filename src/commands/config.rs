use crate::config::{self, load_config};
use crate::error::{GcopError, Result};
use crate::llm::provider::create_provider;
use crate::ui;
use colored::Colorize;
use dialoguer::Select;
use std::process::Command;

/// 编辑后用户可选的操作
enum EditAction {
    Retry,   // 重新编辑
    Restore, // 恢复原配置
    Ignore,  // 忽略错误
}

pub async fn run(action: Option<crate::cli::ConfigAction>, colored: bool) -> Result<()> {
    // 默认行为：调用 edit
    let action = action.unwrap_or(crate::cli::ConfigAction::Edit);

    match action {
        crate::cli::ConfigAction::Edit => edit(colored),
        crate::cli::ConfigAction::Validate => validate(colored).await,
    }
}

/// 打开编辑器编辑配置文件（带校验）
fn edit(colored: bool) -> Result<()> {
    let config_dir = config::get_config_dir()
        .ok_or_else(|| GcopError::Config("Failed to determine config directory".to_string()))?;

    let config_file = config_dir.join("config.toml");

    // 如果配置文件不存在，提示运行 init
    if !config_file.exists() {
        ui::error("Config file not found", colored);
        println!();
        println!("Run 'gcop-rs init' to create it, or create manually:");
        println!("  mkdir -p {}", config_dir.display());
        println!(
            "  cp examples/config.toml.example {}",
            config_file.display()
        );
        return Err(GcopError::Config("Config file not found".to_string()));
    }

    // 备份当前配置
    let backup_file = config_file.with_extension("toml.bak");
    std::fs::copy(&config_file, &backup_file).map_err(|e| {
        GcopError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to backup config: {}", e),
        ))
    })?;

    // 获取编辑器
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vim".to_string());

    // 编辑-校验循环
    loop {
        println!(
            "{}",
            ui::info(&format!("Opening {} ...", config_file.display()), colored)
        );

        let status = Command::new(&editor).arg(&config_file).status()?;

        if !status.success() {
            // 编辑器异常退出，恢复备份
            std::fs::copy(&backup_file, &config_file).ok();
            std::fs::remove_file(&backup_file).ok();
            return Err(GcopError::Other("Editor exited with error".to_string()));
        }

        // 校验配置
        match load_config() {
            Ok(_) => {
                // 校验成功，删除备份
                std::fs::remove_file(&backup_file).ok();
                ui::success("Config file updated", colored);
                return Ok(());
            }
            Err(e) => {
                // 校验失败
                println!();
                ui::error(&format!("Config validation failed: {}", e), colored);
                println!();

                match prompt_edit_action(colored)? {
                    EditAction::Retry => {
                        // 继续循环，重新编辑
                        continue;
                    }
                    EditAction::Restore => {
                        std::fs::copy(&backup_file, &config_file).map_err(|e| {
                            GcopError::Io(std::io::Error::new(
                                e.kind(),
                                format!("Failed to restore config: {}", e),
                            ))
                        })?;
                        std::fs::remove_file(&backup_file).ok();
                        ui::warning("Config restored to previous version", colored);
                        return Ok(());
                    }
                    EditAction::Ignore => {
                        std::fs::remove_file(&backup_file).ok();
                        ui::warning("Config saved with errors", colored);
                        return Ok(());
                    }
                }
            }
        }
    }
}

/// 提示用户选择操作
fn prompt_edit_action(colored: bool) -> Result<EditAction> {
    let items: Vec<String> = if colored {
        vec![
            format!(
                "{} {}",
                "✎".yellow().bold(),
                "Re-edit the config file".yellow()
            ),
            format!("{} {}", "↩".blue().bold(), "Restore previous config".blue()),
            format!(
                "{} {} {}",
                "⚠".red().bold(),
                "Ignore errors and keep current".red(),
                "(dangerous)".red().bold()
            ),
        ]
    } else {
        vec![
            "✎ Re-edit the config file".to_string(),
            "↩ Restore previous config".to_string(),
            "⚠ Ignore errors and keep current (dangerous)".to_string(),
        ]
    };

    let prompt = if colored {
        format!("{}", "What would you like to do?".cyan().bold())
    } else {
        "What would you like to do?".to_string()
    };

    let selection = Select::new()
        .with_prompt(prompt)
        .items(&items)
        .default(0)
        .interact()
        .map_err(|e| GcopError::Other(format!("Failed to get user input: {}", e)))?;

    Ok(match selection {
        0 => EditAction::Retry,
        1 => EditAction::Restore,
        _ => EditAction::Ignore,
    })
}

/// 验证配置
async fn validate(colored: bool) -> Result<()> {
    ui::step("1/2", "Loading configuration...", colored);

    // 加载配置
    let config = load_config()?;

    ui::success("Configuration loaded successfully", colored);
    println!();

    // 显示配置的 providers
    println!("Configured providers:");
    for name in config.llm.providers.keys() {
        println!("  • {}", name);
    }
    println!();

    // 测试默认 provider 连接
    ui::step("2/2", "Testing default provider connection...", colored);

    let provider = create_provider(&config, None)?;
    provider.validate().await?;

    ui::success(
        &format!(
            "Provider '{}' validated successfully",
            config.llm.default_provider
        ),
        colored,
    );

    Ok(())
}
