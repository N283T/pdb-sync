//! CLI argument definitions and subcommands.
//!
//! This module is organized into submodules for better maintainability:
//! - [`enums`]: ValueEnum types (OutputFormat, SortField, etc.)
//! - [`global`]: Global CLI structures and STYLES constant
//! - [`sync`]: Sync subcommand arguments
//! - [`commands`]: Individual command argument structs

mod commands;
mod enums;
mod global;
mod parsers;
mod sync;

// Re-export public types for backward compatibility
pub use enums::{ExperimentalMethod, NotifyMethod, OutputFormat, SortField, SyncFormat};

// Re-export validation functions for use in argument structs
#[allow(unused_imports)]
pub use parsers::{validate_interval, validate_organism, validate_resolution};

// Re-export global CLI types (for external use by main.rs)
pub use global::{parse_cli, Cli, Commands};

// Shared argument group types (for internal use by commands module)
pub use global::{DryRunArgs, MirrorArgs, PdbDirArgs, ProgressArgs};

// Individual command arguments
pub use commands::{
    ConfigAction, ConfigArgs, ConvertArgs, CopyArgs, DownloadArgs, EnvAction, EnvArgs, FindArgs,
    InfoArgs, InitArgs, JobsAction, JobsArgs, ListArgs, StatsArgs, TreeArgs, UpdateArgs,
    ValidateArgs, WatchArgs,
};

// Sync-related arguments (including re-exported data types)
// TODO: PdbjDataType and PdbeDataType will be used in sync command implementation
#[allow(unused_imports)]
pub use sync::{
    PdbeDataType, PdbeSyncArgs, PdbjDataType, PdbjSyncArgs, ShortcutSyncArgs, SyncArgs,
    SyncCommand, WwpdbSyncArgs,
};
