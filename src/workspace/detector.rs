//! Workspace 配置文件检测与解析

use std::path::Path;

use crate::error::Result;

use super::{WorkspaceInfo, WorkspaceMember, WorkspaceType, glob_pattern_to_prefix};

/// 检测 workspace 配置，返回 None 表示不是 monorepo
pub fn detect_workspace(root: &Path) -> Result<Option<WorkspaceInfo>> {
    let mut workspace_types = Vec::new();
    let mut members = Vec::new();

    // Cargo.toml [workspace]
    if let Some(cargo_members) = detect_cargo_workspace(root)? {
        workspace_types.push(WorkspaceType::Cargo);
        members.extend(cargo_members);
    }

    // pnpm-workspace.yaml
    if let Some(pnpm_members) = detect_pnpm_workspace(root)? {
        workspace_types.push(WorkspaceType::Pnpm);
        members.extend(pnpm_members);
    }

    // package.json workspaces + nx.json / turbo.json 检测
    if let Some((npm_members, extra_type)) = detect_npm_workspace(root)? {
        workspace_types.push(WorkspaceType::Npm);
        if let Some(t) = extra_type {
            workspace_types.push(t);
        }
        members.extend(npm_members);
    }

    // lerna.json
    if let Some(lerna_members) = detect_lerna_workspace(root)? {
        if !workspace_types
            .iter()
            .any(|t| matches!(t, WorkspaceType::Lerna))
        {
            workspace_types.push(WorkspaceType::Lerna);
        }
        members.extend(lerna_members);
    }

    if workspace_types.is_empty() {
        return Ok(None);
    }

    // 去重：按 prefix 排序后去重
    members.sort_by(|a, b| a.prefix.cmp(&b.prefix));
    members.dedup_by(|a, b| a.prefix == b.prefix);

    // 移除空 prefix
    members.retain(|m| !m.prefix.is_empty());

    Ok(Some(WorkspaceInfo {
        workspace_types,
        members,
        root: root.to_path_buf(),
    }))
}

/// 检测 Cargo.toml [workspace] members
fn detect_cargo_workspace(root: &Path) -> Result<Option<Vec<WorkspaceMember>>> {
    let cargo_path = root.join("Cargo.toml");
    if !cargo_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&cargo_path)?;
    let value: toml::Value = match toml::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Failed to parse Cargo.toml: {}", e);
            return Ok(None);
        }
    };

    let workspace = match value.get("workspace") {
        Some(w) => w,
        None => return Ok(None),
    };

    let members_array = match workspace.get("members").and_then(|m| m.as_array()) {
        Some(arr) => arr,
        None => return Ok(None),
    };

    let members: Vec<WorkspaceMember> = members_array
        .iter()
        .filter_map(|v| v.as_str())
        .map(|pattern| WorkspaceMember {
            prefix: glob_pattern_to_prefix(pattern),
            pattern: pattern.to_string(),
        })
        .collect();

    if members.is_empty() {
        return Ok(None);
    }

    Ok(Some(members))
}

/// 检测 pnpm-workspace.yaml
fn detect_pnpm_workspace(root: &Path) -> Result<Option<Vec<WorkspaceMember>>> {
    let yaml_path = root.join("pnpm-workspace.yaml");
    if !yaml_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&yaml_path)?;

    #[derive(serde::Deserialize)]
    struct PnpmWorkspace {
        packages: Option<Vec<String>>,
    }

    let parsed: PnpmWorkspace = match serde_yml::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Failed to parse pnpm-workspace.yaml: {}", e);
            return Ok(None);
        }
    };

    match parsed.packages {
        Some(patterns) => {
            let members: Vec<WorkspaceMember> = patterns
                .iter()
                .map(|p| WorkspaceMember {
                    prefix: glob_pattern_to_prefix(p),
                    pattern: p.clone(),
                })
                .collect();
            if members.is_empty() {
                Ok(None)
            } else {
                Ok(Some(members))
            }
        }
        None => Ok(None),
    }
}

/// 检测 package.json workspaces，同时检测 nx.json / turbo.json
fn detect_npm_workspace(
    root: &Path,
) -> Result<Option<(Vec<WorkspaceMember>, Option<WorkspaceType>)>> {
    let pkg_path = root.join("package.json");
    if !pkg_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&pkg_path)?;
    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Failed to parse package.json: {}", e);
            return Ok(None);
        }
    };

    // workspaces 可以是数组或 { packages: [...] }（yarn 风格）
    let workspace_patterns = match value.get("workspaces") {
        Some(serde_json::Value::Array(arr)) => arr.clone(),
        Some(serde_json::Value::Object(obj)) => {
            match obj.get("packages").and_then(|p| p.as_array()) {
                Some(arr) => arr.clone(),
                None => return Ok(None),
            }
        }
        _ => return Ok(None),
    };

    let members: Vec<WorkspaceMember> = workspace_patterns
        .iter()
        .filter_map(|v| v.as_str())
        .map(|p| WorkspaceMember {
            prefix: glob_pattern_to_prefix(p),
            pattern: p.to_string(),
        })
        .collect();

    if members.is_empty() {
        return Ok(None);
    }

    // 检测 nx / turbo
    let extra_type = if root.join("nx.json").exists() {
        Some(WorkspaceType::Nx)
    } else if root.join("turbo.json").exists() {
        Some(WorkspaceType::Turbo)
    } else {
        None
    };

    Ok(Some((members, extra_type)))
}

/// 检测 lerna.json packages
fn detect_lerna_workspace(root: &Path) -> Result<Option<Vec<WorkspaceMember>>> {
    let lerna_path = root.join("lerna.json");
    if !lerna_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&lerna_path)?;
    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Failed to parse lerna.json: {}", e);
            return Ok(None);
        }
    };

    let packages = match value.get("packages").and_then(|p| p.as_array()) {
        Some(arr) => arr,
        None => return Ok(None),
    };

    let members: Vec<WorkspaceMember> = packages
        .iter()
        .filter_map(|v| v.as_str())
        .map(|p| WorkspaceMember {
            prefix: glob_pattern_to_prefix(p),
            pattern: p.to_string(),
        })
        .collect();

    if members.is_empty() {
        Ok(None)
    } else {
        Ok(Some(members))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_detect_cargo_workspace() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"
[workspace]
members = ["crates/*", "apps/cli"]
"#,
        )
        .unwrap();

        let result = detect_cargo_workspace(dir.path()).unwrap().unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].prefix, "crates/");
        assert_eq!(result[1].prefix, "apps/cli/");
    }

    #[test]
    fn test_detect_cargo_no_workspace() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"
[package]
name = "my-app"
version = "0.1.0"
"#,
        )
        .unwrap();

        let result = detect_cargo_workspace(dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_pnpm_workspace() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("pnpm-workspace.yaml"),
            "packages:\n  - 'packages/*'\n  - 'apps/*'\n",
        )
        .unwrap();

        let result = detect_pnpm_workspace(dir.path()).unwrap().unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].prefix, "packages/");
        assert_eq!(result[1].prefix, "apps/");
    }

    #[test]
    fn test_detect_npm_workspaces_array() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"workspaces": ["packages/*", "apps/*"]}"#,
        )
        .unwrap();

        let (members, extra) = detect_npm_workspace(dir.path()).unwrap().unwrap();
        assert_eq!(members.len(), 2);
        assert!(extra.is_none());
    }

    #[test]
    fn test_detect_npm_workspaces_yarn_style() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"workspaces": {"packages": ["packages/*"]}}"#,
        )
        .unwrap();

        let (members, _) = detect_npm_workspace(dir.path()).unwrap().unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].prefix, "packages/");
    }

    #[test]
    fn test_detect_npm_with_nx() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"workspaces": ["packages/*"]}"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("nx.json"), "{}").unwrap();

        let (_, extra) = detect_npm_workspace(dir.path()).unwrap().unwrap();
        assert_eq!(extra, Some(WorkspaceType::Nx));
    }

    #[test]
    fn test_detect_npm_with_turbo() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"workspaces": ["packages/*"]}"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("turbo.json"), "{}").unwrap();

        let (_, extra) = detect_npm_workspace(dir.path()).unwrap().unwrap();
        assert_eq!(extra, Some(WorkspaceType::Turbo));
    }

    #[test]
    fn test_detect_lerna_workspace() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("lerna.json"),
            r#"{"packages": ["packages/*"]}"#,
        )
        .unwrap();

        let result = detect_lerna_workspace(dir.path()).unwrap().unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].prefix, "packages/");
    }

    #[test]
    fn test_detect_no_workspace() {
        let dir = tempdir().unwrap();
        let result = detect_workspace(dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_deduplicates_members() {
        let dir = tempdir().unwrap();
        // pnpm 和 npm 都定义了 packages/*
        std::fs::write(
            dir.path().join("pnpm-workspace.yaml"),
            "packages:\n  - 'packages/*'\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"workspaces": ["packages/*"]}"#,
        )
        .unwrap();

        let info = detect_workspace(dir.path()).unwrap().unwrap();
        // prefix "packages/" 应只出现一次
        let count = info
            .members
            .iter()
            .filter(|m| m.prefix == "packages/")
            .count();
        assert_eq!(count, 1);
    }
}
