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
