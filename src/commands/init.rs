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
            &rust_i18n::t!("init.exists", path = config_file.display()),
            colored,
        );
        println!();
        println!("{}", rust_i18n::t!("init.use_force"));
        println!("{}", rust_i18n::t!("init.config_edit"));
        return Ok(());
    }

    // 3. 创建配置目录
    fs::create_dir_all(&config_dir)?;
    ui::success(
        &rust_i18n::t!("init.dir_created", path = config_dir.display()),
        colored,
    );

    // 4. 复制示例配置
    let example_config = include_str!("../../examples/config.toml.example");
    fs::write(&config_file, example_config)?;
    ui::success(
        &rust_i18n::t!("init.file_created", path = config_file.display()),
        colored,
    );

    // 5. 设置文件权限（仅 Unix）
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&config_file)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&config_file, perms)?;
        ui::success(&rust_i18n::t!("init.permissions"), colored);
    }

    // 6. 显示下一步提示
    println!();
    println!("{}", ui::info(&rust_i18n::t!("init.next_steps"), colored));
    println!("{}", rust_i18n::t!("init.step1"));
    println!("{}", rust_i18n::t!("init.step1_cmd"));
    println!();
    println!("{}", rust_i18n::t!("init.step2"));
    println!("{}", rust_i18n::t!("init.step2_url"));
    println!();

    // 7. 询问是否安装 git aliases
    let install_aliases = ui::confirm(&rust_i18n::t!("init.install_aliases"), true)?;

    if install_aliases {
        println!();
        // 调用 alias 模块
        match crate::commands::alias::install_all(force, colored) {
            Ok(_) => {}
            Err(e) => {
                ui::warning(&rust_i18n::t!("init.alias_failed", error = e.to_string()), colored);
                println!();
                println!("{}", rust_i18n::t!("init.alias_later"));
                println!("{}", rust_i18n::t!("init.alias_cmd"));
            }
        }
    } else {
        println!();
        println!("{}", rust_i18n::t!("init.alias_skipped"));
        println!("{}", rust_i18n::t!("init.alias_run_later"));
    }

    println!();
    println!("{}", rust_i18n::t!("init.docs"));

    Ok(())
}
