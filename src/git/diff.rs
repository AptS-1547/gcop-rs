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
}
