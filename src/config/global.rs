// Global configuration singleton management
//
// Use OnceLock + ArcSwap to implement thread-safe global configuration singleton.

use arc_swap::ArcSwap;
use std::sync::{Arc, OnceLock};

use super::loader;
use super::structs::AppConfig;
use crate::error::Result;

static CONFIG: OnceLock<ArcSwap<AppConfig>> = OnceLock::new();

/// Initialize global configuration (called once at startup)
///
/// Load configuration and initialize global singleton. This function will only be executed once,
/// Subsequent calls are ignored (idempotence).
pub fn init_config() -> Result<()> {
    tracing::debug!("Initializing global configuration...");
    let config = loader::load_config()?;
    CONFIG.get_or_init(|| {
        tracing::info!("Configuration loaded successfully");
        ArcSwap::from_pointee(config)
    });
    Ok(())
}

/// Get global configuration (return Arc, cheap clone)
///
/// If the configuration has not been initialized (i.e. `init_config()` has not been called), an error is returned.
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
        // Test configuration initialization
        let result = init_config();
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_get_config_after_init() {
        // Initial configuration
        init_config().unwrap();

        // Get configuration
        let config1 = get_config().unwrap();
        let config2 = get_config().unwrap();

        // Verify that the same Arc is returned (pointers are equal)
        assert!(Arc::ptr_eq(&config1, &config2));
    }

    #[test]
    #[serial]
    fn test_init_config_idempotent() {
        // Multiple initialization should be idempotent
        init_config().unwrap();
        init_config().unwrap();
        init_config().unwrap();

        let config = get_config().unwrap();
        assert!(!config.llm.default_provider.is_empty());
    }
}
