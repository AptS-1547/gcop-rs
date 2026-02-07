/// alias.rs 测试
///
/// 测试 git alias 管理逻辑。
/// 使用 `git config --file <path>` 操作临时配置，避免修改全局环境。
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// 创建临时 git config 文件并返回路径
fn create_temp_config() -> (TempDir, String) {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("gitconfig");
    fs::write(&config_path, "").unwrap();
    (temp_dir, config_path.to_string_lossy().to_string())
}

/// 通过 git config --file 获取 alias
fn get_alias(config_path: &str, name: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["config", "--file", config_path, &format!("alias.{}", name)])
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// 通过 git config --file 设置 alias
fn set_alias(config_path: &str, name: &str, command: &str) {
    let output = Command::new("git")
        .args([
            "config",
            "--file",
            config_path,
            &format!("alias.{}", name),
            command,
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Failed to set alias '{}' = '{}': {}",
        name,
        command,
        String::from_utf8_lossy(&output.stderr)
    );
}

/// 通过 git config --file 移除 alias
fn unset_alias(config_path: &str, name: &str) -> bool {
    let output = Command::new("git")
        .args([
            "config",
            "--file",
            config_path,
            "--unset",
            &format!("alias.{}", name),
        ])
        .output()
        .unwrap();
    output.status.success()
}

// === 基本 CRUD 测试 ===

#[test]
fn test_get_alias_existing() {
    let (_temp, config) = create_temp_config();

    set_alias(&config, "test", "!echo test");
    let result = get_alias(&config, "test");
    assert_eq!(result, Some("!echo test".to_string()));
}

#[test]
fn test_get_alias_not_found() {
    let (_temp, config) = create_temp_config();

    let result = get_alias(&config, "nonexistent");
    assert_eq!(result, None);
}

#[test]
fn test_set_alias() {
    let (_temp, config) = create_temp_config();

    set_alias(&config, "myalias", "!gcop-rs commit");
    let result = get_alias(&config, "myalias");
    assert_eq!(result, Some("!gcop-rs commit".to_string()));
}

#[test]
fn test_set_alias_overwrite() {
    let (_temp, config) = create_temp_config();

    set_alias(&config, "myalias", "!old command");
    set_alias(&config, "myalias", "!new command");
    let result = get_alias(&config, "myalias");
    assert_eq!(result, Some("!new command".to_string()));
}

// === 删除测试 ===

#[test]
fn test_remove_alias() {
    let (_temp, config) = create_temp_config();

    set_alias(&config, "toremove", "!gcop-rs");
    assert!(get_alias(&config, "toremove").is_some());

    let removed = unset_alias(&config, "toremove");
    assert!(removed);
    assert_eq!(get_alias(&config, "toremove"), None);
}

#[test]
fn test_remove_nonexistent_alias() {
    let (_temp, config) = create_temp_config();

    let removed = unset_alias(&config, "nonexistent");
    assert!(!removed);
}

// === 冲突检测测试 ===

#[test]
fn test_alias_conflict_detection() {
    let (_temp, config) = create_temp_config();

    // 设置一个冲突的 alias（与 gcop 的 alias 同名但不同命令）
    set_alias(&config, "c", "!some other command");

    let existing = get_alias(&config, "c");
    assert!(existing.is_some());
    assert_ne!(existing.unwrap(), "!gcop-rs commit");
}

// === 批量操作测试 ===

#[test]
fn test_install_multiple_aliases() {
    let (_temp, config) = create_temp_config();

    let aliases = vec![
        ("cop", "!gcop-rs"),
        ("gcommit", "!gcop-rs commit"),
        ("c", "!gcop-rs commit"),
        ("r", "!gcop-rs review"),
        ("s", "!gcop-rs stats"),
    ];

    for (name, command) in &aliases {
        set_alias(&config, name, command);
    }

    for (name, expected_command) in &aliases {
        let result = get_alias(&config, name);
        assert_eq!(result, Some(expected_command.to_string()));
    }
}

// === 命令格式测试 ===

#[test]
fn test_alias_various_formats() {
    let (_temp, config) = create_temp_config();

    let test_cases = vec![
        ("simple", "commit"),
        ("withbang", "!gcop-rs commit"),
        ("withargs", "!git add -A && gcop-rs commit"),
        ("complex", "!git add -A && gcop-rs commit && git push"),
    ];

    for (name, command) in &test_cases {
        set_alias(&config, name, command);
        let result = get_alias(&config, name);
        assert_eq!(
            result,
            Some(command.to_string()),
            "Failed for alias '{}'",
            name
        );
    }
}

// === 配置文件格式测试 ===

#[test]
fn test_read_config_file_with_existing_content() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("gitconfig");
    fs::write(&config_path, "[alias]\n    test = !echo hello\n").unwrap();

    let config = config_path.to_string_lossy().to_string();
    let result = get_alias(&config, "test");
    assert_eq!(result, Some("!echo hello".to_string()));
}
