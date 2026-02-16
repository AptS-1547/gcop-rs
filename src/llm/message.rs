use serde::Serialize;

/// Claude API system block structure (supports prompt caching)
#[derive(Debug, Clone, Serialize)]
pub struct SystemBlock {
    #[serde(rename = "type")]
    /// Claude block type, usually `"text"`.
    pub block_type: String,
    /// System prompt text content.
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional prompt-caching policy.
    pub cache_control: Option<CacheControl>,
}

impl SystemBlock {
    /// Create a common system block
    #[allow(dead_code)]
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            block_type: "text".to_string(),
            text: content.into(),
            cache_control: None,
        }
    }

    /// Create system block with cache_control (ephemeral = 5 minute cache)
    pub fn cached(content: impl Into<String>) -> Self {
        Self {
            block_type: "text".to_string(),
            text: content.into(),
            cache_control: Some(CacheControl::ephemeral()),
        }
    }
}

/// Claude prompt caching control
#[derive(Debug, Clone, Serialize)]
pub struct CacheControl {
    #[serde(rename = "type")]
    /// Cache control strategy identifier (e.g. `"ephemeral"`).
    pub control_type: String,
}

impl CacheControl {
    /// Create ephemeral cache control (5 minute cache)
    pub fn ephemeral() -> Self {
        Self {
            control_type: "ephemeral".to_string(),
        }
    }
}
