//! Changed files → package mapping

use std::collections::BTreeMap;

use super::WorkspaceMember;

/// Match a single file to the package it belongs to
///
/// Returns the package path (such as `"packages/core"`), or None if there is no match.
pub fn match_file_to_package(file_path: &str, members: &[WorkspaceMember]) -> Option<String> {
    for member in members {
        if member.prefix.is_empty() {
            continue;
        }
        if file_path.starts_with(&member.prefix) {
            let rest = &file_path[member.prefix.len()..];
            let is_glob = member.pattern.contains('*') || member.pattern.contains('?');

            if is_glob {
                // Glob pattern (such as packages/*): there must be subdirectories in rest
                // packages/core/src/lib.rs → rest = "core/src/lib.rs" → package "packages/core"
                // packages/README.md → rest = "README.md" → does not match (not in sub-package)
                if let Some(slash_pos) = rest.find('/') {
                    let package_dir = &rest[..slash_pos];
                    if !package_dir.is_empty() {
                        let prefix_base = member.prefix.trim_end_matches('/');
                        return Some(format!("{prefix_base}/{package_dir}"));
                    }
                }
            } else {
                // Exact path (e.g. apps/cli): the file belongs directly to this package
                let prefix_base = member.prefix.trim_end_matches('/');
                return Some(prefix_base.to_string());
            }
        }
    }
    None
}

/// Map all changed files to corresponding packages
///
/// return (package → files map, root-level files)
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
        // The file is directly under the glob prefix (such as packages/README.md), without subdirectories → does not count any packages
        let members = make_members();
        assert_eq!(match_file_to_package("packages/README.md", &members), None);
    }

    #[test]
    fn test_match_exact_path_member() {
        // Exact path member (no glob), the file belongs directly to the package
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
