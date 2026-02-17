use clap::{Parser, Subcommand, builder::styling};

const STYLES: styling::Styles = styling::Styles::styled()
    .header(styling::AnsiColor::Green.on_default().bold())
    .usage(styling::AnsiColor::Green.on_default().bold())
    .literal(styling::AnsiColor::Cyan.on_default().bold())
    .placeholder(styling::AnsiColor::Cyan.on_default());

#[derive(Parser)]
#[command(name = "gcop-rs")]
#[command(author, version, long_about = None)]
#[command(styles = STYLES)]
/// Top-level CLI options shared by all subcommands.
pub struct Cli {
    /// Selected subcommand and its arguments.
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output.
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Override the default LLM provider (used by `commit` and `review`).
    #[arg(short, long, global = true)]
    pub provider: Option<String>,
}

#[derive(Subcommand)]
/// Supported gcop-rs subcommands.
pub enum Commands {
    /// Generate a commit message for staged changes.
    Commit {
        /// Skip the interactive editor.
        #[arg(short, long)]
        no_edit: bool,

        /// Skip confirmation before committing.
        #[arg(short = 'y', long)]
        yes: bool,

        /// Generate and print a commit message without creating a commit.
        #[arg(short, long)]
        dry_run: bool,

        /// Output format: `text` or `json` (`json` implies `--dry-run`).
        #[arg(short, long, default_value = "text")]
        format: String,

        /// Shortcut for `--format json`.
        #[arg(long)]
        json: bool,

        /// Split staged changes into multiple atomic commits.
        #[arg(short = 's', long)]
        split: bool,

        /// Feedback or constraints passed to commit message generation.
        #[arg(trailing_var_arg = true)]
        feedback: Vec<String>,
    },

    /// Review code changes.
    Review {
        /// Review target.
        #[command(subcommand)]
        target: ReviewTarget,

        /// Output format: `text`, `json`, or `markdown`.
        #[arg(short, long, default_value = "text")]
        format: String,

        /// Shortcut for `--format json`.
        #[arg(long)]
        json: bool,
    },

    /// Initialize a configuration file.
    Init {
        /// Force overwriting existing config.
        #[arg(short, long)]
        force: bool,

        /// Initialize `.gcop/config.toml` in the current repository root.
        #[arg(long)]
        project: bool,
    },

    /// Manage configuration.
    Config {
        /// Optional configuration action. If omitted, defaults to interactive edit flow.
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },

    /// Manage Git aliases.
    Alias {
        /// Force overwriting existing aliases.
        #[arg(short, long)]
        force: bool,

        /// List all available aliases and their status.
        #[arg(short, long)]
        list: bool,

        /// Remove all gcop-related aliases.
        #[arg(short, long)]
        remove: bool,
    },

    /// Show repository statistics.
    Stats {
        /// Output format: `text`, `json`, or `markdown`.
        #[arg(short, long, default_value = "text")]
        format: String,

        /// Shortcut for `--format json`.
        #[arg(long)]
        json: bool,

        /// Filter by author name or email.
        #[arg(long)]
        author: Option<String>,
    },

    /// Manage git hooks (prepare-commit-msg)
    Hook {
        /// Hook action to run.
        #[command(subcommand)]
        action: HookAction,
    },
}

#[derive(Subcommand, Debug)]
/// Target scope for the `review` command.
pub enum ReviewTarget {
    /// Review unstaged working tree changes (`index -> workdir`).
    Changes,

    /// Review a specific commit.
    Commit {
        /// Commit hash.
        hash: String,
    },

    /// Review a range of commits.
    Range {
        /// Commit range (for example `main..feature`).
        range: String,
    },

    /// Review a specific file.
    File {
        /// Path to file.
        path: String,
    },
}

#[derive(Subcommand)]
/// Actions for the `config` command.
pub enum ConfigAction {
    /// Edit the configuration file.
    Edit,

    /// Validate config and test provider-chain connectivity.
    Validate,
}

#[derive(Subcommand)]
/// Actions for the `hook` command.
pub enum HookAction {
    /// Install the `prepare-commit-msg` hook in the current repository.
    Install {
        /// Force overwriting an existing hook.
        #[arg(short, long)]
        force: bool,
    },

    /// Uninstall the `prepare-commit-msg` hook from the current repository.
    Uninstall,

    /// Run hook logic (called by Git, not intended for direct use).
    #[command(hide = true)]
    Run {
        /// Path to the commit message file (provided by Git).
        commit_msg_file: String,

        /// Source of the commit message.
        #[arg(default_value = "")]
        source: String,

        /// Commit SHA (for amend).
        #[arg(default_value = "")]
        sha: String,
    },
}
