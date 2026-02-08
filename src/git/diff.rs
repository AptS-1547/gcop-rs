use crate::error::Result;
use crate::git::DiffStats;

/// 单个文件的 diff 信息
#[derive(Debug, Clone)]
pub struct FileDiff {
    /// 文件名（相对于仓库根目录）
    pub filename: String,
    /// 该文件的完整 diff patch（从 "diff --git" 到下一个文件边界）
    pub content: String,
    /// 新增行数
    pub insertions: usize,
    /// 删除行数
    pub deletions: usize,
}

fn extract_filename_from_diff_header(line: &str) -> Option<String> {
    const PREFIX: &str = "diff --git ";
    if !line.starts_with(PREFIX) {
        return None;
    }

    let rest = &line[PREFIX.len()..];

    // 通过 " b/" 分隔符定位 a/ 和 b/ 的边界，避免空格路径被截断。
    if let Some(b_pos) = rest.find(" b/") {
        return rest[..b_pos]
            .strip_prefix("a/")
            .map(|filename| filename.to_string());
    }

    // 处理带引号的路径：diff --git "a/path with spaces.rs" "b/path with spaces.rs"
    if let Some(stripped) = rest.strip_prefix('"')
        && let Some(end) = stripped.find('"')
    {
        return stripped[..end]
            .strip_prefix("a/")
            .map(|filename| filename.to_string());
    }

    // Fallback：保持兼容性
    rest.split_whitespace()
        .next()
        .and_then(|s| s.strip_prefix("a/"))
        .map(|s| s.to_string())
}

/// 从 diff 文本中提取统计信息
pub fn parse_diff_stats(diff: &str) -> Result<DiffStats> {
    let mut files_changed = Vec::new();
    let mut insertions = 0;
    let mut deletions = 0;

    for line in diff.lines() {
        if line.starts_with("diff --git") {
            if let Some(filename) = extract_filename_from_diff_header(line) {
                files_changed.push(filename);
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

/// 将原始 diff 文本按文件边界拆分为 `Vec<FileDiff>`
///
/// 每个 `FileDiff` 包含一个文件的完整 diff patch 及其统计信息。
/// 保持原始文件顺序。
pub fn split_diff_by_file(diff: &str) -> Vec<FileDiff> {
    if diff.is_empty() {
        return Vec::new();
    }

    let mut files: Vec<FileDiff> = Vec::new();
    let mut current_filename: Option<String> = None;
    let mut current_lines: Vec<&str> = Vec::new();
    let mut current_insertions = 0usize;
    let mut current_deletions = 0usize;

    for line in diff.lines() {
        if line.starts_with("diff --git") {
            // 遇到新文件边界，保存上一个文件
            if let Some(filename) = current_filename.take() {
                let content = current_lines.join("\n");
                files.push(FileDiff {
                    filename,
                    content,
                    insertions: current_insertions,
                    deletions: current_deletions,
                });
                current_lines.clear();
                current_insertions = 0;
                current_deletions = 0;
            }
            current_filename = extract_filename_from_diff_header(line);
            current_lines.push(line);
        } else {
            if current_filename.is_some() {
                if line.starts_with('+') && !line.starts_with("+++") {
                    current_insertions += 1;
                } else if line.starts_with('-') && !line.starts_with("---") {
                    current_deletions += 1;
                }
            }
            current_lines.push(line);
        }
    }

    // 保存最后一个文件
    if let Some(filename) = current_filename {
        let content = current_lines.join("\n");
        files.push(FileDiff {
            filename,
            content,
            insertions: current_insertions,
            deletions: current_deletions,
        });
    }

    files
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
        assert_eq!(stats.files_changed.len(), 1);
        assert_eq!(stats.files_changed[0], "path with spaces/file name.rs");
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

    // === split_diff_by_file 测试 ===

    #[test]
    fn test_split_diff_by_file_empty() {
        let files = split_diff_by_file("");
        assert!(files.is_empty());
    }

    #[test]
    fn test_split_diff_by_file_single() {
        let diff = "diff --git a/src/main.rs b/src/main.rs\n\
                     index 1234567..abcdefg 100644\n\
                     --- a/src/main.rs\n\
                     +++ b/src/main.rs\n\
                     @@ -1,3 +1,5 @@\n\
                     +line1\n\
                     +line2\n\
                     -old_line";
        let files = split_diff_by_file(diff);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "src/main.rs");
        assert_eq!(files[0].insertions, 2);
        assert_eq!(files[0].deletions, 1);
        assert!(files[0].content.starts_with("diff --git"));
    }

    #[test]
    fn test_split_diff_by_file_multiple() {
        let diff = "diff --git a/src/main.rs b/src/main.rs\n\
                     --- a/src/main.rs\n\
                     +++ b/src/main.rs\n\
                     +line1\n\
                     diff --git a/src/lib.rs b/src/lib.rs\n\
                     --- a/src/lib.rs\n\
                     +++ b/src/lib.rs\n\
                     +line2\n\
                     -old_line\n\
                     diff --git a/Cargo.toml b/Cargo.toml\n\
                     --- a/Cargo.toml\n\
                     +++ b/Cargo.toml\n\
                     -removed";
        let files = split_diff_by_file(diff);
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].filename, "src/main.rs");
        assert_eq!(files[0].insertions, 1);
        assert_eq!(files[0].deletions, 0);
        assert_eq!(files[1].filename, "src/lib.rs");
        assert_eq!(files[1].insertions, 1);
        assert_eq!(files[1].deletions, 1);
        assert_eq!(files[2].filename, "Cargo.toml");
        assert_eq!(files[2].insertions, 0);
        assert_eq!(files[2].deletions, 1);
    }

    #[test]
    fn test_split_diff_by_file_binary() {
        let diff = "diff --git a/image.png b/image.png\n\
                     Binary files a/image.png and b/image.png differ";
        let files = split_diff_by_file(diff);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "image.png");
        assert_eq!(files[0].insertions, 0);
        assert_eq!(files[0].deletions, 0);
    }
}
