//! Commit scope inference

use super::matcher::map_files_to_packages;
use super::{PackageScope, WorkspaceInfo};

/// Infer commit scope from changed files and workspace information
///
/// rule:
/// - Manual scope priority
/// - 1 package → scope = package short name (last segment of path)
/// - 2-3 packages → scope = comma separated short names
/// - 4+ packages or root files only → None
pub fn infer_scope(
    files_changed: &[String],
    workspace: &WorkspaceInfo,
    manual_scope: Option<&str>,
) -> PackageScope {
    if let Some(scope) = manual_scope {
        return PackageScope {
            packages: vec![],
            root_files: vec![],
            suggested_scope: Some(scope.to_string()),
        };
    }

    let (package_files, root_files) = map_files_to_packages(files_changed, &workspace.members);
    let packages: Vec<String> = package_files.keys().cloned().collect();

    let suggested_scope = match packages.len() {
        0 => None,
        1 => {
            let pkg = &packages[0];
            let short_name = pkg.rsplit('/').next().unwrap_or(pkg);
            Some(short_name.to_string())
        }
        2..=3 => {
            let short_names: Vec<&str> = packages
                .iter()
                .map(|p| p.rsplit('/').next().unwrap_or(p.as_str()))
                .collect();
            Some(short_names.join(","))
        }
        _ => None,
    };

    PackageScope {
        packages,
        root_files,
        suggested_scope,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{WorkspaceInfo, WorkspaceMember, WorkspaceType};
    use std::path::PathBuf;

    fn make_workspace() -> WorkspaceInfo {
        WorkspaceInfo {
            workspace_types: vec![WorkspaceType::Cargo],
            members: vec![
                WorkspaceMember {
                    pattern: "crates/*".into(),
                    prefix: "crates/".into(),
                },
                WorkspaceMember {
                    pattern: "apps/*".into(),
                    prefix: "apps/".into(),
                },
            ],
            root: PathBuf::from("/tmp/test"),
        }
    }

    #[test]
    fn test_single_package_scope() {
        let ws = make_workspace();
        let files = vec![
            "crates/core/src/lib.rs".into(),
            "crates/core/Cargo.toml".into(),
        ];
        let scope = infer_scope(&files, &ws, None);

        assert_eq!(scope.suggested_scope, Some("core".to_string()));
        assert_eq!(scope.packages.len(), 1);
        assert!(scope.root_files.is_empty());
    }

    #[test]
    fn test_two_packages_scope() {
        let ws = make_workspace();
        let files = vec!["crates/core/src/lib.rs".into(), "apps/cli/main.rs".into()];
        let scope = infer_scope(&files, &ws, None);

        assert_eq!(scope.suggested_scope, Some("cli,core".to_string()));
        assert_eq!(scope.packages.len(), 2);
    }

    #[test]
    fn test_many_packages_no_scope() {
        let ws = WorkspaceInfo {
            workspace_types: vec![WorkspaceType::Npm],
            members: vec![WorkspaceMember {
                pattern: "packages/*".into(),
                prefix: "packages/".into(),
            }],
            root: PathBuf::from("/tmp/test"),
        };
        let files = vec![
            "packages/a/index.ts".into(),
            "packages/b/index.ts".into(),
            "packages/c/index.ts".into(),
            "packages/d/index.ts".into(),
        ];
        let scope = infer_scope(&files, &ws, None);

        assert!(scope.suggested_scope.is_none());
        assert_eq!(scope.packages.len(), 4);
    }

    #[test]
    fn test_root_only_no_scope() {
        let ws = make_workspace();
        let files = vec!["README.md".into(), "Cargo.toml".into()];
        let scope = infer_scope(&files, &ws, None);

        assert!(scope.suggested_scope.is_none());
        assert!(scope.packages.is_empty());
        assert_eq!(scope.root_files.len(), 2);
    }

    #[test]
    fn test_manual_scope_override() {
        let ws = make_workspace();
        let files = vec!["crates/core/src/lib.rs".into()];
        let scope = infer_scope(&files, &ws, Some("my-scope"));

        assert_eq!(scope.suggested_scope, Some("my-scope".to_string()));
    }

    #[test]
    fn test_mixed_package_and_root() {
        let ws = make_workspace();
        let files = vec!["crates/core/src/lib.rs".into(), "README.md".into()];
        let scope = infer_scope(&files, &ws, None);

        assert_eq!(scope.suggested_scope, Some("core".to_string()));
        assert_eq!(scope.packages.len(), 1);
        assert_eq!(scope.root_files.len(), 1);
    }
}
