use std::str::FromStr;

/// 输出格式枚举
///
/// 统一处理 CLI 中的 `--format` 和 `--json` 参数
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
    Markdown,
}

impl FromStr for OutputFormat {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "json" => Self::Json,
            "markdown" | "md" => Self::Markdown,
            _ => Self::Text,
        })
    }
}

impl OutputFormat {
    /// 从 CLI 参数解析输出格式
    ///
    /// `--json` 优先于 `--format`
    pub fn from_cli(format: &str, json: bool) -> Self {
        if json {
            Self::Json
        } else {
            format.parse().unwrap_or_default()
        }
    }

    /// 是否为 JSON 格式
    pub fn is_json(&self) -> bool {
        matches!(self, Self::Json)
    }

    /// 是否为机器可读格式（JSON / Markdown）
    ///
    /// 用于决定是否跳过交互式 UI 元素（spinner、step 提示等）。
    pub fn is_machine_readable(&self) -> bool {
        matches!(self, Self::Json | Self::Markdown)
    }

    /// 获取有效的 colored 设置（机器可读格式禁用颜色）
    pub fn effective_colored(&self, config_colored: bool) -> bool {
        if self.is_machine_readable() {
            false
        } else {
            config_colored
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_cli_json_flag() {
        assert_eq!(OutputFormat::from_cli("text", true), OutputFormat::Json);
        assert_eq!(OutputFormat::from_cli("markdown", true), OutputFormat::Json);
    }

    #[test]
    fn test_from_cli_format_string() {
        assert_eq!(OutputFormat::from_cli("json", false), OutputFormat::Json);
        assert_eq!(
            OutputFormat::from_cli("markdown", false),
            OutputFormat::Markdown
        );
        assert_eq!(OutputFormat::from_cli("md", false), OutputFormat::Markdown);
        assert_eq!(OutputFormat::from_cli("text", false), OutputFormat::Text);
        assert_eq!(OutputFormat::from_cli("unknown", false), OutputFormat::Text);
    }

    #[test]
    fn test_effective_colored() {
        assert!(!OutputFormat::Json.effective_colored(true));
        assert!(!OutputFormat::Markdown.effective_colored(true));
        assert!(OutputFormat::Text.effective_colored(true));
        assert!(!OutputFormat::Text.effective_colored(false));
    }

    #[test]
    fn test_is_machine_readable() {
        assert!(OutputFormat::Json.is_machine_readable());
        assert!(OutputFormat::Markdown.is_machine_readable());
        assert!(!OutputFormat::Text.is_machine_readable());
    }
}
