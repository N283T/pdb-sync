//! CLI argument definitions.
//!
//! This module is organized into submodules for better maintainability:
//! - [`enums`]: ValueEnum types (OutputFormat, SyncFormat, etc.)
//! - [`global`]: Global CLI structures and STYLES constant
//! - [`sync`]: Sync command arguments
//! - [`config`]: Config command arguments

pub mod config;
mod enums;
pub mod env;
mod global;
pub mod sync;

// Re-export global CLI types (for external use by main.rs)
pub use global::{parse_cli, SyncCommand};

// Sync-related arguments
pub use sync::SyncArgs;
