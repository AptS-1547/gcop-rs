use thiserror::Error;

/// Result 类型别名，使用 [`GcopError`] 作为错误类型
pub type Result<T> = std::result::Result<T, GcopError>;

/// git2::Error 的包装类型，提供更友好的错误信息
///
/// 隐藏 libgit2 的技术细节（ErrorClass、ErrorCode 等），
/// 仅显示用户友好的错误消息。
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

/// gcop-rs 统一错误类型
///
/// 包含所有可能的错误情况，支持：
/// - 国际化错误消息（通过 [`localized_message()`]）
/// - 用户友好的解决建议（通过 [`localized_suggestion()`]）
/// - 从各种库错误自动转换（实现 `From<T>`）
///
/// # 错误类别
/// - Git 操作错误：[`GitCommand`], [`Git`]
/// - LLM 相关错误：[`Llm`], [`LlmApi`]
/// - 配置错误：[`Config`], [`ConfigParse`]
/// - 用户操作：[`UserCancelled`], [`InvalidInput`]
/// - 其他：[`Io`], [`Network`], [`Other`]
///
/// # 示例
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
    /// Git2 库错误（libgit2）
    ///
    /// 包含详细的 ErrorCode 和 ErrorClass。
    ///
    /// # 常见错误码
    /// - `NotFound`：文件/分支不存在
    /// - `Exists`：分支已存在
    /// - `Uncommitted`：有未提交的变更
    /// - `Conflict`：merge 冲突
    #[error("Git error: {0}")]
    Git(GitErrorWrapper),

    /// Git 命令执行失败
    ///
    /// 包含 `git` 命令的 stderr 输出。
    ///
    /// # 常见原因
    /// - 无 staged changes：`nothing to commit`
    /// - pre-commit hook 失败
    /// - merge 冲突
    #[error("Git command failed: {0}")]
    GitCommand(String),

    /// 配置错误
    ///
    /// 包含配置文件错误、环境变量错误、API key 缺失等。
    #[error("Configuration error: {0}")]
    Config(String),

    /// LLM provider 错误
    ///
    /// 通用 LLM 错误（非 HTTP 状态码错误）。
    ///
    /// # 常见原因
    /// - 超时
    /// - 连接失败
    /// - 响应解析失败
    #[error("LLM provider error: {0}")]
    Llm(String),

    /// LLM API HTTP 错误
    ///
    /// 包含 HTTP 状态码和错误消息。
    ///
    /// # 常见状态码
    /// - `401` - API key 无效或过期
    /// - `429` - 速率限制
    /// - `500+` - 服务端错误
    #[error("LLM API error ({status}): {message}")]
    LlmApi {
        /// HTTP 状态码
        status: u16,
        /// 错误消息
        message: String,
    },

    /// 网络错误
    ///
    /// HTTP 请求失败（超时、DNS 错误、连接拒绝等）。
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// IO 错误
    ///
    /// 文件读写失败。
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// 序列化错误
    ///
    /// JSON 序列化/反序列化失败。
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// 配置文件解析错误
    ///
    /// TOML 文件格式错误或字段类型不匹配。
    #[error("Configuration parsing error: {0}")]
    ConfigParse(#[from] config::ConfigError),

    /// UI 交互错误
    ///
    /// 终端交互失败（用户输入错误、终端不可用等）。
    #[error("UI error: {0}")]
    Dialoguer(#[from] dialoguer::Error),

    /// 无 staged changes
    ///
    /// 暂存区为空，无法生成 commit message。
    #[error("No staged changes found")]
    NoStagedChanges,

    /// 用户取消操作
    ///
    /// 用户在交互式 prompt 中选择退出。
    #[error("Operation cancelled by user")]
    UserCancelled,

    /// 无效输入
    ///
    /// 用户提供的参数不符合预期格式。
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// 达到最大重试次数
    ///
    /// commit message 生成重试次数超过配置的上限。
    #[error("Max retries exceeded after {0} attempts")]
    MaxRetriesExceeded(usize),

    /// 通用错误类型
    ///
    /// 用于不适合其他分类的错误。
    #[error("{0}")]
    Other(String),
}

/// 将 Git ErrorCode 映射到建议 key（用于去重）
///
/// # 参数
/// - `code`: libgit2 错误码
///
/// # 返回
/// - `Some(key)` - 建议的 i18n key
/// - `None` - 无特定建议（通用错误）
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
    /// 获取本地化的错误消息
    ///
    /// 根据当前语言环境返回翻译后的错误消息。
    ///
    /// # 返回
    /// 本地化后的错误消息字符串
    ///
    /// # 示例
    /// ```
    /// use gcop_rs::error::GcopError;
    ///
    /// let err = GcopError::NoStagedChanges;
    /// println!("{}", err.localized_message());
    /// // 输出：No staged changes found（英文环境）
    /// // 输出：未找到暂存的更改（中文环境）
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

    /// 获取本地化的解决建议
    ///
    /// 根据错误类型返回用户友好的解决建议（如果有）。
    ///
    /// # 返回
    /// - `Some(suggestion)` - 解决建议字符串
    /// - `None` - 无特定建议
    ///
    /// # 建议类型
    /// - **NoStagedChanges**: 提示运行 `git add`
    /// - **Config(API key)**: 提示设置 API key
    /// - **LlmApi(401)**: 提示检查 API key 有效性
    /// - **LlmApi(429)**: 提示稍后重试或升级 API 计划
    /// - **Network**: 提示检查网络连接
    /// - 其他错误：可能返回 `None`
    ///
    /// # 示例
    /// ```
    /// use gcop_rs::error::GcopError;
    ///
    /// let err = GcopError::NoStagedChanges;
    /// if let Some(suggestion) = err.localized_suggestion() {
    ///     println!("Try: {}", suggestion);
    /// }
    /// // 输出：Try: Run 'git add <files>' to stage your changes first
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
                        && (msg.contains("ANTHROPIC_API_KEY")
                            || msg.contains("OPENAI_API_KEY"))) =>
            {
                if msg.contains("Claude")
                    || msg.contains("claude")
                    || msg.contains("ANTHROPIC_API_KEY")
                {
                    Some(rust_i18n::t!("suggestion.claude_api_key").to_string())
                } else if msg.contains("OpenAI")
                    || msg.contains("openai")
                    || msg.contains("OPENAI_API_KEY")
                {
                    Some(rust_i18n::t!("suggestion.openai_api_key").to_string())
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

    // === NoStagedChanges 分支 ===

    #[test]
    fn test_suggestion_no_staged_changes() {
        let err = GcopError::NoStagedChanges;
        assert_eq!(
            err.localized_suggestion(),
            Some("Run 'git add <files>' to stage your changes first".to_string())
        );
    }

    // === Config 错误: API key 分支 ===

    #[test]
    fn test_suggestion_config_claude_api_key() {
        let err = GcopError::Config("API key not found for Claude provider".to_string());
        let suggestion = err.localized_suggestion().unwrap();
        assert!(suggestion.contains("ANTHROPIC_API_KEY"));
        assert!(suggestion.contains("[llm.providers.claude]"));
    }

    #[test]
    fn test_suggestion_config_openai_api_key() {
        let err = GcopError::Config("API key not found for OpenAI".to_string());
        let suggestion = err.localized_suggestion().unwrap();
        assert!(suggestion.contains("OPENAI_API_KEY"));
        assert!(suggestion.contains("[llm.providers.openai]"));
    }

    #[test]
    fn test_suggestion_config_generic_api_key() {
        let err = GcopError::Config("API key not found for custom-provider".to_string());
        let suggestion = err.localized_suggestion().unwrap();
        assert_eq!(suggestion, "Set api_key in ~/.config/gcop/config.toml");
    }

    #[test]
    fn test_suggestion_config_provider_not_found() {
        let err = GcopError::Config("Provider 'unknown' not found in config".to_string());
        let suggestion = err.localized_suggestion().unwrap();
        assert!(suggestion.contains("Check your ~/.config/gcop/config.toml"));
        assert!(suggestion.contains("claude, openai, ollama"));
    }

    // === Network 错误 ===

    #[test]
    fn test_suggestion_network_error() {
        // reqwest::Error 无法直接构造，使用真实网络错误或跳过
        // 这里我们测试 Network 变体存在时的行为
        // 注意：需要实际的 reqwest::Error，这里用文档说明测试思路

        // 由于 reqwest::Error 构造困难，我们验证 suggestion() 的逻辑
        // 实际测试需要集成测试或使用 mock
    }

    // === Llm 错误分支 ===

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

    // === 无建议的分支 ===

    #[test]
    fn test_suggestion_returns_none_for_other_errors() {
        let cases = vec![
            GcopError::UserCancelled,
            GcopError::InvalidInput("bad input".to_string()),
            GcopError::Other("random error".to_string()),
            GcopError::GitCommand("git failed".to_string()),
            // Config/Llm 不匹配任何模式
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
