//! Monorepo workspace detection and scope inference
//!
//! Automatically detect monorepo structures such as Cargo workspace, pnpm, npm/yarn, Lerna, etc.
//! Map changed files to corresponding packages and infer commit scope.

pub mod detector;
pub mod matcher;
pub mod scope;

use std::path::PathBuf;

use serde::Serialize;

/// Detected workspace type
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceType {
    /// Rust Cargo workspace (`Cargo.toml [workspace]`).
    Cargo,
    /// pnpm workspace (`pnpm-workspace.yaml`).
    Pnpm,
    /// npm/yarn workspaces (`package.json#workspaces`).
    Npm,
    /// Lerna monorepo (`lerna.json`).
    Lerna,
    /// Nx workspace (`nx.json`).
    Nx,
    /// Turborepo workspace (`turbo.json`).
    Turbo,
}

impl std::fmt::Display for WorkspaceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cargo => write!(f, "cargo"),
            Self::Pnpm => write!(f, "pnpm"),
            Self::Npm => write!(f, "npm"),
            Self::Lerna => write!(f, "lerna"),
            Self::Nx => write!(f, "nx"),
            Self::Turbo => write!(f, "turbo"),
        }
    }
}

/// Resolved workspace member
#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceMember {
    /// Raw glob pattern (e.g. `"packages/*"`)
    pub pattern: String,
    /// Match with prefix (such as `"packages/"`)
    pub prefix: String,
}

/// Workspace detection result.
#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceInfo {
    /// Detected workspace type
    pub workspace_types: Vec<WorkspaceType>,
    /// Parsed member list
    pub members: Vec<WorkspaceMember>,
    /// Repository root directory.
    pub root: PathBuf,
}

/// Package scope inference results
#[derive(Debug, Clone, Default, Serialize)]
pub struct PackageScope {
    /// Affected package paths
    pub packages: Vec<String>,
    /// File that does not belong to any package
    pub root_files: Vec<String>,
    /// Suggested scope string (None means no suggestion)
    pub suggested_scope: Option<String>,
}

/// Convert glob pattern to matching prefix
///
/// - `"packages/*"` → `"packages/"`
/// - `"crates/**"` → `"crates/"`
/// - `"apps/cli"` → `"apps/cli/"` (treated as the exact directory)
/// - `"*"` → `""`
pub fn glob_pattern_to_prefix(pattern: &str) -> String {
    let trimmed = pattern.trim_matches('\'').trim_matches('"');

    // skip negation pattern
    if trimmed.starts_with('!') {
        return String::new();
    }

    if let Some(pos) = trimmed.find(['*', '?', '{']) {
        let prefix = &trimmed[..pos];
        if prefix.is_empty() {
            return String::new();
        }
        if prefix.ends_with('/') {
            prefix.to_string()
        } else {
            format!("{prefix}/")
        }
    } else {
        // no glob characters → treat as exact directory
        if trimmed.ends_with('/') {
            trimmed.to_string()
        } else {
            format!("{trimmed}/")
        }
    }
}

/// Detect workspace configuration from repository root directory
///
/// Returns `None` to indicate it is not a monorepo.
/// Log warning and return `None` (non-fatal) when detection fails.
pub fn detect_workspace(root: &std::path::Path) -> Option<WorkspaceInfo> {
    match detector::detect_workspace(root) {
        Ok(info) => info,
        Err(e) => {
            tracing::warn!(
                "{}",
                rust_i18n::t!("workspace.detection_failed", error = e.to_string())
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_pattern_to_prefix_star() {
        assert_eq!(glob_pattern_to_prefix("packages/*"), "packages/");
    }

    #[test]
    fn test_glob_pattern_to_prefix_double_star() {
        assert_eq!(glob_pattern_to_prefix("crates/**"), "crates/");
    }

    #[test]
    fn test_glob_pattern_to_prefix_exact() {
        assert_eq!(glob_pattern_to_prefix("apps/cli"), "apps/cli/");
    }

    #[test]
    fn test_glob_pattern_to_prefix_quoted() {
        assert_eq!(glob_pattern_to_prefix("'packages/*'"), "packages/");
    }

    #[test]
    fn test_glob_pattern_to_prefix_bare_star() {
        assert_eq!(glob_pattern_to_prefix("*"), "");
    }

    #[test]
    fn test_glob_pattern_to_prefix_negation() {
        assert_eq!(glob_pattern_to_prefix("!**/test/**"), "");
    }

    #[test]
    fn test_glob_pattern_to_prefix_trailing_slash() {
        assert_eq!(glob_pattern_to_prefix("apps/"), "apps/");
    }
}
