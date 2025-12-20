use crate::llm::{CommitContext, ReviewType};

/// 默认的 commit prompt 模板
const DEFAULT_COMMIT_PROMPT: &str = r#"You are an expert software engineer reviewing a git diff to generate a concise, informative commit message.

    ## Git Diff:
    ```
    {diff}
    ```

    ## Context:
    - Files changed: {files_changed}
    - Insertions: {insertions}
    - Deletions: {deletions}
    {branch_info}

    ## Instructions:
    1. Analyze the changes carefully
    2. Generate a commit message following conventional commits format
    3. First line: type(scope): brief summary (max 72 chars)
    4. Blank line
    5. Body: explain what and why (not how), if necessary
    6. Keep it concise but informative

    Common types: feat, fix, docs, style, refactor, test, chore

    Output only the commit message, no explanations."#;

/// 默认的 review prompt 模板
const DEFAULT_REVIEW_PROMPT: &str = r#"You are an expert code reviewer. Review the following code changes carefully.

    ## Code to Review:
    ```
    {diff}
    ```

    ## Review Criteria:
    1. **Correctness**: Are there any bugs or logical errors?
    2. **Security**: Are there any security vulnerabilities?
    3. **Performance**: Are there any performance issues?
    4. **Maintainability**: Is the code readable and maintainable?
    5. **Best Practices**: Does it follow best practices?"#;

/// 默认的 JSON 输出格式说明
const DEFAULT_JSON_FORMAT: &str = r#"## Output Format:
    Provide your review in JSON format
    Do not include any explanations outside the JSON structure. Format as follows:
    {{
    "summary": "Brief overall assessment",
    "issues": [
        {{
        "severity": "critical" | "warning" | "info",
        "description": "Issue description",
        "file": "filename (if applicable)",
        "line": line_number (if applicable)
        }}
    ],
    "suggestions": [
        "Improvement suggestion 1"
    ]
    }}

    If no issues found, return empty issues array but provide constructive suggestions."#;

/// 构建 commit message 生成的 prompt
pub fn build_commit_prompt(
    diff: &str,
    context: &CommitContext,
    custom_template: Option<&str>,
) -> String {
    let template = match custom_template {
        Some(t) if !t.contains("{diff}") => {
            // 自定义模板缺少 {diff}，追加默认的 diff 部分
            format!(
                "{}\n\n## Git Diff:\n```\n{{diff}}\n```\n\n## Context:\n- Files: {{files_changed}}\n- Changes: +{{insertions}} -{{deletions}}",
                t
            )
        }
        Some(t) => t.to_string(),
        None => DEFAULT_COMMIT_PROMPT.to_string(),
    };

    let branch_info = context
        .branch_name
        .as_ref()
        .map(|b| format!("- Branch: {}", b))
        .unwrap_or_default();

    let mut prompt = template
        .replace("{diff}", diff)
        .replace("{files_changed}", &context.files_changed.join(", "))
        .replace("{insertions}", &context.insertions.to_string())
        .replace("{deletions}", &context.deletions.to_string())
        .replace(
            "{branch_name}",
            context.branch_name.as_deref().unwrap_or(""),
        )
        .replace("{branch_info}", &branch_info);

    // 在 prompt 尾部追加用户反馈
    if !context.user_feedback.is_empty() {
        prompt.push_str("\n\n## Additional User Requirements:\n");
        for (i, fb) in context.user_feedback.iter().enumerate() {
            prompt.push_str(&format!("{}. {}\n", i + 1, fb));
        }
    }

    prompt
}

/// 构建代码审查的 prompt
pub fn build_review_prompt(
    diff: &str,
    _review_type: &ReviewType,
    custom_template: Option<&str>,
) -> String {
    let mut template = custom_template
        .map(|t| t.to_string())
        .unwrap_or_else(|| DEFAULT_REVIEW_PROMPT.to_string());

    // 检测并追加缺失的 {diff}
    if !template.contains("{diff}") {
        template.push_str("\n\n## Code to Review:\n```\n{diff}\n```");
    }

    // 始终追加 JSON 格式说明
    template.push_str("\n\n");
    template.push_str(DEFAULT_JSON_FORMAT);

    template.replace("{diff}", diff)
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

    // === build_commit_prompt 测试 ===

    #[test]
    fn test_build_commit_prompt_default_template() {
        let diff = "diff --git a/foo.rs";
        let ctx = create_context(vec!["foo.rs"], 10, 5, None, vec![]);
        let result = build_commit_prompt(diff, &ctx, None);

        assert!(result.contains("diff --git a/foo.rs"));
        assert!(result.contains("foo.rs"));
        assert!(result.contains("10"));
        assert!(result.contains("5"));
    }

    #[test]
    fn test_build_commit_prompt_empty_context() {
        let diff = "";
        let ctx = create_context(vec![], 0, 0, None, vec![]);
        let result = build_commit_prompt(diff, &ctx, None);

        assert!(result.contains("Files changed:"));
        assert!(result.contains("Insertions: 0"));
        assert!(result.contains("Deletions: 0"));
    }

    #[test]
    fn test_build_commit_prompt_no_branch() {
        let ctx = create_context(vec!["a.rs"], 1, 1, None, vec![]);
        let result = build_commit_prompt("diff", &ctx, None);

        // branch_info 应该是空的
        assert!(!result.contains("Branch:"));
    }

    #[test]
    fn test_build_commit_prompt_with_branch() {
        let ctx = create_context(vec!["a.rs"], 1, 1, Some("feature/test"), vec![]);
        let result = build_commit_prompt("diff", &ctx, None);

        assert!(result.contains("Branch: feature/test"));
    }

    #[test]
    fn test_build_commit_prompt_custom_template_with_diff() {
        let custom = "My custom template with {diff} placeholder";
        let ctx = create_context(vec!["a.rs"], 1, 1, None, vec![]);
        let result = build_commit_prompt("actual_diff", &ctx, Some(custom));

        assert!(result.contains("My custom template with actual_diff placeholder"));
        // 不应该追加默认的 diff 部分
        assert!(!result.contains("## Git Diff:"));
    }

    #[test]
    fn test_build_commit_prompt_custom_template_missing_diff() {
        let custom = "My custom template without diff";
        let ctx = create_context(vec!["a.rs"], 1, 1, None, vec![]);
        let result = build_commit_prompt("actual_diff", &ctx, Some(custom));

        // 应该追加默认的 diff 部分
        assert!(result.contains("My custom template without diff"));
        assert!(result.contains("## Git Diff:"));
        assert!(result.contains("actual_diff"));
    }

    #[test]
    fn test_build_commit_prompt_single_feedback() {
        let ctx = create_context(vec!["a.rs"], 1, 1, None, vec!["请使用中文"]);
        let result = build_commit_prompt("diff", &ctx, None);

        assert!(result.contains("## Additional User Requirements:"));
        assert!(result.contains("1. 请使用中文"));
    }

    #[test]
    fn test_build_commit_prompt_multiple_feedbacks() {
        let ctx = create_context(
            vec!["a.rs"],
            1,
            1,
            None,
            vec!["请使用中文", "不要超过50字符", "使用 feat 类型"],
        );
        let result = build_commit_prompt("diff", &ctx, None);

        assert!(result.contains("1. 请使用中文"));
        assert!(result.contains("2. 不要超过50字符"));
        assert!(result.contains("3. 使用 feat 类型"));
    }

    #[test]
    fn test_build_commit_prompt_no_feedback() {
        let ctx = create_context(vec!["a.rs"], 1, 1, None, vec![]);
        let result = build_commit_prompt("diff", &ctx, None);

        assert!(!result.contains("## Additional User Requirements:"));
    }

    // === build_review_prompt 测试 ===

    #[test]
    fn test_build_review_prompt_default_template() {
        let result = build_review_prompt("diff_content", &ReviewType::UncommittedChanges, None);

        assert!(result.contains("diff_content"));
        assert!(result.contains("Code to Review"));
        // 应该包含 JSON 格式说明
        assert!(result.contains("Output Format"));
        assert!(result.contains("\"summary\""));
    }

    #[test]
    fn test_build_review_prompt_custom_template_with_diff() {
        let custom = "Review this: {diff}";
        let result = build_review_prompt("my_diff", &ReviewType::UncommittedChanges, Some(custom));

        assert!(result.contains("Review this: my_diff"));
        // 不应该重复追加 diff 部分
        assert_eq!(result.matches("my_diff").count(), 1);
    }

    #[test]
    fn test_build_review_prompt_custom_template_missing_diff() {
        let custom = "Review without diff placeholder";
        let result = build_review_prompt("my_diff", &ReviewType::UncommittedChanges, Some(custom));

        assert!(result.contains("Review without diff placeholder"));
        // 应该追加 diff 部分
        assert!(result.contains("## Code to Review:"));
        assert!(result.contains("my_diff"));
    }

    #[test]
    fn test_build_review_prompt_always_appends_json_format() {
        let custom = "Custom template with {diff}";
        let result = build_review_prompt("diff", &ReviewType::UncommittedChanges, Some(custom));

        // 即使使用自定义模板，也应该追加 JSON 格式说明
        assert!(result.contains("Output Format"));
        assert!(result.contains("\"severity\""));
    }

    #[test]
    fn test_build_review_prompt_empty_diff() {
        let result = build_review_prompt("", &ReviewType::UncommittedChanges, None);

        // 空 diff 应该正常替换，模板中的 {diff} 会被替换为空字符串
        assert!(result.contains("Code to Review"));
        // 验证 {diff} 已被替换（不再包含占位符）
        assert!(!result.contains("{diff}"));
    }

    #[test]
    fn test_build_review_prompt_preserves_custom_content() {
        let custom = "Special instructions: {diff}\nExtra notes here";
        let result = build_review_prompt("code", &ReviewType::UncommittedChanges, Some(custom));

        assert!(result.contains("Special instructions: code"));
        assert!(result.contains("Extra notes here"));
    }
}
