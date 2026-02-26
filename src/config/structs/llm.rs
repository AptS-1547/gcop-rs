//! LLM provider configuration structures.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// LLM API backend type.
///
/// Determines which provider implementation to instantiate.
/// If [`ProviderConfig::api_style`] is `None`, the style is inferred from the provider name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiStyle {
    /// Anthropic Claude API.
    Claude,
    /// OpenAI API (and OpenAI-compatible APIs).
    #[serde(rename = "openai")]
    OpenAI,
    /// Ollama local model API.
    Ollama,
    /// Google Gemini API.
    Gemini,
}

impl std::fmt::Display for ApiStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiStyle::Claude => write!(f, "claude"),
            ApiStyle::OpenAI => write!(f, "openai"),
            ApiStyle::Ollama => write!(f, "ollama"),
            ApiStyle::Gemini => write!(f, "gemini"),
        }
    }
}

impl std::str::FromStr for ApiStyle {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude" => Ok(ApiStyle::Claude),
            "openai" => Ok(ApiStyle::OpenAI),
            "ollama" => Ok(ApiStyle::Ollama),
            "gemini" => Ok(ApiStyle::Gemini),
            _ => Err(format!("Unknown API style: '{}'", s)),
        }
    }
}

impl ApiStyle {
    /// Returns the default model name for this API style.
    pub fn default_model(&self) -> &'static str {
        match self {
            ApiStyle::Claude => "claude-sonnet-4-5-20250929",
            ApiStyle::OpenAI => "gpt-4o-mini",
            ApiStyle::Ollama => "llama3.2",
            ApiStyle::Gemini => "gemini-3-flash-preview",
        }
    }
}

/// Provider configuration.
///
/// Settings for one entry under `[llm.providers.<name>]`.
///
/// # Fields
/// - `api_style`: API style (see [`ApiStyle`])
/// - `endpoint`: custom API endpoint (optional)
/// - `api_key`: API key (optional; usually required for Claude/OpenAI, optional for Ollama)
/// - `model`: model name
/// - `max_tokens`: maximum generated token count (optional)
/// - `temperature`: sampling temperature in `0.0..=2.0` (optional)
/// - `extra`: additional provider-specific parameters
///
/// # Example
/// ```toml
/// [llm.providers.claude]
/// model = "claude-sonnet-4-5-20250929"
/// api_key = "sk-ant-..."
/// max_tokens = 1000
/// temperature = 0.7
/// endpoint = "https://api.anthropic.com" # optional
/// ```
#[derive(Clone, Deserialize, Serialize)]
pub struct ProviderConfig {
    /// API style used to select the backend implementation.
    ///
    /// If omitted, it is inferred from the provider name.
    #[serde(default)]
    pub api_style: Option<ApiStyle>,

    /// API endpoint.
    pub endpoint: Option<String>,

    /// API key.
    ///
    /// Usually required for Claude/OpenAI; optional for Ollama.
    #[serde(skip_serializing)]
    pub api_key: Option<String>,

    /// Model name.
    pub model: String,

    /// Maximum generated token count.
    pub max_tokens: Option<u32>,

    /// Sampling temperature in `0.0..=2.0`.
    pub temperature: Option<f32>,

    /// Additional provider-specific parameters.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl std::fmt::Debug for ProviderConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use crate::llm::provider::utils::mask_api_key;
        let masked_key = self.api_key.as_deref().map(mask_api_key);
        f.debug_struct("ProviderConfig")
            .field("api_style", &self.api_style)
            .field("endpoint", &self.endpoint)
            .field("api_key", &masked_key)
            .field("model", &self.model)
            .field("max_tokens", &self.max_tokens)
            .field("temperature", &self.temperature)
            .finish()
    }
}

impl ProviderConfig {
    /// Validates provider configuration.
    pub fn validate(&self, name: &str) -> Result<()> {
        use crate::error::GcopError;
        if let Some(temp) = self.temperature
            && !(0.0..=2.0).contains(&temp)
        {
            return Err(GcopError::Config(format!(
                "Provider '{}': temperature {} out of range [0.0, 2.0]",
                name, temp
            )));
        }
        if let Some(ref key) = self.api_key
            && key.trim().is_empty()
        {
            return Err(GcopError::Config(format!(
                "Provider '{}': api_key is empty",
                name
            )));
        }
        Ok(())
    }
}

/// LLM configuration.
///
/// Selects providers and controls prompt input size.
///
/// # Fields
/// - `default_provider`: provider name, matching a key under `[llm.providers.<name>]`
/// - `fallback_providers`: providers to try in order if the primary provider fails
/// - `providers`: per-provider settings map
/// - `max_diff_size`: maximum diff size sent to the LLM in bytes for commit/review/hook non-split flows (default: 100 KiB)
///
/// # Example
/// ```toml
/// [llm]
/// default_provider = "claude"
/// fallback_providers = ["openai", "gemini", "ollama"]
/// max_diff_size = 102400
///
/// [llm.providers.claude]
/// api_key = "sk-ant-..."
/// model = "claude-sonnet-4-5-20250929"
///
/// [llm.providers.openai]
/// api_key = "sk-..."
/// model = "gpt-4"
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LLMConfig {
    /// Provider name used by default.
    ///
    /// Must match a key under `[llm.providers.<name>]`.
    pub default_provider: String,

    /// Providers tried in order when `default_provider` fails.
    #[serde(default)]
    pub fallback_providers: Vec<String>,

    /// Provider settings keyed by provider name.
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,

    /// Maximum diff size in bytes sent to the LLM.
    ///
    /// Oversized diffs are truncated before prompt generation in commit/review/hook non-split flows.
    #[serde(default = "default_max_diff_size")]
    pub max_diff_size: usize,
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            default_provider: "claude".to_string(),
            fallback_providers: Vec::new(),
            providers: HashMap::new(),
            max_diff_size: default_max_diff_size(),
        }
    }
}

fn default_max_diff_size() -> usize {
    100 * 1024 // 100KB
}
