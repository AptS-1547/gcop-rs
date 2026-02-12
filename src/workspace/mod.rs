//! Monorepo workspace 检测与 scope 推断
//!
//! 自动检测 Cargo workspace、pnpm、npm/yarn、Lerna 等 monorepo 结构，
//! 将 changed files 映射到对应的 package，推断 commit scope。

pub mod detector;
pub mod matcher;
pub mod scope;

use std::path::PathBuf;

use serde::Serialize;

/// 检测到的 workspace 类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceType {
    Cargo,
    Pnpm,
    Npm,
    Lerna,
    Nx,
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

/// 已解析的 workspace member
#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceMember {
    /// 原始 glob pattern（如 `"packages/*"`）
    pub pattern: String,
    /// 匹配用前缀（如 `"packages/"`）
    pub prefix: String,
}

/// Workspace 检测结果
#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceInfo {
    /// 检测到的 workspace 类型
    pub workspace_types: Vec<WorkspaceType>,
    /// 解析后的 member 列表
    pub members: Vec<WorkspaceMember>,
    /// 仓库根目录
    pub root: PathBuf,
}

/// 包 scope 推断结果
#[derive(Debug, Clone, Default, Serialize)]
pub struct PackageScope {
    /// 受影响的包路径
    pub packages: Vec<String>,
    /// 不属于任何包的文件
    pub root_files: Vec<String>,
    /// 建议的 scope 字符串（None 表示无建议）
    pub suggested_scope: Option<String>,
}

/// 将 glob pattern 转换为匹配前缀
///
/// - `"packages/*"` → `"packages/"`
/// - `"crates/**"` → `"crates/"`
/// - `"apps/cli"` → `"apps/cli/"`（视为精确目录）
/// - `"*"` → `""`
pub fn glob_pattern_to_prefix(pattern: &str) -> String {
    let trimmed = pattern.trim_matches('\'').trim_matches('"');

    // 跳过否定 pattern
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
        // 无 glob 字符 → 视为精确目录
        if trimmed.ends_with('/') {
            trimmed.to_string()
        } else {
            format!("{trimmed}/")
        }
    }
}

/// 从仓库根目录检测 workspace 配置
///
/// 返回 `None` 表示不是 monorepo。
/// 检测失败时 log warning 并返回 `None`（非致命）。
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
