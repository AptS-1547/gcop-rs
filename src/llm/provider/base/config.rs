//! Provider 配置提取工具
//!
//! 提供从 ProviderConfig 中提取各种参数的辅助函数

use crate::config::ProviderConfig;
use crate::error::{GcopError, Result};

use super::super::utils::complete_endpoint;

/// 默认 max_tokens
const DEFAULT_MAX_TOKENS: u32 = 2000;

/// 默认 temperature
const DEFAULT_TEMPERATURE: f32 = 0.3;

/// 提取 API key
///
/// 从配置文件读取。普通用户在 config.toml 中设置，CI 模式使用 `GCOP_CI_API_KEY`。
///
/// # Arguments
/// * `config` - Provider 配置
/// * `provider_name` - Provider 名称（用于错误提示）
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

/// 构建完整 endpoint
///
/// 从配置文件读取 endpoint，未配置时使用默认值。
///
/// # Arguments
/// * `config` - Provider 配置
/// * `default_base` - 默认 base URL
/// * `suffix` - API 路径后缀
pub fn build_endpoint(config: &ProviderConfig, default_base: &str, suffix: &str) -> String {
    let base = config.endpoint.as_deref().unwrap_or(default_base);
    complete_endpoint(base, suffix)
}

/// 提取 extra 配置中的 u32 值
pub fn extract_extra_u32(config: &ProviderConfig, key: &str) -> Option<u32> {
    config
        .extra
        .get(key)
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
}

/// 提取 extra 配置中的 f32 值
pub fn extract_extra_f32(config: &ProviderConfig, key: &str) -> Option<f32> {
    config
        .extra
        .get(key)
        .and_then(|v| v.as_f64())
        .map(|v| v as f32)
}

/// 从配置中获取 max_tokens（优先显式字段，fallback 到 extra，最后使用默认值）
pub fn get_max_tokens(config: &ProviderConfig) -> u32 {
    config
        .max_tokens
        .or_else(|| extract_extra_u32(config, "max_tokens"))
        .unwrap_or(DEFAULT_MAX_TOKENS)
}

/// 从配置中获取 max_tokens（可选，用于 OpenAI 等不强制要求的场景）
pub fn get_max_tokens_optional(config: &ProviderConfig) -> Option<u32> {
    config
        .max_tokens
        .or_else(|| extract_extra_u32(config, "max_tokens"))
}

/// 从配置中获取 temperature（优先显式字段，fallback 到 extra，最后使用默认值）
pub fn get_temperature(config: &ProviderConfig) -> f32 {
    config
        .temperature
        .or_else(|| extract_extra_f32(config, "temperature"))
        .unwrap_or(DEFAULT_TEMPERATURE)
}

/// 从配置中获取 temperature（可选）
pub fn get_temperature_optional(config: &ProviderConfig) -> Option<f32> {
    config
        .temperature
        .or_else(|| extract_extra_f32(config, "temperature"))
}
