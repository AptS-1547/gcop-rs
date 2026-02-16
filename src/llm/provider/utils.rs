//! Provider utility functions
//!
//! Contains common functions such as URL processing and endpoint completion

/// Claude API endpoint suffix
pub const CLAUDE_API_SUFFIX: &str = "/v1/messages";

/// OpenAI API endpoint suffix
pub const OPENAI_API_SUFFIX: &str = "/v1/chat/completions";

/// Ollama API endpoint suffix
pub const OLLAMA_API_SUFFIX: &str = "/api/generate";

/// Claude default base URL
pub const DEFAULT_CLAUDE_BASE: &str = "https://api.anthropic.com";

/// OpenAI default base URL
pub const DEFAULT_OPENAI_BASE: &str = "https://api.openai.com";

/// Ollama default base URL
pub const DEFAULT_OLLAMA_BASE: &str = "http://localhost:11434";

/// Gemini default base URL
pub const DEFAULT_GEMINI_BASE: &str = "https://generativelanguage.googleapis.com";

/// Smart completion API endpoint
///
/// # Behavior
/// 1. Remove trailing slashes
/// 2. Check whether the URL contains the full path
/// 3. If incomplete, automatically complete suffix
///
/// # Example
/// ```
/// use gcop_rs::llm::provider::utils::complete_endpoint;
///
/// assert_eq!(
///     complete_endpoint("https://api.deepseek.com", "/v1/chat/completions"),
///     "https://api.deepseek.com/v1/chat/completions"
/// );
///
/// assert_eq!(
///     complete_endpoint("https://api.deepseek.com/v1/chat/completions", "/v1/chat/completions"),
///     "https://api.deepseek.com/v1/chat/completions"
/// );
///
/// assert_eq!(
///     complete_endpoint("https://api.deepseek.com/", "/v1/chat/completions"),
///     "https://api.deepseek.com/v1/chat/completions"
/// );
/// ```
pub fn complete_endpoint(base_url: &str, expected_suffix: &str) -> String {
    // 1. Clean URLs: Remove trailing slashes
    let url = base_url.trim_end_matches('/');
    let suffix = expected_suffix.trim_start_matches('/');

    // 2. If the expected suffix is ​​already included, return directly
    if url.ends_with(suffix) {
        return url.to_string();
    }

    // 3. Check whether the URL contains the partial prefix of suffix
    // For example: url is "https://api.com/v1", suffix is ​​"v1/chat/completions"
    // Then we should only complete "/chat/completions"
    let suffix_parts: Vec<&str> = suffix.split('/').collect();

    // Check from back to front to see if the URL already contains the suffix prefix
    for i in 0..suffix_parts.len() {
        let partial_suffix = suffix_parts[..=i].join("/");
        if url.ends_with(&partial_suffix) {
            // The URL already contains part of the suffix, only the remaining part is completed.
            let remaining_suffix = &suffix_parts[i + 1..].join("/");
            if remaining_suffix.is_empty() {
                return url.to_string();
            }
            return format!("{}/{}", url, remaining_suffix);
        }
    }

    // 4. Check whether it is a customized complete API path
    if is_complete_api_path(url) {
        return url.to_string();
    }

    // 5. Complete the complete suffix
    format!("{}/{}", url, suffix)
}

/// Check if the URL is already a full API path
///
/// Heuristic rules:
/// - Path depth >= 2 is considered a complete path (such as /v1/chat, /api/generate)
/// - This allows users to use fully customized endpoints
fn is_complete_api_path(url: &str) -> bool {
    // Extract the path part (remove the protocol and domain name)
    let path = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .and_then(|rest| rest.split_once('/'))
        .map(|(_, path)| path)
        .unwrap_or("");

    if path.is_empty() {
        return false;
    }

    // Count non-empty path segments
    let segment_count = path.split('/').filter(|s| !s.is_empty()).count();

    // Path depth >= 2 is considered a user-defined complete path
    segment_count >= 2
}

/// Mask API key to prevent log leaks
///
/// # rule
/// - length > 8: display first 4 characters + `...` + last 4 characters
/// - length <= 8: display `****`
///
/// # Example
/// ```
/// use gcop_rs::llm::provider::utils::mask_api_key;
///
/// assert_eq!(mask_api_key("sk-ant-api03-abcdefgh"), "sk-a...efgh");
/// assert_eq!(mask_api_key("short"), "****");
/// assert_eq!(mask_api_key(""), "****");
/// ```
pub fn mask_api_key(key: &str) -> String {
    if key.len() > 8 {
        format!("{}...{}", &key[..4], &key[key.len() - 4..])
    } else {
        "****".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_api_key() {
        // Long key: first 4 + ... + last 4
        assert_eq!(mask_api_key("sk-ant-api03-abcdefgh"), "sk-a...efgh");
        assert_eq!(mask_api_key("AIzaSyD-1234567890abcdef"), "AIza...cdef");

        // short key
        assert_eq!(mask_api_key("12345678"), "****");
        assert_eq!(mask_api_key("short"), "****");

        // null
        assert_eq!(mask_api_key(""), "****");

        // Exactly 9 characters
        assert_eq!(mask_api_key("123456789"), "1234...6789");
    }

    #[test]
    fn test_complete_endpoint_basic() {
        // Basic completion
        assert_eq!(
            complete_endpoint("https://api.deepseek.com", "/v1/chat/completions"),
            "https://api.deepseek.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_complete_endpoint_with_trailing_slash() {
        // with trailing slash
        assert_eq!(
            complete_endpoint("https://api.deepseek.com/", "/v1/chat/completions"),
            "https://api.deepseek.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_complete_endpoint_already_complete() {
        // Already complete
        assert_eq!(
            complete_endpoint(
                "https://api.deepseek.com/v1/chat/completions",
                "/v1/chat/completions"
            ),
            "https://api.deepseek.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_complete_endpoint_with_version_only() {
        // Only the version number needs to be completed
        assert_eq!(
            complete_endpoint("https://api.deepseek.com/v1", "/v1/chat/completions"),
            "https://api.deepseek.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_complete_endpoint_custom_path() {
        // Customize the full path and keep it as is
        assert_eq!(
            complete_endpoint("https://custom.com/my/custom/path", "/v1/chat/completions"),
            "https://custom.com/my/custom/path"
        );
    }

    #[test]
    fn test_is_complete_api_path() {
        // full path
        assert!(is_complete_api_path("https://api.com/v1/chat"));
        assert!(is_complete_api_path("http://localhost:11434/api/generate"));

        // Incomplete path
        assert!(!is_complete_api_path("https://api.com"));
        assert!(!is_complete_api_path("https://api.com/"));
        assert!(!is_complete_api_path("https://api.com/v1"));
    }

    #[test]
    fn test_ollama_localhost() {
        // Ollama local address
        assert_eq!(
            complete_endpoint("http://localhost:11434", "/api/generate"),
            "http://localhost:11434/api/generate"
        );
    }

    #[test]
    fn test_claude_endpoint() {
        // Claude API
        assert_eq!(
            complete_endpoint("https://api.anthropic.com", "/v1/messages"),
            "https://api.anthropic.com/v1/messages"
        );

        // Claude Agent
        assert_eq!(
            complete_endpoint("https://cc.autobits.cc", "/v1/messages"),
            "https://cc.autobits.cc/v1/messages"
        );
    }

    #[test]
    fn test_suffix_variations() {
        // suffix with leading slash
        assert_eq!(
            complete_endpoint("https://api.com", "/v1/test"),
            "https://api.com/v1/test"
        );

        // suffix without leading slash
        assert_eq!(
            complete_endpoint("https://api.com", "v1/test"),
            "https://api.com/v1/test"
        );
    }
}
