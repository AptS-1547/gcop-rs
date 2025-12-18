use git2::{DiffOptions, Repository};
use std::io::Write;

use crate::error::{GcopError, Result};
use crate::git::{DiffStats, GitOperations};

pub struct GitRepository {
    repo: Repository,
}

impl GitRepository {
    /// 打开当前目录的 git 仓库
    pub fn open() -> Result<Self> {
        let repo = Repository::open(".")?;
        Ok(Self { repo })
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
        // 获取 HEAD tree
        let head = self.repo.head()?;
        let head_tree = head.peel_to_tree()?;

        // 获取 index
        let index = self.repo.index()?;

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
                GcopError::InvalidInput(format!("Invalid commit hash: {}", commit_hash))
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
            return Err(GcopError::InvalidInput(format!(
                "Invalid range format: {}. Expected format: base..head",
                range
            )));
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
        let content = std::fs::read_to_string(path)?;
        Ok(content)
    }

    fn commit(&self, message: &str) -> Result<()> {
        crate::git::commit::commit_changes(message)
    }

    fn get_current_branch(&self) -> Result<Option<String>> {
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
}
