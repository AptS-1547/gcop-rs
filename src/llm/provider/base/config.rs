//! Provider configuration extraction tool
//!
//! Provides helper functions to extract various parameters from ProviderConfig

use crate::config::ProviderConfig;
use crate::error::{GcopError, Result};

use super::super::utils::complete_endpoint;

/// Default max_tokens
const DEFAULT_MAX_TOKENS: u32 = 2000;

/// Default temperature
const DEFAULT_TEMPERATURE: f32 = 0.3;

/// Extract API key
///
/// Read from configuration file. Ordinary users set it in config.toml, and CI mode uses `GCOP_CI_API_KEY`.
///
/// # Arguments
/// * `config` - Provider configuration
/// * `provider_name` - Provider name (used for error prompts)
pub fn extract_api_key(config: &ProviderConfig, provider_name: &str) -> Result<String> {
    config.api_key.clone().ok_or_else(|| {
        GcopError::Config(
            rust_i18n::t!(
                "provider.api_key_not_found_simple",
                provider = provider_name
            )
            .to_string(),
        )
    })
}

/// Build a complete endpoint
///
/// Read the endpoint from the configuration file, and use the default value if not configured.
///
/// # Arguments
/// * `config` - Provider configuration
/// * `default_base` - default base URL
/// * `suffix` - API path suffix
pub fn build_endpoint(config: &ProviderConfig, default_base: &str, suffix: &str) -> String {
    let base = config.endpoint.as_deref().unwrap_or(default_base);
    complete_endpoint(base, suffix)
}

/// Extract u32 value from extra configuration
pub fn extract_extra_u32(config: &ProviderConfig, key: &str) -> Option<u32> {
    config
        .extra
        .get(key)
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
}

/// Extract f32 value in extra configuration
pub fn extract_extra_f32(config: &ProviderConfig, key: &str) -> Option<f32> {
    config
        .extra
        .get(key)
        .and_then(|v| v.as_f64())
        .map(|v| v as f32)
}

/// Get max_tokens from configuration (explicit fields first, fallback to extra, lastly use default)
pub fn get_max_tokens(config: &ProviderConfig) -> u32 {
    config
        .max_tokens
        .or_else(|| extract_extra_u32(config, "max_tokens"))
        .unwrap_or(DEFAULT_MAX_TOKENS)
}

/// Get max_tokens from the configuration (optional, used in scenarios such as OpenAI that are not required)
pub fn get_max_tokens_optional(config: &ProviderConfig) -> Option<u32> {
    config
        .max_tokens
        .or_else(|| extract_extra_u32(config, "max_tokens"))
}

/// Get temperature from configuration (explicit fields first, fallback to extra, lastly use default value)
pub fn get_temperature(config: &ProviderConfig) -> f32 {
    config
        .temperature
        .or_else(|| extract_extra_f32(config, "temperature"))
        .unwrap_or(DEFAULT_TEMPERATURE)
}

/// Get temperature from configuration (optional)
pub fn get_temperature_optional(config: &ProviderConfig) -> Option<f32> {
    config
        .temperature
        .or_else(|| extract_extra_f32(config, "temperature"))
}
