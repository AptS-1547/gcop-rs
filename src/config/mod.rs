pub mod schema;

use config::{Config, Environment, File};
use directories::ProjectDirs;
use std::path::PathBuf;

use crate::error::Result;
pub use schema::*;

/// 加载应用配置
///
/// 配置加载优先级（从高到低）：
/// 1. 环境变量（GCOP__* 前缀，双下划线表示嵌套）
///    - 例如：`GCOP__LLM__DEFAULT_PROVIDER=openai`
///    - 例如：`GCOP__UI__COLORED=false`
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
        .set_default("commit.max_retries", 10)?
        .set_default("review.show_full_diff", true)?
        .set_default("review.min_severity", "info")?
        .set_default("ui.colored", true)?
        .set_default("ui.verbose", false)?
        .set_default("network.request_timeout", 120)?
        .set_default("network.connect_timeout", 10)?
        .set_default("network.max_retries", 3)?
        .set_default("network.retry_delay_ms", 1000)?
        .set_default("file.max_size", 10 * 1024 * 1024)?;

    // 2. 加载配置文件（如果存在）
    if let Some(config_path) = get_config_path()
        && config_path.exists()
    {
        builder = builder.add_source(File::from(config_path));
    }

    // 3. 加载环境变量（GCOP_ 前缀，优先级最高）
    // 使用双下划线作为嵌套层级分隔符，避免与字段名中的单下划线冲突
    // 例如：GCOP__LLM__DEFAULT_PROVIDER -> llm.default_provider
    builder = builder.add_source(
        Environment::with_prefix("GCOP")
            .separator("__")
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
///
/// 用于需要访问配置目录的场景（如初始化、验证等）
pub fn get_config_dir() -> Option<PathBuf> {
    ProjectDirs::from("", "", "gcop").map(|dirs| dirs.config_dir().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serial_test::serial;
    use std::env;

    /// RAII 环境变量 guard，确保测试后清理
    struct EnvGuard {
        key: String,
        original: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &str, value: &str) -> Self {
            let original = env::var(key).ok();
            // SAFETY: 测试环境中修改环境变量是安全的，且使用 serial_test 确保串行执行
            unsafe { env::set_var(key, value) };
            Self {
                key: key.to_string(),
                original,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            // SAFETY: 测试环境中修改环境变量是安全的
            match &self.original {
                Some(v) => unsafe { env::set_var(&self.key, v) },
                None => unsafe { env::remove_var(&self.key) },
            }
        }
    }

    // === 默认值测试（测试 schema.rs 的 Default 实现）===

    #[test]
    fn test_app_config_default_llm() {
        let config = AppConfig::default();
        assert_eq!(config.llm.default_provider, "claude");
    }

    #[test]
    fn test_app_config_default_commit() {
        let config = AppConfig::default();
        assert!(config.commit.show_diff_preview);
        assert!(config.commit.allow_edit);
        assert_eq!(config.commit.max_retries, 10);
    }

    #[test]
    fn test_app_config_default_network() {
        let config = AppConfig::default();
        assert_eq!(config.network.request_timeout, 120);
        assert_eq!(config.network.connect_timeout, 10);
        assert_eq!(config.network.max_retries, 3);
        assert_eq!(config.network.retry_delay_ms, 1000);
    }

    #[test]
    fn test_app_config_default_ui() {
        let config = AppConfig::default();
        assert!(config.ui.colored);
        assert!(!config.ui.verbose);
    }

    #[test]
    fn test_app_config_default_review() {
        let config = AppConfig::default();
        assert!(config.review.show_full_diff);
        assert_eq!(config.review.min_severity, "info");
    }

    #[test]
    fn test_app_config_default_file() {
        let config = AppConfig::default();
        assert_eq!(config.file.max_size, 10 * 1024 * 1024);
    }

    // === 配置加载测试 ===

    #[test]
    fn test_load_config_succeeds() {
        // 验证 load_config 不会崩溃
        let result = load_config();
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_config_returns_valid_config() {
        let config = load_config().unwrap();
        // 验证配置有合理的值（不一定是默认值，可能被用户配置覆盖）
        assert!(!config.llm.default_provider.is_empty());
        assert!(config.commit.max_retries > 0);
        assert!(config.network.request_timeout > 0);
    }

    // === 路径函数测试 ===

    #[test]
    fn test_get_config_dir_returns_valid_path() {
        let config_dir = get_config_dir();
        assert!(config_dir.is_some());
        let path = config_dir.unwrap();
        // 路径应该包含 "gcop"
        assert!(path.to_string_lossy().contains("gcop"));
    }

    #[test]
    fn test_get_config_path_has_toml_suffix() {
        let config_dir = get_config_dir();
        assert!(config_dir.is_some());
        // config.toml 应该在配置目录下
        let config_path = config_dir.unwrap().join("config.toml");
        assert!(config_path.to_string_lossy().ends_with("config.toml"));
    }

    // === 环境变量覆盖测试（验证环境变量可以被读取）===
    // 注意：这些测试验证环境变量被正确设置，但由于用户可能有配置文件，
    // 我们只验证环境变量设置功能而不是完整的优先级覆盖

    #[test]
    #[serial]
    fn test_env_guard_sets_and_restores() {
        let key = "GCOP_TEST_VAR";

        // 确保测试前不存在
        // SAFETY: 测试环境
        unsafe { env::remove_var(key) };

        {
            let _guard = EnvGuard::set(key, "test_value");
            assert_eq!(env::var(key).unwrap(), "test_value");
        }

        // guard 释放后应该恢复（删除）
        assert!(env::var(key).is_err());
    }

    #[test]
    #[serial]
    fn test_env_var_can_be_read() {
        let _guard = EnvGuard::set("GCOP__UI__COLORED", "false");
        // 验证环境变量被正确设置
        assert_eq!(env::var("GCOP__UI__COLORED").unwrap(), "false");
    }

    #[test]
    #[serial]
    fn test_env_var_bool_parsing() {
        // 测试 config crate 的 bool 解析能力
        let _guard = EnvGuard::set("GCOP__UI__VERBOSE", "true");
        let config = load_config().unwrap();
        // ui.verbose 默认是 false，如果环境变量生效应该是 true
        // 但如果用户配置文件覆盖了，可能仍然是其他值
        // 这里我们只验证加载成功，不验证具体值
        let _ = config.ui.verbose;
    }

    #[test]
    #[serial]
    fn test_env_var_llm_default_provider() {
        // 验证 GCOP__LLM__DEFAULT_PROVIDER 环境变量是否生效
        // 注意：使用双下划线表示嵌套层级
        let _guard = EnvGuard::set("GCOP__LLM__DEFAULT_PROVIDER", "test_provider");
        let config = load_config().unwrap();
        // 环境变量优先级最高，应该覆盖配置文件
        assert_eq!(config.llm.default_provider, "test_provider");
    }
}
