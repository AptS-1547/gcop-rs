use crate::error::Result;
use crate::git::DiffStats;

/// 从 diff 文本中提取统计信息
pub fn parse_diff_stats(diff: &str) -> Result<DiffStats> {
    let mut files_changed = Vec::new();
    let mut insertions = 0;
    let mut deletions = 0;

    for line in diff.lines() {
        if line.starts_with("diff --git") {
            // 提取文件名：diff --git a/file.rs b/file.rs
            if let Some(file_part) = line.split_whitespace().nth(2) {
                // 去掉 "a/" 前缀
                if let Some(filename) = file_part.strip_prefix("a/") {
                    files_changed.push(filename.to_string());
                }
            }
        } else if line.starts_with('+') && !line.starts_with("+++") {
            insertions += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            deletions += 1;
        }
    }

    Ok(DiffStats {
        files_changed,
        insertions,
        deletions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_diff_stats() {
        let diff = r#"diff --git a/src/main.rs b/src/main.rs
index 1234567..abcdefg 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,5 @@
 fn main() {
+    println!("Hello");
+    println!("World");
-    println!("Old");
 }
"#;

        let stats = parse_diff_stats(diff).unwrap();
        assert_eq!(stats.files_changed, vec!["src/main.rs"]);
        assert_eq!(stats.insertions, 2);
        assert_eq!(stats.deletions, 1);
    }

    // === 新增边界用例 ===

    #[test]
    fn test_parse_diff_stats_empty_diff() {
        let diff = "";
        let stats = parse_diff_stats(diff).unwrap();
        assert!(stats.files_changed.is_empty());
        assert_eq!(stats.insertions, 0);
        assert_eq!(stats.deletions, 0);
    }

    #[test]
    fn test_parse_diff_stats_multiple_files() {
        let diff = r#"diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
+line1
diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
+line2
-old_line
diff --git a/Cargo.toml b/Cargo.toml
--- a/Cargo.toml
+++ b/Cargo.toml
-removed
"#;
        let stats = parse_diff_stats(diff).unwrap();
        assert_eq!(stats.files_changed.len(), 3);
        assert!(stats.files_changed.contains(&"src/main.rs".to_string()));
        assert!(stats.files_changed.contains(&"src/lib.rs".to_string()));
        assert!(stats.files_changed.contains(&"Cargo.toml".to_string()));
        assert_eq!(stats.insertions, 2);
        assert_eq!(stats.deletions, 2);
    }

    #[test]
    fn test_parse_diff_stats_only_insertions() {
        let diff = r#"diff --git a/new_file.rs b/new_file.rs
--- /dev/null
+++ b/new_file.rs
+fn new_function() {
+    println!("Hello");
+}
"#;
        let stats = parse_diff_stats(diff).unwrap();
        assert_eq!(stats.insertions, 3);
        assert_eq!(stats.deletions, 0);
    }

    #[test]
    fn test_parse_diff_stats_only_deletions() {
        let diff = r#"diff --git a/old_file.rs b/old_file.rs
--- a/old_file.rs
+++ /dev/null
-fn deleted() {
-    // gone
-}
"#;
        let stats = parse_diff_stats(diff).unwrap();
        assert_eq!(stats.insertions, 0);
        assert_eq!(stats.deletions, 3);
    }

    #[test]
    fn test_parse_diff_stats_file_with_spaces() {
        let diff = r#"diff --git a/path with spaces/file name.rs b/path with spaces/file name.rs
--- a/path with spaces/file name.rs
+++ b/path with spaces/file name.rs
+new content
"#;
        let stats = parse_diff_stats(diff).unwrap();
        // 注意：当前实现使用 split_whitespace().nth(2)，空格路径会被截断
        // 这是一个已知局限，测试验证当前行为
        assert_eq!(stats.files_changed.len(), 1);
        // 会提取 "a/path"（第三个 token）
        assert_eq!(stats.files_changed[0], "path");
        assert_eq!(stats.insertions, 1);
    }

    #[test]
    fn test_parse_diff_stats_chinese_filename() {
        let diff = r#"diff --git a/src/中文文件.rs b/src/中文文件.rs
--- a/src/中文文件.rs
+++ b/src/中文文件.rs
+println!("你好");
"#;
        let stats = parse_diff_stats(diff).unwrap();
        assert_eq!(stats.files_changed, vec!["src/中文文件.rs".to_string()]);
        assert_eq!(stats.insertions, 1);
    }

    #[test]
    fn test_parse_diff_stats_binary_file() {
        // 二进制文件 diff 格式
        let diff = r#"diff --git a/image.png b/image.png
Binary files a/image.png and b/image.png differ
"#;
        let stats = parse_diff_stats(diff).unwrap();
        assert_eq!(stats.files_changed, vec!["image.png".to_string()]);
        // 二进制文件没有 +/- 行
        assert_eq!(stats.insertions, 0);
        assert_eq!(stats.deletions, 0);
    }
}
