use chrono::{DateTime, Local, TimeZone};
use git2::{DiffOptions, Repository, Sort};
use std::io::Write;

use crate::config::FileConfig;
use crate::error::{GcopError, Result};
use crate::git::{CommitInfo, DiffStats, GitOperations};

/// 默认最大文件大小（10MB）
const DEFAULT_MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

pub struct GitRepository {
    repo: Repository,
    max_file_size: u64,
}

impl GitRepository {
    /// 打开当前目录的 git 仓库
    ///
    /// # Arguments
    /// * `file_config` - 可选的文件配置，None 则使用默认值
    pub fn open(file_config: Option<&FileConfig>) -> Result<Self> {
        let repo = Repository::discover(".")?;
        let max_file_size = file_config
            .map(|c| c.max_size)
            .unwrap_or(DEFAULT_MAX_FILE_SIZE);
        Ok(Self {
            repo,
            max_file_size,
        })
    }

    /// 将 git2::Diff 转换为字符串
    fn diff_to_string(&self, diff: &git2::Diff) -> Result<String> {
        let mut output = Vec::new();
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            // 获取行的类型标记（origin）
            let origin = line.origin();

            // 如果 origin 是可打印字符（+、-、空格等），先写入它
            match origin {
                '+' | '-' | ' ' => {
                    let _ = output.write_all(&[origin as u8]);
                }
                _ => {}
            }

            // 再写入行内容
            let _ = output.write_all(line.content());
            true
        })?;
        Ok(String::from_utf8_lossy(&output).to_string())
    }
}

impl GitOperations for GitRepository {
    fn get_staged_diff(&self) -> Result<String> {
        // 获取 index
        let index = self.repo.index()?;

        // 空仓库时，对比 empty tree (None) 和 index
        if self.is_empty()? {
            let mut opts = DiffOptions::new();
            let diff = self
                .repo
                .diff_tree_to_index(None, Some(&index), Some(&mut opts))?;
            return self.diff_to_string(&diff);
        }

        // 获取 HEAD tree
        let head = self.repo.head()?;
        let head_tree = head.peel_to_tree()?;

        // 创建 diff（HEAD tree vs index）
        let mut opts = DiffOptions::new();
        let diff = self
            .repo
            .diff_tree_to_index(Some(&head_tree), Some(&index), Some(&mut opts))?;

        self.diff_to_string(&diff)
    }

    fn get_uncommitted_diff(&self) -> Result<String> {
        // 获取 index
        let index = self.repo.index()?;

        // 创建 diff（index vs workdir）
        let mut opts = DiffOptions::new();
        let diff = self
            .repo
            .diff_index_to_workdir(Some(&index), Some(&mut opts))?;

        self.diff_to_string(&diff)
    }

    fn get_commit_diff(&self, commit_hash: &str) -> Result<String> {
        // 查找 commit
        let commit = self
            .repo
            .find_commit(git2::Oid::from_str(commit_hash).map_err(|_| {
                GcopError::InvalidInput(
                    rust_i18n::t!("git.invalid_commit_hash", hash = commit_hash).to_string(),
                )
            })?)?;

        let commit_tree = commit.tree()?;

        // 获取 parent commit（如果有）
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };

        // 创建 diff
        let mut opts = DiffOptions::new();
        let diff = self.repo.diff_tree_to_tree(
            parent_tree.as_ref(),
            Some(&commit_tree),
            Some(&mut opts),
        )?;

        self.diff_to_string(&diff)
    }

    fn get_range_diff(&self, range: &str) -> Result<String> {
        // 解析范围（如 "main..feature"）
        let parts: Vec<&str> = range.split("..").collect();
        if parts.len() != 2 {
            return Err(GcopError::InvalidInput(
                rust_i18n::t!("git.invalid_range_format", range = range).to_string(),
            ));
        }

        let base_commit = self.repo.revparse_single(parts[0])?.peel_to_commit()?;
        let head_commit = self.repo.revparse_single(parts[1])?.peel_to_commit()?;

        let base_tree = base_commit.tree()?;
        let head_tree = head_commit.tree()?;

        let mut opts = DiffOptions::new();
        let diff =
            self.repo
                .diff_tree_to_tree(Some(&base_tree), Some(&head_tree), Some(&mut opts))?;

        self.diff_to_string(&diff)
    }

    fn get_file_content(&self, path: &str) -> Result<String> {
        let metadata = std::fs::metadata(path)?;
        if metadata.len() > self.max_file_size {
            return Err(GcopError::InvalidInput(
                rust_i18n::t!(
                    "git.file_too_large",
                    size = metadata.len(),
                    max = self.max_file_size
                )
                .to_string(),
            ));
        }

        let content = std::fs::read_to_string(path)?;
        Ok(content)
    }

    fn commit(&self, message: &str) -> Result<()> {
        crate::git::commit::commit_changes(message)
    }

    fn get_current_branch(&self) -> Result<Option<String>> {
        // Unborn branch 没有真正的分支信息
        if self.is_empty()? {
            return Ok(None);
        }

        let head = self.repo.head()?;

        if head.is_branch() {
            // 获取分支名
            let branch_name = head.shorthand().map(|s| s.to_string());
            Ok(branch_name)
        } else {
            // HEAD 处于 detached 状态
            Ok(None)
        }
    }

    fn get_diff_stats(&self, diff: &str) -> Result<DiffStats> {
        crate::git::diff::parse_diff_stats(diff)
    }

    fn has_staged_changes(&self) -> Result<bool> {
        let diff = self.get_staged_diff()?;
        Ok(!diff.trim().is_empty())
    }

    fn get_commit_history(&self) -> Result<Vec<CommitInfo>> {
        // 空仓库没有历史记录
        if self.is_empty()? {
            return Ok(Vec::new());
        }

        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(Sort::TIME)?;

        let mut commits = Vec::new();

        for oid in revwalk {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;

            let author = commit.author();
            let author_name = author.name().unwrap_or("Unknown").to_string();
            let author_email = author.email().unwrap_or("").to_string();

            // 转换 git2::Time 到 chrono::DateTime<Local>
            let git_time = commit.time();
            let timestamp: DateTime<Local> = Local
                .timestamp_opt(git_time.seconds(), 0)
                .single()
                .unwrap_or_else(|| {
                    eprintln!(
                        "{}",
                        rust_i18n::t!(
                            "git.invalid_timestamp_warning",
                            timestamp = git_time.seconds(),
                            commit = commit.id().to_string()
                        )
                    );
                    Local::now()
                });

            let message = commit
                .message()
                .unwrap_or("")
                .lines()
                .next()
                .unwrap_or("")
                .to_string();

            commits.push(CommitInfo {
                author_name,
                author_email,
                timestamp,
                message,
            });
        }

        Ok(commits)
    }

    fn is_empty(&self) -> Result<bool> {
        // 检测 unborn branch：尝试获取 HEAD，如果失败且错误码是 UnbornBranch，则为空仓库
        match self.repo.head() {
            Ok(_) => Ok(false),
            Err(e) if e.code() == git2::ErrorCode::UnbornBranch => Ok(true),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    /// 创建临时 git 仓库用于测试
    fn create_test_repo() -> (TempDir, GitRepository) {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // 设置用户信息
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        let git_repo = GitRepository {
            repo,
            max_file_size: DEFAULT_MAX_FILE_SIZE,
        };

        (dir, git_repo)
    }

    /// 在仓库中创建文件
    fn create_file(dir: &Path, name: &str, content: &str) {
        let file_path = dir.join(name);
        fs::write(&file_path, content).unwrap();
    }

    /// 暂存文件
    fn stage_file(repo: &Repository, name: &str) {
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(name)).unwrap();
        index.write().unwrap();
    }

    /// 创建 commit
    fn create_commit(repo: &Repository, message: &str) {
        let mut index = repo.index().unwrap();
        let oid = index.write_tree().unwrap();
        let tree = repo.find_tree(oid).unwrap();
        let sig = repo.signature().unwrap();

        let parent_commit = repo.head().ok().and_then(|h| h.peel_to_commit().ok());

        if let Some(parent) = parent_commit {
            repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])
                .unwrap();
        } else {
            repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[])
                .unwrap();
        }
    }

    // === 测试 is_empty ===

    #[test]
    fn test_is_empty_true_for_new_repo() {
        let (_dir, git_repo) = create_test_repo();
        assert!(git_repo.is_empty().unwrap());
    }

    #[test]
    fn test_is_empty_false_after_commit() {
        let (dir, git_repo) = create_test_repo();
        create_file(dir.path(), "test.txt", "hello");
        stage_file(&git_repo.repo, "test.txt");
        create_commit(&git_repo.repo, "Initial commit");

        assert!(!git_repo.is_empty().unwrap());
    }

    // === 测试 get_current_branch ===

    #[test]
    fn test_get_current_branch_empty_repo() {
        let (_dir, git_repo) = create_test_repo();
        assert_eq!(git_repo.get_current_branch().unwrap(), None);
    }

    #[test]
    fn test_get_current_branch_normal() {
        let (dir, git_repo) = create_test_repo();
        create_file(dir.path(), "test.txt", "hello");
        stage_file(&git_repo.repo, "test.txt");
        create_commit(&git_repo.repo, "Initial commit");

        let branch = git_repo.get_current_branch().unwrap();
        assert!(branch.is_some());
        // 默认分支是 master 或 main
        let branch_name = branch.unwrap();
        assert!(branch_name == "master" || branch_name == "main");
    }

    #[test]
    fn test_get_current_branch_detached_head() {
        let (dir, git_repo) = create_test_repo();
        create_file(dir.path(), "test.txt", "hello");
        stage_file(&git_repo.repo, "test.txt");
        create_commit(&git_repo.repo, "Initial commit");

        // 获取 commit hash 并 checkout 到 detached HEAD
        let head = git_repo.repo.head().unwrap();
        let commit = head.peel_to_commit().unwrap();
        git_repo.repo.set_head_detached(commit.id()).unwrap();

        assert_eq!(git_repo.get_current_branch().unwrap(), None);
    }

    // === 测试 has_staged_changes ===

    #[test]
    fn test_has_staged_changes_false_empty_repo() {
        let (_dir, git_repo) = create_test_repo();
        assert!(!git_repo.has_staged_changes().unwrap());
    }

    #[test]
    fn test_has_staged_changes_true() {
        let (dir, git_repo) = create_test_repo();
        create_file(dir.path(), "test.txt", "hello");
        stage_file(&git_repo.repo, "test.txt");

        assert!(git_repo.has_staged_changes().unwrap());
    }

    #[test]
    fn test_has_staged_changes_false_after_commit() {
        let (dir, git_repo) = create_test_repo();
        create_file(dir.path(), "test.txt", "hello");
        stage_file(&git_repo.repo, "test.txt");
        create_commit(&git_repo.repo, "Initial commit");

        assert!(!git_repo.has_staged_changes().unwrap());
    }

    // === 测试 get_staged_diff ===

    #[test]
    fn test_get_staged_diff_empty_repo() {
        let (dir, git_repo) = create_test_repo();
        create_file(dir.path(), "test.txt", "hello world");
        stage_file(&git_repo.repo, "test.txt");

        let diff = git_repo.get_staged_diff().unwrap();
        assert!(diff.contains("hello world"));
        assert!(diff.contains("+hello world"));
    }

    #[test]
    fn test_get_staged_diff_normal() {
        let (dir, git_repo) = create_test_repo();
        create_file(dir.path(), "test.txt", "hello");
        stage_file(&git_repo.repo, "test.txt");
        create_commit(&git_repo.repo, "Initial commit");

        // 修改文件并暂存
        create_file(dir.path(), "test.txt", "hello world");
        stage_file(&git_repo.repo, "test.txt");

        let diff = git_repo.get_staged_diff().unwrap();
        assert!(diff.contains("-hello"));
        assert!(diff.contains("+hello world"));
    }

    // === 测试 get_uncommitted_diff ===

    #[test]
    fn test_get_uncommitted_diff() {
        let (dir, git_repo) = create_test_repo();
        create_file(dir.path(), "test.txt", "hello");
        stage_file(&git_repo.repo, "test.txt");
        create_commit(&git_repo.repo, "Initial commit");

        // 修改文件但不暂存
        create_file(dir.path(), "test.txt", "hello world");

        let diff = git_repo.get_uncommitted_diff().unwrap();
        assert!(diff.contains("-hello"));
        assert!(diff.contains("+hello world"));
    }

    // === 测试 get_commit_diff ===

    #[test]
    fn test_get_commit_diff_initial_commit() {
        let (dir, git_repo) = create_test_repo();
        create_file(dir.path(), "test.txt", "hello");
        stage_file(&git_repo.repo, "test.txt");
        create_commit(&git_repo.repo, "Initial commit");

        let head = git_repo.repo.head().unwrap();
        let commit = head.peel_to_commit().unwrap();
        let hash = commit.id().to_string();

        let diff = git_repo.get_commit_diff(&hash).unwrap();
        assert!(diff.contains("+hello"));
    }

    #[test]
    fn test_get_commit_diff_normal() {
        let (dir, git_repo) = create_test_repo();
        create_file(dir.path(), "test.txt", "hello");
        stage_file(&git_repo.repo, "test.txt");
        create_commit(&git_repo.repo, "Initial commit");

        // 第二次提交
        create_file(dir.path(), "test.txt", "hello world");
        stage_file(&git_repo.repo, "test.txt");
        create_commit(&git_repo.repo, "Second commit");

        let head = git_repo.repo.head().unwrap();
        let commit = head.peel_to_commit().unwrap();
        let hash = commit.id().to_string();

        let diff = git_repo.get_commit_diff(&hash).unwrap();
        assert!(diff.contains("-hello"));
        assert!(diff.contains("+hello world"));
    }

    #[test]
    fn test_get_commit_diff_invalid_hash() {
        let (_dir, git_repo) = create_test_repo();
        let result = git_repo.get_commit_diff("invalid_hash");
        assert!(result.is_err());
    }

    // === 测试 get_range_diff ===

    #[test]
    fn test_get_range_diff() {
        let (dir, git_repo) = create_test_repo();
        create_file(dir.path(), "test.txt", "version1");
        stage_file(&git_repo.repo, "test.txt");
        create_commit(&git_repo.repo, "First commit");

        let first_commit = git_repo.repo.head().unwrap().peel_to_commit().unwrap();

        create_file(dir.path(), "test.txt", "version2");
        stage_file(&git_repo.repo, "test.txt");
        create_commit(&git_repo.repo, "Second commit");

        let second_commit = git_repo.repo.head().unwrap().peel_to_commit().unwrap();

        let range = format!("{}..{}", first_commit.id(), second_commit.id());
        let diff = git_repo.get_range_diff(&range).unwrap();

        assert!(diff.contains("-version1"));
        assert!(diff.contains("+version2"));
    }

    #[test]
    fn test_get_range_diff_invalid_format() {
        let (dir, git_repo) = create_test_repo();
        create_file(dir.path(), "test.txt", "hello");
        stage_file(&git_repo.repo, "test.txt");
        create_commit(&git_repo.repo, "Initial commit");

        let result = git_repo.get_range_diff("invalid_range");
        assert!(result.is_err());
    }

    // === 测试 get_file_content ===

    #[test]
    fn test_get_file_content() {
        let (dir, git_repo) = create_test_repo();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "hello world").unwrap();

        let content = git_repo
            .get_file_content(file_path.to_str().unwrap())
            .unwrap();
        assert_eq!(content, "hello world");
    }

    #[test]
    fn test_get_file_content_too_large() {
        let (dir, git_repo) = create_test_repo();
        let file_path = dir.path().join("large.txt");

        // 创建超过 max_file_size 的文件
        let large_content = "x".repeat((DEFAULT_MAX_FILE_SIZE + 1) as usize);
        fs::write(&file_path, large_content).unwrap();

        let result = git_repo.get_file_content(file_path.to_str().unwrap());
        assert!(result.is_err());
    }

    // === 测试 get_commit_history ===

    #[test]
    fn test_get_commit_history_empty_repo() {
        let (_dir, git_repo) = create_test_repo();
        let commits = git_repo.get_commit_history().unwrap();
        assert!(commits.is_empty());
    }

    #[test]
    fn test_get_commit_history() {
        let (dir, git_repo) = create_test_repo();

        create_file(dir.path(), "test.txt", "v1");
        stage_file(&git_repo.repo, "test.txt");
        create_commit(&git_repo.repo, "First commit");

        create_file(dir.path(), "test.txt", "v2");
        stage_file(&git_repo.repo, "test.txt");
        create_commit(&git_repo.repo, "Second commit");

        let commits = git_repo.get_commit_history().unwrap();
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].message, "Second commit");
        assert_eq!(commits[1].message, "First commit");
        assert_eq!(commits[0].author_name, "Test User");
        assert_eq!(commits[0].author_email, "test@example.com");
    }

    // === 测试 get_diff_stats ===

    #[test]
    fn test_get_diff_stats() {
        let (_dir, git_repo) = create_test_repo();
        let diff = r#"
diff --git a/test.txt b/test.txt
index 1234567..abcdefg 100644
--- a/test.txt
+++ b/test.txt
@@ -1,1 +1,2 @@
 hello
+world
"#;
        let stats = git_repo.get_diff_stats(diff).unwrap();
        assert_eq!(stats.files_changed.len(), 1);
        assert_eq!(stats.insertions, 1);
        assert_eq!(stats.deletions, 0);
    }
}
