//! # gcop-rs
//!
//! AI-powered Git tool for generating commit messages and code reviews.
//!
//! This is a Rust rewrite of the original Python project [gcop](https://github.com/Undertone0809/gcop).
//!
//! ## Features
//! - **Commit message generation**: Generates messages from staged changes (Conventional Commits by default, configurable).
//! - **Code review**: Analyzes diffs to surface potential issues and improvement suggestions.
//! - **Multiple providers**: Claude, OpenAI, Gemini, and Ollama (local models).
//! - **High availability**: Built-in fallback chain when the primary provider fails.
//! - **Streaming output**: Real-time typewriter-style output (Claude/OpenAI/Gemini).
//! - **Internationalization**: Supports English and Chinese.
//!
//! ## Quick Start
//!
//! ### Use as a CLI
//! ```bash
//! # Install
//! cargo install gcop-rs
//!
//! # Initialize configuration
//! gcop-rs init
//!
//! # Generate commit message
//! git add .
//! gcop-rs commit
//!
//! # Code review
//! gcop-rs review changes
//! ```
//!
//! ### Use as a library
//! ```ignore
//! use gcop_rs::git::repository::GitRepository;
//! use gcop_rs::git::GitOperations;
//! use gcop_rs::llm::provider::openai::OpenAIProvider;
//! use gcop_rs::llm::LLMProvider;
//! use gcop_rs::config::{ProviderConfig, NetworkConfig};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // 1. Initialize Git repository
//! let repo = GitRepository::open(None)?;
//! let diff = repo.get_staged_diff()?;
//!
//! // 2. Initialize LLM provider
//! let config = ProviderConfig {
//!     api_key: Some("sk-...".to_string()),
//!     model: "gpt-4o-mini".to_string(),
//!     ..Default::default()
//! };
//! let network_config = NetworkConfig::default();
//! let provider = OpenAIProvider::new(&config, "openai", &network_config, false)?;
//!
//! // 3. Generate commit message
//! let message = provider.generate_commit_message(&diff, None, None).await?;
//! println!("Generated: {}", message);
//! # Ok(())
//! # }
//! ```
//!
//! ## Core Modules
//! - [`git`] - Git operation abstractions.
//! - [`llm`] - LLM provider traits and implementations.
//! - [`commands`] - CLI command implementations.
//! - [`config`] - Configuration loading and management.
//! - [`error`] - Unified error types.
//! - [`ui`] - Terminal UI utilities.
//!
//! ## Configuration
//! Configuration file locations:
//! - Linux: `~/.config/gcop/config.toml`
//! - macOS: `~/Library/Application Support/gcop/config.toml`
//! - Windows: `%APPDATA%\gcop\config\config.toml`
//! - Project-level (optional): `<repo>/.gcop/config.toml`
//!
//! Example configuration:
//! ```toml
//! [llm]
//! default_provider = "claude"
//! fallback_providers = ["openai"]
//!
//! [llm.providers.claude]
//! api_key = "sk-ant-..."
//! model = "claude-sonnet-4-5-20250929"
//!
//! [commit]
//! max_retries = 10
//! show_diff_preview = true
//! ```

#[macro_use]
extern crate rust_i18n;

/// Command-line argument definitions and parsing.
pub mod cli;
/// CLI command implementations and shared helpers.
pub mod commands;
/// Configuration loading, defaults, and validation.
pub mod config;
/// Unified error types and localization helpers.
pub mod error;
/// Git repository abstractions and operations.
pub mod git;
/// LLM traits, message types, prompts, and providers.
pub mod llm;
/// Terminal UI helpers (colors, prompts, spinner, streaming output).
pub mod ui;
/// Workspace detection and commit scope inference for monorepos.
pub mod workspace;

// Initialize i18n for library modules.
i18n!("locales", fallback = "en");
