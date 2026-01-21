//! CLI argument definitions and subcommands.
//!
//! This module is organized into submodules for better maintainability:
//! - [`enums`]: ValueEnum types (OutputFormat, SortField, etc.)
//! - [`global`]: Global CLI structures and STYLES constant
//! - [`sync`]: Sync subcommand arguments
//! - [`commands`]: Individual command argument structs

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

// Individual command arguments
pub use commands::{
    ConfigAction, ConfigArgs, ConvertArgs, CopyArgs, DownloadArgs, EnvAction,
    EnvArgs, FindArgs, InfoArgs, InitArgs, JobsAction, JobsArgs, ListArgs,
    StatsArgs, TreeArgs, UpdateArgs, ValidateArgs, WatchArgs,
};

// Sync-related arguments (including re-exported data types)
// TODO: PdbjDataType and PdbeDataType will be used in sync command implementation
#[allow(unused_imports)]
pub use sync::{PdbjDataType, PdbjSyncArgs, PdbeDataType, PdbeSyncArgs, ShortcutSyncArgs,
               SyncArgs, SyncCommand, WwpdbSyncArgs};
