//! Configuration data structures and validation logic.
//!
//! Defines the runtime config schema used by file loading, environment
//! overrides, and command execution.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::{GcopError, Result};

/// Application configuration.
///
/// Top-level runtime configuration for `gcop-rs`.
///
/// Effective configuration is merged from multiple sources (low to high):
/// 1. Rust defaults (`Default` + `serde(default)`)
/// 2. User-level config file (platform-specific config directory)
/// 3. Project-level config (`.gcop/config.toml`, discovered from repository root)
/// 4. `GCOP__*` environment variables
/// 5. CI mode overrides (`CI=1` + `GCOP_CI_*`)
///
/// # Configuration File Locations
/// - Linux: `~/.config/gcop/config.toml`
/// - macOS: `~/Library/Application Support/gcop/config.toml`
/// - Windows: `%APPDATA%\gcop\config\config.toml`
/// - Project level (optional): `<repo>/.gcop/config.toml`
///
/// # Example
/// ```toml
/// [llm]
/// default_provider = "claude"
/// fallback_providers = ["openai"]
///
/// [llm.providers.claude]
/// api_key = "sk-ant-..."
/// model = "claude-sonnet-4-5-20250929"
///
/// [commit]
/// max_retries = 10
/// show_diff_preview = true
///
/// [ui]
/// colored = true
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AppConfig {
    /// LLM provider and prompt settings.
    #[serde(default)]
    pub llm: LLMConfig,

    /// Commit command behavior.
    #[serde(default)]
    pub commit: CommitConfig,

    /// Review command behavior.
    #[serde(default)]
    pub review: ReviewConfig,

    /// Terminal UI behavior.
    #[serde(default)]
    pub ui: UIConfig,

    /// HTTP timeout and retry settings.
    #[serde(default)]
    pub network: NetworkConfig,

    /// File I/O limits.
    #[serde(default)]
    pub file: FileConfig,

    /// Workspace detection and scope inference (monorepo support).
    #[serde(default)]
    pub workspace: WorkspaceConfig,
}

/// LLM configuration.
///
/// Selects providers and controls prompt input size.
///
/// # Fields
/// - `default_provider`: provider name, matching a key under `[llm.providers.<name>]`
/// - `fallback_providers`: providers to try in order if the primary provider fails
/// - `providers`: per-provider settings map
/// - `max_diff_size`: maximum diff size sent to the LLM in bytes (default: 100 KiB)
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
    /// Oversized diffs are truncated before prompt generation.
    #[serde(default = "default_max_diff_size")]
    pub max_diff_size: usize,
}

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

/// Commit message convention style.
///
/// Controls the target format requested from the LLM.
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ConventionStyle {
    /// Conventional Commits: `type(scope): description`.
    #[default]
    Conventional,
    /// Gitmoji: `:emoji: description`.
    Gitmoji,
    /// Custom format defined by [`CommitConvention::template`].
    Custom,
}

/// Commit convention configuration.
///
/// Defines team-specific commit rules injected into prompt generation.
///
/// # Example
/// ```toml
/// [commit.convention]
/// style = "conventional"
/// types = ["feat", "fix", "docs", "style", "refactor", "perf", "test", "chore", "ci"]
/// extra_prompt = "All commit messages must be in English"
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
pub struct CommitConvention {
    /// Convention style.
    #[serde(default)]
    pub style: ConventionStyle,

    /// Allowed commit types (used when `style = "conventional"` or `style = "custom"`).
    pub types: Option<Vec<String>>,

    /// Custom template (used when `style = "custom"`).
    /// Placeholders: `{type}`, `{scope}`, `{subject}`, `{body}`.
    pub template: Option<String>,

    /// Additional prompt text appended after built-in instructions.
    pub extra_prompt: Option<String>,
}

/// Commit command configuration.
///
/// Controls commit message generation behavior.
///
/// # Fields
/// - `show_diff_preview`: show diff preview before generation (default: `true`)
/// - `allow_edit`: allow editing generated messages (default: `true`)
/// - `split`: enable atomic split commit mode by default (default: `false`)
/// - `custom_prompt`: extra prompt text (optional)
/// - `max_retries`: maximum generation attempts, including the first one (default: `10`)
/// - `convention`: optional commit convention config
///
/// # Example
/// ```toml
/// [commit]
/// show_diff_preview = true
/// allow_edit = true
/// split = false
/// max_retries = 10
/// custom_prompt = "Generate a concise commit message"
///
/// [commit.convention]
/// style = "conventional"
/// types = ["feat", "fix", "docs", "refactor", "test", "chore"]
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommitConfig {
    /// Whether to show a diff preview before generation.
    #[serde(default = "default_true")]
    pub show_diff_preview: bool,

    /// Whether to allow editing generated messages.
    #[serde(default = "default_true")]
    pub allow_edit: bool,

    /// Whether to use atomic split commit mode by default.
    #[serde(default)]
    pub split: bool,

    /// Additional prompt text appended to the commit system prompt.
    ///
    /// No placeholder substitution is performed (`{diff}` is passed literally).
    #[serde(default)]
    pub custom_prompt: Option<String>,

    /// Maximum generation attempts, including the first attempt.
    #[serde(default = "default_commit_max_retries")]
    pub max_retries: usize,

    /// Optional commit convention config, usually set in `.gcop/config.toml`.
    #[serde(default)]
    pub convention: Option<CommitConvention>,
}

/// Review command configuration.
///
/// Controls code-review behavior.
///
/// # Fields
/// - `min_severity`: minimum issue severity shown in text output (`"info"`, `"warning"`, `"critical"`)
/// - `custom_prompt`: additional prompt text (optional)
///
/// # Example
/// ```toml
/// [review]
/// min_severity = "warning"
/// custom_prompt = "Focus on security issues"
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReviewConfig {
    /// Minimum issue severity displayed in text output.
    ///
    /// Note: this filter currently applies only to `review --format text`.
    /// `json` and `markdown` output keep the full issue list.
    #[serde(default = "default_severity")]
    pub min_severity: String,

    /// Additional prompt text appended to the review system prompt.
    ///
    /// No placeholder substitution is performed (`{diff}` is passed literally).
    #[serde(default)]
    pub custom_prompt: Option<String>,
}

/// UI configuration.
///
/// Controls terminal display behavior.
///
/// # Fields
/// - `colored`: enable colored output (default: `true`)
/// - `streaming`: enable streaming output (typewriter effect, default: `true`)
/// - `language`: UI language in BCP 47 format (for example `"en"`, `"zh-CN"`), auto-detected by default
///
/// # Example
/// ```toml
/// [ui]
/// colored = true
/// streaming = true
/// language = "zh-CN"
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UIConfig {
    /// Whether to enable color output.
    #[serde(default = "default_true")]
    pub colored: bool,

    /// Whether to enable streaming output (real-time typing effect).
    #[serde(default = "default_true")]
    pub streaming: bool,

    /// UI language in BCP 47 format (for example `"en"`, `"zh-CN"`).
    /// `None` means auto-detect from system locale.
    #[serde(default)]
    pub language: Option<String>,
}

/// Network configuration.
///
/// Controls timeout and retry behavior for HTTP requests.
///
/// # Fields
/// - `request_timeout`: HTTP request timeout in seconds (default: `120`)
/// - `connect_timeout`: HTTP connect timeout in seconds (default: `10`)
/// - `max_retries`: max retries for LLM API requests (default: `3`)
/// - `retry_delay_ms`: initial retry delay in milliseconds (default: `1000`)
/// - `max_retry_delay_ms`: max retry delay in milliseconds (default: `60000`)
///
/// # Example
/// ```toml
/// [network]
/// request_timeout = 30
/// connect_timeout = 10
/// max_retries = 3
/// retry_delay_ms = 1000
/// max_retry_delay_ms = 60000
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NetworkConfig {
    /// HTTP request timeout in seconds.
    #[serde(default = "default_request_timeout")]
    pub request_timeout: u64,

    /// HTTP connect timeout in seconds.
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout: u64,

    /// Maximum retries for LLM API requests.
    #[serde(default = "default_network_max_retries")]
    pub max_retries: usize,

    /// Initial retry delay in milliseconds.
    #[serde(default = "default_retry_delay_ms")]
    pub retry_delay_ms: u64,

    /// Maximum retry delay in milliseconds.
    #[serde(default = "default_max_retry_delay_ms")]
    pub max_retry_delay_ms: u64,
}

/// File configuration.
///
/// Controls local file-read limits.
///
/// # Fields
/// - `max_size`: max file size in bytes (default: 10 MiB)
///   Used by `review file <PATH>` when reading workspace files.
///
/// # Example
/// ```toml
/// [file]
/// max_size = 10485760  # 10MB
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileConfig {
    /// Maximum file size in bytes.
    ///
    /// Current read limit for `review file <PATH>`.
    #[serde(default = "default_max_file_size")]
    pub max_size: u64,
}

fn default_true() -> bool {
    true
}

/// Workspace configuration (monorepo support).
///
/// Controls workspace detection and scope inference.
/// Auto-detection is enabled by default; this section is for manual overrides.
///
/// # Example
/// ```toml
/// [workspace]
/// enabled = true
/// members = ["packages/*", "apps/*"]
/// scope_mappings = { "packages/core" = "core", "packages/ui" = "ui" }
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkspaceConfig {
    /// Whether workspace detection is enabled (default: `true`).
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Manual scope mapping: package path -> scope name.
    ///
    /// Overrides automatically inferred package short names.
    #[serde(default)]
    pub scope_mappings: HashMap<String, String>,

    /// Explicit workspace member globs.
    ///
    /// When set, auto-detection is skipped and this list is used directly.
    #[serde(default)]
    pub members: Option<Vec<String>>,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            scope_mappings: HashMap::new(),
            members: None,
        }
    }
}

fn default_severity() -> String {
    "info".to_string()
}

fn default_commit_max_retries() -> usize {
    10
}

fn default_request_timeout() -> u64 {
    120
}

fn default_connect_timeout() -> u64 {
    10
}

fn default_network_max_retries() -> usize {
    3
}

fn default_retry_delay_ms() -> u64 {
    1000
}

fn default_max_retry_delay_ms() -> u64 {
    60_000 // 60 seconds
}

fn default_max_file_size() -> u64 {
    10 * 1024 * 1024 // 10MB
}

fn default_max_diff_size() -> usize {
    100 * 1024 // 100KB
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

impl Default for CommitConfig {
    fn default() -> Self {
        Self {
            show_diff_preview: true,
            allow_edit: true,
            split: false,
            custom_prompt: None,
            max_retries: default_commit_max_retries(),
            convention: None,
        }
    }
}

impl Default for ReviewConfig {
    fn default() -> Self {
        Self {
            min_severity: "info".to_string(),
            custom_prompt: None,
        }
    }
}

impl Default for UIConfig {
    fn default() -> Self {
        Self {
            colored: true,
            streaming: true,
            language: None,
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            request_timeout: default_request_timeout(),
            connect_timeout: default_connect_timeout(),
            max_retries: default_network_max_retries(),
            retry_delay_ms: default_retry_delay_ms(),
            max_retry_delay_ms: default_max_retry_delay_ms(),
        }
    }
}

impl Default for FileConfig {
    fn default() -> Self {
        Self {
            max_size: default_max_file_size(),
        }
    }
}

// Validation logic.

impl AppConfig {
    /// Validates configuration consistency.
    pub fn validate(&self) -> Result<()> {
        // Ensure the configured default provider exists.
        if !self.llm.providers.is_empty()
            && !self.llm.providers.contains_key(&self.llm.default_provider)
        {
            return Err(GcopError::Config(format!(
                "default_provider '{}' not found in [llm.providers]",
                self.llm.default_provider
            )));
        }

        // Ensure all configured fallback providers exist.
        for name in &self.llm.fallback_providers {
            if !self.llm.providers.contains_key(name) {
                return Err(GcopError::Config(format!(
                    "fallback_providers: '{}' not found in [llm.providers]",
                    name
                )));
            }
        }

        for (name, provider) in &self.llm.providers {
            provider.validate(name)?;
        }
        self.network.validate()?;
        Ok(())
    }
}

impl ProviderConfig {
    /// Validates provider configuration.
    pub fn validate(&self, name: &str) -> Result<()> {
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

impl NetworkConfig {
    /// Validates network configuration.
    pub fn validate(&self) -> Result<()> {
        if self.request_timeout == 0 {
            return Err(GcopError::Config(
                "network.request_timeout cannot be 0".into(),
            ));
        }
        if self.connect_timeout == 0 {
            return Err(GcopError::Config(
                "network.connect_timeout cannot be 0".into(),
            ));
        }
        Ok(())
    }
}
