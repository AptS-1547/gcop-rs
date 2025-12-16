pub mod schema;

use config::{Config, Environment, File};
use directories::ProjectDirs;
use std::path::PathBuf;

use crate::error::Result;
pub use schema::*;

/// 加载应用配置
///
/// 配置加载优先级（从高到低）：
/// 1. 环境变量（GCOP_* 前缀）
/// 2. 配置文件（~/.config/gcop/config.toml）
/// 3. 默认值
pub fn load_config() -> Result<AppConfig> {
    let mut builder = Config::builder();

    // 1. 设置默认值
    builder = builder
        .set_default("llm.default_provider", "claude")?
        .set_default("commit.show_diff_preview", true)?
        .set_default("commit.allow_edit", true)?
        .set_default("commit.confirm_before_commit", true)?
        .set_default("review.show_full_diff", true)?
        .set_default("review.min_severity", "info")?
        .set_default("ui.colored", true)?
        .set_default("ui.verbose", false)?;

    // 2. 加载配置文件（如果存在）
    if let Some(config_path) = get_config_path()
        && config_path.exists() {
            builder = builder.add_source(File::from(config_path));
        }

    // 3. 加载环境变量（GCOP_ 前缀，优先级最高）
    builder = builder.add_source(
        Environment::with_prefix("GCOP")
            .separator("_")
            .try_parsing(true),
    );

    // 构建并反序列化配置
    let config = builder.build()?;
    let app_config: AppConfig = config.try_deserialize()?;

    Ok(app_config)
}

/// 获取配置文件路径
///
/// 返回 ~/.config/gcop/config.toml
fn get_config_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "gcop").map(|dirs| dirs.config_dir().join("config.toml"))
}

/// 获取配置目录路径
pub fn get_config_dir() -> Option<PathBuf> {
    ProjectDirs::from("", "", "gcop").map(|dirs| dirs.config_dir().to_path_buf())
}
