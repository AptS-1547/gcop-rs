use std::str::FromStr;

/// Output format enum
///
/// Unified processing of `--format` and `--json` parameters in CLI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Human-readable terminal output.
    #[default]
    Text,
    /// Machine-readable JSON output.
    Json,
    /// Markdown report output.
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
    /// Parse output format from CLI parameters
    ///
    /// `--json` takes precedence over `--format`
    pub fn from_cli(format: &str, json: bool) -> Self {
        if json {
            Self::Json
        } else {
            format.parse().unwrap_or_default()
        }
    }

    /// Is it in JSON format?
    pub fn is_json(&self) -> bool {
        matches!(self, Self::Json)
    }

    /// Is it in a machine-readable format (JSON/Markdown)
    ///
    /// Used to decide whether to skip interactive UI elements (spinner, step prompt, etc.).
    pub fn is_machine_readable(&self) -> bool {
        matches!(self, Self::Json | Self::Markdown)
    }

    /// Get the effective colored setting (color disabled in machine-readable format)
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
