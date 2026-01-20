use crate::llm::{CommitContext, ReviewType};

/// 静态系统指令（可缓存）- 用于 system/user 分离模式
const COMMIT_SYSTEM_PROMPT: &str = r#"You are a git commit message generator.

Rules:
- Use conventional commits: type(scope): description
- First line max 72 chars
- Common types: feat, fix, docs, style, refactor, test, chore
- Output ONLY the commit message, no explanation"#;

/// Review 基础系统指令（可被自定义覆盖）
const REVIEW_SYSTEM_PROMPT_BASE: &str = r#"You are an expert code reviewer.

Review criteria:
1. Correctness: bugs or logical errors
2. Security: vulnerabilities
3. Performance: issues
4. Maintainability: readability
5. Best practices"#;

/// JSON 格式约束（始终追加）
const REVIEW_JSON_CONSTRAINT: &str = r#"

Output JSON format:
{
  "summary": "Brief assessment",
  "issues": [{"severity": "critical|warning|info", "description": "...", "file": "...", "line": N}],
  "suggestions": ["..."]
}"#;

/// 格式化用户反馈列表
fn format_feedbacks(feedbacks: &[String]) -> String {
    if feedbacks.is_empty() {
        return String::new();
    }
    let mut result = String::from("\n\n## User Requirements:\n");
    for (i, fb) in feedbacks.iter().enumerate() {
        result.push_str(&format!("{}. {}\n", i + 1, fb));
    }
    result
}

/// 构建拆分的 commit prompt（system + user）
///
/// 返回 (system_prompt, user_message)
/// - system_prompt: 静态指令，可被 LLM 缓存
/// - user_message: 动态内容（diff + context + feedback）
pub fn build_commit_prompt_split(
    diff: &str,
    context: &CommitContext,
    custom_template: Option<&str>,
) -> (String, String) {
    // 自定义模板用作 system prompt
    let system = custom_template.unwrap_or(COMMIT_SYSTEM_PROMPT).to_string();

    // user message 包含动态内容
    let branch_info = context
        .branch_name
        .as_ref()
        .map(|b| format!("\nBranch: {}", b))
        .unwrap_or_default();

    let user = format!(
        "## Diff:\n```\n{}\n```\n\n## Context:\nFiles: {}\nChanges: +{} -{}{}{}",
        diff,
        context.files_changed.join(", "),
        context.insertions,
        context.deletions,
        branch_info,
        format_feedbacks(&context.user_feedback)
    );

    (system, user)
}

/// 构建拆分的 review prompt（system + user）
///
/// 返回 (system_prompt, user_message)
/// - system_prompt: 自定义模板（或默认） + JSON 格式约束（始终追加）
/// - user_message: 待审查的代码
pub fn build_review_prompt_split(
    diff: &str,
    _review_type: &ReviewType,
    custom_template: Option<&str>,
) -> (String, String) {
    // 自定义模板用作基础 system prompt，始终追加 JSON 约束
    let base = custom_template.unwrap_or(REVIEW_SYSTEM_PROMPT_BASE);
    let system = format!("{}{}", base, REVIEW_JSON_CONSTRAINT);

    let user = format!("## Code to Review:\n```\n{}\n```", diff);

    (system, user)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn create_context(
        files: Vec<&str>,
        insertions: usize,
        deletions: usize,
        branch: Option<&str>,
        feedbacks: Vec<&str>,
    ) -> CommitContext {
        CommitContext {
            files_changed: files.into_iter().map(String::from).collect(),
            insertions,
            deletions,
            branch_name: branch.map(String::from),
            custom_prompt: None,
            user_feedback: feedbacks.into_iter().map(String::from).collect(),
        }
    }

    // === build_commit_prompt_split 测试 ===

    #[test]
    fn test_commit_prompt_split_default() {
        let ctx = create_context(vec!["foo.rs"], 10, 5, None, vec![]);
        let (system, user) = build_commit_prompt_split("diff content", &ctx, None);

        // system 应该包含角色定义和规则
        assert!(system.contains("git commit message generator"));
        assert!(system.contains("conventional commits"));

        // user 应该包含 diff 和 context
        assert!(user.contains("diff content"));
        assert!(user.contains("foo.rs"));
        assert!(user.contains("+10 -5"));
    }

    #[test]
    fn test_commit_prompt_split_with_branch() {
        let ctx = create_context(vec!["a.rs"], 1, 1, Some("feature/test"), vec![]);
        let (_, user) = build_commit_prompt_split("diff", &ctx, None);

        assert!(user.contains("Branch: feature/test"));
    }

    #[test]
    fn test_commit_prompt_split_with_feedback() {
        let ctx = create_context(
            vec!["a.rs"],
            1,
            1,
            None,
            vec!["请使用中文", "不要超过50字符"],
        );
        let (_, user) = build_commit_prompt_split("diff", &ctx, None);

        assert!(user.contains("User Requirements"));
        assert!(user.contains("1. 请使用中文"));
        assert!(user.contains("2. 不要超过50字符"));
    }

    #[test]
    fn test_commit_prompt_split_custom_template() {
        let ctx = create_context(vec!["a.rs"], 1, 1, None, vec![]);
        let (system, _) = build_commit_prompt_split("diff", &ctx, Some("Custom system prompt"));

        // 自定义模板应该用作 system prompt
        assert_eq!(system, "Custom system prompt");
    }

    // === build_review_prompt_split 测试 ===

    #[test]
    fn test_review_prompt_split_default() {
        let (system, user) =
            build_review_prompt_split("code diff", &ReviewType::UncommittedChanges, None);

        // system 应该包含审查规则和 JSON 格式
        assert!(system.contains("code reviewer"));
        assert!(system.contains("JSON format"));

        // user 应该包含代码
        assert!(user.contains("code diff"));
        assert!(user.contains("Code to Review"));
    }

    #[test]
    fn test_review_prompt_split_custom_template() {
        let (system, _) =
            build_review_prompt_split("diff", &ReviewType::UncommittedChanges, Some("Custom"));

        // 自定义模板 + JSON 约束始终追加
        assert!(system.starts_with("Custom"));
        assert!(system.contains("JSON format"));
        assert!(system.contains("\"summary\""));
    }
}
