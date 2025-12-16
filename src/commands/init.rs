use crate::config;
use crate::error::{GcopError, Result};
use crate::ui;
use std::fs;

/// 初始化配置文件
pub fn run(force: bool, colored: bool) -> Result<()> {
    // 1. 获取配置目录和文件路径
    let config_dir = config::get_config_dir()
        .ok_or_else(|| GcopError::Config("Failed to determine config directory".to_string()))?;

    let config_file = config_dir.join("config.toml");

    // 2. 检查配置文件是否已存在
    if config_file.exists() && !force {
        ui::warning(
            &format!("Config file already exists: {}", config_file.display()),
            colored,
        );
        println!();
        println!("Use --force to overwrite, or edit it directly:");
        println!("  gcop-rs config edit");
        return Ok(());
    }

    // 3. 创建配置目录
    fs::create_dir_all(&config_dir)?;
    ui::success(
        &format!("Created config directory: {}", config_dir.display()),
        colored,
    );

    // 4. 复制示例配置
    let example_config = include_str!("../../examples/config.toml.example");
    fs::write(&config_file, example_config)?;
    ui::success(
        &format!("Created config file: {}", config_file.display()),
        colored,
    );

    // 5. 设置文件权限（仅 Unix）
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&config_file)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&config_file, perms)?;
        ui::success("Set file permissions: 600", colored);
    }

    // 6. 显示下一步提示
    println!();
    println!("{}", ui::info("Next steps:", colored));
    println!("  1. Edit the config file:");
    println!("     gcop-rs config edit");
    println!();
    println!("  2. Add your API key to [llm.providers.claude]");
    println!("     Get key from: https://console.anthropic.com/");
    println!();

    // 7. 询问是否安装 git aliases
    let install_aliases = ui::confirm("Install git aliases (git c, git ac, etc.)?", true)?;

    if install_aliases {
        println!();
        // 调用 alias 模块
        match crate::commands::alias::install_all(force, colored) {
            Ok(_) => {}
            Err(e) => {
                ui::warning(&format!("Failed to install aliases: {}", e), colored);
                println!();
                println!("You can install them later with:");
                println!("  gcop-rs alias");
            }
        }
    } else {
        println!();
        println!("Skipped alias installation.");
        println!("Run 'gcop-rs alias' later if needed.");
    }

    println!();
    println!("See docs/configuration.md for more options.");

    Ok(())
}
