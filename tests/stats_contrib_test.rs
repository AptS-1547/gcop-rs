/// stats contrib 功能集成测试
///
/// 测试 compute_contrib_stats() 的行级统计功能：
/// - 基本统计（insertions, deletions, total）
/// - 作者过滤
/// - 跳过 merge commits
/// - 百分比计算
use gcop_rs::commands::stats::compute_contrib_stats;
use gcop_rs::error::Result;
use gcop_rs::git::{GitOperations, repository::GitRepository};
use serial_test::serial;
use std::env;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

// ========== 辅助函数 ==========

fn init_git_repo(path: &Path) -> Result<git2::Repository> {
    git2::Repository::init(path).map_err(gcop_rs::error::GcopError::from)
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

// ========== 基本统计测试 ==========

#[test]
#[serial]
fn test_compute_contrib_stats_basic() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    let repo = init_git_repo(repo_path)?;

    // 第一个 commit（5 行插入）
    create_test_file(repo_path, "test.txt", "line1\nline2\nline3\nline4\nline5")?;
    add_file_to_index(&repo, "test.txt")?;
    create_commit(&repo, "Initial commit", vec![])?;

    let original_dir = env::current_dir()?;
    env::set_current_dir(repo_path)?;

    let git_repo = GitRepository::open(None)?;
    let commits = git_repo.get_commit_history()?;
    let contrib = compute_contrib_stats(&commits, &git_repo, None)?;

    assert_eq!(contrib.total_insertions, 5);
    assert_eq!(contrib.total_deletions, 0);
    assert_eq!(contrib.total_lines, 5);
    assert_eq!(contrib.authors.len(), 1);
    assert_eq!(contrib.authors[0].name, "Test User");
    assert_eq!(contrib.authors[0].insertions, 5);
    assert_eq!(contrib.authors[0].deletions, 0);
    assert_eq!(contrib.authors[0].total, 5);
    assert!((contrib.authors[0].percentage - 100.0).abs() < 0.01);

    env::set_current_dir(original_dir)?;
    Ok(())
}

#[test]
#[serial]
fn test_compute_contrib_stats_multiple_commits() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    let repo = init_git_repo(repo_path)?;

    // 第一个 commit（3 行）
    create_test_file(repo_path, "test.txt", "line1\nline2\nline3")?;
    add_file_to_index(&repo, "test.txt")?;
    let first_commit_id = create_commit(&repo, "First commit", vec![])?;

    // 第二个 commit（删除 1 行，添加 2 行）
    create_test_file(repo_path, "test.txt", "line1\nline2_modified\nline3\nline4")?;
    add_file_to_index(&repo, "test.txt")?;
    let first_commit = repo.find_commit(first_commit_id)?;
    create_commit(&repo, "Second commit", vec![&first_commit])?;

    let original_dir = env::current_dir()?;
    env::set_current_dir(repo_path)?;

    let git_repo = GitRepository::open(None)?;
    let commits = git_repo.get_commit_history()?;
    let contrib = compute_contrib_stats(&commits, &git_repo, None)?;

    // 总计：3 + 3 = 6 insertions, 2 deletions
    // （第二个 commit 的统计是 3 insertions, 2 deletions）
    assert_eq!(contrib.total_insertions, 6);
    assert_eq!(contrib.total_deletions, 2);
    assert_eq!(contrib.total_lines, 8);

    env::set_current_dir(original_dir)?;
    Ok(())
}

#[test]
#[serial]
fn test_compute_contrib_stats_skip_merge_commits() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    let repo = init_git_repo(repo_path)?;

    // 创建主分支的第一个 commit
    create_test_file(repo_path, "main.txt", "main content")?;
    add_file_to_index(&repo, "main.txt")?;
    let main_commit_id = create_commit(&repo, "Main commit", vec![])?;
    let main_commit = repo.find_commit(main_commit_id)?;

    // 在主分支上再创建一个 commit
    create_test_file(repo_path, "main2.txt", "main2 content")?;
    add_file_to_index(&repo, "main2.txt")?;
    let main2_commit_id = create_commit(&repo, "Main commit 2", vec![&main_commit])?;
    let main2_commit = repo.find_commit(main2_commit_id)?;

    // 创建一个 merge commit（手动指定 2 个 parent）
    // 注意：这里我们不实际创建分支，只是创建一个有 2 个 parent 的 commit
    create_test_file(repo_path, "merge.txt", "merge content")?;
    add_file_to_index(&repo, "merge.txt")?;

    // 创建 merge commit（parent_count = 2）
    let mut index = repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let sig = git2::Signature::now("Test User", "test@example.com")?;

    // 使用 2 个 parent 创建 merge commit
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "Merge commit",
        &tree,
        &[&main2_commit, &main_commit], // 2 个 parent
    )?;

    let original_dir = env::current_dir()?;
    env::set_current_dir(repo_path)?;

    let git_repo = GitRepository::open(None)?;
    let commits = git_repo.get_commit_history()?;
    let contrib = compute_contrib_stats(&commits, &git_repo, None)?;

    // Merge commit 应该被跳过
    assert_eq!(contrib.merge_commits_skipped, 1);

    env::set_current_dir(original_dir)?;
    Ok(())
}

#[test]
#[serial]
fn test_compute_contrib_stats_empty_repo() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    init_git_repo(repo_path)?;

    let original_dir = env::current_dir()?;
    env::set_current_dir(repo_path)?;

    let git_repo = GitRepository::open(None)?;
    let commits = git_repo.get_commit_history()?;
    let contrib = compute_contrib_stats(&commits, &git_repo, None)?;

    assert_eq!(contrib.total_insertions, 0);
    assert_eq!(contrib.total_deletions, 0);
    assert_eq!(contrib.total_lines, 0);
    assert_eq!(contrib.authors.len(), 0);

    env::set_current_dir(original_dir)?;
    Ok(())
}
