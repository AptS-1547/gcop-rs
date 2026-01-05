use crate::config::{self, load_config};
use crate::error::{GcopError, Result};
use crate::llm::provider::create_provider;
use crate::ui;
use colored::Colorize;
use dialoguer::Select;

/// 编辑后用户可选的操作
enum EditAction {
    Retry,  // 重新编辑
    Keep,   // 保留原配置（不修改）
    Ignore, // 忽略错误强制保存
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

    // 初始读取配置内容
    let mut content = std::fs::read_to_string(&config_file)?;

    // 编辑-校验循环
    loop {
        println!(
            "{}",
            ui::info(&format!("Editing {} ...", config_file.display()), colored)
        );

        // 使用 edit crate 编辑（自动选择 $VISUAL > $EDITOR > platform default）
        let edited =
            edit::edit(&content).map_err(|e| GcopError::Other(format!("Editor error: {}", e)))?;

        // 校验配置（直接在内存校验）
        match toml::from_str::<config::AppConfig>(&edited) {
            Ok(_) => {
                // 校验成功，写入文件
                std::fs::write(&config_file, &edited)?;
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
                        // 保留编辑后的内容继续编辑
                        content = edited;
                        continue;
                    }
                    EditAction::Keep => {
                        // 原文件从未被修改，直接返回
                        println!("{}", ui::info("Original config unchanged", colored));
                        return Ok(());
                    }
                    EditAction::Ignore => {
                        // 强制保存错误的配置
                        std::fs::write(&config_file, &edited)?;
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
            format!("{} {}", "↩".blue().bold(), "Keep original config".blue()),
            format!(
                "{} {} {}",
                "⚠".red().bold(),
                "Ignore errors and save anyway".red(),
                "(dangerous)".red().bold()
            ),
        ]
    } else {
        vec![
            "✎ Re-edit the config file".to_string(),
            "↩ Keep original config".to_string(),
            "⚠ Ignore errors and save anyway (dangerous)".to_string(),
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
        1 => EditAction::Keep,
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
    ui::step("2/2", "Testing provider connection...", colored);

    let provider = create_provider(&config, None)?;

    match provider.validate().await {
        Ok(_) => {
            ui::success(
                &format!(
                    "Provider '{}' validated successfully",
                    config.llm.default_provider
                ),
                colored,
            );
        }
        Err(e) => {
            ui::error(&format!("Validation failed: {}", e), colored);
            if let Some(suggestion) = e.suggestion() {
                println!();
                println!("💡 Suggestion: {}", suggestion);
            }
            return Err(e);
        }
    }

    Ok(())
}
