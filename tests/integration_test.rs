//! 集成测试
//!
//! 测试核心功能的完整流程

use gcop_rs::config::AppConfig;
use gcop_rs::git::diff::parse_diff_stats;
use gcop_rs::llm::prompt::{build_commit_prompt_split, build_review_prompt_split};
use gcop_rs::llm::provider::base::{clean_json_response, parse_review_response};
use gcop_rs::llm::{CommitContext, ReviewType};

/// 测试默认配置值正确
#[test]
fn test_config_default_values() {
    let config = AppConfig::default();

    // LLM 配置
    assert_eq!(config.llm.default_provider, "claude");

    // Commit 配置
    assert!(config.commit.show_diff_preview);
    assert!(config.commit.allow_edit);
    assert_eq!(config.commit.max_retries, 10);

    // Network 配置
    assert_eq!(config.network.request_timeout, 120);
    assert_eq!(config.network.connect_timeout, 10);
    assert_eq!(config.network.max_retries, 3);
    assert_eq!(config.network.retry_delay_ms, 1000);

    // UI 配置
    assert!(config.ui.colored);
}

/// 测试 Git diff 解析功能
#[test]
fn test_git_diff_parsing() {
    let diff = r#"diff --git a/src/main.rs b/src/main.rs
index 1234567..abcdefg 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,5 +1,7 @@
 fn main() {
-    println!("Hello");
+    println!("Hello, World!");
+    // Added comment
}"#;

    let stats = parse_diff_stats(diff).unwrap();
    assert_eq!(stats.insertions, 2);
    assert_eq!(stats.deletions, 1);
    assert_eq!(stats.files_changed, vec!["src/main.rs"]);
}

/// 测试 Prompt 生成完整流程
#[test]
fn test_prompt_generation_flow() {
    // 模拟真实的 commit 流程
    let diff = r#"diff --git a/src/main.rs b/src/main.rs
index 1234567..abcdefg 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,5 +1,7 @@
 fn main() {
-    println!("Hello");
+    println!("Hello, World!");
+    // Added comment
 }"#;

    let context = CommitContext {
        files_changed: vec!["src/main.rs".to_string()],
        insertions: 2,
        deletions: 1,
        branch_name: Some("feature/greeting".to_string()),
        custom_prompt: None,
        user_feedback: vec![],
        convention: None,
        scope_info: None,
    };

    let (system, user) = build_commit_prompt_split(diff, &context, None, None);

    // 验证 system prompt 包含角色和规则
    assert!(system.contains("git commit message generator"));
    assert!(system.contains("conventional commits"));

    // 验证 user message 包含所有必要信息
    assert!(user.contains("diff --git"));
    assert!(user.contains("src/main.rs"));
    assert!(user.contains("Branch: feature/greeting"));
    assert!(user.contains("+2 -1"));
}

/// 测试 Review 响应解析完整流程
#[test]
fn test_review_response_parsing_flow() {
    // 模拟 LLM 返回的带 markdown 包装的 JSON
    let llm_response = r#"Based on my analysis, here's the review:

```json
{
    "summary": "Overall good code quality with minor suggestions",
    "issues": [
        {
            "severity": "warning",
            "description": "Consider using a constant for the greeting message",
            "file": "src/main.rs",
            "line": 2
        },
        {
            "severity": "info",
            "description": "Good use of comments"
        }
    ],
    "suggestions": [
        "Consider adding error handling",
        "Add unit tests for the new functionality"
    ]
}
```

Let me know if you need more details!"#;

    // 清理 JSON
    let cleaned = clean_json_response(llm_response);
    assert!(cleaned.starts_with('{'));
    assert!(cleaned.ends_with('}'));

    // 解析为结构化数据
    let result = parse_review_response(llm_response).unwrap();

    assert_eq!(
        result.summary,
        "Overall good code quality with minor suggestions"
    );
    assert_eq!(result.issues.len(), 2);
    assert_eq!(result.suggestions.len(), 2);

    // 验证第一个 issue 的详细信息
    let first_issue = &result.issues[0];
    assert!(matches!(
        first_issue.severity,
        gcop_rs::llm::IssueSeverity::Warning
    ));
    assert_eq!(first_issue.file, Some("src/main.rs".to_string()));
    assert_eq!(first_issue.line, Some(2));
}

/// 测试 Review prompt 生成
#[test]
fn test_review_prompt_generation() {
    let diff = "diff --git a/foo.rs b/foo.rs\n+new line";
    let (system, user) = build_review_prompt_split(diff, &ReviewType::UncommittedChanges, None);

    // 验证 system prompt 包含审查规则和 JSON 格式
    assert!(system.contains("code reviewer"));
    assert!(system.contains("JSON format"));
    assert!(system.contains("\"summary\""));
    assert!(system.contains("\"issues\""));
    assert!(system.contains("\"severity\""));

    // 验证 user message 包含代码
    assert!(user.contains("Code to Review"));
    assert!(user.contains("diff --git"));
}

/// 测试用户反馈累积
#[test]
fn test_user_feedback_accumulation() {
    let context = CommitContext {
        files_changed: vec!["test.rs".to_string()],
        insertions: 1,
        deletions: 0,
        branch_name: None,
        custom_prompt: None,
        user_feedback: vec![
            "请使用中文".to_string(),
            "不要超过50字符".to_string(),
            "使用 feat 类型".to_string(),
        ],
        convention: None,
        scope_info: None,
    };

    let (_, user) = build_commit_prompt_split("diff", &context, None, None);

    // 验证所有反馈都被追加且编号正确
    assert!(user.contains("User Requirements"));
    assert!(user.contains("1. 请使用中文"));
    assert!(user.contains("2. 不要超过50字符"));
    assert!(user.contains("3. 使用 feat 类型"));
}
