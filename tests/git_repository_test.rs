//! Git Repository 边界情况和错误处理测试
//!
//! 测试 GitRepository 的各种边界情况：
//! - 空仓库
//! - 大文件限制
//! - 特殊字符路径
//! - 第一个 commit
//! - 错误处理
//! - Detached HEAD
//! - 时间戳处理

use gcop_rs::config::FileConfig;
use gcop_rs::error::{GcopError, Result};
use gcop_rs::git::{GitOperations, repository::GitRepository};
use serial_test::serial;
use std::env;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

// ========== 辅助函数 ==========

fn init_git_repo(path: &Path) -> Result<git2::Repository> {
    git2::Repository::init(path).map_err(GcopError::from)
}

fn create_test_file(repo_path: &Path, filename: &str, content: &str) -> Result<()> {
    let file_path = repo_path.join(filename);
    fs::write(&file_path, content)?;
    Ok(())
}

fn add_file_to_index(repo: &git2::Repository, filename: &str) -> Result<()> {
    let mut index = repo.index()?;
    index.add_path(Path::new(filename))?;
    index.write()?;
    Ok(())
}

fn create_commit(
    repo: &git2::Repository,
    message: &str,
    parents: Vec<&git2::Commit>,
) -> Result<git2::Oid> {
    let mut index = repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let sig = git2::Signature::now("Test User", "test@example.com")?;

    let parent_commits: Vec<&git2::Commit> = parents.to_vec();

    let oid = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_commits)?;

    Ok(oid)
}

// ========== 空仓库测试 ==========

#[test]
#[serial]
fn test_is_empty_on_fresh_repo() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();

    init_git_repo(repo_path)?;

    let original_dir = env::current_dir()?;
    env::set_current_dir(repo_path)?;

    let git_repo = GitRepository::open(None)?;
    assert!(git_repo.is_empty()?);

    env::set_current_dir(original_dir)?;
    Ok(())
}

#[test]
#[serial]
fn test_is_empty_with_commit() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    let repo = init_git_repo(repo_path)?;

    // 创建第一个 commit
    create_test_file(repo_path, "test.txt", "content")?;
    add_file_to_index(&repo, "test.txt")?;
    create_commit(&repo, "Initial commit", vec![])?;

    let original_dir = env::current_dir()?;
    env::set_current_dir(repo_path)?;

    let git_repo = GitRepository::open(None)?;
    assert!(!git_repo.is_empty()?);

    env::set_current_dir(original_dir)?;
    Ok(())
}

#[test]
#[serial]
fn test_get_staged_diff_on_empty_repo() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    let repo = init_git_repo(repo_path)?;

    // 添加文件到 staging
    create_test_file(repo_path, "test.txt", "content")?;
    add_file_to_index(&repo, "test.txt")?;

    let original_dir = env::current_dir()?;
    env::set_current_dir(repo_path)?;

    let git_repo = GitRepository::open(None)?;
    let diff = git_repo.get_staged_diff()?;

    assert!(diff.contains("test.txt"));
    assert!(diff.contains("+content"));

    env::set_current_dir(original_dir)?;
    Ok(())
}

// ========== 大文件限制测试 ==========

#[test]
#[serial]
fn test_get_file_content_exceeds_max_size() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    let file_path = repo_path.join("large.txt");

    // 创建 11MB 文件（超过默认 10MB 限制）
    let large_content = "x".repeat(11 * 1024 * 1024);
    fs::write(&file_path, large_content)?;

    init_git_repo(repo_path)?;

    let original_dir = env::current_dir()?;
    env::set_current_dir(repo_path)?;

    let git_repo = GitRepository::open(None)?;
    let result = git_repo.get_file_content("large.txt");

    assert!(result.is_err());
    match result.unwrap_err() {
        GcopError::InvalidInput(msg) => {
            assert!(msg.contains("File too large"));
        }
        _ => panic!("Expected InvalidInput error"),
    }

    env::set_current_dir(original_dir)?;
    Ok(())
}

#[test]
#[serial]
fn test_get_file_content_respects_custom_max_size() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    let file_path = repo_path.join("small.txt");

    // 创建 1KB 文件
    let content = "x".repeat(1024);
    fs::write(&file_path, content)?;

    init_git_repo(repo_path)?;

    let original_dir = env::current_dir()?;
    env::set_current_dir(repo_path)?;

    // 自定义 max_size = 512 bytes
    let file_config = FileConfig { max_size: 512 };
    let git_repo = GitRepository::open(Some(&file_config))?;
    let result = git_repo.get_file_content("small.txt");

    assert!(result.is_err());

    env::set_current_dir(original_dir)?;
    Ok(())
}

// ========== 特殊字符路径测试 ==========

#[test]
#[serial]
fn test_paths_with_spaces() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    let repo = init_git_repo(repo_path)?;

    create_test_file(repo_path, "file with spaces.txt", "content")?;
    add_file_to_index(&repo, "file with spaces.txt")?;

    let original_dir = env::current_dir()?;
    env::set_current_dir(repo_path)?;

    let git_repo = GitRepository::open(None)?;
    let diff = git_repo.get_staged_diff()?;

    assert!(diff.contains("file with spaces.txt"));

    env::set_current_dir(original_dir)?;
    Ok(())
}

#[test]
#[serial]
#[cfg_attr(windows, ignore)] // Windows 的 Unicode 路径处理可能有问题
fn test_paths_with_unicode() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    let repo = init_git_repo(repo_path)?;

    create_test_file(repo_path, "文件名.txt", "内容")?;
    add_file_to_index(&repo, "文件名.txt")?;

    let original_dir = env::current_dir()?;
    env::set_current_dir(repo_path)?;

    let git_repo = GitRepository::open(None)?;
    let diff = git_repo.get_staged_diff()?;

    // git2 可能会以 quoted path 形式显示 Unicode
    // 只要 diff 包含内容即可
    assert!(!diff.is_empty());

    env::set_current_dir(original_dir)?;
    Ok(())
}

// ========== 第一个 commit 测试 ==========

#[test]
#[serial]
fn test_get_commit_diff_for_first_commit() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    let repo = init_git_repo(repo_path)?;

    // 创建第一个 commit
    create_test_file(repo_path, "test.txt", "content")?;
    add_file_to_index(&repo, "test.txt")?;
    let commit_id = create_commit(&repo, "Initial commit", vec![])?;

    let original_dir = env::current_dir()?;
    env::set_current_dir(repo_path)?;

    let git_repo = GitRepository::open(None)?;
    let diff = git_repo.get_commit_diff(&commit_id.to_string())?;

    assert!(diff.contains("+content"));

    env::set_current_dir(original_dir)?;
    Ok(())
}

// ========== 错误处理测试 ==========

#[test]
#[serial]
fn test_get_commit_diff_invalid_hash() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    init_git_repo(repo_path)?;

    let original_dir = env::current_dir()?;
    env::set_current_dir(repo_path)?;

    let git_repo = GitRepository::open(None)?;
    let result = git_repo.get_commit_diff("invalid_hash");

    assert!(result.is_err());
    match result.unwrap_err() {
        GcopError::InvalidInput(msg) => {
            assert!(msg.contains("Invalid commit hash"));
        }
        _ => panic!("Expected InvalidInput error"),
    }

    env::set_current_dir(original_dir)?;
    Ok(())
}

#[test]
#[serial]
fn test_get_range_diff_invalid_format() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    init_git_repo(repo_path)?;

    let original_dir = env::current_dir()?;
    env::set_current_dir(repo_path)?;

    let git_repo = GitRepository::open(None)?;
    let result = git_repo.get_range_diff("invalid-range-format");

    assert!(result.is_err());
    match result.unwrap_err() {
        GcopError::InvalidInput(msg) => {
            assert!(msg.contains("Invalid range format"));
            assert!(msg.contains("Expected format: base..head"));
        }
        _ => panic!("Expected InvalidInput error"),
    }

    env::set_current_dir(original_dir)?;
    Ok(())
}

// ========== Detached HEAD 测试 ==========

#[test]
#[serial]
fn test_get_current_branch_detached_head() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    let repo = init_git_repo(repo_path)?;

    // 创建 commit
    create_test_file(repo_path, "test.txt", "content")?;
    add_file_to_index(&repo, "test.txt")?;
    let commit_id = create_commit(&repo, "Test commit", vec![])?;

    // Detach HEAD
    let commit = repo.find_commit(commit_id)?;
    repo.set_head_detached(commit.id())?;

    let original_dir = env::current_dir()?;
    env::set_current_dir(repo_path)?;

    let git_repo = GitRepository::open(None)?;
    let branch = git_repo.get_current_branch()?;

    assert_eq!(branch, None);

    env::set_current_dir(original_dir)?;
    Ok(())
}

// ========== 时间戳测试 ==========

#[test]
#[serial]
fn test_get_commit_history_normal() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    let repo = init_git_repo(repo_path)?;

    // 创建第一个 commit
    create_test_file(repo_path, "test1.txt", "content1")?;
    add_file_to_index(&repo, "test1.txt")?;
    let first_commit_id = create_commit(&repo, "First commit", vec![])?;

    // 创建第二个 commit
    create_test_file(repo_path, "test2.txt", "content2")?;
    add_file_to_index(&repo, "test2.txt")?;
    let first_commit = repo.find_commit(first_commit_id)?;
    create_commit(&repo, "Second commit", vec![&first_commit])?;

    let original_dir = env::current_dir()?;
    env::set_current_dir(repo_path)?;

    let git_repo = GitRepository::open(None)?;
    let history = git_repo.get_commit_history()?;

    assert_eq!(history.len(), 2);
    assert_eq!(history[0].message, "Second commit");
    assert_eq!(history[1].message, "First commit");
    assert_eq!(history[0].author_name, "Test User");

    env::set_current_dir(original_dir)?;
    Ok(())
}

// ========== has_staged_changes 测试 ==========

#[test]
#[serial]
fn test_has_staged_changes_true() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    let repo = init_git_repo(repo_path)?;

    create_test_file(repo_path, "test.txt", "content")?;
    add_file_to_index(&repo, "test.txt")?;

    let original_dir = env::current_dir()?;
    env::set_current_dir(repo_path)?;

    let git_repo = GitRepository::open(None)?;
    assert!(git_repo.has_staged_changes()?);

    env::set_current_dir(original_dir)?;
    Ok(())
}

#[test]
#[serial]
fn test_has_staged_changes_false() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    init_git_repo(repo_path)?;

    let original_dir = env::current_dir()?;
    env::set_current_dir(repo_path)?;

    let git_repo = GitRepository::open(None)?;
    assert!(!git_repo.has_staged_changes()?);

    env::set_current_dir(original_dir)?;
    Ok(())
}
