//! command option structure
//!
//! Provide a unified parameter passing method for each command, which is constructed from CLI parameter parsing.
//!
//! # Design
//! - Use references to avoid clones (performance optimization)
//! - Unified `effective_colored()` method to handle output format
//! - Centrally manage all command options for easy maintenance
//!
//! # Example
//! ```no_run
//! use gcop_rs::commands::options::CommitOptions;
//! use gcop_rs::commands::format::OutputFormat;
//!
//! let options = CommitOptions {
//!     no_edit: false,
//!     yes: false,
//!     dry_run: true,
//!     split: false,
//!     format: OutputFormat::Text,
//!     feedback: &[],
//!     verbose: false,
//!     provider_override: None,
//! };
//! ```

use super::format::OutputFormat;
use crate::cli::{Cli, ReviewTarget};
use crate::config::AppConfig;

/// Commit command options
///
/// Constructed from CLI parameters and passed to `commands::commit::run()`.
///
/// # Field description
/// - `no_edit`: skip editor interaction (use the generated message directly)
/// - `yes`: automatically accept the generated message (skip confirmation)
/// - `dry_run`: only generates message and does not execute commit
/// - `format`: output format (Text/JSON)
/// - `feedback`: initial feedback/instruction (such as "use Chinese", "be concise")
/// - `verbose`: verbose mode (display API requests/responses)
/// - `provider_override`: override the provider in the configuration (such as `--provider openai`)
///
/// # Example
/// ```no_run
/// use gcop_rs::commands::options::CommitOptions;
/// use gcop_rs::commands::format::OutputFormat;
///
/// let options = CommitOptions {
///     no_edit: false,
///     yes: true, // automatically accepted
///     dry_run: false,
///     split: false,
///     format: OutputFormat::Text,
///     feedback: &["use conventional commits".to_string()],
///     verbose: false,
///     provider_override: None,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct CommitOptions<'a> {
    /// Whether to skip editor interaction
    pub no_edit: bool,

    /// Whether to skip confirmation (auto-accept)
    pub yes: bool,

    /// Whether to only generate and not submit
    pub dry_run: bool,

    /// Whether to use split (atomic) commit mode
    pub split: bool,

    /// Output format
    pub format: OutputFormat,

    /// Initial feedback/instructions (quotes, avoid clones)
    pub feedback: &'a [String],

    /// Whether to use verbose mode
    pub verbose: bool,

    /// Covered providers
    pub provider_override: Option<&'a str>,
}

impl<'a> CommitOptions<'a> {
    /// Constructed from CLI parameters
    ///
    /// # Parameters
    /// - `cli`: parsed CLI parameters
    /// - `no_edit`: `--no-edit` flag
    /// - `yes`: `--yes` flag
    /// - `dry_run`: `--dry-run` flag
    /// - `format`: `--format` parameter ("text", "json")
    /// - `json`: `--json` flag (short for `--format json`)
    /// - `feedback`: positional parameter `FEEDBACK...` (for additional generation instructions)
    ///
    /// # Returns
    /// Constructed `CommitOptions` instance
    #[allow(clippy::too_many_arguments)]
    pub fn from_cli(
        cli: &'a Cli,
        no_edit: bool,
        yes: bool,
        dry_run: bool,
        split: bool,
        format: &str,
        json: bool,
        feedback: &'a [String],
        config: &AppConfig,
    ) -> Self {
        Self {
            no_edit,
            yes,
            dry_run,
            split: split || config.commit.split,
            format: OutputFormat::from_cli(format, json),
            feedback,
            verbose: cli.verbose,
            provider_override: cli.provider.as_deref(),
        }
    }

    /// Get valid colored settings
    ///
    /// Combines the output format and the colored setting of the configuration file.
    ///
    /// # Parameters
    /// - `config`: application configuration
    ///
    /// # Returns
    /// - `true` - enable color output
    /// - `false` - disable color output
    ///
    /// # rule
    /// - JSON format: always disable colors
    /// - Text format: Use the `ui.colored` setting of the configuration file
    pub fn effective_colored(&self, config: &AppConfig) -> bool {
        self.format.effective_colored(config.ui.colored)
    }
}

/// Review command options
///
/// Constructed from CLI parameters and passed to `commands::review::run()`.
///
/// # Field description
/// - `target`: review target (unstaged changes/single commit/scope/file)
/// - `format`: output format
/// - `verbose`: verbose mode (currently not used, reserved)
/// - `provider_override`: override the provider in the configuration
///
/// # Example
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
    /// review goals
    pub target: &'a ReviewTarget,

    /// Output format
    pub format: OutputFormat,

    /// Whether to use verbose mode
    // TODO: Currently the review command does not use verbose. Detailed output may need to be added in the future.
    #[allow(dead_code)]
    pub verbose: bool,

    /// Covered providers
    pub provider_override: Option<&'a str>,
}

impl<'a> ReviewOptions<'a> {
    /// Constructed from CLI parameters
    ///
    /// # Parameters
    /// - `cli`: parsed CLI parameters
    /// - `target`: review target
    /// - `format`: `--format` parameter
    /// - `json`: `--json` flag
    ///
    /// # Returns
    /// Constructed `ReviewOptions` instance
    pub fn from_cli(cli: &'a Cli, target: &'a ReviewTarget, format: &str, json: bool) -> Self {
        Self {
            target,
            format: OutputFormat::from_cli(format, json),
            verbose: cli.verbose,
            provider_override: cli.provider.as_deref(),
        }
    }

    /// Get valid colored settings
    ///
    /// # Parameters
    /// - `config`: application configuration
    ///
    /// # Returns
    /// - `true` - enable color output
    /// - `false` - disable color output
    pub fn effective_colored(&self, config: &AppConfig) -> bool {
        self.format.effective_colored(config.ui.colored)
    }
}

/// Stats command options
///
/// Constructed from CLI parameters and passed to `commands::stats::run()`.
///
/// # Field description
/// - `format`: output format
/// - `author`: filter by author (optional)
///
/// # Example
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
    /// Output format
    pub format: OutputFormat,

    /// Filter by author
    pub author: Option<&'a str>,
}

impl<'a> StatsOptions<'a> {
    /// Constructed from CLI parameters
    ///
    /// # Parameters
    /// - `format`: `--format` parameter
    /// - `json`: `--json` flag
    /// - `author`: `--author` parameter (optional)
    ///
    /// # Returns
    /// Constructed `StatsOptions` instance
    pub fn from_cli(format: &str, json: bool, author: Option<&'a str>) -> Self {
        Self {
            format: OutputFormat::from_cli(format, json),
            author,
        }
    }

    /// Get valid colored settings
    ///
    /// # Parameters
    /// - `config_colored`: `ui.colored` setting of configuration file
    ///
    /// # Returns
    /// - `true` - enable color output
    /// - `false` - disable color output
    pub fn effective_colored(&self, config_colored: bool) -> bool {
        self.format.effective_colored(config_colored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_cli() -> Cli {
        Cli {
            command: crate::cli::Commands::Init {
                force: false,
                project: false,
            },
            verbose: true,
            provider: Some("test-provider".to_string()),
        }
    }

    fn mock_config() -> AppConfig {
        AppConfig::default()
    }

    #[test]
    fn test_commit_options_from_cli() {
        let cli = mock_cli();
        let config = mock_config();
        let feedback = vec!["use conventional commits".to_string()];
        let opts = CommitOptions::from_cli(
            &cli, false, true, true, false, "text", false, &feedback, &config,
        );

        assert!(!opts.no_edit);
        assert!(opts.yes);
        assert!(opts.dry_run);
        assert!(!opts.split);
        assert_eq!(opts.format, OutputFormat::Text);
        assert_eq!(opts.feedback.len(), 1);
        assert!(opts.verbose);
        assert_eq!(opts.provider_override, Some("test-provider"));
    }

    #[test]
    fn test_commit_options_json_flag() {
        let cli = mock_cli();
        let config = mock_config();
        let feedback: Vec<String> = vec![];
        let opts = CommitOptions::from_cli(
            &cli, false, false, false, false, "text", true, &feedback, &config,
        );

        assert_eq!(opts.format, OutputFormat::Json);
    }

    #[test]
    fn test_commit_options_split_from_config() {
        let cli = mock_cli();
        let mut config = mock_config();
        config.commit.split = true;
        let feedback: Vec<String> = vec![];
        let opts = CommitOptions::from_cli(
            &cli, false, false, false, false, "text", false, &feedback, &config,
        );

        // CLI --split=false, but config.commit.split=true → split enabled
        assert!(opts.split);
    }

    #[test]
    fn test_commit_options_split_cli_overrides() {
        let cli = mock_cli();
        let config = mock_config(); // split defaults to false
        let feedback: Vec<String> = vec![];
        let opts = CommitOptions::from_cli(
            &cli, false, false, false, true, "text", false, &feedback, &config,
        );

        // CLI --split=true overrides config
        assert!(opts.split);
    }

    #[test]
    fn test_stats_options() {
        let opts = StatsOptions::from_cli("markdown", false, Some("author@example.com"));

        assert_eq!(opts.format, OutputFormat::Markdown);
        assert_eq!(opts.author, Some("author@example.com"));
    }
}
