//! 响应处理和 JSON 清理
//!
//! 处理 LLM API 响应，包括 JSON 清理、解析和预览

use crate::error::{GcopError, Result};
use crate::llm::ReviewResult;

/// 错误预览最大长度
const ERROR_PREVIEW_LENGTH: usize = 500;

/// 清理 JSON 响应（移除 markdown 代码块标记）
pub fn clean_json_response(response: &str) -> &str {
    let trimmed = response.trim();

    // 提取 { 到 } 之间的内容
    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}'))
        && start < end
    {
        return &trimmed[start..=end];
    }

    // Backup: 回退到移除 markdown 代码块标记
    let without_prefix = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```JSON"))
        .or_else(|| trimmed.strip_prefix("```"))
        .map(|s| s.trim_start()) // 移除前缀后的换行符
        .unwrap_or(trimmed);

    without_prefix
        .strip_suffix("```")
        .map(|s| s.trim_end()) // 移除后缀前的换行符
        .unwrap_or(without_prefix)
        .trim()
}

/// 截断字符串用于错误预览
pub fn truncate_for_preview(s: &str) -> String {
    if s.len() > ERROR_PREVIEW_LENGTH {
        format!("{}...", &s[..ERROR_PREVIEW_LENGTH])
    } else {
        s.to_string()
    }
}

/// 解析 review 响应 JSON
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

/// 处理 commit message 响应并记录日志
pub fn process_commit_response(response: String) -> String {
    tracing::debug!("Generated commit message: {}", response);
    response
}

/// 处理 review 响应并记录日志
pub fn process_review_response(response: &str) -> Result<ReviewResult> {
    tracing::debug!("LLM review response: {}", response);
    parse_review_response(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::IssueSeverity;
    use pretty_assertions::assert_eq;

    // === clean_json_response 测试 ===

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

    // === truncate_for_preview 测试 ===

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

    // === parse_review_response 测试 ===

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

    // === 额外的边界测试 ===

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
        // 应该能正确解析
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
}
