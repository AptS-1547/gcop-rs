// 全局配置单例管理
//
// 使用 OnceLock + ArcSwap 实现线程安全的全局配置单例。

use arc_swap::ArcSwap;
use std::sync::{Arc, OnceLock};

use super::loader;
use super::structs::AppConfig;
use crate::error::Result;

static CONFIG: OnceLock<ArcSwap<AppConfig>> = OnceLock::new();

/// 初始化全局配置（启动时调用一次）
///
/// 加载配置并初始化全局单例。此函数只会执行一次，
/// 后续调用会被忽略（幂等性）。
pub fn init_config() -> Result<()> {
    tracing::debug!("Initializing global configuration...");
    let config = loader::load_config()?;
    CONFIG.get_or_init(|| {
        tracing::info!("Configuration loaded successfully");
        ArcSwap::from_pointee(config)
    });
    Ok(())
}

/// 获取全局配置（返回 Arc，cheap clone）
///
/// 如果配置未初始化（即未调用 `init_config()`），返回错误。
pub fn get_config() -> Result<Arc<AppConfig>> {
    CONFIG.get().map(|c| c.load_full()).ok_or_else(|| {
        crate::error::GcopError::Config(
            "Config not initialized. Call init_config() first.".to_string(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_init_config_succeeds() {
        // 测试配置初始化
        let result = init_config();
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_get_config_after_init() {
        // 初始化配置
        init_config().unwrap();

        // 获取配置
        let config1 = get_config().unwrap();
        let config2 = get_config().unwrap();

        // 验证返回的是同一个 Arc（指针相等）
        assert!(Arc::ptr_eq(&config1, &config2));
    }

    #[test]
    #[serial]
    fn test_init_config_idempotent() {
        // 多次初始化应该是幂等的
        init_config().unwrap();
        init_config().unwrap();
        init_config().unwrap();

        let config = get_config().unwrap();
        assert!(!config.llm.default_provider.is_empty());
    }
}
