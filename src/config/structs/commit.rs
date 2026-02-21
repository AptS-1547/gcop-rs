//! Commit command configuration structures.

use serde::{Deserialize, Serialize};

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

fn default_true() -> bool {
    true
}

fn default_commit_max_retries() -> usize {
    10
}
