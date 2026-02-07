use serde::Serialize;

use crate::error::{GcopError, Result};

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

/// 通用的 JSON 输出结构
#[derive(Debug, Serialize)]
pub struct JsonOutput<T: Serialize> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorJson>,
}

/// 输出 JSON 格式的错误（通用函数）
///
/// # 类型参数
/// * `T` - 数据类型，必须实现 Serialize trait
///
/// # 示例
/// ```no_run
/// use gcop_rs::commands::json;
/// use gcop_rs::error::GcopError;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // 对于任何数据类型，都可以使用这个函数
/// json::output_json_error::<String>(&GcopError::UserCancelled)?;
/// # Ok(())
/// # }
/// ```
pub fn output_json_error<T: Serialize>(err: &GcopError) -> Result<()> {
    let output = JsonOutput::<T> {
        success: false,
        data: None,
        error: Some(ErrorJson::from_error(err)),
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
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
