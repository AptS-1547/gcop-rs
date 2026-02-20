use chrono::{DateTime, Local, TimeZone};
use git2::{DiffOptions, Repository, Sort};
use std::io::Write;

use crate::config::FileConfig;
use crate::error::{GcopError, Result};
use crate::git::{CommitInfo, DiffStats, GitOperations};

/// Default maximum file size (10MB)
const DEFAULT_MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// `git2`-based repository implementation used by gcop-rs.
pub struct GitRepository {
    repo: Repository,
    max_file_size: u64,
}

impl GitRepository {
    /// Open the git repository of the current directory
    ///
    /// # Arguments
    /// * `file_config` - optional file configuration, None uses default value
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

    /// Convert git2::Diff to string
    fn diff_to_string(&self, diff: &git2::Diff) -> Result<String> {
        let mut output = Vec::new();
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            // Get the type tag (origin) of the row
            let origin = line.origin();

            // If origin is a printable character (+, -, space, etc.), write it first
            match origin {
                '+' | '-' | ' ' => {
                    let _ = output.write_all(&[origin as u8]);
                }
                _ => {}
            }

            // Then write the row content
            let _ = output.write_all(line.content());
            true
        })?;
        Ok(String::from_utf8_lossy(&output).to_string())
    }
}

impl GitOperations for GitRepository {
    fn get_staged_diff(&self) -> Result<String> {
        // Read index.
        let index = self.repo.index()?;

        // For an empty repository, compare empty tree (None) against the index.
        if self.is_empty()? {
            let mut opts = DiffOptions::new();
            let diff = self
                .repo
                .diff_tree_to_index(None, Some(&index), Some(&mut opts))?;
            return self.diff_to_string(&diff);
        }

        // Read HEAD tree.
        let head = self.repo.head()?;
        let head_tree = head.peel_to_tree()?;

        // Create diff (HEAD tree vs index)
        let mut opts = DiffOptions::new();
        let diff = self
            .repo
            .diff_tree_to_index(Some(&head_tree), Some(&index), Some(&mut opts))?;

        self.diff_to_string(&diff)
    }

    fn get_uncommitted_diff(&self) -> Result<String> {
        // Read index.
        let index = self.repo.index()?;

        // Create diff (index vs workdir)
        let mut opts = DiffOptions::new();
        let diff = self
            .repo
            .diff_index_to_workdir(Some(&index), Some(&mut opts))?;

        self.diff_to_string(&diff)
    }

    fn get_commit_diff(&self, commit_hash: &str) -> Result<String> {
        // Find commit
        let commit = self
            .repo
            .find_commit(git2::Oid::from_str(commit_hash).map_err(|_| {
                GcopError::InvalidInput(
                    rust_i18n::t!("git.invalid_commit_hash", hash = commit_hash).to_string(),
                )
            })?)?;

        let commit_tree = commit.tree()?;

        // Get the parent commit (if any)
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };

        // Build diff.
        let mut opts = DiffOptions::new();
        let diff = self.repo.diff_tree_to_tree(
            parent_tree.as_ref(),
            Some(&commit_tree),
            Some(&mut opts),
        )?;

        self.diff_to_string(&diff)
    }

    fn get_range_diff(&self, range: &str) -> Result<String> {
        // Parse range expression (for example "main..feature").
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
        // Unborn branch has no real branch information
        if self.is_empty()? {
            return Ok(None);
        }

        let head = self.repo.head()?;

        if head.is_branch() {
            // Read branch name.
            let branch_name = head.shorthand().map(|s| s.to_string());
            Ok(branch_name)
        } else {
            // HEAD is in detached state
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
        // Empty repository has no history.
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

            // Convert git2::Time to chrono::DateTime<Local>
            let git_time = commit.time();
            let timestamp: DateTime<Local> = Local
                .timestamp_opt(git_time.seconds(), 0)
                .single()
                .unwrap_or_else(|| {
                    tracing::warn!(
                        "Invalid git timestamp {} for commit {}",
                        git_time.seconds(),
                        commit.id()
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
        // Detect unborn branch: if `head()` fails with `UnbornBranch`, the repository is empty.
        match self.repo.head() {
            Ok(_) => Ok(false),
            Err(e) if e.code() == git2::ErrorCode::UnbornBranch => Ok(true),
            Err(e) => Err(e.into()),
        }
    }

    fn get_staged_files(&self) -> Result<Vec<String>> {
        let mut index = self.repo.index()?;
        // Force-reload from disk so that changes made by external git processes
        // (e.g. `git reset HEAD` in unstage_all) are visible.
        index.read(true)?;
        let tree = if self.is_empty()? {
            None
        } else {
            let head = self.repo.head()?;
            Some(head.peel_to_tree()?)
        };
        let mut opts = DiffOptions::new();
        let diff = self
            .repo
            .diff_tree_to_index(tree.as_ref(), Some(&index), Some(&mut opts))?;

        Ok(diff
            .deltas()
            .filter_map(|delta| delta.new_file().path())
            .map(|p| p.to_string_lossy().into_owned())
            .collect())
    }

    fn unstage_all(&self) -> Result<()> {
        use std::process::Command;

        let workdir = self
            .repo
            .workdir()
            .ok_or_else(|| crate::error::GcopError::GitCommand("bare repository".to_string()))?;

        if self.is_empty()? {
            // Empty repo: no HEAD to reset to, use git rm --cached
            let output = Command::new("git")
                .current_dir(workdir)
                .args(["rm", "--cached", "-r", "."])
                .output()?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(crate::error::GcopError::GitCommand(
                    stderr.trim().to_string(),
                ));
            }
        } else {
            let output = Command::new("git")
                .current_dir(workdir)
                .args(["reset", "HEAD"])
                .output()?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(crate::error::GcopError::GitCommand(
                    stderr.trim().to_string(),
                ));
            }
        }
        Ok(())
    }

    fn stage_files(&self, files: &[String]) -> Result<()> {
        use std::process::Command;

        if files.is_empty() {
            return Ok(());
        }

        let workdir = self
            .repo
            .workdir()
            .ok_or_else(|| crate::error::GcopError::GitCommand("bare repository".to_string()))?;

        let output = Command::new("git")
            .current_dir(workdir)
            .env("GIT_LITERAL_PATHSPECS", "1")
            .arg("add")
            .args(files)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::error::GcopError::GitCommand(
                stderr.trim().to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    /// Create a temporary git repository for testing
    fn create_test_repo() -> (TempDir, GitRepository) {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Set user information
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        let git_repo = GitRepository {
            repo,
            max_file_size: DEFAULT_MAX_FILE_SIZE,
        };

        (dir, git_repo)
    }

    /// Create files in the repository
    fn create_file(dir: &Path, name: &str, content: &str) {
        let file_path = dir.join(name);
        fs::write(&file_path, content).unwrap();
    }

    /// Temporary files
    fn stage_file(repo: &Repository, name: &str) {
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(name)).unwrap();
        index.write().unwrap();
    }

    /// Create commit
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

    // === Test is_empty ===

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

    // === Test get_current_branch ===

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
        // The default branch is master or main
        let branch_name = branch.unwrap();
        assert!(branch_name == "master" || branch_name == "main");
    }

    #[test]
    fn test_get_current_branch_detached_head() {
        let (dir, git_repo) = create_test_repo();
        create_file(dir.path(), "test.txt", "hello");
        stage_file(&git_repo.repo, "test.txt");
        create_commit(&git_repo.repo, "Initial commit");

        // Get commit hash and checkout to detached HEAD
        let head = git_repo.repo.head().unwrap();
        let commit = head.peel_to_commit().unwrap();
        git_repo.repo.set_head_detached(commit.id()).unwrap();

        assert_eq!(git_repo.get_current_branch().unwrap(), None);
    }

    // === Test has_staged_changes ===

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

    // === Test get_staged_diff ===

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

        // Modify files and save temporarily
        create_file(dir.path(), "test.txt", "hello world");
        stage_file(&git_repo.repo, "test.txt");

        let diff = git_repo.get_staged_diff().unwrap();
        assert!(diff.contains("-hello"));
        assert!(diff.contains("+hello world"));
    }

    // === Test get_uncommitted_diff ===

    #[test]
    fn test_get_uncommitted_diff() {
        let (dir, git_repo) = create_test_repo();
        create_file(dir.path(), "test.txt", "hello");
        stage_file(&git_repo.repo, "test.txt");
        create_commit(&git_repo.repo, "Initial commit");

        // Modify files but don't stage them
        create_file(dir.path(), "test.txt", "hello world");

        let diff = git_repo.get_uncommitted_diff().unwrap();
        assert!(diff.contains("-hello"));
        assert!(diff.contains("+hello world"));
    }

    // === Test get_commit_diff ===

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

        // Second submission
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

    // === Test get_range_diff ===

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

    // === Test get_file_content ===

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

        // Create files larger than max_file_size
        let large_content = "x".repeat((DEFAULT_MAX_FILE_SIZE + 1) as usize);
        fs::write(&file_path, large_content).unwrap();

        let result = git_repo.get_file_content(file_path.to_str().unwrap());
        assert!(result.is_err());
    }

    // === Test get_commit_history ===

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

    // === Test get_diff_stats ===

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

    // === Test stage_files ===

    #[test]
    fn test_stage_files_literal_glob_path() {
        // Verify that stage_files treats paths with bracket characters literally.
        // Without GIT_LITERAL_PATHSPECS=1, git would interpret `[locale]` as a
        // character-class glob and might stage unintended files.
        let (dir, git_repo) = create_test_repo();

        // Create initial commit so the repo is non-empty.
        create_file(dir.path(), "init.txt", "init");
        stage_file(&git_repo.repo, "init.txt");
        create_commit(&git_repo.repo, "initial");

        // Create a directory named literally `[locale]` and a sibling `l/`.
        let bracket_dir = dir.path().join("[locale]");
        let sibling_dir = dir.path().join("l");
        fs::create_dir_all(&bracket_dir).unwrap();
        fs::create_dir_all(&sibling_dir).unwrap();

        fs::write(bracket_dir.join("page.tsx"), "bracket content").unwrap();
        fs::write(sibling_dir.join("page.tsx"), "sibling content").unwrap();

        // Stage only the bracket-path file via git2 directly.
        let mut index = git_repo.repo.index().unwrap();
        index
            .add_path(std::path::Path::new("[locale]/page.tsx"))
            .unwrap();
        index.write().unwrap();

        // Now unstage everything and re-stage via stage_files.
        git_repo.unstage_all().unwrap();

        git_repo
            .stage_files(&["[locale]/page.tsx".to_string()])
            .unwrap();

        // Only `[locale]/page.tsx` should be staged; `l/page.tsx` must NOT be.
        let staged = git_repo.get_staged_files().unwrap();
        assert!(
            staged.contains(&"[locale]/page.tsx".to_string()),
            "expected [locale]/page.tsx to be staged"
        );
        assert!(
            !staged.contains(&"l/page.tsx".to_string()),
            "l/page.tsx should NOT be staged (glob expansion guard)"
        );
    }

    #[test]
    fn test_stage_files_glob_path_missing_literal_errors_not_sibling() {
        // When the literal `[locale]/page.tsx` does NOT exist but a sibling `l/page.tsx` does,
        // stage_files must return an error rather than silently staging the sibling.
        let (dir, git_repo) = create_test_repo();

        create_file(dir.path(), "init.txt", "init");
        stage_file(&git_repo.repo, "init.txt");
        create_commit(&git_repo.repo, "initial");

        // Only create `l/page.tsx` — NOT `[locale]/page.tsx`.
        let sibling_dir = dir.path().join("l");
        fs::create_dir_all(&sibling_dir).unwrap();
        fs::write(sibling_dir.join("page.tsx"), "sibling").unwrap();

        // Staging the non-existent literal path must fail.
        let result = git_repo.stage_files(&["[locale]/page.tsx".to_string()]);
        assert!(
            result.is_err(),
            "staging a non-existent literal path should fail"
        );

        // The sibling must not have been staged as a side-effect.
        let staged = git_repo.get_staged_files().unwrap();
        assert!(
            !staged.contains(&"l/page.tsx".to_string()),
            "l/page.tsx must NOT be staged as a glob side-effect"
        );
    }

    #[test]
    fn test_unstage_all_then_stage_subset_does_not_touch_unstaged_file() {
        // Simulate split commit: after unstage_all + stage_files(subset),
        // only the requested files should be staged.
        // Files with purely unstaged modifications must never be staged.
        let (dir, git_repo) = create_test_repo();

        // Initial commit with three files.
        create_file(dir.path(), "a.rs", "v1");
        create_file(dir.path(), "b.rs", "v1");
        create_file(dir.path(), "c.rs", "v1");
        stage_file(&git_repo.repo, "a.rs");
        stage_file(&git_repo.repo, "b.rs");
        stage_file(&git_repo.repo, "c.rs");
        create_commit(&git_repo.repo, "initial");

        // Stage a.rs and b.rs; leave c.rs unstaged.
        create_file(dir.path(), "a.rs", "v2");
        create_file(dir.path(), "b.rs", "v2");
        create_file(dir.path(), "c.rs", "v2");
        stage_file(&git_repo.repo, "a.rs");
        stage_file(&git_repo.repo, "b.rs");
        // c.rs intentionally NOT staged.

        let staged_before = git_repo.get_staged_files().unwrap();
        assert!(staged_before.contains(&"a.rs".to_string()));
        assert!(staged_before.contains(&"b.rs".to_string()));
        assert!(!staged_before.contains(&"c.rs".to_string()));

        // Split commit simulation: unstage all, then re-stage only a.rs.
        git_repo.unstage_all().unwrap();
        git_repo.stage_files(&["a.rs".to_string()]).unwrap();

        let staged_after = git_repo.get_staged_files().unwrap();
        assert!(
            staged_after.contains(&"a.rs".to_string()),
            "a.rs should be staged"
        );
        assert!(
            !staged_after.contains(&"b.rs".to_string()),
            "b.rs should NOT be staged (belongs to a different group)"
        );
        assert!(
            !staged_after.contains(&"c.rs".to_string()),
            "c.rs should NOT be staged (was never in the staging area)"
        );
    }
}
