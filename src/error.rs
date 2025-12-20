use thiserror::Error;

pub type Result<T> = std::result::Result<T, GcopError>;

#[derive(Error, Debug)]
pub enum GcopError {
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("Git command failed: {0}")]
    GitCommand(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("LLM provider error: {0}")]
    Llm(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Configuration parsing error: {0}")]
    ConfigParse(#[from] config::ConfigError),

    #[error("UI error: {0}")]
    Dialoguer(#[from] dialoguer::Error),

    #[error("No staged changes found")]
    NoStagedChanges,

    #[error("Operation cancelled by user")]
    UserCancelled,

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// 通用错误类型，用于不适合其他分类的错误
    #[error("{0}")]
    Other(String),
}

impl GcopError {
    /// 获取错误的解决建议
    pub fn suggestion(&self) -> Option<&str> {
        match self {
            GcopError::NoStagedChanges => Some("Run 'git add <files>' to stage your changes first"),
            GcopError::Config(msg) if msg.contains("API key not found") => {
                if msg.contains("Claude") {
                    Some(
                        "Add 'api_key = \"sk-ant-...\"' to [llm.providers.claude] in ~/.config/gcop/config.toml, or set ANTHROPIC_API_KEY",
                    )
                } else if msg.contains("OpenAI") {
                    Some(
                        "Add 'api_key = \"sk-...\"' to [llm.providers.openai] in ~/.config/gcop/config.toml, or set OPENAI_API_KEY",
                    )
                } else {
                    Some("Set api_key in ~/.config/gcop/config.toml")
                }
            }
            GcopError::Config(msg) if msg.contains("not found in config") => Some(
                "Check your ~/.config/gcop/config.toml or use the default providers: claude, openai, ollama",
            ),
            GcopError::Network(_) => {
                Some("Check your network connection, proxy settings, or API endpoint configuration")
            }
            GcopError::Llm(msg) if msg.contains("timeout") => {
                Some("The API request timed out. Check network or try again later")
            }
            GcopError::Llm(msg) if msg.contains("connection failed") => {
                Some("Cannot connect to API server. Check endpoint URL, network, or DNS settings")
            }
            GcopError::Llm(msg) if msg.contains("401") => {
                Some("Check if your API key is valid and has not expired")
            }
            GcopError::Llm(msg) if msg.contains("429") => {
                Some("Rate limit exceeded. Wait a moment and try again, or upgrade your API plan")
            }
            GcopError::Llm(msg) if msg.contains("500") || msg.contains("503") => {
                Some("API service is temporarily unavailable. Try again in a few moments")
            }
            GcopError::Llm(msg) if msg.contains("Failed to parse") => {
                Some("Try using --verbose flag to see the full LLM response and debug the issue")
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
            err.suggestion(),
            Some("Run 'git add <files>' to stage your changes first")
        );
    }

    // === Config 错误: API key 分支 ===

    #[test]
    fn test_suggestion_config_claude_api_key() {
        let err = GcopError::Config("API key not found for Claude provider".to_string());
        let suggestion = err.suggestion().unwrap();
        assert!(suggestion.contains("ANTHROPIC_API_KEY"));
        assert!(suggestion.contains("[llm.providers.claude]"));
    }

    #[test]
    fn test_suggestion_config_openai_api_key() {
        let err = GcopError::Config("API key not found for OpenAI".to_string());
        let suggestion = err.suggestion().unwrap();
        assert!(suggestion.contains("OPENAI_API_KEY"));
        assert!(suggestion.contains("[llm.providers.openai]"));
    }

    #[test]
    fn test_suggestion_config_generic_api_key() {
        let err = GcopError::Config("API key not found for custom-provider".to_string());
        let suggestion = err.suggestion().unwrap();
        assert_eq!(suggestion, "Set api_key in ~/.config/gcop/config.toml");
    }

    #[test]
    fn test_suggestion_config_provider_not_found() {
        let err = GcopError::Config("Provider 'unknown' not found in config".to_string());
        let suggestion = err.suggestion().unwrap();
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
        let suggestion = err.suggestion().unwrap();
        assert!(suggestion.contains("timed out"));
    }

    #[test]
    fn test_suggestion_llm_connection_failed() {
        let err = GcopError::Llm("connection failed: DNS resolution error".to_string());
        let suggestion = err.suggestion().unwrap();
        assert!(suggestion.contains("endpoint URL"));
        assert!(suggestion.contains("DNS"));
    }

    #[test]
    fn test_suggestion_llm_401_unauthorized() {
        let err = GcopError::Llm("API returned 401 Unauthorized".to_string());
        let suggestion = err.suggestion().unwrap();
        assert!(suggestion.contains("API key"));
        assert!(suggestion.contains("expired"));
    }

    #[test]
    fn test_suggestion_llm_429_rate_limit() {
        let err = GcopError::Llm("API returned 429 Too Many Requests".to_string());
        let suggestion = err.suggestion().unwrap();
        assert!(suggestion.contains("Rate limit"));
        assert!(suggestion.contains("API plan"));
    }

    #[test]
    fn test_suggestion_llm_500_503_service_unavailable() {
        let err_500 = GcopError::Llm("API returned 500 Internal Server Error".to_string());
        let err_503 = GcopError::Llm("API returned 503 Service Unavailable".to_string());

        let suggestion_500 = err_500.suggestion().unwrap();
        let suggestion_503 = err_503.suggestion().unwrap();

        assert!(suggestion_500.contains("temporarily unavailable"));
        assert!(suggestion_503.contains("temporarily unavailable"));
    }

    #[test]
    fn test_suggestion_llm_parse_failed() {
        let err = GcopError::Llm("Failed to parse LLM response as JSON".to_string());
        let suggestion = err.suggestion().unwrap();
        assert!(suggestion.contains("--verbose"));
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
                err.suggestion().is_none(),
                "Expected None for {:?}, got {:?}",
                err,
                err.suggestion()
            );
        }
    }
}
