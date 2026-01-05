pub mod commit;
pub mod diff;
pub mod repository;

use crate::error::Result;
use chrono::{DateTime, Local};

#[cfg(any(test, feature = "test-utils"))]
use mockall::automock;

/// Commit 信息
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub author_name: String,
    pub author_email: String,
    pub timestamp: DateTime<Local>,
    #[allow(dead_code)] // 预留字段，未来可用于 commit message 统计
    pub message: String,
}

/// Git 操作的统一接口
#[cfg_attr(any(test, feature = "test-utils"), automock)]
pub trait GitOperations {
    /// 获取 staged changes 的 diff
    fn get_staged_diff(&self) -> Result<String>;

    /// 获取未提交变更的 diff
    fn get_uncommitted_diff(&self) -> Result<String>;

    /// 获取指定 commit 的 diff
    fn get_commit_diff(&self, commit_hash: &str) -> Result<String>;

    /// 获取 commit 范围的 diff
    fn get_range_diff(&self, range: &str) -> Result<String>;

    /// 获取文件或目录的完整内容
    fn get_file_content(&self, path: &str) -> Result<String>;

    /// 执行 git commit
    fn commit(&self, message: &str) -> Result<()>;

    /// 获取当前分支名
    fn get_current_branch(&self) -> Result<Option<String>>;

    /// 获取变更统计
    fn get_diff_stats(&self, diff: &str) -> Result<DiffStats>;

    /// 检查是否有 staged changes
    fn has_staged_changes(&self) -> Result<bool>;

    /// 获取 commit 历史
    fn get_commit_history(&self) -> Result<Vec<CommitInfo>>;

    /// 检查仓库是否为空（未初始化任何 commit）
    fn is_empty(&self) -> Result<bool>;
}

/// Diff 统计信息
#[derive(Debug, Clone)]
pub struct DiffStats {
    pub files_changed: Vec<String>,
    pub insertions: usize,
    pub deletions: usize,
}
