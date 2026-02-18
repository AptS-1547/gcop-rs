//! Response handling and JSON cleaning
//!
//! Handle LLM API responses, including JSON cleaning, parsing, and previewing

use crate::error::{GcopError, Result};
use crate::llm::ReviewResult;

/// Error preview maximum length
const ERROR_PREVIEW_LENGTH: usize = 500;

/// Clean JSON response (remove markdown code block tags)
pub fn clean_json_response(response: &str) -> &str {
    let trimmed = response.trim();

    // Extract content between { to }
    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}'))
        && start < end
    {
        return &trimmed[start..=end];
    }

    // Backup: Fallback to removing markdown code block tags
    let without_prefix = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```JSON"))
        .or_else(|| trimmed.strip_prefix("```"))
        .map(|s| s.trim_start()) // Remove newline character after prefix
        .unwrap_or(trimmed);

    without_prefix
        .strip_suffix("```")
        .map(|s| s.trim_end()) // Remove newline character before suffix
        .unwrap_or(without_prefix)
        .trim()
}

/// Truncate string for error preview (safe handling of multibyte characters)
pub fn truncate_for_preview(s: &str) -> String {
    if s.len() <= ERROR_PREVIEW_LENGTH {
        return s.to_string();
    }
    // Find the last char boundary that does not exceed max_len
    let boundary = s
        .char_indices()
        .map(|(i, _)| i)
        .take_while(|&i| i <= ERROR_PREVIEW_LENGTH)
        .last()
        .unwrap_or(0);
    format!("{}...", &s[..boundary])
}

/// Parse review response JSON
pub fn parse_review_response(response: &str) -> Result<ReviewResult> {
    let cleaned = clean_json_response(response);
    serde_json::from_str(cleaned).map_err(|e| {
        let preview = truncate_for_preview(response);
        GcopError::Llm(
            rust_i18n::t!(
                "provider.parse_review_result_failed",
                error = e.to_string(),
                preview = preview.as_str()
            )
            .to_string(),
        )
    })
}

/// Clean commit message response (remove markdown code block fences)
///
/// LLMs sometimes wrap commit messages in code fences like:
/// ````text
/// ```
/// feat(auth): add login
/// ```
/// ````
/// This function strips those fences.
pub fn clean_commit_response(response: &str) -> String {
    let trimmed = response.trim();

    // Try ```<lang>\n...\n``` pattern (with optional language tag)
    if let Some(rest) = trimmed.strip_prefix("```") {
        // Skip optional language tag (e.g., "text", "markdown", etc.)
        let after_lang = if let Some(newline_pos) = rest.find('\n') {
            let lang_part = &rest[..newline_pos];
            // Only skip if it looks like a language tag (no spaces, short)
            if lang_part.trim().len() <= 20 && !lang_part.contains(' ') {
                &rest[newline_pos + 1..]
            } else {
                rest
            }
        } else {
            rest
        };

        if let Some(inner) = after_lang.strip_suffix("```") {
            return inner.trim().to_string();
        }
    }

    trimmed.to_string()
}

/// Process commit message response: clean code fences and log
pub fn process_commit_response(response: String) -> String {
    let cleaned = clean_commit_response(&response);
    tracing::debug!("Generated commit message: {}", cleaned);
    cleaned
}

/// Process review responses and log them
pub fn process_review_response(response: &str) -> Result<ReviewResult> {
    tracing::debug!("LLM review response: {}", response);
    parse_review_response(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::IssueSeverity;
    use pretty_assertions::assert_eq;

    // === clean_json_response test ===

    #[test]
    fn test_clean_json_plain() {
        let input = r#"{"key": "value"}"#;
        assert_eq!(clean_json_response(input), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_clean_json_markdown_lowercase() {
        let input = "```json\n{\"key\": \"value\"}\n```";
        assert_eq!(clean_json_response(input), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_clean_json_markdown_uppercase() {
        let input = "```JSON\n{\"key\": \"value\"}\n```";
        assert_eq!(clean_json_response(input), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_clean_json_markdown_no_lang() {
        let input = "```\n{\"key\": \"value\"}\n```";
        assert_eq!(clean_json_response(input), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_clean_json_with_prefix_text() {
        let input = "Here is the result:\n{\"key\": \"value\"}";
        assert_eq!(clean_json_response(input), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_clean_json_with_suffix_text() {
        let input = "{\"key\": \"value\"}\nHope this helps!";
        assert_eq!(clean_json_response(input), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_clean_json_with_both_prefix_suffix() {
        let input = "Result:\n{\"key\": \"value\"}\nDone.";
        assert_eq!(clean_json_response(input), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_clean_json_nested_braces() {
        let input = r#"{"outer": {"inner": "value"}}"#;
        assert_eq!(
            clean_json_response(input),
            r#"{"outer": {"inner": "value"}}"#
        );
    }

    #[test]
    fn test_clean_json_empty_string() {
        assert_eq!(clean_json_response(""), "");
    }

    #[test]
    fn test_clean_json_no_braces() {
        let input = "Just some text without JSON";
        assert_eq!(clean_json_response(input), "Just some text without JSON");
    }

    // === truncate_for_preview test ===

    #[test]
    fn test_truncate_short_string() {
        let short = "This is a short string";
        assert_eq!(truncate_for_preview(short), short);
    }

    #[test]
    fn test_truncate_long_string() {
        let long = "a".repeat(600);
        let result = truncate_for_preview(&long);

        assert!(result.len() < long.len());
        assert!(result.ends_with("..."));
        assert_eq!(result.len(), ERROR_PREVIEW_LENGTH + 3); // 500 + "..."
    }

    #[test]
    fn test_truncate_multibyte_chars() {
        // 3 bytes per Chinese character, 200 = 600 bytes > 500
        let chinese = "你".repeat(200);
        let result = truncate_for_preview(&chinese);
        assert!(result.ends_with("..."));
        // Make sure to truncate on the char boundary without panic
        // 500 / 3 = 166 complete characters = 498 bytes
        assert!(result.len() <= ERROR_PREVIEW_LENGTH + 3 + 3);
    }

    #[test]
    fn test_truncate_emoji() {
        // emoji 4 bytes, 150 = 600 bytes > 500
        let emoji = "🎉".repeat(150);
        let result = truncate_for_preview(&emoji);
        assert!(result.ends_with("..."));
    }

    // === parse_review_response test ===

    #[test]
    fn test_parse_review_valid_json() {
        let json = r#"{
            "summary": "Good code",
            "issues": [
                {
                    "severity": "warning",
                    "description": "Consider adding comments"
                }
            ],
            "suggestions": ["Add tests"]
        }"#;

        let result = parse_review_response(json).unwrap();
        assert_eq!(result.summary, "Good code");
        assert_eq!(result.issues.len(), 1);
        assert!(matches!(result.issues[0].severity, IssueSeverity::Warning));
        assert_eq!(result.suggestions.len(), 1);
    }

    #[test]
    fn test_parse_review_with_markdown() {
        let json = r#"```json
{
    "summary": "Clean code",
    "issues": [],
    "suggestions": []
}
```"#;

        let result = parse_review_response(json).unwrap();
        assert_eq!(result.summary, "Clean code");
        assert!(result.issues.is_empty());
    }

    #[test]
    fn test_parse_review_invalid_json() {
        let invalid = "This is not valid JSON";
        let result = parse_review_response(invalid);

        assert!(result.is_err());
        if let Err(GcopError::Llm(msg)) = result {
            assert!(msg.contains("Failed to parse review result"));
        }
    }

    #[test]
    fn test_parse_review_empty_issues() {
        let json = r#"{
            "summary": "Perfect!",
            "issues": [],
            "suggestions": ["Keep up the good work"]
        }"#;

        let result = parse_review_response(json).unwrap();
        assert!(result.issues.is_empty());
        assert_eq!(result.suggestions.len(), 1);
    }

    // === Additional boundary testing ===

    #[test]
    fn test_clean_json_with_whitespace() {
        let input = "   \n  {\"key\": \"value\"}  \n   ";
        assert_eq!(clean_json_response(input), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_clean_json_complex_nested() {
        let input = r#"Here's the review:
{
    "summary": "Test",
    "issues": [{"severity": "info", "description": "ok"}],
    "suggestions": []
}
Let me know if you need more."#;

        let result = clean_json_response(input);
        // should be able to parse correctly
        let parsed: serde_json::Value = serde_json::from_str(result).unwrap();
        assert_eq!(parsed["summary"], "Test");
    }

    #[test]
    fn test_parse_review_with_file_and_line() {
        let json = r#"{
            "summary": "Found issue",
            "issues": [
                {
                    "severity": "critical",
                    "description": "Memory leak",
                    "file": "main.rs",
                    "line": 42
                }
            ],
            "suggestions": []
        }"#;

        let result = parse_review_response(json).unwrap();
        assert_eq!(result.issues[0].file, Some("main.rs".to_string()));
        assert_eq!(result.issues[0].line, Some(42));
    }

    // === clean_commit_response tests ===

    #[test]
    fn test_clean_commit_plain_message() {
        let input = "feat(auth): add login validation";
        assert_eq!(
            clean_commit_response(input),
            "feat(auth): add login validation"
        );
    }

    #[test]
    fn test_clean_commit_bare_fences() {
        let input = "```\nfeat(auth): add login validation\n```";
        assert_eq!(
            clean_commit_response(input),
            "feat(auth): add login validation"
        );
    }

    #[test]
    fn test_clean_commit_text_lang_tag() {
        let input = "```text\nfeat(auth): add login validation\n```";
        assert_eq!(
            clean_commit_response(input),
            "feat(auth): add login validation"
        );
    }

    #[test]
    fn test_clean_commit_markdown_lang_tag() {
        let input = "```markdown\nfix(ui): resolve button alignment\n```";
        assert_eq!(
            clean_commit_response(input),
            "fix(ui): resolve button alignment"
        );
    }

    #[test]
    fn test_clean_commit_multiline_body() {
        let input = "```\nfeat(auth): add login validation\n\nAdded email and password validation.\nCloses #42\n```";
        assert_eq!(
            clean_commit_response(input),
            "feat(auth): add login validation\n\nAdded email and password validation.\nCloses #42"
        );
    }

    #[test]
    fn test_clean_commit_no_closing_fence() {
        // Only opening fence, no closing — should not strip
        let input = "```\nfeat(auth): add login validation";
        assert_eq!(
            clean_commit_response(input),
            "```\nfeat(auth): add login validation"
        );
    }

    #[test]
    fn test_clean_commit_with_whitespace() {
        let input = "  \n```\nfeat: update deps\n```\n  ";
        assert_eq!(clean_commit_response(input), "feat: update deps");
    }

    #[test]
    fn test_clean_commit_already_clean() {
        let input = "chore: bump version to 1.2.3";
        assert_eq!(clean_commit_response(input), "chore: bump version to 1.2.3");
    }

    #[test]
    fn test_process_commit_response_strips_fences() {
        let input = "```\nfeat: new feature\n```".to_string();
        assert_eq!(process_commit_response(input), "feat: new feature");
    }

    #[test]
    fn test_clean_commit_multiline_with_list_bare_fences() {
        // Real-world case: LLM wraps a multi-line commit message with bullet
        // list in bare code fences (no language tag).
        let input = "```\nperf(config): 优化图片缓存策略以支持及时更新\n\n- 将图片缓存 TTL 从 1 年调整为 1 小时\n- 修改静态资源缓存策略为 1 天 + SWR 1 周\n- 允许图片更新后更快速地刷新展示\n```";
        assert_eq!(
            clean_commit_response(input),
            "perf(config): 优化图片缓存策略以支持及时更新\n\n- 将图片缓存 TTL 从 1 年调整为 1 小时\n- 修改静态资源缓存策略为 1 天 + SWR 1 周\n- 允许图片更新后更快速地刷新展示"
        );
    }
}
