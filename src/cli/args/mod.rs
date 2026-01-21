//! CLI argument definitions and subcommands.
//!
//! This module is organized into submodules for better maintainability:
//! - [`enums`]: ValueEnum types (OutputFormat, SortField, etc.)
//! - [`global`]: Global CLI structures and STYLES constant
//! - [`sync`]: Sync subcommand arguments
//! - [`commands`]: Individual command argument structs
//! - [`parsers`]: Custom value parsers and validators

mod enums;
mod global;
mod sync;
mod commands;

// Re-export public types for backward compatibility
pub use enums::{
    ExperimentalMethod, NotifyMethod, OutputFormat, SortField, SyncFormat,
};
// TODO: These exports will be used in Phase 3 (shared arg groups)
#[allow(unused_imports)]
pub use global::{Cli, Commands, GlobalArgs, PdbDirArgs, parse_cli, STYLES};
pub use commands::*;
pub use sync::*;
