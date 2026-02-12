//! Changed files → package 映射

use std::collections::BTreeMap;

use super::WorkspaceMember;

/// 将单个文件匹配到所属 package
///
/// 返回包路径（如 `"packages/core"`），不匹配则返回 None。
pub fn match_file_to_package(file_path: &str, members: &[WorkspaceMember]) -> Option<String> {
    for member in members {
        if member.prefix.is_empty() {
            continue;
        }
        if file_path.starts_with(&member.prefix) {
            let rest = &file_path[member.prefix.len()..];
            let is_glob = member.pattern.contains('*') || member.pattern.contains('?');

            if is_glob {
                // Glob pattern（如 packages/*）: rest 中必须有子目录
                // packages/core/src/lib.rs → rest = "core/src/lib.rs" → 包 "packages/core"
                // packages/README.md → rest = "README.md" → 不匹配（不在子包里）
                if let Some(slash_pos) = rest.find('/') {
                    let package_dir = &rest[..slash_pos];
                    if !package_dir.is_empty() {
                        let prefix_base = member.prefix.trim_end_matches('/');
                        return Some(format!("{prefix_base}/{package_dir}"));
                    }
                }
            } else {
                // 精确路径（如 apps/cli）: 文件直接属于此包
                let prefix_base = member.prefix.trim_end_matches('/');
                return Some(prefix_base.to_string());
            }
        }
    }
    None
}

/// 将所有 changed files 映射到对应的 package
///
/// 返回 (package → files 映射, root-level files)
pub fn map_files_to_packages(
    files: &[String],
    members: &[WorkspaceMember],
) -> (BTreeMap<String, Vec<String>>, Vec<String>) {
    let mut package_files: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut root_files = Vec::new();

    for file in files {
        match match_file_to_package(file, members) {
            Some(pkg) => package_files.entry(pkg).or_default().push(file.clone()),
            None => root_files.push(file.clone()),
        }
    }

    (package_files, root_files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::WorkspaceMember;

    fn make_members() -> Vec<WorkspaceMember> {
        vec![
            WorkspaceMember {
                pattern: "packages/*".into(),
                prefix: "packages/".into(),
            },
            WorkspaceMember {
                pattern: "apps/*".into(),
                prefix: "apps/".into(),
            },
        ]
    }

    #[test]
    fn test_match_package_file() {
        let members = make_members();
        assert_eq!(
            match_file_to_package("packages/core/src/lib.rs", &members),
            Some("packages/core".to_string())
        );
    }

    #[test]
    fn test_match_apps_file() {
        let members = make_members();
        assert_eq!(
            match_file_to_package("apps/cli/main.rs", &members),
            Some("apps/cli".to_string())
        );
    }

    #[test]
    fn test_match_root_file() {
        let members = make_members();
        assert_eq!(match_file_to_package("README.md", &members), None);
        assert_eq!(match_file_to_package("Cargo.toml", &members), None);
    }

    #[test]
    fn test_match_nested_file() {
        let members = make_members();
        assert_eq!(
            match_file_to_package("packages/core/src/nested/deep.rs", &members),
            Some("packages/core".to_string())
        );
    }

    #[test]
    fn test_match_file_directly_in_prefix() {
        // 文件直接在 glob prefix 下（如 packages/README.md），没有子目录 → 不算任何包
        let members = make_members();
        assert_eq!(match_file_to_package("packages/README.md", &members), None);
    }

    #[test]
    fn test_match_exact_path_member() {
        // 精确路径成员（无 glob），文件直接属于该包
        let members = vec![WorkspaceMember {
            pattern: "apps/cli".into(),
            prefix: "apps/cli/".into(),
        }];
        assert_eq!(
            match_file_to_package("apps/cli/main.rs", &members),
            Some("apps/cli".to_string())
        );
    }

    #[test]
    fn test_map_files_to_packages() {
        let members = make_members();
        let files = vec![
            "packages/core/src/lib.rs".to_string(),
            "packages/core/Cargo.toml".to_string(),
            "apps/cli/main.rs".to_string(),
            "README.md".to_string(),
        ];
        let (pkg_map, root) = map_files_to_packages(&files, &members);

        assert_eq!(pkg_map.len(), 2);
        assert_eq!(pkg_map["packages/core"].len(), 2);
        assert_eq!(pkg_map["apps/cli"].len(), 1);
        assert_eq!(root, vec!["README.md"]);
    }

    #[test]
    fn test_map_single_package() {
        let members = make_members();
        let files = vec![
            "packages/ui/src/button.tsx".to_string(),
            "packages/ui/src/input.tsx".to_string(),
        ];
        let (pkg_map, root) = map_files_to_packages(&files, &members);

        assert_eq!(pkg_map.len(), 1);
        assert!(root.is_empty());
    }

    #[test]
    fn test_map_all_root_files() {
        let members = make_members();
        let files = vec![
            "README.md".to_string(),
            ".gitignore".to_string(),
            "Cargo.toml".to_string(),
        ];
        let (pkg_map, root) = map_files_to_packages(&files, &members);

        assert!(pkg_map.is_empty());
        assert_eq!(root.len(), 3);
    }
}
