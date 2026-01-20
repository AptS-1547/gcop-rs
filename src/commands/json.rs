use serde::Serialize;

use crate::error::GcopError;

/// JSON 错误输出结构（统一）
#[derive(Debug, Serialize)]
pub struct ErrorJson {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl ErrorJson {
    /// 从 GcopError 创建 ErrorJson
    pub fn from_error(err: &GcopError) -> Self {
        Self {
            code: error_to_code(err),
            message: err.to_string(),
            suggestion: err.suggestion().map(String::from),
        }
    }
}

/// 将错误类型映射为 code 字符串
pub fn error_to_code(err: &GcopError) -> String {
    match err {
        GcopError::NoStagedChanges => "NO_STAGED_CHANGES",
        GcopError::InvalidInput(_) => "INVALID_INPUT",
        GcopError::UserCancelled => "USER_CANCELLED",
        GcopError::MaxRetriesExceeded(_) => "MAX_RETRIES_EXCEEDED",
        GcopError::Config(_) => "CONFIG_ERROR",
        GcopError::Llm(_) => "LLM_ERROR",
        GcopError::LlmApi { .. } => "LLM_API_ERROR",
        GcopError::Network(_) => "NETWORK_ERROR",
        GcopError::Git(_) => "GIT_ERROR",
        GcopError::Io(_) => "IO_ERROR",
        _ => "UNKNOWN_ERROR",
    }
    .to_string()
}
