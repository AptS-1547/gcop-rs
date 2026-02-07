// 配置加载逻辑
//
// 此文件负责从文件、环境变量和 CI 模式加载配置。

use config::{Config, Environment, File};
use directories::ProjectDirs;
use std::path::PathBuf;

use super::structs::{AppConfig, ProviderConfig};
use crate::error::Result;

/// 加载应用配置
///
/// 配置加载优先级（从高到低）：
/// 1. 环境变量（GCOP__* 前缀，双下划线表示嵌套）
///    - 例如：`GCOP__LLM__DEFAULT_PROVIDER=openai`
///    - 例如：`GCOP__UI__COLORED=false`
/// 2. 配置文件（~/.config/gcop/config.toml）
/// 3. 默认值（来自 structs.rs 的 Default trait 和 serde(default) 属性）
pub fn load_config() -> Result<AppConfig> {
    let mut builder = Config::builder();

    // 1. 加载配置文件（如果存在）
    if let Some(config_path) = get_config_path()
        && config_path.exists()
    {
        builder = builder.add_source(File::from(config_path));
    }

    // 2. 加载环境变量（GCOP__*，优先级最高）
    // 使用双下划线作为嵌套层级分隔符，避免与字段名中的单下划线冲突
    // 例如：GCOP__LLM__DEFAULT_PROVIDER -> llm.default_provider
    builder = builder.add_source(
        Environment::with_prefix("GCOP")
            .separator("__")
            .try_parsing(true),
    );

    // 构建并反序列化配置
    let config = builder.build()?;
    let mut app_config: AppConfig = config.try_deserialize()?;

    // 3. CI 模式覆盖（优先级最高）
    // 当 CI=1 时，使用 GCOP_CI_* 环境变量构建临时 provider 配置
    apply_ci_mode_overrides(&mut app_config)?;

    Ok(app_config)
}

/// 应用 CI 模式环境变量覆盖
///
/// 当 `CI=1` 时，从以下环境变量构建 provider 配置：
/// - `GCOP_CI_PROVIDER`: "claude", "openai", 或 "ollama"（必需）
/// - `GCOP_CI_API_KEY`: API key（必需）
/// - `GCOP_CI_MODEL`: 模型名称（可选，有默认值）
/// - `GCOP_CI_ENDPOINT`: 自定义端点（可选）
///
/// 该 provider 将被注入为 "ci" 并设为 default_provider。
fn apply_ci_mode_overrides(config: &mut AppConfig) -> Result<()> {
    use std::env;

    // 检查是否启用 CI 模式
    let ci_enabled = env::var("CI").ok().as_deref() == Some("1");

    if !ci_enabled {
        return Ok(());
    }

    // 读取 GCOP_CI_PROVIDER（必需）
    let provider_type = env::var("GCOP_CI_PROVIDER").map_err(|_| {
        crate::error::GcopError::Config(
            "CI mode enabled but GCOP_CI_PROVIDER not set. Must be 'claude', 'openai', or 'ollama'."
                .to_string(),
        )
    })?;

    // 验证 provider_type
    if !matches!(provider_type.as_str(), "claude" | "openai" | "ollama") {
        return Err(crate::error::GcopError::Config(format!(
            "Invalid GCOP_CI_PROVIDER '{}'. Must be 'claude', 'openai', or 'ollama'.",
            provider_type
        )));
    }

    // 读取 GCOP_CI_API_KEY（必需）
    let api_key = env::var("GCOP_CI_API_KEY").map_err(|_| {
        crate::error::GcopError::Config("CI mode enabled but GCOP_CI_API_KEY not set.".to_string())
    })?;

    // 读取 GCOP_CI_MODEL（可选，有默认值）
    let model = env::var("GCOP_CI_MODEL").unwrap_or_else(|_| match provider_type.as_str() {
        "claude" => "claude-sonnet-4-5-20250929".to_string(),
        "openai" => "gpt-4o-mini".to_string(),
        "ollama" => "llama3.2".to_string(),
        _ => unreachable!(), // 已验证
    });

    // 读取 GCOP_CI_ENDPOINT（可选）
    let endpoint = env::var("GCOP_CI_ENDPOINT").ok();

    // 构建 ProviderConfig
    let provider_config = ProviderConfig {
        api_style: Some(provider_type.clone()),
        endpoint,
        api_key: Some(api_key),
        model,
        max_tokens: None,
        temperature: None,
        extra: Default::default(),
    };

    // 注入到配置中
    config
        .llm
        .providers
        .insert("ci".to_string(), provider_config);
    config.llm.default_provider = "ci".to_string();

    tracing::info!("CI mode enabled, using GCOP_CI_PROVIDER={}", provider_type);

    Ok(())
}

/// 获取配置文件路径
///
/// 返回 ~/.config/gcop/config.toml
fn get_config_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "gcop").map(|dirs| dirs.config_dir().join("config.toml"))
}

/// 获取配置目录路径
///
/// 用于需要访问配置目录的场景（如初始化、验证等）
pub fn get_config_dir() -> Option<PathBuf> {
    ProjectDirs::from("", "", "gcop").map(|dirs| dirs.config_dir().to_path_buf())
}
