//! Test utilities for provider tests
//!
//! Provides common test configuration builders to reduce duplication
//! across provider test suites.

use crate::config::{NetworkConfig, ProviderConfig};
use std::collections::HashMap;

/// Install rustls crypto provider in tests
///
/// reqwest 0.13 + rustls-no-provider requires manual installation of crypto provider,
/// Production code is done in main.rs and tests need to be called separately.
/// It is safe to call it multiple times (just ignore install_default if it fails).
pub fn ensure_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

/// Create a `NetworkConfig` with max_retries set to 0 (no retry)
///
/// Useful for testing API error responses without waiting for retries.
///
/// # Example
/// ```
/// use gcop_rs::llm::provider::test_utils::test_network_config_no_retry;
///
/// let config = test_network_config_no_retry();
/// assert_eq!(config.max_retries, 0);
/// ```
pub fn test_network_config_no_retry() -> NetworkConfig {
    NetworkConfig {
        max_retries: 0,
        ..Default::default()
    }
}

/// Create a `ProviderConfig` for testing
///
/// # Parameters
/// - `base_url` - Mock server URL (e.g., from `mockito::Server`)
/// - `api_key` - Optional API key (use `Some("sk-test")` for providers that require it)
/// - `model` - Model name (e.g., `"gpt-4"`, `"claude-3-haiku"`, `"llama3"`)
///
/// # Example
/// ```
/// use gcop_rs::llm::provider::test_utils::test_provider_config;
///
/// // For OpenAI/Claude/Gemini
/// let config = test_provider_config(
///     "http://localhost:8080".to_string(),
///     Some("sk-test".to_string()),
///     "gpt-4".to_string()
/// );
///
/// // For Ollama (no API key)
/// let config = test_provider_config(
///     "http://localhost:11434".to_string(),
///     None,
///     "llama3".to_string()
/// );
/// ```
pub fn test_provider_config(
    base_url: String,
    api_key: Option<String>,
    model: String,
) -> ProviderConfig {
    ProviderConfig {
        api_style: None,
        endpoint: Some(base_url),
        api_key,
        model,
        max_tokens: None,
        temperature: None,
        extra: HashMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_config_no_retry_has_zero_retries() {
        let config = test_network_config_no_retry();
        assert_eq!(config.max_retries, 0);
    }

    #[test]
    fn test_provider_config_with_api_key() {
        let config = test_provider_config(
            "http://test.com".to_string(),
            Some("sk-test".to_string()),
            "test-model".to_string(),
        );

        assert_eq!(config.endpoint, Some("http://test.com".to_string()));
        assert_eq!(config.api_key, Some("sk-test".to_string()));
        assert_eq!(config.model, "test-model");
    }

    #[test]
    fn test_provider_config_without_api_key() {
        let config = test_provider_config(
            "http://localhost:11434".to_string(),
            None,
            "llama3".to_string(),
        );

        assert_eq!(config.endpoint, Some("http://localhost:11434".to_string()));
        assert_eq!(config.api_key, None);
        assert_eq!(config.model, "llama3");
    }
}
