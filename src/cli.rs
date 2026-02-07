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
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Override default LLM provider (used by commit/review)
    #[arg(short, long, global = true)]
    pub provider: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate commit message for staged changes
    Commit {
        /// Skip interactive editor
        #[arg(short, long)]
        no_edit: bool,

        /// Skip confirmation before committing
        #[arg(short = 'y', long)]
        yes: bool,

        /// Only generate and print commit message, do not commit
        #[arg(short, long)]
        dry_run: bool,

        /// Output format: text | json (json implies --dry-run)
        #[arg(short, long, default_value = "text")]
        format: String,

        /// Shortcut for --format json
        #[arg(long)]
        json: bool,

        /// Feedback/instruction for commit message generation
        #[arg(trailing_var_arg = true)]
        feedback: Vec<String>,
    },

    /// Review code changes
    Review {
        /// What to review
        #[command(subcommand)]
        target: ReviewTarget,

        /// Output format: text | json | markdown
        #[arg(short, long, default_value = "text")]
        format: String,

        /// Shortcut for --format json
        #[arg(long)]
        json: bool,
    },

    /// Initialize configuration file
    Init {
        /// Force overwrite existing config
        #[arg(short, long)]
        force: bool,
    },

    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },

    /// Manage git aliases
    Alias {
        /// Force overwrite existing aliases
        #[arg(short, long)]
        force: bool,

        /// List all available aliases and their status
        #[arg(short, long)]
        list: bool,

        /// Remove all gcop-related aliases
        #[arg(short, long)]
        remove: bool,
    },

    /// Show repository statistics
    Stats {
        /// Output format: text | json | markdown
        #[arg(short, long, default_value = "text")]
        format: String,

        /// Shortcut for --format json
        #[arg(long)]
        json: bool,

        /// Filter by author name or email
        #[arg(long)]
        author: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum ReviewTarget {
    /// Review uncommitted changes
    Changes,

    /// Review a specific commit
    Commit {
        /// Commit hash
        hash: String,
    },

    /// Review a range of commits
    Range {
        /// Commit range (e.g., main..feature)
        range: String,
    },

    /// Review a specific file
    File {
        /// Path to file
        path: String,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Edit configuration file
    Edit,

    /// Validate configuration and test provider chain connection
    Validate,
}
