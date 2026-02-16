//! Configuration loading, merging, and validation.
//!
//! This module exposes the public configuration API used across command flows,
//! provider initialization, and runtime behavior.

mod global;
mod loader;
mod structs;

#[cfg(test)]
mod tests;

// Public API exports.
pub use global::{get_config, init_config};
pub use loader::{get_config_dir, load_config};
pub use structs::{
    ApiStyle, AppConfig, CommitConfig, CommitConvention, ConventionStyle, FileConfig, LLMConfig,
    NetworkConfig, ProviderConfig, ReviewConfig, UIConfig,
};
