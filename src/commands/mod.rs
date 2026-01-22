pub mod alias;
pub mod commit;
pub mod commit_state_machine;
pub mod config;
pub mod format;
pub mod init;
pub mod json;
pub mod options;
pub mod review;
pub mod stats;

// Re-export for external use (tests, lib users)
#[allow(unused_imports)]
pub use format::OutputFormat;
pub use options::{CommitOptions, ReviewOptions, StatsOptions};
