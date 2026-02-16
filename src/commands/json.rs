use serde::Serialize;

use crate::error::{GcopError, Result};

/// JSON error output structure (unified)
#[derive(Debug, Serialize)]
pub struct ErrorJson {
    /// Stable machine-readable error code.
    pub code: String,
    /// Human-readable error message.
    pub message: String,
    /// Optional remediation hint for users.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl ErrorJson {
    /// Create ErrorJson from GcopError
    pub fn from_error(err: &GcopError) -> Self {
        Self {
            code: error_to_code(err),
            message: err.to_string(),
            suggestion: err.localized_suggestion(),
        }
    }
}

/// Generic JSON output structure
#[derive(Debug, Serialize)]
pub struct JsonOutput<T: Serialize> {
    /// Whether the command completed successfully.
    pub success: bool,
    /// Optional success payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Optional error payload when `success == false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorJson>,
}

/// Output errors in JSON format (generic function)
///
/// #Type parameters
/// * `T` - data type, must implement the Serialize trait
///
/// # Example
/// ```no_run
/// use gcop_rs::commands::json;
/// use gcop_rs::error::GcopError;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // This function can be used for any data type
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

/// Map error type to code string
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
