use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "gcop-rs")]
#[command(author, version, about = "Git Copilot in Rust", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Override default LLM provider
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
    },

    /// Review code changes
    Review {
        /// What to review
        #[command(subcommand)]
        target: ReviewTarget,

        /// Output format: text | json | markdown
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Initialize gcop-rs
    Init {
        #[command(subcommand)]
        action: Option<InitAction>,
    },

    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
}

#[derive(Subcommand)]
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

    /// Review a file or directory
    File {
        /// Path to file or directory
        path: String,
    },
}

#[derive(Subcommand)]
pub enum InitAction {
    /// Initialize configuration file
    Config {
        /// Force overwrite existing config
        #[arg(short, long)]
        force: bool,
    },

    /// Add git alias for 'git cop'
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
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Edit configuration file
    Edit,

    /// Validate configuration and test provider connection
    Validate,
}
