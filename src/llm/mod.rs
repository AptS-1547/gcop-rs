//! LLM abstractions, shared types, and provider traits.
//!
//! This module defines the provider interface used by commit generation
//! and code review flows.

/// Provider message payload helper types.
pub mod message;
/// Prompt-building utilities for commit/review flows.
pub mod prompt;
/// Built-in provider implementations and factory helpers.
pub mod provider;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::error::Result;

/// Progress reporting interface for LLM operations.
///
/// The LLM layer reports status changes (retry, fallback switch, etc.) through this trait
/// instead of depending on a concrete UI implementation.
pub trait ProgressReporter: Send + Sync {
    /// Appends an informative suffix to a progress message (for retries/fallbacks).
    fn append_suffix(&self, suffix: &str);
}

/// Stream chunks emitted by streaming providers.
///
/// Used for incremental delivery while generating commit messages.
///
/// # Variants
/// - [`Delta`] - text delta (append to existing content)
/// - [`Done`] - stream ended normally
/// - [`Error`] - stream terminated with an error
///
/// [`Delta`]: StreamChunk::Delta
/// [`Done`]: StreamChunk::Done
/// [`Error`]: StreamChunk::Error
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// Text delta (append to existing content).
    Delta(String),
    /// Stream ended normally.
    Done,
    /// Stream terminated with an error description.
    Error(String),
}

/// Handle for receiving a streaming response.
///
/// Wraps a Tokio channel receiver for incoming stream chunks.
///
/// # Usage example
/// ```no_run
/// use gcop_rs::llm::StreamChunk;
///
/// # async fn example(mut handle: gcop_rs::llm::StreamHandle) {
/// while let Some(chunk) = handle.receiver.recv().await {
///     match chunk {
///         StreamChunk::Delta(text) => print!("{}", text),
///         StreamChunk::Done => break,
///         StreamChunk::Error(err) => {
///             eprintln!("Error: {}", err);
///             break;
///         }
///     }
/// }
/// # }
/// ```
pub struct StreamHandle {
    /// Stream chunk receiver.
    pub receiver: mpsc::Receiver<StreamChunk>,
}

/// Unified interface implemented by all LLM providers.
///
/// Required capabilities:
/// - Commit message generation
/// - Code review
/// - Optional streaming output
///
/// # Implementer Notes
/// 1. Implement `Send + Sync` (required in async contexts).
/// 2. Handle network failures, timeouts, and rate limits.
/// 3. If `supports_streaming()` returns `false`, `generate_commit_message_streaming()` falls back to non-streaming.
///
/// # Built-In Implementations
/// - [`ClaudeProvider`](provider::claude::ClaudeProvider) - Anthropic Claude
/// - [`OpenAIProvider`](provider::openai::OpenAIProvider) - OpenAI/compatible API
/// - [`OllamaProvider`](provider::ollama::OllamaProvider) - Ollama local model
/// - [`FallbackProvider`](provider::fallback::FallbackProvider) - fallback wrapper for high availability
///
/// # Custom Provider Example
/// ```no_run
/// use async_trait::async_trait;
/// use gcop_rs::llm::{LLMProvider, CommitContext, ReviewResult, ReviewType};
/// use gcop_rs::error::Result;
///
/// struct MyProvider {
///     api_key: String,
/// }
///
/// #[async_trait]
/// impl LLMProvider for MyProvider {
///     async fn generate_commit_message(
///         &self,
///         diff: &str,
///         context: Option<CommitContext>,
///         progress: Option<&dyn gcop_rs::llm::ProgressReporter>,
///     ) -> Result<String> {
///         // Call custom API...
///         todo!()
///     }
///
///     async fn review_code(
///         &self,
///         diff: &str,
///         review_type: ReviewType,
///         custom_prompt: Option<&str>,
///         progress: Option<&dyn gcop_rs::llm::ProgressReporter>,
///     ) -> Result<ReviewResult> {
///         todo!()
///     }
///
///     fn name(&self) -> &str {
///         "my-provider"
///     }
///
///     async fn validate(&self) -> Result<()> {
///         // Validate API key validity...
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Generates a commit message.
    ///
    /// Generates a commit message from a diff and optional context.
    ///
    /// # Parameters
    /// - `diff`: git diff content (typically from `git diff --staged`)
    /// - `context`: optional context (branch, file list, user feedback, etc.)
    /// - `progress`: optional progress reporter for retry/fallback feedback
    ///
    /// # Returns
    /// - `Ok(message)` - generated commit message text
    /// - `Err(_)` - API error, network error, timeout, etc.
    ///
    /// # Error Handling
    /// Implementers should handle:
    /// - Network errors (retry based on `network.max_retries`, default: 3)
    /// - HTTP 429 (rate limiting, optionally using `Retry-After`)
    /// - Timeouts and other HTTP failures
    ///
    /// # Example
    /// ```ignore
    /// use gcop_rs::llm::{LLMProvider, CommitContext, provider::openai::OpenAIProvider};
    /// use gcop_rs::config::{ProviderConfig, NetworkConfig};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let config = ProviderConfig::default();
    /// let network_config = NetworkConfig::default();
    /// let provider = OpenAIProvider::new(&config, "openai", &network_config, false)?;
    /// let diff = "diff --git a/main.rs...";
    /// let message = provider.generate_commit_message(diff, None, None).await?;
    /// println!("Generated: {}", message);
    /// # Ok(())
    /// # }
    /// ```
    async fn generate_commit_message(
        &self,
        diff: &str,
        context: Option<CommitContext>,
        progress: Option<&dyn ProgressReporter>,
    ) -> Result<String>;

    /// Runs code review.
    ///
    /// Analyzes code changes and returns issues plus suggestions.
    ///
    /// # Parameters
    /// - `diff`: diff content to review
    /// - `review_type`: target scope (unstaged, single commit, range, file)
    /// - `custom_prompt`: optional user prompt appended to system guidance
    /// - `progress`: optional progress reporter
    ///
    /// # Returns
    /// - `Ok(result)` - structured review result
    /// - `Err(_)` - API error or network error
    ///
    /// # Review dimensions
    /// - Code quality (duplicate code, complexity, naming, etc.)
    /// - Potential bugs (null pointer, array out of bounds, resource leak, etc.)
    /// - Security issues (SQL injection, XSS, sensitive information leakage, etc.)
    /// - Performance issues (O(n²) algorithm, unnecessary copying, etc.)
    async fn review_code(
        &self,
        diff: &str,
        review_type: ReviewType,
        custom_prompt: Option<&str>,
        progress: Option<&dyn ProgressReporter>,
    ) -> Result<ReviewResult>;

    /// Provider name.
    ///
    /// Used for logs and error messages.
    ///
    /// # Example
    /// ```ignore
    /// use gcop_rs::llm::{LLMProvider, provider::openai::OpenAIProvider};
    /// use gcop_rs::config::{ProviderConfig, NetworkConfig};
    ///
    /// let config = ProviderConfig::default();
    /// let network_config = NetworkConfig::default();
    /// let provider = OpenAIProvider::new(&config, "openai", &network_config, false).unwrap();
    /// assert_eq!(provider.name(), "openai");
    /// ```
    #[allow(dead_code)]
    fn name(&self) -> &str;

    /// Validates provider configuration.
    ///
    /// Sends a lightweight test request to verify API key, endpoint, and model configuration.
    ///
    /// # Returns
    /// - `Ok(())` - configuration is valid
    /// - `Err(_)` - invalid configuration or network error
    ///
    /// # Error Types
    /// - [`GcopError::Llm`] - API key is invalid, model does not exist, etc.
    /// - [`GcopError::Network`] - Network errors, timeouts, etc.
    ///
    /// [`GcopError::Llm`]: crate::error::GcopError::Llm
    /// [`GcopError::Network`]: crate::error::GcopError::Network
    async fn validate(&self) -> Result<()>;

    /// Whether streaming output is supported.
    ///
    /// # Returns
    /// - `true` - supports streaming (SSE)
    /// - `false` - only non-streaming is supported (default)
    fn supports_streaming(&self) -> bool {
        false
    }

    /// Sends a direct query with pre-built system and user prompts.
    ///
    /// Unlike [`generate_commit_message`], this method does **not** run automatic
    /// prompt construction. The caller is responsible for building both prompts
    /// (e.g., via [`build_split_commit_prompt`](crate::llm::prompt::build_split_commit_prompt)).
    ///
    /// # Parameters
    /// - `system_prompt`: fully constructed system prompt
    /// - `user_prompt`: fully constructed user message
    /// - `progress`: optional progress reporter for retry/fallback feedback
    ///
    /// # Returns
    /// - `Ok(response)` - raw LLM response text
    /// - `Err(_)` - API error, network error, timeout, etc.
    async fn query(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        progress: Option<&dyn ProgressReporter>,
    ) -> Result<String> {
        // Default: not implemented. Overridden by the ApiBackend blanket impl
        // and FallbackProvider.
        let _ = (system_prompt, user_prompt, progress);
        Err(crate::error::GcopError::Llm(
            "query() not implemented for this provider".into(),
        ))
    }

    /// Generates a commit message as a stream.
    ///
    /// Returns a stream handle that yields text deltas in real time.
    ///
    /// # Parameters
    /// - `diff`: Git diff content
    /// - `context`: optional context information
    ///
    /// # Returns
    /// - `Ok(handle)` - stream handle
    /// - `Err(_)` - API error or network error
    ///
    /// # Default Implementation
    /// If streaming is unsupported, this falls back to `generate_commit_message()`
    /// and emits the full message as a single delta.
    ///
    /// # Example
    /// ```ignore
    /// use gcop_rs::llm::{LLMProvider, StreamChunk, provider::claude::ClaudeProvider};
    /// use gcop_rs::config::{ProviderConfig, NetworkConfig};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let config = ProviderConfig::default();
    /// let network_config = NetworkConfig::default();
    /// let provider = ClaudeProvider::new(&config, "claude", &network_config, false)?;
    /// let diff = "diff --git a/main.rs...";
    /// let mut handle = provider.generate_commit_message_streaming(diff, None).await?;
    ///
    /// while let Some(chunk) = handle.receiver.recv().await {
    ///     match chunk {
    ///         StreamChunk::Delta(text) => print!("{}", text),
    ///         StreamChunk::Done => break,
    ///         StreamChunk::Error(err) => eprintln!("Error: {}", err),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn generate_commit_message_streaming(
        &self,
        diff: &str,
        context: Option<CommitContext>,
    ) -> Result<StreamHandle> {
        let (tx, rx) = mpsc::channel(32);

        // Fall back to non-streaming and emit one full-message chunk.
        let result = self.generate_commit_message(diff, context, None).await;

        match result {
            Ok(message) => {
                let _ = tx.send(StreamChunk::Delta(message)).await;
                let _ = tx.send(StreamChunk::Done).await;
            }
            Err(e) => {
                let _ = tx.send(StreamChunk::Error(e.to_string())).await;
            }
        }

        Ok(StreamHandle { receiver: rx })
    }
}

use crate::config::CommitConvention;

/// Workspace scope metadata for monorepos.
///
/// Additional scope context passed into commit prompt generation.
///
/// # Fields
/// - `workspace_types`: detected workspace systems (for example `"cargo"`, `"pnpm"`)
/// - `packages`: list of affected package paths
/// - `suggested_scope`: suggested scope string (may be `None`)
/// - `has_root_changes`: whether root-level (non-package) files were changed
#[derive(Debug, Clone, Default)]
pub struct ScopeInfo {
    /// Detected workspace systems.
    pub workspace_types: Vec<String>,
    /// Affected package paths.
    pub packages: Vec<String>,
    /// Suggested commit scope string.
    pub suggested_scope: Option<String>,
    /// Whether there are root-level changes.
    pub has_root_changes: bool,
}

/// Context passed to commit-message generation.
///
/// Enriches prompt construction with git metadata and user constraints.
///
/// # Fields
/// - `files_changed`: list of changed file paths
/// - `insertions`: number of inserted lines
/// - `deletions`: number of deleted lines
/// - `branch_name`: current branch name (may be `None`, for example detached HEAD)
/// - `custom_prompt`: user-defined prompt (appended to system prompt)
/// - `user_feedback`: user feedback (used when regenerating, supports accumulation)
/// - `convention`: optional commit-convention config
///
/// # Example
/// ```
/// use gcop_rs::llm::CommitContext;
///
/// let context = CommitContext {
///     files_changed: vec!["src/main.rs".to_string()],
///     insertions: 10,
///     deletions: 3,
///     branch_name: Some("feature/login".to_string()),
///     custom_prompt: Some("Focus on security changes".to_string()),
///     user_feedback: vec!["Be more specific".to_string()],
///     convention: None,
///     scope_info: None,
/// };
/// ```
#[derive(Debug, Clone, Default)]
pub struct CommitContext {
    /// Changed file paths used as additional model context.
    pub files_changed: Vec<String>,
    /// Number of inserted lines in the diff.
    pub insertions: usize,
    /// Number of deleted lines in the diff.
    pub deletions: usize,
    /// Current branch name, if available.
    pub branch_name: Option<String>,
    /// Optional user-provided prompt additions.
    pub custom_prompt: Option<String>,
    /// Accumulated feedback from previous retry attempts.
    pub user_feedback: Vec<String>,
    /// Optional commit convention constraints.
    pub convention: Option<CommitConvention>,
    /// Workspace scope metadata (`None` when detection is disabled or not applicable).
    pub scope_info: Option<ScopeInfo>,
}

/// Review target type.
///
/// Selects which code changes to review.
///
/// # Variants
/// - [`UncommittedChanges`] - unstaged working tree changes (`index -> workdir`)
/// - [`SingleCommit`] - one commit by hash
/// - [`CommitRange`] - commit range (for example `HEAD~3..HEAD`)
/// - [`FileOrDir`] - one file path (directories are currently unsupported)
///
/// [`UncommittedChanges`]: ReviewType::UncommittedChanges
/// [`SingleCommit`]: ReviewType::SingleCommit
/// [`CommitRange`]: ReviewType::CommitRange
/// [`FileOrDir`]: ReviewType::FileOrDir
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum ReviewType {
    /// Review unstaged workspace changes (`index -> workdir`).
    UncommittedChanges,
    /// Review a single commit by hash.
    SingleCommit(String),
    /// Review a commit range (`A..B`).
    CommitRange(String),
    /// Review a single file path (directory recursion is not supported).
    FileOrDir(String),
}

/// Structured result returned by code review.
///
/// Parsed output from an LLM review response.
///
/// # Fields
/// - `summary`: high-level summary
/// - `issues`: issues discovered by the reviewer
/// - `suggestions`: additional improvement suggestions
///
/// # Example
/// ```
/// use gcop_rs::llm::{ReviewResult, ReviewIssue, IssueSeverity};
///
/// let result = ReviewResult {
///     summary: "Found 2 security issues".to_string(),
///     issues: vec![
///         ReviewIssue {
///             severity: IssueSeverity::Critical,
///             description: "Potential SQL injection".to_string(),
///             file: Some("db.rs".to_string()),
///             line: Some(42),
///         },
///     ],
///     suggestions: vec!["Use parameterized queries".to_string()],
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewResult {
    /// High-level summary generated by the reviewer model.
    pub summary: String,
    /// Structured list of discovered issues.
    pub issues: Vec<ReviewIssue>,
    /// Additional improvement suggestions.
    pub suggestions: Vec<String>,
}

/// A single issue found during review.
///
/// # Fields
/// - `severity`: issue severity (`Critical`/`Warning`/`Info`)
/// - `description`: issue description
/// - `file`: related file path (optional)
/// - `line`: related line number (optional)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewIssue {
    /// Severity level assigned to this issue.
    pub severity: IssueSeverity,
    /// Human-readable description of the issue.
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional file path related to the issue.
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional 1-based line number related to the issue.
    pub line: Option<usize>,
}

/// Issue severity level.
///
/// # Variants
/// - `Critical` - severe issue (security/correctness risk)
/// - `Warning` - notable issue (performance/maintainability concern)
/// - `Info` - informational suggestion
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    /// Critical issue (e.g., correctness/security risk).
    Critical,
    /// Warning-level issue (e.g., maintainability/performance concern).
    Warning,
    /// Informational suggestion.
    Info,
}

impl IssueSeverity {
    /// Numeric severity level used for filtering (`0` is most severe).
    pub fn level(&self) -> u8 {
        match self {
            Self::Critical => 0,
            Self::Warning => 1,
            Self::Info => 2,
        }
    }

    /// Parses severity from a config string.
    pub fn from_config_str(s: &str) -> Self {
        match s {
            "critical" => Self::Critical,
            "warning" => Self::Warning,
            _ => Self::Info,
        }
    }

    /// Returns localized label text.
    pub fn label(&self, colored: bool) -> String {
        match (self, colored) {
            (Self::Critical, true) => rust_i18n::t!("review.severity.critical").to_string(),
            (Self::Critical, false) => {
                rust_i18n::t!("review.severity.bracket_critical").to_string()
            }
            (Self::Warning, true) => rust_i18n::t!("review.severity.warning").to_string(),
            (Self::Warning, false) => rust_i18n::t!("review.severity.bracket_warning").to_string(),
            (Self::Info, true) => rust_i18n::t!("review.severity.info").to_string(),
            (Self::Info, false) => rust_i18n::t!("review.severity.bracket_info").to_string(),
        }
    }

    /// Returns a colored severity label.
    pub fn colored_label(&self) -> String {
        use colored::Colorize;
        let label = self.label(true);
        match self {
            Self::Critical => label.red().bold().to_string(),
            Self::Warning => label.yellow().bold().to_string(),
            Self::Info => label.blue().bold().to_string(),
        }
    }
}
