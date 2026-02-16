pub mod base;
/// Anthropic Claude provider implementation.
pub mod claude;
/// Multi-provider fallback wrapper.
pub mod fallback;
/// Google Gemini provider implementation.
pub mod gemini;
/// Ollama provider implementation for local models.
pub mod ollama;
/// OpenAI-compatible provider implementation.
pub mod openai;
pub mod streaming;
pub mod utils;

#[cfg(test)]
pub mod test_utils;

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use reqwest::Client;

use crate::config::{ApiStyle, AppConfig, NetworkConfig, ProviderConfig};
use crate::error::{GcopError, Result};
use crate::llm::LLMProvider;

/// Global HTTP client (shared connection pool)
static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

/// Global HTTP client initialization error message
///
/// If the first creation fails, save the error string to avoid subsequent repeated creations and potential panics.
static HTTP_CLIENT_ERROR: OnceLock<String> = OnceLock::new();

/// Get or create a global HTTP client
///
/// Use OnceLock to ensure it is created only once and all providers share the same connection pool.
/// The first call to NetworkConfig determines the timeout configuration.
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

/// Create LLM Provider based on configuration
///
/// If fallback_providers is configured, a FallbackProvider will be created to wrap multiple providers.
/// When the main provider fails, providers in the fallback list are automatically tried.
pub fn create_provider(
    config: &AppConfig,
    provider_name: Option<&str>,
) -> Result<Arc<dyn LLMProvider>> {
    fallback::FallbackProvider::from_config(config, provider_name)
}

/// Create a single Provider
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

/// Create specific Provider implementation based on configuration
fn create_provider_from_config(
    provider_config: &ProviderConfig,
    name: &str,
    network_config: &NetworkConfig,
    colored: bool,
) -> Result<Arc<dyn LLMProvider>> {
    // Decide which API style to use
    // Prefer using api_style field, otherwise infer from provider name (backward compatibility)
    let api_style = match provider_config.api_style {
        Some(style) => style,
        None => name.parse::<ApiStyle>().map_err(|_| {
            GcopError::Config(
                rust_i18n::t!(
                    "provider.unsupported_api_style",
                    style = name,
                    provider = name
                )
                .to_string(),
            )
        })?,
    };

    // Create corresponding Provider implementation according to API style (exhaustive matching)
    match api_style {
        ApiStyle::Claude => {
            let provider =
                claude::ClaudeProvider::new(provider_config, name, network_config, colored)?;
            Ok(Arc::new(provider))
        }
        ApiStyle::OpenAI => {
            let provider =
                openai::OpenAIProvider::new(provider_config, name, network_config, colored)?;
            Ok(Arc::new(provider))
        }
        ApiStyle::Ollama => {
            let provider =
                ollama::OllamaProvider::new(provider_config, name, network_config, colored)?;
            Ok(Arc::new(provider))
        }
        ApiStyle::Gemini => {
            let provider =
                gemini::GeminiProvider::new(provider_config, name, network_config, colored)?;
            Ok(Arc::new(provider))
        }
    }
}
