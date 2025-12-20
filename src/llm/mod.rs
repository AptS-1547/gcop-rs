pub mod message;
pub mod prompt;
pub mod provider;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Result;

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
