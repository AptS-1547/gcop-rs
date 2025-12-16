use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[derive(Default)]
pub struct AppConfig {
    #[serde(default)]
    pub llm: LLMConfig,

    #[serde(default)]
    pub commit: CommitConfig,

    #[serde(default)]
    pub review: ReviewConfig,

    #[serde(default)]
    pub ui: UIConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LLMConfig {
    /// 默认使用的 provider: "claude" | "openai" | "ollama"
    pub default_provider: String,

    /// 各 provider 的配置
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderConfig {
    /// API endpoint
    pub endpoint: Option<String>,

    /// API key（优先从环境变量读取）
    pub api_key: Option<String>,

    /// 模型名称
    pub model: String,

    /// 其他参数（temperature 等）
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommitConfig {
    /// 生成前是否显示 diff 预览
    #[serde(default = "default_true")]
    pub show_diff_preview: bool,

    /// 是否允许编辑生成的消息
    #[serde(default = "default_true")]
    pub allow_edit: bool,

    /// 提交前是否需要确认
    #[serde(default = "default_true")]
    pub confirm_before_commit: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReviewConfig {
    /// 审查时是否显示完整 diff
    #[serde(default = "default_true")]
    pub show_full_diff: bool,

    /// 最低显示的问题严重性
    #[serde(default = "default_severity")]
    pub min_severity: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UIConfig {
    /// 是否启用彩色输出
    #[serde(default = "default_true")]
    pub colored: bool,

    /// 是否显示详细信息
    #[serde(default)]
    pub verbose: bool,
}

fn default_true() -> bool {
    true
}

fn default_severity() -> String {
    "info".to_string()
}


impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            default_provider: "claude".to_string(),
            providers: HashMap::new(),
        }
    }
}

impl Default for CommitConfig {
    fn default() -> Self {
        Self {
            show_diff_preview: true,
            allow_edit: true,
            confirm_before_commit: true,
        }
    }
}

impl Default for ReviewConfig {
    fn default() -> Self {
        Self {
            show_full_diff: true,
            min_severity: "info".to_string(),
        }
    }
}

impl Default for UIConfig {
    fn default() -> Self {
        Self {
            colored: true,
            verbose: false,
        }
    }
}
