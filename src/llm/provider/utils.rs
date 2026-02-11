//! Provider 工具函数
//!
//! 包含 URL 处理、endpoint 补全等通用功能

/// Claude API endpoint suffix
pub const CLAUDE_API_SUFFIX: &str = "/v1/messages";

/// OpenAI API endpoint suffix
pub const OPENAI_API_SUFFIX: &str = "/v1/chat/completions";

/// Ollama API endpoint suffix
pub const OLLAMA_API_SUFFIX: &str = "/api/generate";

/// Claude 默认 base URL
pub const DEFAULT_CLAUDE_BASE: &str = "https://api.anthropic.com";

/// OpenAI 默认 base URL
pub const DEFAULT_OPENAI_BASE: &str = "https://api.openai.com";

/// Ollama 默认 base URL
pub const DEFAULT_OLLAMA_BASE: &str = "http://localhost:11434";

/// Gemini 默认 base URL
pub const DEFAULT_GEMINI_BASE: &str = "https://generativelanguage.googleapis.com";

/// 智能补全 API endpoint
///
/// # 行为
/// 1. 移除 trailing slashes
/// 2. 检测 URL 是否已包含完整路径
/// 3. 如果不完整，自动补全 suffix
///
/// # 示例
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
    // 1. 清理 URL: 移除尾部斜杠
    let url = base_url.trim_end_matches('/');
    let suffix = expected_suffix.trim_start_matches('/');

    // 2. 如果已经包含期望的 suffix，直接返回
    if url.ends_with(suffix) {
        return url.to_string();
    }

    // 3. 检测 URL 是否包含 suffix 的部分前缀
    // 例如: url 是 "https://api.com/v1", suffix 是 "v1/chat/completions"
    // 那么我们应该只补全 "/chat/completions"
    let suffix_parts: Vec<&str> = suffix.split('/').collect();

    // 从后往前检查，看 URL 是否已经包含 suffix 的前缀
    for i in 0..suffix_parts.len() {
        let partial_suffix = suffix_parts[..=i].join("/");
        if url.ends_with(&partial_suffix) {
            // URL 已经包含了部分 suffix，只补全剩余部分
            let remaining_suffix = &suffix_parts[i + 1..].join("/");
            if remaining_suffix.is_empty() {
                return url.to_string();
            }
            return format!("{}/{}", url, remaining_suffix);
        }
    }

    // 4. 检测是否是自定义的完整 API 路径
    if is_complete_api_path(url) {
        return url.to_string();
    }

    // 5. 补全完整 suffix
    format!("{}/{}", url, suffix)
}

/// 检测 URL 是否已经是完整的 API 路径
///
/// 启发式规则:
/// - 路径深度 >= 2 认为是完整路径 (如 /v1/chat, /api/generate)
/// - 这允许用户使用完全自定义的 endpoint
fn is_complete_api_path(url: &str) -> bool {
    // 提取路径部分 (去掉协议和域名)
    let path = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .and_then(|rest| rest.split_once('/'))
        .map(|(_, path)| path)
        .unwrap_or("");

    if path.is_empty() {
        return false;
    }

    // 统计非空路径段
    let segment_count = path.split('/').filter(|s| !s.is_empty()).count();

    // 路径深度 >= 2 认为是用户自定义的完整路径
    segment_count >= 2
}

/// 掩码 API key，防止日志泄露
///
/// # 规则
/// - 长度 > 8：显示前 4 字符 + `...` + 后 4 字符
/// - 长度 <= 8：显示 `****`
///
/// # 示例
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
        // 长 key：前4 + ... + 后4
        assert_eq!(mask_api_key("sk-ant-api03-abcdefgh"), "sk-a...efgh");
        assert_eq!(mask_api_key("AIzaSyD-1234567890abcdef"), "AIza...cdef");

        // 短 key
        assert_eq!(mask_api_key("12345678"), "****");
        assert_eq!(mask_api_key("short"), "****");

        // 空
        assert_eq!(mask_api_key(""), "****");

        // 刚好 9 字符
        assert_eq!(mask_api_key("123456789"), "1234...6789");
    }

    #[test]
    fn test_complete_endpoint_basic() {
        // 基本补全
        assert_eq!(
            complete_endpoint("https://api.deepseek.com", "/v1/chat/completions"),
            "https://api.deepseek.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_complete_endpoint_with_trailing_slash() {
        // 带尾部斜杠
        assert_eq!(
            complete_endpoint("https://api.deepseek.com/", "/v1/chat/completions"),
            "https://api.deepseek.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_complete_endpoint_already_complete() {
        // 已经完整
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
        // 只有版本号，需要补全
        assert_eq!(
            complete_endpoint("https://api.deepseek.com/v1", "/v1/chat/completions"),
            "https://api.deepseek.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_complete_endpoint_custom_path() {
        // 自定义完整路径，保持原样
        assert_eq!(
            complete_endpoint("https://custom.com/my/custom/path", "/v1/chat/completions"),
            "https://custom.com/my/custom/path"
        );
    }

    #[test]
    fn test_is_complete_api_path() {
        // 完整路径
        assert!(is_complete_api_path("https://api.com/v1/chat"));
        assert!(is_complete_api_path("http://localhost:11434/api/generate"));

        // 不完整路径
        assert!(!is_complete_api_path("https://api.com"));
        assert!(!is_complete_api_path("https://api.com/"));
        assert!(!is_complete_api_path("https://api.com/v1"));
    }

    #[test]
    fn test_ollama_localhost() {
        // Ollama 本地地址
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

        // Claude 代理
        assert_eq!(
            complete_endpoint("https://cc.autobits.cc", "/v1/messages"),
            "https://cc.autobits.cc/v1/messages"
        );
    }

    #[test]
    fn test_suffix_variations() {
        // suffix 带前导斜杠
        assert_eq!(
            complete_endpoint("https://api.com", "/v1/test"),
            "https://api.com/v1/test"
        );

        // suffix 不带前导斜杠
        assert_eq!(
            complete_endpoint("https://api.com", "v1/test"),
            "https://api.com/v1/test"
        );
    }
}
