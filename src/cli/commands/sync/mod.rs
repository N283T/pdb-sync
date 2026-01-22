//! Sync command handler for custom rsync configurations.

pub mod common;
pub mod wwpdb;

use crate::context::AppContext;
use crate::error::Result;

/// Main entry point for the sync command.
///
/// Syncs PDB data from configured rsync sources.
/// - If `name` is `Some(...)`, runs that specific config
/// - If `name` is `None`, runs all configs (default behavior)
#[allow(dead_code)]
pub async fn run_sync(name: Option<String>, ctx: AppContext) -> Result<()> {
    use crate::cli::args::SyncArgs;

    // Create minimal args with default values
    let args = SyncArgs {
        name,
        all: false,
        progress: false,
        dest: None,
    };

    // Run sync based on whether name was specified
    if let Some(ref n) = args.name {
        wwpdb::run_custom(n.clone(), args, ctx).await
    } else {
        // No name specified, run all
        wwpdb::run_custom_all(args, ctx).await
    }
}
