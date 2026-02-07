use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 应用配置
///
/// gcop-rs 的顶层配置结构，从 `~/.config/gcop/config.toml` 加载。
///
/// # 配置文件位置
/// - Linux: `~/.config/gcop/config.toml`
/// - macOS: `~/Library/Application Support/gcop/config.toml`
/// - Windows: `%APPDATA%\gcop\config.toml`
///
/// # 配置示例
/// ```toml
/// [llm]
/// default_provider = "claude"
/// fallback_providers = ["openai"]
///
/// [llm.providers.claude]
/// api_key = "sk-ant-..."
/// model = "claude-sonnet-4-5-20250929"
///
/// [commit]
/// max_retries = 10
/// show_diff_preview = true
///
/// [ui]
/// colored = true
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AppConfig {
    /// LLM 配置
    #[serde(default)]
    pub llm: LLMConfig,

    /// Commit 配置
    #[serde(default)]
    pub commit: CommitConfig,

    /// Review 配置
    #[serde(default)]
    pub review: ReviewConfig,

    /// UI 配置
    #[serde(default)]
    pub ui: UIConfig,

    /// 网络配置
    #[serde(default)]
    pub network: NetworkConfig,

    /// 文件配置
    #[serde(default)]
    pub file: FileConfig,
}

/// LLM 配置
///
/// 管理 LLM provider 的选择和配置。
///
/// # 字段
/// - `default_provider`: 默认 provider（"claude", "openai", "ollama"）
/// - `fallback_providers`: 备用 provider 列表（按顺序尝试）
/// - `providers`: 各 provider 的详细配置
///
/// # 示例
/// ```toml
/// [llm]
/// default_provider = "claude"
/// fallback_providers = ["openai", "ollama"]
///
/// [llm.providers.claude]
/// api_key = "sk-ant-..."
/// model = "claude-sonnet-4-5-20250929"
///
/// [llm.providers.openai]
/// api_key = "sk-..."
/// model = "gpt-4"
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LLMConfig {
    /// 默认使用的 provider: "claude" | "openai" | "ollama"
    pub default_provider: String,

    /// 备用 provider 列表，当主 provider 失败时按顺序尝试
    #[serde(default)]
    pub fallback_providers: Vec<String>,

    /// 各 provider 的配置
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
}

/// Provider 配置
///
/// 单个 LLM provider 的配置。
///
/// # 字段
/// - `api_style`: API 风格（"claude", "openai", "ollama"）
/// - `endpoint`: 自定义 API 端点（可选）
/// - `api_key`: API key（可选，优先从环境变量读取）
/// - `model`: 模型名称
/// - `max_tokens`: 最大生成 token 数（可选）
/// - `temperature`: 温度参数 0.0-1.0（可选）
/// - `extra`: 其他自定义参数
///
/// # 示例
/// ```toml
/// [llm.providers.claude]
/// model = "claude-sonnet-4-5-20250929"
/// api_key = "sk-ant-..."
/// max_tokens = 1000
/// temperature = 0.7
/// endpoint = "https://api.anthropic.com"  # 可选
/// ```
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

/// Commit 命令配置
///
/// 控制 commit message 生成的行为。
///
/// # 字段
/// - `show_diff_preview`: 生成前是否显示 diff 预览（默认 true）
/// - `allow_edit`: 是否允许编辑生成的消息（默认 true）
/// - `confirm_before_commit`: 提交前是否需要确认（默认 true）
/// - `custom_prompt`: 自定义 prompt 模板（可选）
/// - `max_retries`: 最大重试次数（默认 10）
///
/// # 示例
/// ```toml
/// [commit]
/// show_diff_preview = true
/// allow_edit = true
/// confirm_before_commit = true
/// max_retries = 10
/// custom_prompt = "Generate a concise commit message"
/// ```
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

/// Review 命令配置
///
/// 控制代码审查的行为。
///
/// # 字段
/// - `show_full_diff`: 审查时是否显示完整 diff（默认 true）
/// - `min_severity`: 最低显示的问题严重性（"info", "warning", "critical"）
/// - `custom_prompt`: 自定义 prompt 模板（可选）
///
/// # 示例
/// ```toml
/// [review]
/// show_full_diff = true
/// min_severity = "warning"
/// custom_prompt = "Focus on security issues"
/// ```
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

/// UI 配置
///
/// 控制用户界面的显示行为。
///
/// # 字段
/// - `colored`: 是否启用彩色输出（默认 true）
/// - `verbose`: 是否显示详细信息（默认 false）
/// - `streaming`: 是否启用流式输出（打字机效果，默认 true）
/// - `language`: 界面语言（BCP 47 格式，如 "en", "zh-CN"，默认自动检测）
///
/// # 示例
/// ```toml
/// [ui]
/// colored = true
/// verbose = false
/// streaming = true
/// language = "zh-CN"
/// ```
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

    /// 界面语言（BCP 47 格式，如 "en", "zh-CN"）
    /// None 表示自动检测系统语言
    #[serde(default)]
    pub language: Option<String>,
}

/// 网络配置
///
/// 控制 HTTP 请求的超时和重试行为。
///
/// # 字段
/// - `request_timeout`: HTTP 请求超时时间（秒，默认 30）
/// - `connect_timeout`: HTTP 连接超时时间（秒，默认 10）
/// - `max_retries`: LLM API 请求最大重试次数（默认 3）
/// - `retry_delay_ms`: 重试初始延迟（毫秒，默认 1000）
/// - `max_retry_delay_ms`: 重试最大延迟（毫秒，默认 10000）
///
/// # 示例
/// ```toml
/// [network]
/// request_timeout = 30
/// connect_timeout = 10
/// max_retries = 3
/// retry_delay_ms = 1000
/// max_retry_delay_ms = 10000
/// ```
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

    /// 重试最大延迟（毫秒）
    #[serde(default = "default_max_retry_delay_ms")]
    pub max_retry_delay_ms: u64,
}

/// 文件配置
///
/// 控制文件处理的限制。
///
/// # 字段
/// - `max_size`: 最大文件大小（字节，默认 10MB）
///
/// # 示例
/// ```toml
/// [file]
/// max_size = 10485760  # 10MB
/// ```
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

fn default_max_retry_delay_ms() -> u64 {
    60_000 // 60 秒
}

fn default_max_file_size() -> u64 {
    10 * 1024 * 1024 // 10MB
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            default_provider: "claude".to_string(),
            fallback_providers: Vec::new(),
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
            language: None,
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
            max_retry_delay_ms: default_max_retry_delay_ms(),
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
