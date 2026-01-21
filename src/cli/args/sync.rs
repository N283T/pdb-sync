//! Sync command arguments and subcommands.

use crate::data_types::{DataType, Layout};
use clap::{Parser, Subcommand};

use super::enums::SyncFormat;

// Re-export data types for public API
pub use crate::data_types::{PdbeDataType, PdbjDataType};

/// Validate subpath to prevent path traversal attacks.
fn validate_subpath(s: &str) -> Result<String, String> {
    if s.contains("..") {
        return Err("Invalid subpath: must not contain '..'".into());
    }
    if s.starts_with('/') || s.starts_with('\\') {
        return Err("Invalid subpath: must be relative path".into());
    }
    if s.contains('\\') {
        return Err("Invalid subpath: must use forward slashes".into());
    }
    Ok(s.to_string())
}

/// Sync command with subcommands for different data sources.
///
/// The sync command supports multiple data sources:
/// - wwpdb (default): Standard wwPDB data available from all mirrors
/// - structures: Shortcut for `wwpdb --type structures`
/// - assemblies: Shortcut for `wwpdb --type assemblies`
/// - pdbj: PDBj-specific data (EMDB, PDB-IHM, derived data)
/// - pdbe: PDBe-specific data (SIFTS, PDBeChem, Foldseek)
#[derive(Parser, Clone)]
pub struct SyncArgs {
    /// Subcommand for sync target (defaults to wwpdb if omitted)
    #[command(subcommand)]
    pub command: Option<SyncCommand>,

    // === Legacy/backward-compatible options ===
    // These are used when no subcommand is specified (e.g., `pdb-sync sync --type structures`)
    /// Mirror to sync from
    #[arg(short, long, value_enum, global = true)]
    pub mirror: Option<crate::mirrors::MirrorId>,

    /// Data types to sync (can specify multiple times)
    #[arg(short = 't', long = "type", value_enum)]
    pub data_types: Vec<DataType>,

    /// File format to sync
    #[arg(short, long, value_enum, default_value = "mmcif")]
    pub format: SyncFormat,

    /// Directory layout (divided or all)
    #[arg(short, long, value_enum, default_value = "divided")]
    pub layout: Layout,

    /// Destination directory
    #[arg(short, long, global = true)]
    pub dest: Option<std::path::PathBuf>,

    /// Delete files not present on the remote
    #[arg(long, global = true)]
    pub delete: bool,

    /// Bandwidth limit in KB/s
    #[arg(long, global = true)]
    pub bwlimit: Option<u32>,

    /// Perform a dry run without making changes
    #[arg(short = 'n', long, global = true)]
    pub dry_run: bool,

    /// Show detailed progress
    #[arg(short = 'P', long, global = true)]
    pub progress: bool,

    /// Filter patterns (PDB ID prefixes)
    #[arg(trailing_var_arg = true)]
    pub filters: Vec<String>,

    /// Run in background
    #[arg(long)]
    pub bg: bool,
}

/// Sync subcommands for different data sources.
#[derive(Subcommand, Debug, Clone)]
pub enum SyncCommand {
    /// Sync wwPDB standard data (structures, assemblies, etc.)
    #[command(name = "wwpdb")]
    Wwpdb(WwpdbSyncArgs),

    /// Shortcut: sync structures (equivalent to `wwpdb --type structures`)
    Structures(ShortcutSyncArgs),

    /// Shortcut: sync assemblies (equivalent to `wwpdb --type assemblies`)
    Assemblies(ShortcutSyncArgs),

    /// Sync PDBj-specific data (EMDB, PDB-IHM, derived data)
    #[command(name = "pdbj")]
    Pdbj(PdbjSyncArgs),

    /// Sync PDBe-specific data (SIFTS, PDBeChem, Foldseek)
    #[command(name = "pdbe")]
    Pdbe(PdbeSyncArgs),
}

/// Arguments for wwPDB standard data sync.
#[derive(Parser, Debug, Clone)]
pub struct WwpdbSyncArgs {
    /// Data types to sync (can specify multiple times)
    #[arg(short = 't', long = "type", value_enum)]
    pub data_types: Vec<DataType>,

    /// File format to sync
    #[arg(short, long, value_enum, default_value = "mmcif")]
    pub format: SyncFormat,

    /// Directory layout (divided or all)
    #[arg(short, long, value_enum, default_value = "divided")]
    pub layout: Layout,

    /// Filter patterns (PDB ID prefixes)
    #[arg(trailing_var_arg = true)]
    pub filters: Vec<String>,
}

/// Arguments for shortcut commands (structures, assemblies).
/// These commands fix the data type but allow other options.
#[derive(Parser, Debug, Clone)]
pub struct ShortcutSyncArgs {
    /// File format to sync
    #[arg(short, long, value_enum, default_value = "mmcif")]
    pub format: SyncFormat,

    /// Directory layout (divided or all)
    #[arg(short, long, value_enum, default_value = "divided")]
    pub layout: Layout,

    /// Filter patterns (PDB ID prefixes)
    #[arg(trailing_var_arg = true)]
    pub filters: Vec<String>,
}

/// Arguments for PDBj-specific data sync.
#[derive(Parser, Debug, Clone)]
pub struct PdbjSyncArgs {
    /// PDBj data types to sync (required)
    #[arg(short = 't', long = "type", value_enum, required = true)]
    pub data_types: Vec<PdbjDataType>,

    /// Subpath within the data type (optional, for partial sync)
    #[arg(long, value_parser = validate_subpath)]
    pub subpath: Option<String>,
}

/// Arguments for PDBe-specific data sync.
#[derive(Parser, Debug, Clone)]
pub struct PdbeSyncArgs {
    /// PDBe data types to sync (required)
    #[arg(short = 't', long = "type", value_enum, required = true)]
    pub data_types: Vec<PdbeDataType>,

    /// Subpath within the data type (optional, for partial sync)
    #[arg(long, value_parser = validate_subpath)]
    pub subpath: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_subpath() {
        // Valid subpaths
        assert!(validate_subpath("data/structures").is_ok());
        assert!(validate_subpath("2024").is_ok());
        assert!(validate_subpath("structures/divided").is_ok());

        // Path traversal attempts
        assert!(validate_subpath("../etc").is_err());
        assert!(validate_subpath("data/../parent").is_err());

        // Absolute paths
        assert!(validate_subpath("/etc/passwd").is_err());
        assert!(validate_subpath("\\windows\\system32").is_err());

        // Backslashes
        assert!(validate_subpath("data\\structures").is_err());
    }
}
