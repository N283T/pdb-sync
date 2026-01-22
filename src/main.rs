// Library modules - re-exported for use in binary
use pdb_sync::data_types;
use pdb_sync::error;
use pdb_sync::files;

// Binary-specific modules
mod cli;
mod config;
mod context;
mod mirrors;
mod sync;

use cli::args::{run_sync, run_validate};
use cli::{parse_cli, SyncCommand};
use context::AppContext;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = parse_cli();

    // Initialize logging
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("warn")
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    // Load context
    let ctx = AppContext::new()
        .await?
        .with_overrides(cli.pdb_dir.clone(), None);

    // Dispatch to command
    match cli.command {
        SyncCommand::Sync(args) => {
            run_sync(args, ctx).await?;
        }
        SyncCommand::ConfigValidate(args) => {
            run_validate(args, ctx).await?;
        }
    }

    Ok(())
}
