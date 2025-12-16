pub mod claude;

use std::sync::Arc;

use crate::config::AppConfig;
use crate::error::{GcopError, Result};
use crate::llm::LLMProvider;

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

    match name {
        "claude" => {
            let provider = claude::ClaudeProvider::new(provider_config)?;
            Ok(Arc::new(provider))
        }
        _ => Err(GcopError::Config(format!("Unsupported provider: {}", name))),
    }
}
