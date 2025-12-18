//! 全局常量定义

/// LLM 相关常量
pub mod llm {
    /// 默认 max_tokens
    pub const DEFAULT_MAX_TOKENS: u32 = 2000;

    /// 默认 temperature
    pub const DEFAULT_TEMPERATURE: f32 = 0.3;
}

/// Commit 相关常量
pub mod commit {
    /// 最大重试次数
    pub const MAX_RETRIES: usize = 10;
}

/// UI 相关常量
pub mod ui {
    /// 错误预览最大长度
    pub const ERROR_PREVIEW_LENGTH: usize = 500;

    /// 用户反馈最大长度
    pub const MAX_FEEDBACK_LENGTH: usize = 200;
}
