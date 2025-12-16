use thiserror::Error;

pub type Result<T> = std::result::Result<T, GcopError>;

#[derive(Error, Debug)]
pub enum GcopError {
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("LLM provider error: {0}")]
    LLM(String),

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

    #[error("{0}")]
    Other(String),
}
