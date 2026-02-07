//! 命令选项结构体
//!
//! 为各命令提供统一的参数传递方式，从 CLI 参数解析后构造。
//!
//! # 设计理念
//! - 使用引用避免 clone（性能优化）
//! - 统一的 `effective_colored()` 方法处理输出格式
//! - 集中管理所有命令的选项，便于维护
//!
//! # 示例
//! ```no_run
//! use gcop_rs::commands::options::CommitOptions;
//! use gcop_rs::commands::format::OutputFormat;
//!
//! let options = CommitOptions {
//!     no_edit: false,
//!     yes: false,
//!     dry_run: true,
//!     format: OutputFormat::Text,
//!     feedback: &[],
//!     verbose: false,
//!     provider_override: None,
//! };
//! ```

use super::format::OutputFormat;
use crate::cli::{Cli, ReviewTarget};
use crate::config::AppConfig;

/// Commit 命令选项
///
/// 从 CLI 参数构造后传递给 `commands::commit::run()`。
///
/// # 字段说明
/// - `no_edit`: 跳过编辑器交互（直接使用生成的 message）
/// - `yes`: 自动接受生成的 message（跳过确认）
/// - `dry_run`: 只生成 message，不执行 commit
/// - `format`: 输出格式（Text/JSON）
/// - `feedback`: 初始反馈/指令（如 "use Chinese", "be concise"）
/// - `verbose`: 详细模式（显示 API 请求/响应）
/// - `provider_override`: 覆盖配置中的 provider（如 `--provider openai`）
///
/// # 示例
/// ```no_run
/// use gcop_rs::commands::options::CommitOptions;
/// use gcop_rs::commands::format::OutputFormat;
///
/// let options = CommitOptions {
///     no_edit: false,
///     yes: true,  // 自动接受
///     dry_run: false,
///     format: OutputFormat::Text,
///     feedback: &["use conventional commits".to_string()],
///     verbose: false,
///     provider_override: None,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct CommitOptions<'a> {
    /// 是否跳过编辑器交互
    pub no_edit: bool,

    /// 是否跳过确认（自动接受）
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
    ///
    /// # 参数
    /// - `cli`: 解析的 CLI 参数
    /// - `no_edit`: `--no-edit` flag
    /// - `yes`: `--yes` flag
    /// - `dry_run`: `--dry-run` flag
    /// - `format`: `--format` 参数（"text", "json"）
    /// - `json`: `--json` flag（`--format json` 的简写）
    /// - `feedback`: 位置参数 `FEEDBACK...`（用于附加生成指令）
    ///
    /// # 返回
    /// 构造好的 `CommitOptions` 实例
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
    ///
    /// 结合输出格式和配置文件的 colored 设置。
    ///
    /// # 参数
    /// - `config`: 应用配置
    ///
    /// # 返回
    /// - `true` - 启用颜色输出
    /// - `false` - 禁用颜色输出
    ///
    /// # 规则
    /// - JSON 格式：始终禁用颜色
    /// - Text 格式：使用配置文件的 `ui.colored` 设置
    pub fn effective_colored(&self, config: &AppConfig) -> bool {
        self.format.effective_colored(config.ui.colored)
    }
}

/// Review 命令选项
///
/// 从 CLI 参数构造后传递给 `commands::review::run()`。
///
/// # 字段说明
/// - `target`: 审查目标（未提交变更/单个 commit/范围/文件）
/// - `format`: 输出格式
/// - `verbose`: 详细模式（当前未使用，预留）
/// - `provider_override`: 覆盖配置中的 provider
///
/// # 示例
/// ```no_run
/// use gcop_rs::commands::options::ReviewOptions;
/// use gcop_rs::commands::format::OutputFormat;
/// use gcop_rs::cli::ReviewTarget;
///
/// let target = ReviewTarget::Changes;
/// let options = ReviewOptions {
///     target: &target,
///     format: OutputFormat::Text,
///     verbose: false,
///     provider_override: None,
/// };
/// ```
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
    ///
    /// # 参数
    /// - `cli`: 解析的 CLI 参数
    /// - `target`: 审查目标
    /// - `format`: `--format` 参数
    /// - `json`: `--json` flag
    ///
    /// # 返回
    /// 构造好的 `ReviewOptions` 实例
    pub fn from_cli(cli: &'a Cli, target: &'a ReviewTarget, format: &str, json: bool) -> Self {
        Self {
            target,
            format: OutputFormat::from_cli(format, json),
            verbose: cli.verbose,
            provider_override: cli.provider.as_deref(),
        }
    }

    /// 获取有效的 colored 设置
    ///
    /// # 参数
    /// - `config`: 应用配置
    ///
    /// # 返回
    /// - `true` - 启用颜色输出
    /// - `false` - 禁用颜色输出
    pub fn effective_colored(&self, config: &AppConfig) -> bool {
        self.format.effective_colored(config.ui.colored)
    }
}

/// Stats 命令选项
///
/// 从 CLI 参数构造后传递给 `commands::stats::run()`。
///
/// # 字段说明
/// - `format`: 输出格式
/// - `author`: 按作者过滤（可选）
///
/// # 示例
/// ```no_run
/// use gcop_rs::commands::options::StatsOptions;
/// use gcop_rs::commands::format::OutputFormat;
///
/// let options = StatsOptions {
///     format: OutputFormat::Markdown,
///     author: Some("alice@example.com"),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct StatsOptions<'a> {
    /// 输出格式
    pub format: OutputFormat,

    /// 按作者过滤
    pub author: Option<&'a str>,
}

impl<'a> StatsOptions<'a> {
    /// 从 CLI 参数构造
    ///
    /// # 参数
    /// - `format`: `--format` 参数
    /// - `json`: `--json` flag
    /// - `author`: `--author` 参数（可选）
    ///
    /// # 返回
    /// 构造好的 `StatsOptions` 实例
    pub fn from_cli(format: &str, json: bool, author: Option<&'a str>) -> Self {
        Self {
            format: OutputFormat::from_cli(format, json),
            author,
        }
    }

    /// 获取有效的 colored 设置
    ///
    /// # 参数
    /// - `config_colored`: 配置文件的 `ui.colored` 设置
    ///
    /// # 返回
    /// - `true` - 启用颜色输出
    /// - `false` - 禁用颜色输出
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
