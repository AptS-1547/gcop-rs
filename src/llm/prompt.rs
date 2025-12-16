use crate::llm::{CommitContext, ReviewType};

/// 构建 commit message 生成的 prompt
pub fn build_commit_prompt(diff: &str, context: &CommitContext) -> String {
    format!(
        r#"You are an expert software engineer reviewing a git diff to generate a concise, informative commit message.

## Git Diff:
```
{}
```

## Context:
- Files changed: {}
- Insertions: {}
- Deletions: {}
{}

## Instructions:
1. Analyze the changes carefully
2. Generate a commit message following conventional commits format
3. First line: type(scope): brief summary (max 72 chars)
4. Blank line
5. Body: explain what and why (not how), if necessary
6. Keep it concise but informative

Common types: feat, fix, docs, style, refactor, test, chore

Output only the commit message, no explanations."#,
        diff,
        context.files_changed.join(", "),
        context.insertions,
        context.deletions,
        context
            .branch_name
            .as_ref()
            .map(|b| format!("- Branch: {}", b))
            .unwrap_or_default()
    )
}

/// 构建代码审查的 prompt
pub fn build_review_prompt(diff: &str, _review_type: &ReviewType) -> String {
    format!(
        r#"You are an expert code reviewer. Review the following code changes carefully.

## Code to Review:
```
{diff}
```

## Review Criteria:
1. **Correctness**: Are there any bugs or logical errors?
2. **Security**: Are there any security vulnerabilities?
3. **Performance**: Are there any performance issues?
4. **Maintainability**: Is the code readable and maintainable?
5. **Best Practices**: Does it follow best practices?

## Output Format:
Provide your review in JSON format:
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

If no issues found, return empty issues array but provide constructive suggestions."#
    )
}
