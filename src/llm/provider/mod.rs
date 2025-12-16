pub mod claude;
pub mod ollama;
pub mod openai;

use std::sync::Arc;

use reqwest::Client;

use crate::config::AppConfig;
use crate::error::{GcopError, Result};
use crate::llm::LLMProvider;

/// 创建带有自定义 User-Agent 的 HTTP 客户端
pub(crate) fn create_http_client() -> Result<Client> {
    let user_agent = format!(
        "{}/{} ({})",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS
    );

    Client::builder()
        .user_agent(user_agent)
        .build()
        .map_err(GcopError::Network)
}

/// 根据配置创建 LLM Provider
pub fn create_provider(
    config: &AppConfig,
    provider_name: Option<&str>,
) -> Result<Arc<dyn LLMProvider>> {
    let name = provider_name.unwrap_or(&config.llm.default_provider);

    let provider_config = config
        .llm
        .providers
        .get(name)
        .ok_or_else(|| GcopError::Config(format!("Provider '{}' not found in config", name)))?;

    // 决定使用哪种 API 风格
    // 优先使用 api_style 字段，否则使用 provider 名称（向后兼容）
    let api_style = provider_config.api_style.as_deref().unwrap_or(name);

    // 根据 API 风格创建对应的 Provider 实现
    match api_style {
        "claude" => {
            let provider = claude::ClaudeProvider::new(provider_config, name)?;
            Ok(Arc::new(provider))
        }
        "openai" => {
            let provider = openai::OpenAIProvider::new(provider_config, name)?;
            Ok(Arc::new(provider))
        }
        "ollama" => {
            let provider = ollama::OllamaProvider::new(provider_config, name)?;
            Ok(Arc::new(provider))
        }
        _ => Err(GcopError::Config(format!(
            "Unsupported api_style: '{}' for provider '{}'",
            api_style, name
        ))),
    }
}
