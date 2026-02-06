#[macro_use]
extern crate rust_i18n;

pub mod cli;
pub mod commands;
pub mod config;
pub mod error;
pub mod git;
pub mod llm;
pub mod ui;

// Initialize i18n for library modules
i18n!("locales", fallback = "en");
