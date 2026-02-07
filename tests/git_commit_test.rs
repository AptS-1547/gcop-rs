use gcop_rs::git::commit::commit_changes;
use serial_test::serial;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// 创建临时 git 仓库
fn setup_git_repo() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    // 初始化 git 仓库
    Command::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    // 设置 user.name 和 user.email（避免 git 报错）
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    temp_dir
}

/// 创建并暂存文件
fn create_and_stage_file(repo_path: &Path, name: &str, content: &str) {
    let file_path = repo_path.join(name);
    fs::write(&file_path, content).unwrap();

    Command::new("git")
        .args(["add", name])
        .current_dir(repo_path)
        .output()
        .unwrap();
}

// === 正常场景 ===

#[test]
#[serial]
fn test_commit_success() {
    let temp_dir = setup_git_repo();
    let repo_path = temp_dir.path();

    // 创建并暂存文件
    create_and_stage_file(repo_path, "test.txt", "hello");

    // 切换到仓库目录执行 commit
    std::env::set_current_dir(repo_path).unwrap();

    let result = commit_changes("test commit");
    assert!(result.is_ok());

    // 验证 commit 成功
    let output = Command::new("git")
        .args(["log", "--oneline", "-1"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    let log = String::from_utf8_lossy(&output.stdout);
    assert!(log.contains("test commit"));
}

#[test]
#[serial]
fn test_commit_with_multiline_message() {
    let temp_dir = setup_git_repo();
    let repo_path = temp_dir.path();

    create_and_stage_file(repo_path, "test.txt", "hello");
    std::env::set_current_dir(repo_path).unwrap();

    let message =
        "feat: add feature\n\nThis is the body.\n\nCo-Authored-By: Test <test@example.com>";
    let result = commit_changes(message);
    assert!(result.is_ok());

    // 验证多行消息
    let output = Command::new("git")
        .args(["log", "--format=%B", "-1"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    let log = String::from_utf8_lossy(&output.stdout);
    assert!(log.contains("feat: add feature"));
    assert!(log.contains("This is the body."));
    assert!(log.contains("Co-Authored-By: Test"));
}

// === 错误场景 ===

#[test]
#[serial]
fn test_commit_no_staged_changes() {
    let temp_dir = setup_git_repo();
    let repo_path = temp_dir.path();

    std::env::set_current_dir(repo_path).unwrap();

    let result = commit_changes("test commit");
    assert!(result.is_err());

    let err = result.unwrap_err();
    let err_msg = err.to_string();
    // git 会报错 "nothing to commit"
    assert!(
        err_msg.contains("nothing to commit") || err_msg.contains("no changes added to commit")
    );
}

#[test]
#[serial]
fn test_commit_empty_message() {
    let temp_dir = setup_git_repo();
    let repo_path = temp_dir.path();

    create_and_stage_file(repo_path, "test.txt", "hello");
    std::env::set_current_dir(repo_path).unwrap();

    let result = commit_changes("");
    assert!(result.is_err());

    let err = result.unwrap_err();
    let err_msg = err.to_string();
    // git 会报错 "empty commit message"（取决于 git 版本）
    // 有些版本可能允许空消息，但至少可以验证错误处理机制
    // 检查是否包含常见的错误消息
    assert!(
        err_msg.contains("Aborting commit")
            || err_msg.contains("empty")
            || err_msg.contains("message")
    );
}

// === pre-commit hook 场景 ===

#[test]
#[serial]
fn test_commit_hook_failure() {
    let temp_dir = setup_git_repo();
    let repo_path = temp_dir.path();

    // 创建一个会失败的 pre-commit hook
    let hooks_dir = repo_path.join(".git").join("hooks");
    fs::create_dir_all(&hooks_dir).unwrap();

    let hook_path = hooks_dir.join("pre-commit");
    fs::write(&hook_path, "#!/bin/sh\nexit 1\n").unwrap();

    // 设置可执行权限（Unix only）
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&hook_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms).unwrap();
    }

    create_and_stage_file(repo_path, "test.txt", "hello");
    std::env::set_current_dir(repo_path).unwrap();

    let result = commit_changes("test commit");

    #[cfg(unix)]
    {
        // Unix 系统应该执行 hook 并失败
        assert!(result.is_err());
        // 注意：Windows 可能不支持 hook，所以只在 Unix 验证
    }

    #[cfg(not(unix))]
    {
        // Windows 可能无法执行 shell script hook，跳过验证
        let _ = result;
    }
}

// === GPG 签名场景（可选，需要 GPG 配置）===

#[test]
#[serial]
fn test_commit_respects_git_config() {
    let temp_dir = setup_git_repo();
    let repo_path = temp_dir.path();

    // 设置一个自定义的 git config
    Command::new("git")
        .args(["config", "user.name", "Custom User"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    create_and_stage_file(repo_path, "test.txt", "hello");
    std::env::set_current_dir(repo_path).unwrap();

    let result = commit_changes("test commit");
    assert!(result.is_ok());

    // 验证 commit author
    let output = Command::new("git")
        .args(["log", "--format=%an", "-1"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    let author = String::from_utf8_lossy(&output.stdout);
    assert!(author.contains("Custom User"));
}
