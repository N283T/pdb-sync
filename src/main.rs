mod cli;
mod config;
mod context;
mod download;
mod error;
mod files;
mod mirrors;
mod sync;

use clap::Parser;
use cli::{Cli, Commands};
use context::AppContext;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    // Load context
    let ctx = AppContext::new()?.with_overrides(cli.pdb_dir, None);

    // Dispatch to command handlers
    match cli.command {
        Commands::Sync(args) => {
            cli::commands::run_sync(args, ctx).await?;
        }
        Commands::Download(args) => {
            cli::commands::run_download(args, ctx).await?;
        }
        Commands::Copy(args) => {
            cli::commands::run_copy(args, ctx).await?;
        }
        Commands::Config(args) => {
            cli::commands::run_config(args, ctx).await?;
        }
        Commands::Env(args) => {
            cli::commands::run_env(args, ctx).await?;
        }
    }

    Ok(())
}
