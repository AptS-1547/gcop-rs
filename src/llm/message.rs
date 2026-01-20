use serde::Serialize;

/// Claude API system block 结构（支持 prompt caching）
#[derive(Debug, Clone, Serialize)]
pub struct SystemBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

impl SystemBlock {
    /// 创建普通 system block
    #[allow(dead_code)]
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            block_type: "text".to_string(),
            text: content.into(),
            cache_control: None,
        }
    }

    /// 创建带 cache_control 的 system block（ephemeral = 5 分钟缓存）
    pub fn cached(content: impl Into<String>) -> Self {
        Self {
            block_type: "text".to_string(),
            text: content.into(),
            cache_control: Some(CacheControl::ephemeral()),
        }
    }
}

/// Claude prompt caching 控制
#[derive(Debug, Clone, Serialize)]
pub struct CacheControl {
    #[serde(rename = "type")]
    pub control_type: String,
}

impl CacheControl {
    /// 创建 ephemeral 缓存控制（5 分钟缓存）
    pub fn ephemeral() -> Self {
        Self {
            control_type: "ephemeral".to_string(),
        }
    }
}
