//! Terminal UI utilities.
//!
//! Provides reusable components for terminal interaction.
//!
//! # Modules
//! - `colors` - Colored output helpers.
//! - `editor` - External editor integration.
//! - `prompt` - Interactive prompts (confirm/menu/input).
//! - `spinner` - Progress spinner.
//! - `streaming` - Streaming text renderer (typewriter effect).
//!
//! # Example
//! ```ignore
//! use gcop_rs::ui::{Spinner, success};
//!
//! // Show a spinner.
//! let spinner = Spinner::new("Generating commit message...", true);
//! spinner.set_message("Almost done...");
//! // The spinner is cleaned up automatically on drop.
//!
//! // Colored output.
//! success("Commit successful!", true);
//! ```

/// Colored terminal message helpers.
pub mod colors;
/// External editor integration utilities.
pub mod editor;
/// Interactive prompt helpers for commit/review flows.
pub mod prompt;
/// Spinner/progress indicator implementation.
pub mod spinner;
/// Streaming text output helpers.
pub mod streaming;

pub use colors::*;
pub use editor::*;
pub use prompt::{CommitAction, commit_action_menu, confirm, get_retry_feedback};
pub use spinner::*;
pub use streaming::*;
