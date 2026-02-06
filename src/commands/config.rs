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
    let config_dir = config::get_config_dir().ok_or_else(|| {
        GcopError::Config(rust_i18n::t!("config.failed_determine_dir").to_string())
    })?;

    let config_file = config_dir.join("config.toml");

    // 如果配置文件不存在，提示运行 init
    if !config_file.exists() {
        ui::error(&rust_i18n::t!("config.file_not_found"), colored);
        println!();
        println!("{}", rust_i18n::t!("config.run_init"));
        println!("  mkdir -p {}", config_dir.display());
        println!(
            "  cp examples/config.toml.example {}",
            config_file.display()
        );
        return Err(GcopError::Config(
            rust_i18n::t!("config.file_not_found").to_string(),
        ));
    }

    // 初始读取配置内容
    let mut content = std::fs::read_to_string(&config_file)?;

    // 编辑-校验循环
    loop {
        println!(
            "{}",
            ui::info(
                &rust_i18n::t!("config.editing", path = config_file.display().to_string()),
                colored
            )
        );

        // 使用 edit crate 编辑（自动选择 $VISUAL > $EDITOR > platform default）
        let edited = edit::edit(&content).map_err(|e| {
            GcopError::Other(
                rust_i18n::t!("config.editor_error", error = e.to_string()).to_string(),
            )
        })?;

        // 校验配置（直接在内存校验）
        match toml::from_str::<config::AppConfig>(&edited) {
            Ok(_) => {
                // 校验成功，写入文件
                std::fs::write(&config_file, &edited)?;
                ui::success(&rust_i18n::t!("config.file_updated"), colored);
                return Ok(());
            }
            Err(e) => {
                // 校验失败
                println!();
                ui::error(
                    &rust_i18n::t!("config.validation_failed", error = e.to_string()),
                    colored,
                );
                println!();

                match prompt_edit_action(colored)? {
                    EditAction::Retry => {
                        // 保留编辑后的内容继续编辑
                        content = edited;
                        continue;
                    }
                    EditAction::Keep => {
                        // 原文件从未被修改，直接返回
                        println!("{}", ui::info(&rust_i18n::t!("config.unchanged"), colored));
                        return Ok(());
                    }
                    EditAction::Ignore => {
                        // 强制保存错误的配置
                        std::fs::write(&config_file, &edited)?;
                        ui::warning(&rust_i18n::t!("config.saved_with_errors"), colored);
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
            format!("{}", rust_i18n::t!("config.action_reedit").yellow()),
            format!("{}", rust_i18n::t!("config.action_keep").blue()),
            format!("{}", rust_i18n::t!("config.action_ignore").red()),
        ]
    } else {
        vec![
            rust_i18n::t!("config.action_reedit").to_string(),
            rust_i18n::t!("config.action_keep").to_string(),
            rust_i18n::t!("config.action_ignore").to_string(),
        ]
    };

    let prompt = if colored {
        format!("{}", rust_i18n::t!("config.action_prompt").cyan().bold())
    } else {
        rust_i18n::t!("config.action_prompt").to_string()
    };

    let selection = Select::new()
        .with_prompt(prompt)
        .items(&items)
        .default(0)
        .interact()
        .map_err(|e| {
            GcopError::Other(
                rust_i18n::t!("config.input_failed", error = e.to_string()).to_string(),
            )
        })?;

    Ok(match selection {
        0 => EditAction::Retry,
        1 => EditAction::Keep,
        _ => EditAction::Ignore,
    })
}

/// 验证配置
async fn validate(colored: bool) -> Result<()> {
    ui::step("1/2", &rust_i18n::t!("config.loading"), colored);

    // 加载配置
    let config = load_config()?;

    ui::success(&rust_i18n::t!("config.loaded"), colored);
    println!();

    // 显示配置的 providers
    println!("{}", rust_i18n::t!("config.providers"));
    for name in config.llm.providers.keys() {
        println!("  • {}", name);
    }
    println!();

    // 测试默认 provider 连接
    ui::step("2/2", &rust_i18n::t!("config.testing"), colored);

    let provider = create_provider(&config, None)?;

    match provider.validate().await {
        Ok(_) => {
            ui::success(
                &rust_i18n::t!("config.validated", provider = config.llm.default_provider),
                colored,
            );
        }
        Err(e) => {
            ui::error(
                &rust_i18n::t!("config.validation_failed_short", error = e.to_string()),
                colored,
            );
            if let Some(suggestion) = e.localized_suggestion() {
                println!();
                println!(
                    "{}",
                    rust_i18n::t!("config.suggestion", suggestion = suggestion)
                );
            }
            return Err(e);
        }
    }

    Ok(())
}
