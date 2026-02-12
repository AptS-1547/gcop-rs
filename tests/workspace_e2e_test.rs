//! 端到端集成测试：每种 monorepo workspace 检测 + scope 推断

use gcop_rs::workspace;
use tempfile::tempdir;

fn clean(root: &std::path::Path) {
    for name in &[
        "Cargo.toml",
        "pnpm-workspace.yaml",
        "package.json",
        "lerna.json",
        "nx.json",
        "turbo.json",
    ] {
        let _ = std::fs::remove_file(root.join(name));
    }
}

// === 1. Cargo workspace ===

#[test]
fn test_e2e_cargo_workspace() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    clean(root);

    std::fs::write(
        root.join("Cargo.toml"),
        r#"
[workspace]
members = ["crates/*", "apps/cli"]
"#,
    )
    .unwrap();

    let info = workspace::detect_workspace(root).expect("should detect cargo workspace");
    assert!(
        info.workspace_types
            .iter()
            .any(|t| format!("{t}") == "cargo")
    );
    assert_eq!(info.members.len(), 2);

    // 单包 scope
    let files = vec![
        "crates/core/src/lib.rs".into(),
        "crates/core/Cargo.toml".into(),
    ];
    let scope = workspace::scope::infer_scope(&files, &info, None);
    assert_eq!(scope.suggested_scope, Some("core".into()));
    assert_eq!(scope.packages.len(), 1);
    assert!(scope.root_files.is_empty());

    // 精确路径 member (apps/cli)
    let files2 = vec!["apps/cli/src/main.rs".into()];
    let scope2 = workspace::scope::infer_scope(&files2, &info, None);
    assert_eq!(scope2.suggested_scope, Some("cli".into()));
}

#[test]
fn test_e2e_cargo_no_workspace() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    clean(root);

    std::fs::write(
        root.join("Cargo.toml"),
        r#"
[package]
name = "simple-app"
version = "0.1.0"
"#,
    )
    .unwrap();

    let result = workspace::detect_workspace(root);
    assert!(
        result.is_none(),
        "non-workspace Cargo.toml should return None"
    );
}

// === 2. Pnpm workspace ===

#[test]
fn test_e2e_pnpm_workspace() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    clean(root);

    std::fs::write(
        root.join("pnpm-workspace.yaml"),
        "packages:\n  - 'packages/*'\n  - 'apps/*'\n",
    )
    .unwrap();

    let info = workspace::detect_workspace(root).expect("should detect pnpm workspace");
    assert!(
        info.workspace_types
            .iter()
            .any(|t| format!("{t}") == "pnpm")
    );
    assert_eq!(info.members.len(), 2);

    // 双包 scope
    let files = vec![
        "packages/ui/src/button.tsx".into(),
        "apps/web/index.ts".into(),
    ];
    let scope = workspace::scope::infer_scope(&files, &info, None);
    assert!(scope.suggested_scope.is_some());
    assert_eq!(scope.packages.len(), 2);
}

// === 3. Npm workspace (array 格式) ===

#[test]
fn test_e2e_npm_workspace_array() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    clean(root);

    std::fs::write(
        root.join("package.json"),
        r#"{"workspaces": ["packages/*"]}"#,
    )
    .unwrap();

    let info = workspace::detect_workspace(root).expect("should detect npm workspace");
    assert!(info.workspace_types.iter().any(|t| format!("{t}") == "npm"));

    let files = vec!["packages/utils/index.ts".into()];
    let scope = workspace::scope::infer_scope(&files, &info, None);
    assert_eq!(scope.suggested_scope, Some("utils".into()));
}

// === 4. Npm workspace (yarn-style object 格式) ===

#[test]
fn test_e2e_npm_workspace_yarn_style() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    clean(root);

    std::fs::write(
        root.join("package.json"),
        r#"{"workspaces": {"packages": ["packages/*", "libs/*"]}}"#,
    )
    .unwrap();

    let info = workspace::detect_workspace(root).expect("should detect yarn-style workspace");
    assert!(info.workspace_types.iter().any(|t| format!("{t}") == "npm"));
    assert_eq!(info.members.len(), 2);

    // 跨两个 workspace group
    let files = vec![
        "packages/core/src/index.ts".into(),
        "libs/shared/util.ts".into(),
    ];
    let scope = workspace::scope::infer_scope(&files, &info, None);
    assert!(scope.suggested_scope.is_some());
    assert_eq!(scope.packages.len(), 2);
}

// === 5. Npm + Nx ===

#[test]
fn test_e2e_npm_with_nx() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    clean(root);

    std::fs::write(
        root.join("package.json"),
        r#"{"workspaces": ["packages/*"]}"#,
    )
    .unwrap();
    std::fs::write(root.join("nx.json"), "{}").unwrap();

    let info = workspace::detect_workspace(root).expect("should detect npm+nx");
    assert!(info.workspace_types.iter().any(|t| format!("{t}") == "npm"));
    assert!(info.workspace_types.iter().any(|t| format!("{t}") == "nx"));

    let files = vec!["packages/feature-a/src/lib.ts".into()];
    let scope = workspace::scope::infer_scope(&files, &info, None);
    assert_eq!(scope.suggested_scope, Some("feature-a".into()));
}

// === 6. Npm + Turbo ===

#[test]
fn test_e2e_npm_with_turbo() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    clean(root);

    std::fs::write(
        root.join("package.json"),
        r#"{"workspaces": ["packages/*", "apps/*"]}"#,
    )
    .unwrap();
    std::fs::write(root.join("turbo.json"), r#"{"pipeline": {}}"#).unwrap();

    let info = workspace::detect_workspace(root).expect("should detect npm+turbo");
    assert!(info.workspace_types.iter().any(|t| format!("{t}") == "npm"));
    assert!(
        info.workspace_types
            .iter()
            .any(|t| format!("{t}") == "turbo")
    );

    // 三包 scope
    let files = vec![
        "packages/ui/button.tsx".into(),
        "packages/utils/index.ts".into(),
        "apps/web/app.tsx".into(),
    ];
    let scope = workspace::scope::infer_scope(&files, &info, None);
    assert!(scope.suggested_scope.is_some());
    assert_eq!(scope.packages.len(), 3);
}

// === 7. Lerna ===

#[test]
fn test_e2e_lerna_workspace() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    clean(root);

    std::fs::write(
        root.join("lerna.json"),
        r#"{"packages": ["packages/*", "modules/*"]}"#,
    )
    .unwrap();

    let info = workspace::detect_workspace(root).expect("should detect lerna");
    assert!(
        info.workspace_types
            .iter()
            .any(|t| format!("{t}") == "lerna")
    );
    assert_eq!(info.members.len(), 2);

    // 三包 → 逗号分隔 scope
    let files = vec![
        "packages/a/index.js".into(),
        "packages/b/index.js".into(),
        "modules/c/index.js".into(),
    ];
    let scope = workspace::scope::infer_scope(&files, &info, None);
    assert!(scope.suggested_scope.is_some());
    assert_eq!(scope.packages.len(), 3);
}

// === 8. 混合 workspace (Cargo + Pnpm) ===

#[test]
fn test_e2e_mixed_cargo_and_pnpm() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    clean(root);

    std::fs::write(
        root.join("Cargo.toml"),
        r#"
[workspace]
members = ["crates/*"]
"#,
    )
    .unwrap();
    std::fs::write(
        root.join("pnpm-workspace.yaml"),
        "packages:\n  - 'packages/*'\n",
    )
    .unwrap();

    let info = workspace::detect_workspace(root).expect("should detect mixed workspace");
    assert!(info.workspace_types.len() >= 2);

    // cargo 子包
    let files1 = vec!["crates/parser/src/lib.rs".into()];
    let scope1 = workspace::scope::infer_scope(&files1, &info, None);
    assert_eq!(scope1.suggested_scope, Some("parser".into()));

    // pnpm 子包
    let files2 = vec!["packages/ui/button.tsx".into()];
    let scope2 = workspace::scope::infer_scope(&files2, &info, None);
    assert_eq!(scope2.suggested_scope, Some("ui".into()));

    // 跨 workspace 类型
    let files3 = vec![
        "crates/parser/src/lib.rs".into(),
        "packages/ui/button.tsx".into(),
    ];
    let scope3 = workspace::scope::infer_scope(&files3, &info, None);
    assert!(scope3.suggested_scope.is_some());
    assert_eq!(scope3.packages.len(), 2);
}

// === 9. 无 workspace ===

#[test]
fn test_e2e_no_workspace() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    clean(root);

    let result = workspace::detect_workspace(root);
    assert!(result.is_none(), "empty dir should return None");
}

// === 边界场景 ===

#[test]
fn test_e2e_root_files_no_scope() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    clean(root);

    std::fs::write(
        root.join("Cargo.toml"),
        r#"
[workspace]
members = ["crates/*"]
"#,
    )
    .unwrap();

    let info = workspace::detect_workspace(root).expect("should detect cargo workspace");

    // 只有 root 文件
    let files = vec!["README.md".into(), "Cargo.toml".into()];
    let scope = workspace::scope::infer_scope(&files, &info, None);
    assert!(scope.suggested_scope.is_none());
    assert!(scope.packages.is_empty());
    assert_eq!(scope.root_files.len(), 2);
}

#[test]
fn test_e2e_four_plus_packages_no_scope() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    clean(root);

    std::fs::write(
        root.join("package.json"),
        r#"{"workspaces": ["packages/*"]}"#,
    )
    .unwrap();

    let info = workspace::detect_workspace(root).expect("should detect npm workspace");

    // 4+ 包 → scope 为 None
    let files = vec![
        "packages/a/index.ts".into(),
        "packages/b/index.ts".into(),
        "packages/c/index.ts".into(),
        "packages/d/index.ts".into(),
    ];
    let scope = workspace::scope::infer_scope(&files, &info, None);
    assert!(scope.suggested_scope.is_none());
    assert_eq!(scope.packages.len(), 4);
}

#[test]
fn test_e2e_manual_scope_override() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    clean(root);

    std::fs::write(
        root.join("Cargo.toml"),
        r#"
[workspace]
members = ["crates/*"]
"#,
    )
    .unwrap();

    let info = workspace::detect_workspace(root).expect("should detect cargo workspace");

    let files = vec!["crates/core/src/lib.rs".into()];
    let scope = workspace::scope::infer_scope(&files, &info, Some("my-custom-scope"));
    assert_eq!(scope.suggested_scope, Some("my-custom-scope".into()));
}

#[test]
fn test_e2e_dedup_across_workspace_types() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    clean(root);

    // pnpm 和 npm 都声明了 packages/*
    std::fs::write(
        root.join("pnpm-workspace.yaml"),
        "packages:\n  - 'packages/*'\n",
    )
    .unwrap();
    std::fs::write(
        root.join("package.json"),
        r#"{"workspaces": ["packages/*"]}"#,
    )
    .unwrap();

    let info = workspace::detect_workspace(root).expect("should detect both");

    // prefix 应去重
    let pkg_count = info
        .members
        .iter()
        .filter(|m| m.prefix == "packages/")
        .count();
    assert_eq!(
        pkg_count, 1,
        "packages/ should appear only once after dedup"
    );
}
