pub mod message;
pub mod prompt;
pub mod provider;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::error::Result;

/// 流式响应的数据块
///
/// 用于 LLM 流式生成 commit message 的增量数据传输。
///
/// # 变体
/// - [`Delta`] - 文本增量（追加到已有内容）
/// - [`Done`] - 流正常结束
/// - [`Error`] - 流异常终止
///
/// [`Delta`]: StreamChunk::Delta
/// [`Done`]: StreamChunk::Done
/// [`Error`]: StreamChunk::Error
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// 文本增量（追加到已有内容）
    Delta(String),
    /// 流正常结束
    Done,
    /// 流异常终止，包含错误描述
    Error(String),
}

/// 流式生成器句柄
///
/// 包含一个 tokio channel receiver，用于接收流式响应的数据块。
///
/// # 使用示例
/// ```no_run
/// use gcop_rs::llm::StreamChunk;
///
/// # async fn example(mut handle: gcop_rs::llm::StreamHandle) {
/// while let Some(chunk) = handle.receiver.recv().await {
///     match chunk {
///         StreamChunk::Delta(text) => print!("{}", text),
///         StreamChunk::Done => break,
///         StreamChunk::Error(err) => {
///             eprintln!("Error: {}", err);
///             break;
///         }
///     }
/// }
/// # }
/// ```
pub struct StreamHandle {
    /// 数据块接收器
    pub receiver: mpsc::Receiver<StreamChunk>,
}

/// LLM Provider 统一接口
///
/// 该 trait 定义了所有 LLM provider 必须实现的方法，支持：
/// - 生成 commit message
/// - 代码审查
/// - 流式响应（可选）
///
/// # 实现者注意
/// 1. 必须实现 `Send + Sync`（用于 async 上下文）
/// 2. 所有方法需要处理网络错误、超时、速率限制等
/// 3. `supports_streaming()` 返回 `false` 时，`generate_commit_message_streaming()` 会 fallback 到非流式
///
/// # 内置实现
/// - [`ClaudeProvider`](provider::claude::ClaudeProvider) - Anthropic Claude
/// - [`OpenAIProvider`](provider::openai::OpenAIProvider) - OpenAI/兼容 API
/// - [`OllamaProvider`](provider::ollama::OllamaProvider) - Ollama 本地模型
/// - [`FallbackProvider`](provider::fallback::FallbackProvider) - 多 provider 高可用封装
///
/// # 自定义 Provider 示例
/// ```no_run
/// use async_trait::async_trait;
/// use gcop_rs::llm::{LLMProvider, CommitContext, ReviewResult, ReviewType};
/// use gcop_rs::error::Result;
///
/// struct MyProvider {
///     api_key: String,
/// }
///
/// #[async_trait]
/// impl LLMProvider for MyProvider {
///     async fn generate_commit_message(
///         &self,
///         diff: &str,
///         context: Option<CommitContext>,
///         spinner: Option<&gcop_rs::ui::Spinner>,
///     ) -> Result<String> {
///         // 调用自定义 API...
///         todo!()
///     }
///
///     async fn review_code(
///         &self,
///         diff: &str,
///         review_type: ReviewType,
///         custom_prompt: Option<&str>,
///         spinner: Option<&gcop_rs::ui::Spinner>,
///     ) -> Result<ReviewResult> {
///         todo!()
///     }
///
///     fn name(&self) -> &str {
///         "my-provider"
///     }
///
///     async fn validate(&self) -> Result<()> {
///         // 验证 API key 有效性...
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// 生成 commit message
    ///
    /// 基于 diff 内容和可选的上下文信息，生成符合 Conventional Commits 规范的 commit message。
    ///
    /// # 参数
    /// - `diff`: Git diff 内容（通过 `git diff --staged` 获取）
    /// - `context`: 可选的上下文信息（分支名、文件列表、用户反馈等）
    /// - `spinner`: 可选的 spinner（用于取消检测）
    ///
    /// # 返回
    /// - `Ok(message)` - 生成的 commit message
    /// - `Err(_)` - API 错误、网络错误、超时等
    ///
    /// # 错误处理
    /// 实现者需要处理：
    /// - 网络错误（重试 3 次）
    /// - 401/403（API key 无效）
    /// - 429（速率限制）
    /// - 500+（服务端错误）
    ///
    /// # 示例
    /// ```ignore
    /// use gcop_rs::llm::{LLMProvider, CommitContext, provider::openai::OpenAIProvider};
    /// use gcop_rs::config::{ProviderConfig, NetworkConfig};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let config = ProviderConfig::default();
    /// let network_config = NetworkConfig::default();
    /// let provider = OpenAIProvider::new(&config, "openai", &network_config, false)?;
    /// let diff = "diff --git a/main.rs...";
    /// let message = provider.generate_commit_message(diff, None, None).await?;
    /// println!("Generated: {}", message);
    /// # Ok(())
    /// # }
    /// ```
    async fn generate_commit_message(
        &self,
        diff: &str,
        context: Option<CommitContext>,
        spinner: Option<&crate::ui::Spinner>,
    ) -> Result<String>;

    /// 代码审查
    ///
    /// 分析代码变更，识别潜在问题和改进建议。
    ///
    /// # 参数
    /// - `diff`: 要审查的 diff 内容
    /// - `review_type`: 审查类型（未提交变更、单个 commit、范围等）
    /// - `custom_prompt`: 用户自定义 prompt（追加到系统 prompt）
    /// - `spinner`: 可选的 spinner
    ///
    /// # 返回
    /// - `Ok(result)` - 审查结果（总结、问题列表、建议）
    /// - `Err(_)` - API 错误或网络错误
    ///
    /// # 审查内容
    /// - 代码质量（重复代码、复杂度、命名等）
    /// - 潜在 bug（空指针、数组越界、资源泄漏等）
    /// - 安全问题（SQL 注入、XSS、敏感信息泄漏等）
    /// - 性能问题（O(n²) 算法、不必要的复制等）
    async fn review_code(
        &self,
        diff: &str,
        review_type: ReviewType,
        custom_prompt: Option<&str>,
        spinner: Option<&crate::ui::Spinner>,
    ) -> Result<ReviewResult>;

    /// Provider 名称
    ///
    /// 用于日志和错误消息。
    ///
    /// # 示例
    /// ```ignore
    /// use gcop_rs::llm::{LLMProvider, provider::openai::OpenAIProvider};
    /// use gcop_rs::config::{ProviderConfig, NetworkConfig};
    ///
    /// let config = ProviderConfig::default();
    /// let network_config = NetworkConfig::default();
    /// let provider = OpenAIProvider::new(&config, "openai", &network_config, false).unwrap();
    /// assert_eq!(provider.name(), "openai");
    /// ```
    #[allow(dead_code)]
    fn name(&self) -> &str;

    /// 验证配置
    ///
    /// 发送测试请求验证 API key、endpoint 等配置是否正确。
    ///
    /// # 返回
    /// - `Ok(())` - 配置有效
    /// - `Err(_)` - 配置无效或网络错误
    ///
    /// # 错误类型
    /// - [`GcopError::Llm`] - API key 无效、模型不存在等
    /// - [`GcopError::Network`] - 网络错误、超时等
    ///
    /// [`GcopError::Llm`]: crate::error::GcopError::Llm
    /// [`GcopError::Network`]: crate::error::GcopError::Network
    async fn validate(&self) -> Result<()>;

    /// 是否支持流式响应
    ///
    /// # 返回
    /// - `true` - 支持流式（SSE）
    /// - `false` - 仅支持非流式（默认值）
    fn supports_streaming(&self) -> bool {
        false
    }

    /// 流式生成 commit message
    ///
    /// 返回一个流式生成器，实时接收生成的文本增量。
    ///
    /// # 参数
    /// - `diff`: Git diff 内容
    /// - `context`: 可选的上下文信息
    ///
    /// # 返回
    /// - `Ok(handle)` - 流式生成器句柄
    /// - `Err(_)` - API 错误或网络错误
    ///
    /// # 默认实现
    /// 如果 provider 不支持流式，默认实现会 fallback 到非流式方法，
    /// 然后一次性发送完整消息（模拟流式行为）。
    ///
    /// # 示例
    /// ```ignore
    /// use gcop_rs::llm::{LLMProvider, StreamChunk, provider::claude::ClaudeProvider};
    /// use gcop_rs::config::{ProviderConfig, NetworkConfig};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let config = ProviderConfig::default();
    /// let network_config = NetworkConfig::default();
    /// let provider = ClaudeProvider::new(&config, "claude", &network_config, false)?;
    /// let diff = "diff --git a/main.rs...";
    /// let mut handle = provider.generate_commit_message_streaming(diff, None).await?;
    ///
    /// while let Some(chunk) = handle.receiver.recv().await {
    ///     match chunk {
    ///         StreamChunk::Delta(text) => print!("{}", text),
    ///         StreamChunk::Done => break,
    ///         StreamChunk::Error(err) => eprintln!("Error: {}", err),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn generate_commit_message_streaming(
        &self,
        diff: &str,
        context: Option<CommitContext>,
    ) -> Result<StreamHandle> {
        let (tx, rx) = mpsc::channel(32);

        // 调用非流式方法，然后一次性发送
        let result = self.generate_commit_message(diff, context, None).await;

        match result {
            Ok(message) => {
                let _ = tx.send(StreamChunk::Delta(message)).await;
                let _ = tx.send(StreamChunk::Done).await;
            }
            Err(e) => {
                let _ = tx.send(StreamChunk::Error(e.to_string())).await;
            }
        }

        Ok(StreamHandle { receiver: rx })
    }
}

/// Commit 上下文信息
///
/// 提供给 LLM 的额外信息，用于生成更准确的 commit message。
///
/// # 字段
/// - `files_changed`: 变更的文件路径列表
/// - `insertions`: 新增行数
/// - `deletions`: 删除行数
/// - `branch_name`: 当前分支名（可能为 None，如 detached HEAD）
/// - `custom_prompt`: 用户自定义 prompt（追加到系统 prompt）
/// - `user_feedback`: 用户反馈（重新生成时使用，支持累积）
///
/// # 示例
/// ```
/// use gcop_rs::llm::CommitContext;
///
/// let context = CommitContext {
///     files_changed: vec!["src/main.rs".to_string()],
///     insertions: 10,
///     deletions: 3,
///     branch_name: Some("feature/login".to_string()),
///     custom_prompt: Some("Focus on security changes".to_string()),
///     user_feedback: vec!["Be more specific".to_string()],
/// };
/// ```
#[derive(Debug, Clone, Default)]
pub struct CommitContext {
    pub files_changed: Vec<String>,
    pub insertions: usize,
    pub deletions: usize,
    pub branch_name: Option<String>,
    pub custom_prompt: Option<String>,
    pub user_feedback: Vec<String>, // 用户重试反馈（支持累积）
}

/// 审查类型
///
/// 指定要审查的代码范围。
///
/// # 变体
/// - [`UncommittedChanges`] - 未提交且未暂存的变更（index -> workdir）
/// - [`SingleCommit`] - 单个 commit（通过 hash）
/// - [`CommitRange`] - commit 范围（如 `HEAD~3..HEAD`）
/// - [`FileOrDir`] - 特定文件路径（当前不支持目录）
///
/// [`UncommittedChanges`]: ReviewType::UncommittedChanges
/// [`SingleCommit`]: ReviewType::SingleCommit
/// [`CommitRange`]: ReviewType::CommitRange
/// [`FileOrDir`]: ReviewType::FileOrDir
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum ReviewType {
    UncommittedChanges,
    SingleCommit(String),
    CommitRange(String),
    FileOrDir(String),
}

/// 审查结果
///
/// LLM 代码审查的输出结果。
///
/// # 字段
/// - `summary`: 总体评价和摘要
/// - `issues`: 发现的问题列表（按严重程度排序）
/// - `suggestions`: 改进建议列表
///
/// # 示例
/// ```
/// use gcop_rs::llm::{ReviewResult, ReviewIssue, IssueSeverity};
///
/// let result = ReviewResult {
///     summary: "Found 2 security issues".to_string(),
///     issues: vec![
///         ReviewIssue {
///             severity: IssueSeverity::Critical,
///             description: "Potential SQL injection".to_string(),
///             file: Some("db.rs".to_string()),
///             line: Some(42),
///         },
///     ],
///     suggestions: vec!["Use parameterized queries".to_string()],
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewResult {
    pub summary: String,
    pub issues: Vec<ReviewIssue>,
    pub suggestions: Vec<String>,
}

/// 审查问题
///
/// 代码审查中发现的单个问题。
///
/// # 字段
/// - `severity`: 严重程度（Critical/Warning/Info）
/// - `description`: 问题描述
/// - `file`: 相关文件路径（可选）
/// - `line`: 相关行号（可选）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewIssue {
    pub severity: IssueSeverity,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
}

/// 问题严重性
///
/// # 变体
/// - `Critical` - 严重问题（安全漏洞、崩溃风险等）
/// - `Warning` - 警告（性能问题、代码异味等）
/// - `Info` - 提示（风格建议、最佳实践等）
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    Critical,
    Warning,
    Info,
}
