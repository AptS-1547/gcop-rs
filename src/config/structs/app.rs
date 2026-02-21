//! Top-level application configuration and remaining command structures.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::{GcopError, Result};

use super::commit::CommitConfig;
use super::llm::LLMConfig;
use super::network::NetworkConfig;

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

impl Default for ReviewConfig {
    fn default() -> Self {
        Self {
            min_severity: "info".to_string(),
            custom_prompt: None,
        }
    }
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

impl Default for UIConfig {
    fn default() -> Self {
        Self {
            colored: true,
            streaming: true,
            language: None,
        }
    }
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

impl Default for FileConfig {
    fn default() -> Self {
        Self {
            max_size: default_max_file_size(),
        }
    }
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

fn default_true() -> bool {
    true
}

fn default_severity() -> String {
    "info".to_string()
}

fn default_max_file_size() -> u64 {
    10 * 1024 * 1024 // 10MB
}
