//! 命令选项结构体
//!
//! 为各命令提供统一的参数传递方式

use super::format::OutputFormat;
use crate::cli::{Cli, ReviewTarget};
use crate::config::AppConfig;

/// Commit 命令选项
#[derive(Debug, Clone)]
pub struct CommitOptions<'a> {
    /// 是否跳过编辑
    pub no_edit: bool,

    /// 是否跳过确认
    pub yes: bool,

    /// 是否只生成不提交
    pub dry_run: bool,

    /// 输出格式
    pub format: OutputFormat,

    /// 初始反馈/指令（引用，避免 clone）
    pub feedback: &'a [String],

    /// 是否 verbose 模式
    pub verbose: bool,

    /// 覆盖的 provider
    pub provider_override: Option<&'a str>,
}

impl<'a> CommitOptions<'a> {
    /// 从 CLI 参数构造
    pub fn from_cli(
        cli: &'a Cli,
        no_edit: bool,
        yes: bool,
        dry_run: bool,
        format: &str,
        json: bool,
        feedback: &'a [String],
    ) -> Self {
        Self {
            no_edit,
            yes,
            dry_run,
            format: OutputFormat::from_cli(format, json),
            feedback,
            verbose: cli.verbose,
            provider_override: cli.provider.as_deref(),
        }
    }

    /// 获取有效的 colored 设置
    pub fn effective_colored(&self, config: &AppConfig) -> bool {
        self.format.effective_colored(config.ui.colored)
    }
}

/// Review 命令选项
#[derive(Debug, Clone)]
pub struct ReviewOptions<'a> {
    /// 审查目标
    pub target: &'a ReviewTarget,

    /// 输出格式
    pub format: OutputFormat,

    /// 是否 verbose 模式
    // TODO: 目前 review 命令未使用 verbose，未来可能需要添加详细输出
    #[allow(dead_code)]
    pub verbose: bool,

    /// 覆盖的 provider
    pub provider_override: Option<&'a str>,
}

impl<'a> ReviewOptions<'a> {
    /// 从 CLI 参数构造
    pub fn from_cli(cli: &'a Cli, target: &'a ReviewTarget, format: &str, json: bool) -> Self {
        Self {
            target,
            format: OutputFormat::from_cli(format, json),
            verbose: cli.verbose,
            provider_override: cli.provider.as_deref(),
        }
    }

    /// 获取有效的 colored 设置
    pub fn effective_colored(&self, config: &AppConfig) -> bool {
        self.format.effective_colored(config.ui.colored)
    }
}

/// Stats 命令选项
#[derive(Debug, Clone)]
pub struct StatsOptions<'a> {
    /// 输出格式
    pub format: OutputFormat,

    /// 按作者过滤
    pub author: Option<&'a str>,
}

impl<'a> StatsOptions<'a> {
    /// 从 CLI 参数构造
    pub fn from_cli(format: &str, json: bool, author: Option<&'a str>) -> Self {
        Self {
            format: OutputFormat::from_cli(format, json),
            author,
        }
    }

    /// 获取有效的 colored 设置
    pub fn effective_colored(&self, config_colored: bool) -> bool {
        self.format.effective_colored(config_colored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_cli() -> Cli {
        Cli {
            command: crate::cli::Commands::Init { force: false },
            verbose: true,
            provider: Some("test-provider".to_string()),
        }
    }

    #[test]
    fn test_commit_options_from_cli() {
        let cli = mock_cli();
        let feedback = vec!["use conventional commits".to_string()];
        let opts = CommitOptions::from_cli(&cli, false, true, true, "text", false, &feedback);

        assert!(!opts.no_edit);
        assert!(opts.yes);
        assert!(opts.dry_run);
        assert_eq!(opts.format, OutputFormat::Text);
        assert_eq!(opts.feedback.len(), 1);
        assert!(opts.verbose);
        assert_eq!(opts.provider_override, Some("test-provider"));
    }

    #[test]
    fn test_commit_options_json_flag() {
        let cli = mock_cli();
        let feedback: Vec<String> = vec![];
        let opts = CommitOptions::from_cli(&cli, false, false, false, "text", true, &feedback);

        assert_eq!(opts.format, OutputFormat::Json);
    }

    #[test]
    fn test_stats_options() {
        let opts = StatsOptions::from_cli("markdown", false, Some("author@example.com"));

        assert_eq!(opts.format, OutputFormat::Markdown);
        assert_eq!(opts.author, Some("author@example.com"));
    }
}
