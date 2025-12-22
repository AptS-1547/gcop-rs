use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub llm: LLMConfig,

    #[serde(default)]
    pub commit: CommitConfig,

    #[serde(default)]
    pub review: ReviewConfig,

    #[serde(default)]
    pub ui: UIConfig,

    #[serde(default)]
    pub network: NetworkConfig,

    #[serde(default)]
    pub file: FileConfig,
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
    /// API 风格: "claude" | "openai" | "ollama"
    /// 用于指定使用哪种 API 实现
    /// 如果未指定，将使用 provider 名称作为 api_style
    #[serde(default)]
    pub api_style: Option<String>,

    /// API endpoint
    pub endpoint: Option<String>,

    /// API key（优先从环境变量读取）
    pub api_key: Option<String>,

    /// 模型名称
    pub model: String,

    /// 最大生成 token 数
    pub max_tokens: Option<u32>,

    /// 温度参数（0.0-1.0）
    pub temperature: Option<f32>,

    /// 其他参数
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

    /// 自定义 commit message 生成的 prompt 模板
    /// 可用占位符：{diff}, {files_changed}, {insertions}, {deletions}, {branch_name}
    #[serde(default)]
    pub custom_prompt: Option<String>,

    /// 最大重试次数（用户手动重试）
    #[serde(default = "default_commit_max_retries")]
    pub max_retries: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReviewConfig {
    /// 审查时是否显示完整 diff
    #[serde(default = "default_true")]
    pub show_full_diff: bool,

    /// 最低显示的问题严重性
    #[serde(default = "default_severity")]
    pub min_severity: String,

    /// 自定义 code review 的 prompt 模板
    /// 可用占位符：{diff}
    #[serde(default)]
    pub custom_prompt: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UIConfig {
    /// 是否启用彩色输出
    #[serde(default = "default_true")]
    pub colored: bool,

    /// 是否显示详细信息
    #[serde(default)]
    pub verbose: bool,

    /// 是否启用流式输出（实时打字效果）
    #[serde(default = "default_true")]
    pub streaming: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NetworkConfig {
    /// HTTP 请求超时时间（秒）
    #[serde(default = "default_request_timeout")]
    pub request_timeout: u64,

    /// HTTP 连接超时时间（秒）
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout: u64,

    /// LLM API 请求最大重试次数
    #[serde(default = "default_network_max_retries")]
    pub max_retries: usize,

    /// 重试初始延迟（毫秒）
    #[serde(default = "default_retry_delay_ms")]
    pub retry_delay_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileConfig {
    /// 最大文件大小（字节）
    #[serde(default = "default_max_file_size")]
    pub max_size: u64,
}

fn default_true() -> bool {
    true
}

fn default_severity() -> String {
    "info".to_string()
}

fn default_commit_max_retries() -> usize {
    10
}

fn default_request_timeout() -> u64 {
    120
}

fn default_connect_timeout() -> u64 {
    10
}

fn default_network_max_retries() -> usize {
    3
}

fn default_retry_delay_ms() -> u64 {
    1000
}

fn default_max_file_size() -> u64 {
    10 * 1024 * 1024 // 10MB
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
            custom_prompt: None,
            max_retries: default_commit_max_retries(),
        }
    }
}

impl Default for ReviewConfig {
    fn default() -> Self {
        Self {
            show_full_diff: true,
            min_severity: "info".to_string(),
            custom_prompt: None,
        }
    }
}

impl Default for UIConfig {
    fn default() -> Self {
        Self {
            colored: true,
            verbose: false,
            streaming: true,
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            request_timeout: default_request_timeout(),
            connect_timeout: default_connect_timeout(),
            max_retries: default_network_max_retries(),
            retry_delay_ms: default_retry_delay_ms(),
        }
    }
}

impl Default for FileConfig {
    fn default() -> Self {
        Self {
            max_size: default_max_file_size(),
        }
    }
}
