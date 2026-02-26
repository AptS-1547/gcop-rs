//! LLM abstractions, shared types, and provider traits.
//!
//! This module defines the provider interface used by commit generation
//! and code review flows.

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
/// - [`Retry`] - stream is being retried; UI should reset its buffer
///
/// [`Delta`]: StreamChunk::Delta
/// [`Done`]: StreamChunk::Done
/// [`Error`]: StreamChunk::Error
/// [`Retry`]: StreamChunk::Retry
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// Text delta (append to existing content).
    Delta(String),
    /// Stream ended normally.
    Done,
    /// Stream terminated with an error description.
    Error(String),
    /// Stream is being retried; UI should clear buffered output.
    Retry,
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
///         StreamChunk::Retry => { /* stream retrying, reset buffer */ }
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
/// # Architecture
///
/// The only **required** method is [`send_prompt`], which sends a pre-built
/// `(system, user)` prompt pair to the LLM and returns the raw response.
/// All higher-level methods (`generate_commit_message`, `review_code`,
/// `generate_commit_message_streaming`) are default implementations that
/// build prompts via [`llm::prompt`](crate::llm::prompt) and delegate to
/// `send_prompt` / `send_prompt_streaming`.
///
/// Callers that need custom prompt construction (e.g., split-commit flow)
/// call `send_prompt` directly, avoiding double-wrapping.
///
/// # Implementer Notes
/// 1. Implement `Send + Sync` (required in async contexts).
/// 2. Handle network failures, timeouts, and rate limits inside `send_prompt`.
/// 3. Override `send_prompt_streaming` if the backend supports SSE.
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
/// use gcop_rs::llm::{LLMProvider, ReviewResult, ReviewType};
/// use gcop_rs::error::Result;
///
/// struct MyProvider {
///     api_key: String,
/// }
///
/// #[async_trait]
/// impl LLMProvider for MyProvider {
///     async fn send_prompt(
///         &self,
///         system_prompt: &str,
///         user_prompt: &str,
///         _progress: Option<&dyn gcop_rs::llm::ProgressReporter>,
///     ) -> Result<String> {
///         // Call custom API with system_prompt + user_prompt ...
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
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Sends a pre-built prompt pair to the LLM.
    ///
    /// This is the core method all providers must implement.
    /// Callers are responsible for constructing the system and user prompts
    /// (e.g., via [`build_commit_prompt_split`](crate::llm::prompt::build_commit_prompt_split)
    /// or [`build_split_commit_prompt`](crate::llm::prompt::build_split_commit_prompt)).
    ///
    /// # Parameters
    /// - `system_prompt`: fully constructed system prompt
    /// - `user_prompt`: fully constructed user message
    /// - `progress`: optional progress reporter for retry/fallback feedback
    async fn send_prompt(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        progress: Option<&dyn ProgressReporter>,
    ) -> Result<String>;

    /// Sends a pre-built prompt pair as a stream.
    ///
    /// Default: falls back to [`send_prompt`](Self::send_prompt) and emits
    /// the full response as a single delta chunk.
    async fn send_prompt_streaming(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<StreamHandle> {
        let (tx, rx) = mpsc::channel(32);
        let result = self.send_prompt(system_prompt, user_prompt, None).await;
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

    /// Convenience: generates a commit message from diff + context.
    ///
    /// Builds the prompt via [`build_commit_prompt_split`](crate::llm::prompt::build_commit_prompt_split),
    /// then delegates to [`send_prompt`](Self::send_prompt).
    async fn generate_commit_message(
        &self,
        diff: &str,
        context: Option<CommitContext>,
        progress: Option<&dyn ProgressReporter>,
    ) -> Result<String> {
        let ctx = context.unwrap_or_default();
        let (system, user) = crate::llm::prompt::build_commit_prompt_split(
            diff,
            &ctx,
            ctx.custom_prompt.as_deref(),
            ctx.convention.as_ref(),
        );
        let response = self.send_prompt(&system, &user, progress).await?;
        tracing::debug!("Generated commit message: {}", response);
        Ok(response)
    }

    /// Runs code review.
    ///
    /// Analyzes code changes and returns issues plus suggestions.
    ///
    /// # Parameters
    /// - `diff`: diff content to review
    /// - `review_type`: target scope (unstaged, single commit, range, file)
    /// - `custom_prompt`: optional review system prompt override (JSON constraints are still appended)
    /// - `progress`: optional progress reporter
    async fn review_code(
        &self,
        diff: &str,
        review_type: ReviewType,
        custom_prompt: Option<&str>,
        progress: Option<&dyn ProgressReporter>,
    ) -> Result<ReviewResult>;

    /// Provider name (used for logs and error messages).
    #[allow(dead_code)]
    fn name(&self) -> &str;

    /// Validates provider configuration.
    async fn validate(&self) -> Result<()>;

    /// Whether streaming output is supported.
    fn supports_streaming(&self) -> bool {
        false
    }

    /// Convenience: generates a commit message as a stream.
    ///
    /// Builds the prompt via [`build_commit_prompt_split`](crate::llm::prompt::build_commit_prompt_split),
    /// then delegates to [`send_prompt_streaming`](Self::send_prompt_streaming).
    async fn generate_commit_message_streaming(
        &self,
        diff: &str,
        context: Option<CommitContext>,
    ) -> Result<StreamHandle> {
        let ctx = context.unwrap_or_default();
        let (system, user) = crate::llm::prompt::build_commit_prompt_split(
            diff,
            &ctx,
            ctx.custom_prompt.as_deref(),
            ctx.convention.as_ref(),
        );
        self.send_prompt_streaming(&system, &user).await
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
/// - `custom_prompt`: user-defined prompt customization (normal commit replaces base prompt, split commit appends additional constraints)
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
    /// Optional user-provided prompt customization.
    ///
    /// Normal commit mode treats this as a system prompt override.
    /// Split commit mode appends it as extra grouping instructions.
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
