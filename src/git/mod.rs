//! Git abstractions and repository operations.
//!
//! Provides the `GitOperations` trait, common data types, and helpers used by
//! command flows.

/// Commit writing helpers.
pub mod commit;
/// Diff parsing and per-file statistics helpers.
pub mod diff;
/// `git2`-backed repository implementation of [`GitOperations`].
pub mod repository;

use std::path::PathBuf;

use crate::error::Result;
use chrono::{DateTime, Local};
use serde::Serialize;

#[cfg(any(test, feature = "test-utils"))]
use mockall::automock;

/// Git commit metadata.
///
/// Contains author information, timestamp, and message summary.
///
/// # Fields
/// - `author_name`: author name
/// - `author_email`: author email address
/// - `timestamp`: commit timestamp (local timezone)
/// - `message`: commit message content
#[derive(Debug, Clone)]
pub struct CommitInfo {
    /// Commit author name.
    pub author_name: String,
    /// Commit author email.
    pub author_email: String,
    /// Commit timestamp in local timezone.
    pub timestamp: DateTime<Local>,
    /// First line of the commit message.
    #[allow(dead_code)]
    // Reserved for future commit-message analytics.
    pub message: String,
}

/// Unified interface for Git operations.
///
/// This trait abstracts all Git repository operations, making it easier to test and extend.
/// Main implementation: [`GitRepository`](repository::GitRepository).
///
/// # Design
/// - Pure Rust interface, independent of concrete backend implementation.
/// - Supports mocking in tests (via `mockall`).
/// - Uses unified error handling via [`GcopError`](crate::error::GcopError).
///
/// # Example
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
    /// Returns the diff for staged changes.
    ///
    /// Equivalent to `git diff --cached --unified=3`.
    ///
    /// # Returns
    /// - `Ok(diff)` - diff text (possibly empty)
    /// - `Err(_)` - git operation failed
    ///
    /// # Errors
    /// - Repository is not initialized
    /// - Insufficient permissions
    fn get_staged_diff(&self) -> Result<String>;

    /// Returns the diff for unstaged changes.
    ///
    /// Contains only `index -> workdir` changes (unstaged),
    /// equivalent to `git diff` (without `--cached`).
    ///
    /// # Returns
    /// - `Ok(diff)` - diff text (possibly empty)
    /// - `Err(_)` - git operation failed
    fn get_uncommitted_diff(&self) -> Result<String>;

    /// Returns the diff for a specific commit.
    ///
    /// Equivalent to `git diff <commit_hash>^!` (returns only the diff content).
    ///
    /// # Parameters
    /// - `commit_hash`: commit SHA (supports short hash)
    ///
    /// # Returns
    /// - `Ok(diff)` - diff text
    /// - `Err(_)` - commit does not exist or git operation failed
    fn get_commit_diff(&self, commit_hash: &str) -> Result<String>;

    /// Returns the diff for a commit range.
    ///
    /// Supports multiple formats:
    /// - `HEAD~3..HEAD` - last 3 commits
    /// - `main..feature` - difference between branches
    /// - `abc123..def456` - difference between two commits
    ///
    /// # Parameters
    /// - `range`: Git range expression
    ///
    /// # Returns
    /// - `Ok(diff)` - diff text
    /// - `Err(_)` - invalid range or git operation failed
    fn get_range_diff(&self, range: &str) -> Result<String>;

    /// Reads the complete content of a file.
    ///
    /// Reads file contents from the working tree (not from git objects).
    ///
    /// # Parameters
    /// - `path`: file path (relative to the current working directory or absolute path)
    ///
    /// # Returns
    /// - `Ok(content)` - file contents
    /// - `Err(_)` - file does not exist, is not a regular file, or read failed
    fn get_file_content(&self, path: &str) -> Result<String>;

    /// Executes `git commit`.
    ///
    /// Commits staged changes to the repository.
    ///
    /// # Parameters
    /// - `message`: commit message (supports multiple lines)
    ///
    /// # Returns
    /// - `Ok(())` - commit succeeded
    /// - `Err(_)` - no staged changes, hook failure, or another git error
    ///
    /// # Errors
    /// - [`GcopError::GitCommand`] - no staged changes
    /// - [`GcopError::Git`] - libgit2 error
    ///
    /// # Notes
    /// - Triggers pre-commit and commit-msg hooks.
    /// - Uses name/email configured in git config.
    ///
    /// [`GcopError::GitCommand`]: crate::error::GcopError::GitCommand
    /// [`GcopError::Git`]: crate::error::GcopError::Git
    fn commit(&self, message: &str) -> Result<()>;

    /// Executes `git commit --amend`.
    ///
    /// Amends the most recent commit with a new message.
    /// If there are staged changes, they are included in the amended commit.
    ///
    /// # Parameters
    /// - `message`: new commit message
    ///
    /// # Returns
    /// - `Ok(())` - amend succeeded
    /// - `Err(_)` - no commits to amend, hook failure, or another git error
    fn commit_amend(&self, message: &str) -> Result<()>;

    /// Returns the current branch name.
    ///
    /// # Returns
    /// - `Ok(Some(name))` - current branch name (for example `"main"`)
    /// - `Ok(None)` - detached HEAD
    /// - `Err(_)` - git operation failed
    ///
    /// # Example
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

    /// Calculates diff statistics.
    ///
    /// Parses diff text and extracts changed files plus insert/delete counts.
    ///
    /// # Parameters
    /// - `diff`: diff text (from `get_*_diff()` methods)
    ///
    /// # Returns
    /// - `Ok(stats)` - parsed statistics
    /// - `Err(_)` - invalid diff format
    ///
    /// # Example
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

    /// Checks whether the index contains staged changes.
    ///
    /// Fast check for files added to the index with `git add`.
    ///
    /// # Returns
    /// - `Ok(true)` - staged changes exist
    /// - `Ok(false)` - staging area is empty
    /// - `Err(_)` - git operation failed
    fn has_staged_changes(&self) -> Result<bool>;

    /// Returns commit history for the current branch.
    ///
    /// Returns commit entries in reverse chronological order.
    ///
    /// # Returns
    /// - `Ok(history)` - commit list (newest first)
    /// - `Err(_)` - git operation failed
    ///
    /// # Notes
    /// - Only includes history reachable from the current branch HEAD.
    /// - Empty repositories return an empty list.
    fn get_commit_history(&self) -> Result<Vec<CommitInfo>>;

    /// Checks whether the repository has no commits.
    ///
    /// # Returns
    /// - `Ok(true)` - repository is empty (no commits yet)
    /// - `Ok(false)` - repository has at least one commit
    /// - `Err(_)` - git operation failed
    fn is_empty(&self) -> Result<bool>;

    /// Returns the list of currently staged file paths.
    ///
    /// Equivalent to collecting filenames from `git diff --cached --name-only`.
    fn get_staged_files(&self) -> Result<Vec<String>>;

    /// Unstages all currently staged files.
    ///
    /// Equivalent to `git reset HEAD`. For empty repositories (no commits),
    /// uses `git rm --cached -r .` instead.
    fn unstage_all(&self) -> Result<()>;

    /// Stages the specified files.
    ///
    /// Equivalent to `git add <files>`.
    fn stage_files(&self, files: &[String]) -> Result<()>;
}

/// Diff statistics.
///
/// Contains changed files and insert/delete counts.
///
/// # Fields
/// - `files_changed`: changed file paths (relative to repository root)
/// - `insertions`: number of inserted lines
/// - `deletions`: number of deleted lines
///
/// # Example
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
    /// Paths of files changed in the diff.
    pub files_changed: Vec<String>,
    /// Number of inserted lines.
    pub insertions: usize,
    /// Number of deleted lines.
    pub deletions: usize,
}

/// Finds the git repository root by walking upward from the current directory.
///
/// Equivalent to `git rev-parse --show-toplevel`.
/// Checks whether `.git` (directory or file, for submodule/worktree compatibility)
/// exists at each level.
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
