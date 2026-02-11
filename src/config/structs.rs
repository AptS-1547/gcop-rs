// 配置结构定义
//
// 此文件包含所有配置相关的数据结构。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::{GcopError, Result};

/// 应用配置
///
/// gcop-rs 的顶层配置结构。
///
/// 实际生效配置由多来源合并得到（从低到高）：
/// 1. 默认值
/// 2. 用户级配置文件（平台相关目录）
/// 3. 项目级配置文件（`.gcop/config.toml`，从当前目录向上查找）
/// 4. `GCOP__*` 环境变量
/// 5. CI 模式覆盖（`CI=1` + `GCOP_CI_*`）
///
/// # 配置文件位置
/// - Linux: `~/.config/gcop/config.toml`
/// - macOS: `~/Library/Application Support/gcop/config.toml`
/// - Windows: `%APPDATA%\gcop\config\config.toml`
/// - 项目级（可选）: `<repo>/.gcop/config.toml`
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
/// - `default_provider`: 默认 provider 名称（对应 `[llm.providers.<name>]` 的 key）
/// - `fallback_providers`: 备用 provider 列表（按顺序尝试）
/// - `providers`: 各 provider 的详细配置
/// - `max_diff_size`: 发送给 LLM 的最大 diff 大小（字节，默认 100KB）
///
/// # 示例
/// ```toml
/// [llm]
/// default_provider = "claude"
/// fallback_providers = ["openai", "gemini", "ollama"]
/// max_diff_size = 102400
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
    /// 默认使用的 provider 名称（对应 `[llm.providers.<name>]` 的 key）
    pub default_provider: String,

    /// 备用 provider 列表，当主 provider 失败时按顺序尝试
    #[serde(default)]
    pub fallback_providers: Vec<String>,

    /// 各 provider 的配置
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,

    /// 发送给 LLM 的最大 diff 大小（字节），超出部分会被截断
    #[serde(default = "default_max_diff_size")]
    pub max_diff_size: usize,
}

/// API 风格枚举
///
/// 决定使用哪种 LLM API 实现。
/// 如果 `ProviderConfig::api_style` 为 `None`，将根据 provider 名称推断。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiStyle {
    /// Anthropic Claude API
    Claude,
    /// OpenAI API（及兼容 API）
    #[serde(rename = "openai")]
    OpenAI,
    /// Ollama 本地模型 API
    Ollama,
    /// Google Gemini API
    Gemini,
}

impl std::fmt::Display for ApiStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiStyle::Claude => write!(f, "claude"),
            ApiStyle::OpenAI => write!(f, "openai"),
            ApiStyle::Ollama => write!(f, "ollama"),
            ApiStyle::Gemini => write!(f, "gemini"),
        }
    }
}

impl std::str::FromStr for ApiStyle {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude" => Ok(ApiStyle::Claude),
            "openai" => Ok(ApiStyle::OpenAI),
            "ollama" => Ok(ApiStyle::Ollama),
            "gemini" => Ok(ApiStyle::Gemini),
            _ => Err(format!("Unknown API style: '{}'", s)),
        }
    }
}

impl ApiStyle {
    /// 返回该 API 风格的默认模型名称
    pub fn default_model(&self) -> &'static str {
        match self {
            ApiStyle::Claude => "claude-sonnet-4-5-20250929",
            ApiStyle::OpenAI => "gpt-4o-mini",
            ApiStyle::Ollama => "llama3.2",
            ApiStyle::Gemini => "gemini-3-flash-preview",
        }
    }
}

/// Provider 配置
///
/// 单个 LLM provider 的配置。
///
/// # 字段
/// - `api_style`: API 风格（见 [`ApiStyle`]）
/// - `endpoint`: 自定义 API 端点（可选）
/// - `api_key`: API key（可选；Claude/OpenAI 风格通常需要，Ollama 不需要）
/// - `model`: 模型名称
/// - `max_tokens`: 最大生成 token 数（可选）
/// - `temperature`: 温度参数 0.0-2.0（可选）
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
#[derive(Clone, Deserialize, Serialize)]
pub struct ProviderConfig {
    /// API 风格，决定使用哪种 API 实现
    /// 如果未指定，将使用 provider 名称推断
    #[serde(default)]
    pub api_style: Option<ApiStyle>,

    /// API endpoint
    pub endpoint: Option<String>,

    /// API key（当前从 provider 配置读取）
    ///
    /// 对 Claude/OpenAI 风格通常必需；Ollama 风格可为空。
    #[serde(skip_serializing)]
    pub api_key: Option<String>,

    /// 模型名称
    pub model: String,

    /// 最大生成 token 数
    pub max_tokens: Option<u32>,

    /// 温度参数（0.0-2.0）
    pub temperature: Option<f32>,

    /// 其他参数
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl std::fmt::Debug for ProviderConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use crate::llm::provider::utils::mask_api_key;
        let masked_key = self.api_key.as_deref().map(mask_api_key);
        f.debug_struct("ProviderConfig")
            .field("api_style", &self.api_style)
            .field("endpoint", &self.endpoint)
            .field("api_key", &masked_key)
            .field("model", &self.model)
            .field("max_tokens", &self.max_tokens)
            .field("temperature", &self.temperature)
            .finish()
    }
}

/// Commit 规范风格
///
/// 决定 commit message 遵循的规范格式。
/// 该信息会注入到 LLM prompt 中引导生成，不做硬校验。
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ConventionStyle {
    /// Conventional Commits: type(scope): description
    #[default]
    Conventional,
    /// Gitmoji: :emoji: description
    Gitmoji,
    /// 自定义格式（需配合 template 字段）
    Custom,
}

/// Commit 规范配置
///
/// 定义团队统一的 commit message 规范，注入到 LLM prompt 中引导生成。
///
/// # 示例
/// ```toml
/// [commit.convention]
/// style = "conventional"
/// types = ["feat", "fix", "docs", "style", "refactor", "perf", "test", "chore", "ci"]
/// extra_prompt = "All commit messages must be in English"
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
pub struct CommitConvention {
    /// 规范风格
    #[serde(default)]
    pub style: ConventionStyle,

    /// 允许的 type 列表（style = "conventional" 或 "custom" 时生效）
    pub types: Option<Vec<String>>,

    /// 自定义模板（style = "custom" 时生效）
    /// 占位符：{type}, {scope}, {subject}, {body}
    pub template: Option<String>,

    /// 附加 prompt 指令，追加到默认 prompt 之后
    pub extra_prompt: Option<String>,
}

/// Commit 命令配置
///
/// 控制 commit message 生成的行为。
///
/// # 字段
/// - `show_diff_preview`: 生成前是否显示 diff 预览（默认 true）
/// - `allow_edit`: 是否允许编辑生成的消息（默认 true）
/// - `custom_prompt`: 自定义 prompt 模板（可选）
/// - `max_retries`: 最大生成尝试次数（默认 10，包含首次生成）
/// - `convention`: commit 规范配置（可选）
///
/// # 示例
/// ```toml
/// [commit]
/// show_diff_preview = true
/// allow_edit = true
/// max_retries = 10
/// custom_prompt = "Generate a concise commit message"
///
/// [commit.convention]
/// style = "conventional"
/// types = ["feat", "fix", "docs", "refactor", "test", "chore"]
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommitConfig {
    /// 生成前是否显示 diff 预览
    #[serde(default = "default_true")]
    pub show_diff_preview: bool,

    /// 是否允许编辑生成的消息
    #[serde(default = "default_true")]
    pub allow_edit: bool,

    /// 自定义 commit message 生成的 system prompt 文本
    /// 不做占位符替换（如 `{diff}` 会按字面量传递）
    #[serde(default)]
    pub custom_prompt: Option<String>,

    /// 最大生成尝试次数（包含首次生成）
    #[serde(default = "default_commit_max_retries")]
    pub max_retries: usize,

    /// Commit 规范配置（可选，通常在项目级 .gcop/config.toml 中设置）
    #[serde(default)]
    pub convention: Option<CommitConvention>,
}

/// Review 命令配置
///
/// 控制代码审查的行为。
///
/// # 字段
/// - `min_severity`: 文本输出时最低显示的问题严重性（"info", "warning", "critical"）
/// - `custom_prompt`: 自定义 prompt 模板（可选）
///
/// # 示例
/// ```toml
/// [review]
/// min_severity = "warning"
/// custom_prompt = "Focus on security issues"
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReviewConfig {
    /// 文本输出时最低显示的问题严重性
    ///
    /// 说明：当前仅 `review --format text` 会应用该过滤；
    /// `json` / `markdown` 会保留完整问题列表。
    #[serde(default = "default_severity")]
    pub min_severity: String,

    /// 自定义 code review 的 system prompt 文本
    /// 不做占位符替换（如 `{diff}` 会按字面量传递）
    #[serde(default)]
    pub custom_prompt: Option<String>,
}

/// UI 配置
///
/// 控制用户界面的显示行为。
///
/// # 字段
/// - `colored`: 是否启用彩色输出（默认 true）
/// - `streaming`: 是否启用流式输出（打字机效果，默认 true）
/// - `language`: 界面语言（BCP 47 格式，如 "en", "zh-CN"，默认自动检测）
///
/// # 示例
/// ```toml
/// [ui]
/// colored = true
/// streaming = true
/// language = "zh-CN"
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UIConfig {
    /// 是否启用彩色输出
    #[serde(default = "default_true")]
    pub colored: bool,

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
/// - `request_timeout`: HTTP 请求超时时间（秒，默认 120）
/// - `connect_timeout`: HTTP 连接超时时间（秒，默认 10）
/// - `max_retries`: LLM API 请求最大重试次数（默认 3）
/// - `retry_delay_ms`: 重试初始延迟（毫秒，默认 1000）
/// - `max_retry_delay_ms`: 重试最大延迟（毫秒，默认 60000）
///
/// # 示例
/// ```toml
/// [network]
/// request_timeout = 30
/// connect_timeout = 10
/// max_retries = 3
/// retry_delay_ms = 1000
/// max_retry_delay_ms = 60000
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
/// 控制本地文件读取的限制。
///
/// # 字段
/// - `max_size`: 最大文件大小（字节，默认 10MB）
///   - 目前用于 `review file <PATH>` 读取工作区文件时的保护阈值
///
/// # 示例
/// ```toml
/// [file]
/// max_size = 10485760  # 10MB
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileConfig {
    /// 最大文件大小（字节）
    ///
    /// 当前用于 `review file <PATH>` 的文件读取上限。
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

fn default_max_diff_size() -> usize {
    100 * 1024 // 100KB
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            default_provider: "claude".to_string(),
            fallback_providers: Vec::new(),
            providers: HashMap::new(),
            max_diff_size: default_max_diff_size(),
        }
    }
}

impl Default for CommitConfig {
    fn default() -> Self {
        Self {
            show_diff_preview: true,
            allow_edit: true,
            custom_prompt: None,
            max_retries: default_commit_max_retries(),
            convention: None,
        }
    }
}

impl Default for ReviewConfig {
    fn default() -> Self {
        Self {
            min_severity: "info".to_string(),
            custom_prompt: None,
        }
    }
}

impl Default for UIConfig {
    fn default() -> Self {
        Self {
            colored: true,
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

// === 验证逻辑 ===

impl AppConfig {
    /// 验证配置的合法性
    pub fn validate(&self) -> Result<()> {
        // 验证 default_provider 引用的 provider 存在
        if !self.llm.providers.is_empty()
            && !self.llm.providers.contains_key(&self.llm.default_provider)
        {
            return Err(GcopError::Config(format!(
                "default_provider '{}' not found in [llm.providers]",
                self.llm.default_provider
            )));
        }

        // 验证 fallback_providers 引用的 provider 都存在
        for name in &self.llm.fallback_providers {
            if !self.llm.providers.contains_key(name) {
                return Err(GcopError::Config(format!(
                    "fallback_providers: '{}' not found in [llm.providers]",
                    name
                )));
            }
        }

        for (name, provider) in &self.llm.providers {
            provider.validate(name)?;
        }
        self.network.validate()?;
        Ok(())
    }
}

impl ProviderConfig {
    /// 验证 provider 配置
    pub fn validate(&self, name: &str) -> Result<()> {
        if let Some(temp) = self.temperature
            && !(0.0..=2.0).contains(&temp)
        {
            return Err(GcopError::Config(format!(
                "Provider '{}': temperature {} out of range [0.0, 2.0]",
                name, temp
            )));
        }
        if let Some(ref key) = self.api_key
            && key.trim().is_empty()
        {
            return Err(GcopError::Config(format!(
                "Provider '{}': api_key is empty",
                name
            )));
        }
        Ok(())
    }
}

impl NetworkConfig {
    /// 验证网络配置
    pub fn validate(&self) -> Result<()> {
        if self.request_timeout == 0 {
            return Err(GcopError::Config(
                "network.request_timeout cannot be 0".into(),
            ));
        }
        if self.connect_timeout == 0 {
            return Err(GcopError::Config(
                "network.connect_timeout cannot be 0".into(),
            ));
        }
        Ok(())
    }
}
