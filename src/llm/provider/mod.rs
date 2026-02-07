pub mod base;
pub mod claude;
pub mod fallback;
pub mod ollama;
pub mod openai;
pub mod streaming;
pub mod utils;

#[cfg(test)]
pub mod test_utils;

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use reqwest::Client;

use crate::config::{AppConfig, NetworkConfig, ProviderConfig};
use crate::error::{GcopError, Result};
use crate::llm::LLMProvider;

/// 全局 HTTP 客户端（共享连接池）
static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

/// 全局 HTTP 客户端初始化错误信息
///
/// 如果第一次创建失败，保存错误字符串以避免后续重复创建与潜在 panic。
static HTTP_CLIENT_ERROR: OnceLock<String> = OnceLock::new();

/// 获取或创建全局 HTTP 客户端
///
/// 使用 OnceLock 确保只创建一次，所有 provider 共享同一个连接池。
/// 第一次调用时的 NetworkConfig 决定 timeout 配置。
pub(crate) fn create_http_client(network_config: &NetworkConfig) -> Result<Client> {
    if let Some(client) = HTTP_CLIENT.get() {
        return Ok(client.clone());
    }

    if let Some(err_msg) = HTTP_CLIENT_ERROR.get() {
        return Err(GcopError::Llm(
            rust_i18n::t!("provider.http_client_init_failed", error = err_msg.as_str()).to_string(),
        ));
    }

    let user_agent = format!(
        "{}/{} ({})",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS
    );

    match Client::builder()
        .user_agent(user_agent)
        .timeout(Duration::from_secs(network_config.request_timeout))
        .connect_timeout(Duration::from_secs(network_config.connect_timeout))
        .build()
    {
        Ok(client) => {
            let _ = HTTP_CLIENT.set(client.clone());
            Ok(client)
        }
        Err(e) => {
            let err_msg = e.to_string();
            let _ = HTTP_CLIENT_ERROR.set(err_msg.clone());
            Err(GcopError::Llm(
                rust_i18n::t!(
                    "provider.http_client_create_failed",
                    error = err_msg.as_str()
                )
                .to_string(),
            ))
        }
    }
}

/// 根据配置创建 LLM Provider
///
/// 如果配置了 fallback_providers，会创建一个 FallbackProvider 包装多个 provider。
/// 当主 provider 失败时，会自动尝试 fallback 列表中的 provider。
pub fn create_provider(
    config: &AppConfig,
    provider_name: Option<&str>,
) -> Result<Arc<dyn LLMProvider>> {
    fallback::FallbackProvider::from_config(config, provider_name)
}

/// 创建单个 Provider
pub fn create_single_provider(
    config: &AppConfig,
    name: &str,
    colored: bool,
) -> Result<Arc<dyn LLMProvider>> {
    let provider_config = config.llm.providers.get(name).ok_or_else(|| {
        GcopError::Config(rust_i18n::t!("provider.provider_not_found", name = name).to_string())
    })?;

    create_provider_from_config(provider_config, name, &config.network, colored)
}

/// 根据配置创建具体的 Provider 实现
fn create_provider_from_config(
    provider_config: &ProviderConfig,
    name: &str,
    network_config: &NetworkConfig,
    colored: bool,
) -> Result<Arc<dyn LLMProvider>> {
    // 决定使用哪种 API 风格
    // 优先使用 api_style 字段，否则使用 provider 名称（向后兼容）
    let api_style = provider_config.api_style.as_deref().unwrap_or(name);

    // 根据 API 风格创建对应的 Provider 实现
    match api_style {
        "claude" => {
            let provider =
                claude::ClaudeProvider::new(provider_config, name, network_config, colored)?;
            Ok(Arc::new(provider))
        }
        "openai" => {
            let provider =
                openai::OpenAIProvider::new(provider_config, name, network_config, colored)?;
            Ok(Arc::new(provider))
        }
        "ollama" => {
            let provider =
                ollama::OllamaProvider::new(provider_config, name, network_config, colored)?;
            Ok(Arc::new(provider))
        }
        _ => Err(GcopError::Config(
            rust_i18n::t!(
                "provider.unsupported_api_style",
                style = api_style,
                provider = name
            )
            .to_string(),
        )),
    }
}
