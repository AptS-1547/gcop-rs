pub mod commit;
pub mod diff;
pub mod repository;

use std::path::PathBuf;

use crate::error::Result;
use chrono::{DateTime, Local};
use serde::Serialize;

#[cfg(any(test, feature = "test-utils"))]
use mockall::automock;

/// Git commit 信息
///
/// 包含 commit 的作者、时间和消息。
///
/// # 字段
/// - `author_name`: 提交者名称
/// - `author_email`: 提交者邮箱
/// - `timestamp`: 提交时间（本地时区）
/// - `message`: Commit 消息内容
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub author_name: String,
    pub author_email: String,
    pub timestamp: DateTime<Local>,
    #[allow(dead_code)] // 预留字段，未来可用于 commit message 统计
    pub message: String,
}

/// Git 操作的统一接口
///
/// 该 trait 抽象了所有 Git 仓库操作，便于测试和扩展。
/// 主要实现：[`GitRepository`](repository::GitRepository)
///
/// # 设计理念
/// - 纯 Rust 接口，不依赖具体实现
/// - 支持 mock 测试（通过 `mockall`）
/// - 错误处理统一使用 [`GcopError`](crate::error::GcopError)
///
/// # 示例
/// ```no_run
/// use gcop_rs::git::{GitOperations, repository::GitRepository};
///
/// # fn main() -> anyhow::Result<()> {
/// let repo = GitRepository::open(None)?;
/// let diff = repo.get_staged_diff()?;
/// println!("Staged changes:\n{}", diff);
/// # Ok(())
/// # }
/// ```
#[cfg_attr(any(test, feature = "test-utils"), automock)]
pub trait GitOperations {
    /// 获取 staged changes 的 diff
    ///
    /// 等价于 `git diff --cached --unified=3`。
    ///
    /// # 返回
    /// - `Ok(diff)` - diff 内容（可能为空字符串）
    /// - `Err(_)` - Git 操作失败
    ///
    /// # 错误
    /// - 仓库未初始化
    /// - 权限不足
    fn get_staged_diff(&self) -> Result<String>;

    /// 获取未提交变更（未暂存部分）的 diff
    ///
    /// 仅包含 index -> workdir 的变更（unstaged），
    /// 等价于 `git diff`（不含 `--cached`）。
    ///
    /// # 返回
    /// - `Ok(diff)` - diff 内容（可能为空字符串）
    /// - `Err(_)` - Git 操作失败
    fn get_uncommitted_diff(&self) -> Result<String>;

    /// 获取指定 commit 的 diff
    ///
    /// 等价于 `git show <commit_hash>`。
    ///
    /// # 参数
    /// - `commit_hash`: commit SHA（支持短 hash）
    ///
    /// # 返回
    /// - `Ok(diff)` - diff 内容
    /// - `Err(_)` - commit 不存在或 Git 操作失败
    fn get_commit_diff(&self, commit_hash: &str) -> Result<String>;

    /// 获取 commit 范围的 diff
    ///
    /// 支持多种格式：
    /// - `HEAD~3..HEAD` - 最近 3 个 commit
    /// - `main..feature` - 分支间的差异
    /// - `abc123..def456` - 两个 commit 间的差异
    ///
    /// # 参数
    /// - `range`: Git range 表达式
    ///
    /// # 返回
    /// - `Ok(diff)` - diff 内容
    /// - `Err(_)` - range 无效或 Git 操作失败
    fn get_range_diff(&self, range: &str) -> Result<String>;

    /// 获取文件的完整内容
    ///
    /// 读取工作区中的文件内容（不是 Git 对象）。
    ///
    /// # 参数
    /// - `path`: 相对于仓库根目录的文件路径
    ///
    /// # 返回
    /// - `Ok(content)` - 文件内容
    /// - `Err(_)` - 文件不存在、不是普通文件或读取失败
    fn get_file_content(&self, path: &str) -> Result<String>;

    /// 执行 git commit
    ///
    /// 将 staged changes 提交到仓库。调用前需要确保有 staged changes。
    ///
    /// # 参数
    /// - `message`: commit 消息（支持多行）
    ///
    /// # 返回
    /// - `Ok(())` - commit 成功
    /// - `Err(_)` - 无 staged changes、hook 失败、或其他 Git 错误
    ///
    /// # 错误
    /// - [`GcopError::GitCommand`] - 无 staged changes
    /// - [`GcopError::Git`] - libgit2 错误
    ///
    /// # 注意
    /// - 会触发 pre-commit 和 commit-msg hooks
    /// - 使用 git config 中配置的用户名和邮箱
    ///
    /// [`GcopError::GitCommand`]: crate::error::GcopError::GitCommand
    /// [`GcopError::Git`]: crate::error::GcopError::Git
    fn commit(&self, message: &str) -> Result<()>;

    /// 获取当前分支名
    ///
    /// # 返回
    /// - `Ok(Some(name))` - 当前分支名（如 "main"）
    /// - `Ok(None)` - detached HEAD 状态
    /// - `Err(_)` - Git 操作失败
    ///
    /// # 示例
    /// ```no_run
    /// # use gcop_rs::git::{GitOperations, repository::GitRepository};
    /// # fn main() -> anyhow::Result<()> {
    /// let repo = GitRepository::open(None)?;
    /// if let Some(branch) = repo.get_current_branch()? {
    ///     println!("On branch: {}", branch);
    /// } else {
    ///     println!("Detached HEAD");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn get_current_branch(&self) -> Result<Option<String>>;

    /// 获取变更统计
    ///
    /// 解析 diff 内容，提取文件列表和增删行数。
    ///
    /// # 参数
    /// - `diff`: diff 内容（来自 `get_*_diff()` 方法）
    ///
    /// # 返回
    /// - `Ok(stats)` - 统计信息
    /// - `Err(_)` - diff 格式无效
    ///
    /// # 示例
    /// ```no_run
    /// # use gcop_rs::git::{GitOperations, repository::GitRepository};
    /// # fn main() -> anyhow::Result<()> {
    /// let repo = GitRepository::open(None)?;
    /// let diff = repo.get_staged_diff()?;
    /// let stats = repo.get_diff_stats(&diff)?;
    /// println!("{} files, +{} -{}",
    ///     stats.files_changed.len(), stats.insertions, stats.deletions);
    /// # Ok(())
    /// # }
    /// ```
    fn get_diff_stats(&self, diff: &str) -> Result<DiffStats>;

    /// 检查是否有 staged changes
    ///
    /// 快速检查是否有文件被 `git add` 到暂存区。
    ///
    /// # 返回
    /// - `Ok(true)` - 有 staged changes
    /// - `Ok(false)` - 暂存区为空
    /// - `Err(_)` - Git 操作失败
    fn has_staged_changes(&self) -> Result<bool>;

    /// 获取 commit 历史
    ///
    /// 返回当前分支的所有 commit 信息（按时间倒序）。
    ///
    /// # 返回
    /// - `Ok(history)` - commit 列表（最新的在前）
    /// - `Err(_)` - Git 操作失败
    ///
    /// # 注意
    /// - 仅返回当前分支的历史
    /// - 空仓库返回空列表
    fn get_commit_history(&self) -> Result<Vec<CommitInfo>>;

    /// 检查仓库是否为空（无任何提交）
    ///
    /// # 返回
    /// - `Ok(true)` - 仓库为空（未初始化任何 commit）
    /// - `Ok(false)` - 仓库有提交
    /// - `Err(_)` - Git 操作失败
    fn is_empty(&self) -> Result<bool>;
}

/// Diff 统计信息
///
/// 包含文件变更列表和增删行数统计。
///
/// # 字段
/// - `files_changed`: 变更的文件路径列表（相对于仓库根目录）
/// - `insertions`: 新增行数
/// - `deletions`: 删除行数
///
/// # 示例
/// ```
/// use gcop_rs::git::DiffStats;
///
/// let stats = DiffStats {
///     files_changed: vec!["src/main.rs".to_string(), "README.md".to_string()],
///     insertions: 42,
///     deletions: 13,
/// };
/// assert_eq!(stats.files_changed.len(), 2);
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct DiffStats {
    pub files_changed: Vec<String>,
    pub insertions: usize,
    pub deletions: usize,
}

/// 从当前工作目录向上查找 git 仓库根目录
///
/// 等价于 `git rev-parse --show-toplevel`。
/// 检查每一级目录是否存在 `.git`（目录或文件，兼容 submodule/worktree）。
pub fn find_git_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        if dir.join(".git").exists() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}
