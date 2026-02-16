use thiserror::Error;

/// Result type alias, use [`GcopError`] as error type
pub type Result<T> = std::result::Result<T, GcopError>;

/// A wrapper type for git2::Error that provides more friendly error information
///
/// Hide technical details of libgit2 (ErrorClass, ErrorCode, etc.),
/// Display only user-friendly error messages.
#[derive(Debug)]
pub struct GitErrorWrapper(pub git2::Error);

impl std::fmt::Display for GitErrorWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.message())
    }
}

impl std::error::Error for GitErrorWrapper {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

impl From<git2::Error> for GcopError {
    fn from(e: git2::Error) -> Self {
        GcopError::Git(GitErrorWrapper(e))
    }
}

/// gcop-rs unified error types
///
/// Contains all possible error conditions, supporting:
/// - Internationalized error messages (via [`localized_message()`])
/// - User-friendly solution suggestions (via [`localized_suggestion()`])
/// - Automatic conversion from various library errors (implementing `From<T>`)
///
/// # Error category
/// - Git operation errors: [`GitCommand`], [`Git`]
/// - LLM related errors: [`Llm`], [`LlmApi`]
/// - Configuration errors: [`Config`], [`ConfigParse`]
/// - User operations: [`UserCancelled`], [`InvalidInput`]
/// - Others: [`Io`], [`Network`], [`Other`]
///
/// # Example
/// ```
/// use gcop_rs::error::{GcopError, Result};
///
/// fn example() -> Result<()> {
///     let err = GcopError::NoStagedChanges;
///     println!("Error: {}", err.localized_message());
///     if let Some(suggestion) = err.localized_suggestion() {
///         println!("Suggestion: {}", suggestion);
///     }
///     Err(err)
/// }
/// ```
///
/// [`GitCommand`]: GcopError::GitCommand
/// [`Git`]: GcopError::Git
/// [`Llm`]: GcopError::Llm
/// [`LlmApi`]: GcopError::LlmApi
/// [`Config`]: GcopError::Config
/// [`ConfigParse`]: GcopError::ConfigParse
/// [`UserCancelled`]: GcopError::UserCancelled
/// [`InvalidInput`]: GcopError::InvalidInput
/// [`Io`]: GcopError::Io
/// [`Network`]: GcopError::Network
/// [`Other`]: GcopError::Other
/// [`localized_message()`]: GcopError::localized_message
/// [`localized_suggestion()`]: GcopError::localized_suggestion
#[derive(Error, Debug)]
pub enum GcopError {
    /// Git2 library error (libgit2)
    ///
    /// Contains detailed ErrorCode and ErrorClass.
    ///
    /// # Common error codes
    /// - `NotFound`: file/branch does not exist
    /// - `Exists`: branch already exists
    /// - `Uncommitted`: There are uncommitted changes
    /// - `Conflict`: merge conflict
    #[error("Git error: {0}")]
    Git(GitErrorWrapper),

    /// Git command execution failed
    ///
    /// Contains the stderr output of the `git` command.
    ///
    /// # Common reasons
    /// - No staged changes: `nothing to commit`
    /// - pre-commit hook failed
    /// - merge conflicts
    #[error("Git command failed: {0}")]
    GitCommand(String),

    /// Configuration error
    ///
    /// Including configuration file errors, environment variable errors, missing API keys, etc.
    #[error("Configuration error: {0}")]
    Config(String),

    /// LLM provider error
    ///
    /// Generic LLM errors (non-HTTP status code errors).
    ///
    /// # Common reasons
    /// - time out
    /// - Connection failed
    /// - Response parsing failed
    #[error("LLM provider error: {0}")]
    Llm(String),

    /// LLM API HTTP Error
    ///
    /// Contains HTTP status codes and error messages.
    ///
    /// # Common status codes
    /// - `401` - API key is invalid or expired
    /// - `429` - rate limit
    /// - `500+` - Server error
    #[error("LLM API error ({status}): {message}")]
    LlmApi {
        /// HTTP status code
        status: u16,
        /// error message
        message: String,
    },

    /// network error
    ///
    /// HTTP request failed (timeout, DNS error, connection refused, etc.).
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// IO error
    ///
    /// File reading and writing failed.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// serialization error
    ///
    /// JSON serialization/deserialization failed.
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// Configuration file parsing error
    ///
    /// The TOML file is malformed or the field types do not match.
    #[error("Configuration parsing error: {0}")]
    ConfigParse(#[from] config::ConfigError),

    /// UI interaction errors
    ///
    /// Terminal interaction failed (user input error, terminal unavailable, etc.).
    #[error("UI error: {0}")]
    Dialoguer(#[from] dialoguer::Error),

    /// No staged changes
    ///
    /// The staging area is empty and the commit message cannot be generated.
    #[error("No staged changes found")]
    NoStagedChanges,

    /// User cancels operation
    ///
    /// The user chooses to exit at the interactive prompt.
    #[error("Operation cancelled by user")]
    UserCancelled,

    /// Invalid input
    ///
    /// The user-supplied parameter does not conform to the expected format.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Maximum number of retries reached
    ///
    /// The number of commit message generation retries exceeds the configured upper limit.
    #[error("Max retries exceeded after {0} attempts")]
    MaxRetriesExceeded(usize),

    /// Common error types
    ///
    /// Used for errors that do not fit into other categories.
    #[error("{0}")]
    Other(String),
}

/// Map Git ErrorCode to suggestion key (for deduplication)
///
/// # Parameters
/// - `code`: libgit2 error code
///
/// # Returns
/// - `Some(key)` - suggested i18n key
/// - `None` - no specific advice (generic error)
fn git_error_code_to_key(code: git2::ErrorCode) -> Option<&'static str> {
    use git2::ErrorCode;
    match code {
        ErrorCode::GenericError | ErrorCode::BufSize | ErrorCode::User => None,
        ErrorCode::NotFound => Some("git_not_found"),
        ErrorCode::Exists => Some("git_exists"),
        ErrorCode::Ambiguous => Some("git_ambiguous"),
        ErrorCode::BareRepo => Some("git_bare_repo"),
        ErrorCode::UnbornBranch => Some("git_unborn_branch"),
        ErrorCode::Directory => Some("git_directory"),
        ErrorCode::Owner => Some("git_owner"),
        ErrorCode::Unmerged => Some("git_unmerged"),
        ErrorCode::Conflict | ErrorCode::MergeConflict => Some("git_conflict"),
        ErrorCode::NotFastForward => Some("git_not_fast_forward"),
        ErrorCode::InvalidSpec => Some("git_invalid_spec"),
        ErrorCode::Modified => Some("git_modified"),
        ErrorCode::Uncommitted => Some("git_uncommitted"),
        ErrorCode::IndexDirty => Some("git_index_dirty"),
        ErrorCode::Locked => Some("git_locked"),
        ErrorCode::Auth => Some("git_auth"),
        ErrorCode::Certificate => Some("git_certificate"),
        ErrorCode::Applied => Some("git_applied"),
        ErrorCode::ApplyFail => Some("git_apply_fail"),
        ErrorCode::Peel => Some("git_peel"),
        ErrorCode::Eof => Some("git_eof"),
        ErrorCode::Invalid => Some("git_invalid"),
        ErrorCode::HashsumMismatch => Some("git_hashsum_mismatch"),
        ErrorCode::Timeout => Some("git_timeout"),
    }
}

impl GcopError {
    /// Get localized error messages
    ///
    /// Returns a translated error message based on the current locale.
    ///
    /// # Returns
    /// Localized error message string
    ///
    /// # Example
    /// ```
    /// use gcop_rs::error::GcopError;
    ///
    /// let err = GcopError::NoStagedChanges;
    /// println!("{}", err.localized_message());
    /// // Output: No staged changes found (English environment)
    /// // Output: No staged changes found (Chinese environment)
    /// ```
    pub fn localized_message(&self) -> String {
        match self {
            GcopError::Git(wrapper) => {
                rust_i18n::t!("error.git", detail = wrapper.to_string()).to_string()
            }
            GcopError::GitCommand(msg) => {
                rust_i18n::t!("error.git_command", detail = msg.as_str()).to_string()
            }
            GcopError::Config(msg) => {
                rust_i18n::t!("error.config", detail = msg.as_str()).to_string()
            }
            GcopError::Llm(msg) => rust_i18n::t!("error.llm", detail = msg.as_str()).to_string(),
            GcopError::LlmApi { status, message } => {
                rust_i18n::t!("error.llm_api", status = status, message = message.as_str())
                    .to_string()
            }
            GcopError::Network(e) => {
                rust_i18n::t!("error.network", detail = e.to_string()).to_string()
            }
            GcopError::Io(e) => rust_i18n::t!("error.io", detail = e.to_string()).to_string(),
            GcopError::Serde(e) => rust_i18n::t!("error.serde", detail = e.to_string()).to_string(),
            GcopError::ConfigParse(e) => {
                rust_i18n::t!("error.config_parse", detail = e.to_string()).to_string()
            }
            GcopError::Dialoguer(e) => {
                rust_i18n::t!("error.ui", detail = e.to_string()).to_string()
            }
            GcopError::NoStagedChanges => rust_i18n::t!("error.no_staged_changes").to_string(),
            GcopError::UserCancelled => rust_i18n::t!("error.user_cancelled").to_string(),
            GcopError::InvalidInput(msg) => {
                rust_i18n::t!("error.invalid_input", detail = msg.as_str()).to_string()
            }
            GcopError::MaxRetriesExceeded(n) => {
                rust_i18n::t!("error.max_retries", count = n).to_string()
            }
            GcopError::Other(msg) => msg.clone(),
        }
    }

    /// Get localized solutions
    ///
    /// Returns user-friendly resolution suggestions based on the error type (if any).
    ///
    /// # Returns
    /// - `Some(suggestion)` - solution suggestion string
    /// - `None` - no specific suggestions
    ///
    /// # Suggestion type
    /// - **NoStagedChanges**: Prompt to run `git add`
    /// - **Config(API key)**: Prompt to set API key
    /// - **LlmApi(401)**: Prompt to check API key validity
    /// - **LlmApi(429)**: Prompt to try again later or upgrade the API plan
    /// - **Network**: Prompt to check network connection
    /// - Other errors: may return `None`
    ///
    /// # Example
    /// ```
    /// use gcop_rs::error::GcopError;
    ///
    /// let err = GcopError::NoStagedChanges;
    /// if let Some(suggestion) = err.localized_suggestion() {
    ///     println!("Try: {}", suggestion);
    /// }
    /// // Output: Try: Run 'git add <files>' to stage your changes first
    /// ```
    pub fn localized_suggestion(&self) -> Option<String> {
        match self {
            GcopError::Git(wrapper) => git_error_code_to_key(wrapper.0.code())
                .map(|key| rust_i18n::t!(format!("suggestion.{}", key)).to_string()),
            GcopError::NoStagedChanges => {
                Some(rust_i18n::t!("suggestion.no_staged_changes").to_string())
            }
            GcopError::Config(msg)
                if msg.contains("API key not found")
                    || msg.contains("API key")
                    || msg.contains("api_key")
                    || msg.contains("API key 为空")
                    || (msg.contains("未找到")
                        && (msg.contains("API key") || msg.contains("api_key"))) =>
            {
                if msg.contains("Claude") || msg.contains("claude") {
                    Some(rust_i18n::t!("suggestion.claude_api_key").to_string())
                } else if msg.contains("OpenAI") || msg.contains("openai") {
                    Some(rust_i18n::t!("suggestion.openai_api_key").to_string())
                } else if msg.contains("Gemini") || msg.contains("gemini") {
                    Some(rust_i18n::t!("suggestion.gemini_api_key").to_string())
                } else {
                    Some(rust_i18n::t!("suggestion.generic_api_key").to_string())
                }
            }
            GcopError::Config(msg)
                if msg.contains("not found in config")
                    || msg.contains("未找到 provider")
                    || msg.contains("配置中未找到 provider") =>
            {
                Some(rust_i18n::t!("suggestion.provider_not_found").to_string())
            }
            GcopError::Network(_) => Some(rust_i18n::t!("suggestion.network").to_string()),
            GcopError::LlmApi { status: 401, .. } => {
                Some(rust_i18n::t!("suggestion.llm_401").to_string())
            }
            GcopError::LlmApi { status: 429, .. } => {
                Some(rust_i18n::t!("suggestion.llm_429").to_string())
            }
            GcopError::LlmApi { status, .. } if *status >= 500 => {
                Some(rust_i18n::t!("suggestion.llm_5xx").to_string())
            }
            GcopError::Llm(msg) if msg.contains("timeout") || msg.contains("超时") => {
                Some(rust_i18n::t!("suggestion.llm_timeout").to_string())
            }
            GcopError::Llm(msg)
                if msg.contains("connection failed") || msg.contains("连接失败") =>
            {
                Some(rust_i18n::t!("suggestion.llm_connection").to_string())
            }
            GcopError::Llm(msg)
                if msg.contains("Failed to parse")
                    || (msg.contains("解析") && msg.contains("响应")) =>
            {
                Some(rust_i18n::t!("suggestion.llm_parse").to_string())
            }
            GcopError::MaxRetriesExceeded(_) => {
                Some(rust_i18n::t!("suggestion.max_retries").to_string())
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === NoStagedChanges branch ===

    #[test]
    fn test_suggestion_no_staged_changes() {
        let err = GcopError::NoStagedChanges;
        assert_eq!(
            err.localized_suggestion(),
            Some("Run 'git add <files>' to stage your changes first".to_string())
        );
    }

    // === Config Error: API key branch ===

    #[test]
    fn test_suggestion_config_claude_api_key() {
        let err = GcopError::Config("API key not found for Claude provider".to_string());
        let suggestion = err.localized_suggestion().unwrap();
        assert!(!suggestion.contains("GCOP__"));
        assert!(suggestion.contains("[llm.providers.claude]"));
    }

    #[test]
    fn test_suggestion_config_openai_api_key() {
        let err = GcopError::Config("API key not found for OpenAI".to_string());
        let suggestion = err.localized_suggestion().unwrap();
        assert!(!suggestion.contains("GCOP__"));
        assert!(suggestion.contains("[llm.providers.openai]"));
    }

    #[test]
    fn test_suggestion_config_generic_api_key() {
        let err = GcopError::Config("API key not found for custom-provider".to_string());
        let suggestion = err.localized_suggestion().unwrap();
        assert_eq!(suggestion, "Set api_key in config.toml");
    }

    #[test]
    fn test_suggestion_config_provider_not_found() {
        let err = GcopError::Config("Provider 'unknown' not found in config".to_string());
        let suggestion = err.localized_suggestion().unwrap();
        assert!(suggestion.contains("Check your ~/.config/gcop/config.toml"));
        assert!(suggestion.contains("claude, openai, ollama"));
    }

    // === Network Error ===

    #[test]
    fn test_suggestion_network_error() {
        // reqwest::Error cannot be constructed directly, use real network error or skip
        // Here we test the behavior when the Network variant is present
        // Note: Actual reqwest::Error is required. Here is a document describing the test idea.

        // Since reqwest::Error is difficult to construct, we verify the logic of suggestion()
        // Actual testing requires integration testing or using mocks
    }

    // === Llm wrong branch ===

    #[test]
    fn test_suggestion_llm_timeout() {
        let err = GcopError::Llm("Request timeout after 30s".to_string());
        let suggestion = err.localized_suggestion().unwrap();
        assert!(suggestion.contains("timed out"));
    }

    #[test]
    fn test_suggestion_llm_connection_failed() {
        let err = GcopError::Llm("connection failed: DNS resolution error".to_string());
        let suggestion = err.localized_suggestion().unwrap();
        assert!(suggestion.contains("endpoint URL"));
        assert!(suggestion.contains("DNS"));
    }

    #[test]
    fn test_suggestion_llm_api_401_unauthorized() {
        let err = GcopError::LlmApi {
            status: 401,
            message: "Unauthorized".to_string(),
        };
        let suggestion = err.localized_suggestion().unwrap();
        assert!(suggestion.contains("API key"));
        assert!(suggestion.contains("expired"));
    }

    #[test]
    fn test_suggestion_llm_api_429_rate_limit() {
        let err = GcopError::LlmApi {
            status: 429,
            message: "Too Many Requests".to_string(),
        };
        let suggestion = err.localized_suggestion().unwrap();
        assert!(suggestion.contains("Rate limit"));
        assert!(suggestion.contains("API plan"));
    }

    #[test]
    fn test_suggestion_llm_api_5xx_service_unavailable() {
        let err_500 = GcopError::LlmApi {
            status: 500,
            message: "Internal Server Error".to_string(),
        };
        let err_503 = GcopError::LlmApi {
            status: 503,
            message: "Service Unavailable".to_string(),
        };

        let suggestion_500 = err_500.localized_suggestion().unwrap();
        let suggestion_503 = err_503.localized_suggestion().unwrap();

        assert!(suggestion_500.contains("temporarily unavailable"));
        assert!(suggestion_503.contains("temporarily unavailable"));
    }

    #[test]
    fn test_suggestion_llm_parse_failed() {
        let err = GcopError::Llm("Failed to parse LLM response as JSON".to_string());
        let suggestion = err.localized_suggestion().unwrap();
        assert!(suggestion.contains("--verbose"));
    }

    #[test]
    fn test_suggestion_max_retries_exceeded() {
        let err = GcopError::MaxRetriesExceeded(5);
        let suggestion = err.localized_suggestion().unwrap();
        assert!(suggestion.contains("feedback"));
    }

    // === No suggested branches ===

    #[test]
    fn test_suggestion_returns_none_for_other_errors() {
        let cases = vec![
            GcopError::UserCancelled,
            GcopError::InvalidInput("bad input".to_string()),
            GcopError::Other("random error".to_string()),
            GcopError::GitCommand("git failed".to_string()),
            // Config/Llm does not match any pattern
            GcopError::Config("some random config error".to_string()),
            GcopError::Llm("some random llm error".to_string()),
        ];

        for err in cases {
            assert!(
                err.localized_suggestion().is_none(),
                "Expected None for {:?}, got {:?}",
                err,
                err.localized_suggestion()
            );
        }
    }
}
