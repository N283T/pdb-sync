//! Sync command handler with subcommands for different data sources.
//!
//! This module handles the `pdb-sync sync` command and its subcommands:
//! - `sync` (no subcommand): Legacy mode, backward compatible
//! - `sync wwpdb`: Standard wwPDB data from any mirror
//! - `sync structures`: Shortcut for `wwpdb --type structures`
//! - `sync assemblies`: Shortcut for `wwpdb --type assemblies`
//! - `sync pdbj`: PDBj-specific data (EMDB, PDB-IHM, derived)
//! - `sync pdbe`: PDBe-specific data (SIFTS, PDBeChem, Foldseek)

mod common;
mod pdbe;
mod pdbj;
mod wwpdb;

use crate::cli::args::{SyncArgs, SyncCommand};
use crate::context::AppContext;
use crate::error::Result;

/// Main entry point for the sync command.
///
/// Routes to the appropriate handler based on the subcommand.
pub async fn run_sync(args: SyncArgs, ctx: AppContext) -> Result<()> {
    match &args.command {
        Some(SyncCommand::Wwpdb(wwpdb_args)) => wwpdb::run(wwpdb_args.clone(), &args, ctx).await,
        Some(SyncCommand::Structures(shortcut_args)) => {
            wwpdb::run_structures(shortcut_args.clone(), &args, ctx).await
        }
        Some(SyncCommand::Assemblies(shortcut_args)) => {
            wwpdb::run_assemblies(shortcut_args.clone(), &args, ctx).await
        }
        Some(SyncCommand::Pdbj(pdbj_args)) => pdbj::run(pdbj_args.clone(), &args, ctx).await,
        Some(SyncCommand::Pdbe(pdbe_args)) => pdbe::run(pdbe_args.clone(), &args, ctx).await,
        None => {
            // Legacy mode: no subcommand specified
            // Treat as wwpdb sync with top-level args
            wwpdb::run_legacy(args, ctx).await
        }
    }
}
