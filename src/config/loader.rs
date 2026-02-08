// 配置加载逻辑
//
// 此文件负责从文件、环境变量和 CI 模式加载配置。

use config::{Config, Environment, File};
use directories::ProjectDirs;
use std::path::{Path, PathBuf};

use super::structs::{AppConfig, ProviderConfig};
use crate::error::Result;

/// 加载应用配置
///
/// 配置加载优先级（从高到低）：
/// 1. CI 模式覆盖（`CI=1` 时使用 `GCOP_CI_*`，直接修改反序列化后的结构体）
/// 2. 环境变量（`GCOP__*` 前缀，双下划线表示嵌套）
///    - 例如：`GCOP__LLM__DEFAULT_PROVIDER=openai`
///    - 例如：`GCOP__UI__COLORED=false`
/// 3. 项目级配置（`.gcop/config.toml`，从 CWD 向上查找，以 `.git` 为边界）
/// 4. 用户级配置文件（平台相关路径下的 `config.toml`）
/// 5. 默认值（来自 structs.rs 的 Default trait 和 serde(default) 属性）
///
/// 代码执行顺序：先加载低优先级源（用户文件→项目文件→环境变量），config-rs 后加的覆盖先加的；
/// 再在反序列化后应用 CI 覆盖。
pub fn load_config() -> Result<AppConfig> {
    load_config_from_path(get_config_path(), find_project_config())
}

/// 从指定路径加载配置（可测试版本）
///
/// 传入 `None` 跳过对应配置文件加载，仅使用其他源和默认值。
pub(crate) fn load_config_from_path(
    config_path: Option<PathBuf>,
    project_config_path: Option<PathBuf>,
) -> Result<AppConfig> {
    let mut builder = Config::builder();

    // 用户级配置文件（优先级最低，先加载；config-rs 后加的源覆盖先加的）
    if let Some(config_path) = config_path
        && config_path.exists()
    {
        builder = builder.add_source(File::from(config_path));
    }

    // 项目级配置文件（优先级高于用户配置，后加载以实现覆盖）
    if let Some(ref project_path) = project_config_path
        && project_path.exists()
    {
        // 安全检查：项目配置不应包含 api_key
        check_project_config_security(project_path);
        builder = builder.add_source(File::from(project_path.clone()));
    }

    // 环境变量（优先级最高，最后加载以实现覆盖）
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

    // CI 模式覆盖（优先级最高，直接修改反序列化后的结构体）
    apply_ci_mode_overrides(&mut app_config)?;

    // 验证配置合法性
    app_config.validate()?;

    Ok(app_config)
}

/// 从当前工作目录向上查找项目级配置 `.gcop/config.toml`
///
/// 遇到 `.git` 目录即停止查找（不跨 repo 边界）。
/// 只取最近的一个 `.gcop/config.toml`，不做多层合并。
pub(crate) fn find_project_config() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join(".gcop").join("config.toml");
        if candidate.exists() {
            return Some(candidate);
        }
        // 到达 repo 根目录，停止查找
        if dir.join(".git").exists() {
            return None;
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// 检查项目级配置文件的安全性
///
/// 如果项目配置中包含 `api_key` 字段，输出 warning 提示用户迁移到用户级配置或环境变量。
fn check_project_config_security(path: &Path) {
    if let Ok(content) = std::fs::read_to_string(path) {
        // 检查非注释行中是否包含 api_key
        let has_api_key = content.lines().any(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with('#') && trimmed.contains("api_key")
        });
        if has_api_key {
            eprintln!("{}", rust_i18n::t!("config.project_api_key_warning_line1"));
            eprintln!("{}", rust_i18n::t!("config.project_api_key_warning_line2"));
            eprintln!("{}", rust_i18n::t!("config.project_api_key_warning_line3"));
        }
    }
}

/// 应用 CI 模式环境变量覆盖
///
/// 当 `CI=1` 时，从以下环境变量构建 provider 配置：
/// - `GCOP_CI_PROVIDER`: "claude", "openai", "ollama" 或 "gemini"（必需）
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
        crate::error::GcopError::Config(rust_i18n::t!("config.ci_provider_not_set").to_string())
    })?;

    // 验证 provider_type
    let api_style: super::structs::ApiStyle = provider_type.parse().map_err(|_| {
        crate::error::GcopError::Config(
            rust_i18n::t!(
                "config.ci_provider_invalid",
                provider = provider_type.as_str()
            )
            .to_string(),
        )
    })?;

    // 读取 GCOP_CI_API_KEY（必需）
    let api_key = env::var("GCOP_CI_API_KEY").map_err(|_| {
        crate::error::GcopError::Config(rust_i18n::t!("config.ci_api_key_not_set").to_string())
    })?;

    // 读取 GCOP_CI_MODEL（可选，有默认值）
    let model = env::var("GCOP_CI_MODEL").unwrap_or_else(|_| api_style.default_model().to_string());

    // 读取 GCOP_CI_ENDPOINT（可选）
    let endpoint = env::var("GCOP_CI_ENDPOINT").ok();

    // 构建 ProviderConfig
    let provider_config = ProviderConfig {
        api_style: Some(api_style),
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

    tracing::info!("CI mode enabled, using GCOP_CI_PROVIDER={}", api_style);

    Ok(())
}

/// 获取配置文件路径
///
/// 返回平台相关的配置文件路径（`<config_dir>/config.toml`）
fn get_config_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "gcop").map(|dirs| dirs.config_dir().join("config.toml"))
}

/// 获取配置目录路径
///
/// 用于需要访问配置目录的场景（如初始化、验证等）
pub fn get_config_dir() -> Option<PathBuf> {
    ProjectDirs::from("", "", "gcop").map(|dirs| dirs.config_dir().to_path_buf())
}
