pub mod message;
pub mod prompt;
pub mod provider;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::error::Result;

/// 流式响应的数据块
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// 文本增量
    Delta(String),
    /// 流结束
    Done,
    /// 错误
    Error(String),
}

/// 流式生成器句柄
pub struct StreamHandle {
    pub receiver: mpsc::Receiver<StreamChunk>,
}

/// LLM Provider 统一接口
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// 生成 commit message
    async fn generate_commit_message(
        &self,
        diff: &str,
        context: Option<CommitContext>,
        spinner: Option<&crate::ui::Spinner>,
    ) -> Result<String>;

    /// 代码审查
    async fn review_code(
        &self,
        diff: &str,
        review_type: ReviewType,
        custom_prompt: Option<&str>,
        spinner: Option<&crate::ui::Spinner>,
    ) -> Result<ReviewResult>;

    /// Provider 名称
    #[allow(dead_code)]
    fn name(&self) -> &str;

    /// 验证配置
    async fn validate(&self) -> Result<()>;

    /// 是否支持流式响应
    fn supports_streaming(&self) -> bool {
        false
    }

    /// 流式生成 commit message
    /// 默认实现：fallback 到非流式方法
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
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum ReviewType {
    UncommittedChanges,
    SingleCommit(String),
    CommitRange(String),
    FileOrDir(String),
}

/// 审查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewResult {
    pub summary: String,
    pub issues: Vec<ReviewIssue>,
    pub suggestions: Vec<String>,
}

/// 审查问题
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    Critical,
    Warning,
    Info,
}
