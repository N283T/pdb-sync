mod api;
mod cli;
mod config;
mod context;
mod convert;
mod download;
mod error;
mod files;
mod history;
mod mirrors;
mod stats;
mod sync;
mod tree;
mod update;
mod utils;
mod validation;
mod watch;

// Re-export from library crate
pub use pdb_cli::data_types;

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
        EnvFilter::new("warn")
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    // Check for first-run setup
    if cli::commands::needs_setup() {
        cli::commands::run_setup()?;
    }

    // Load context
    let ctx = AppContext::new().await?.with_overrides(cli.pdb_dir, None);

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
        Commands::List(args) => {
            cli::commands::run_list(args, ctx).await?;
        }
        Commands::Find(args) => {
            if let Err(e) = cli::commands::run_find(args, ctx).await {
                if matches!(e, error::PdbCliError::EntriesNotFound(_, _)) {
                    // Exit with code 1 for scripting (no error message)
                    std::process::exit(1);
                }
                return Err(e.into());
            }
        }
        Commands::Config(args) => {
            cli::commands::run_config(args, ctx).await?;
        }
        Commands::Env(args) => {
            cli::commands::run_env(args, ctx).await?;
        }
        Commands::Info(args) => {
            cli::commands::run_info(args, ctx).await?;
        }
        Commands::Validate(args) => {
            cli::commands::run_validate(args, ctx).await?;
        }
        Commands::Watch(args) => {
            cli::commands::run_watch(args, ctx).await?;
        }
        Commands::Convert(args) => {
            cli::commands::run_convert(args, ctx).await?;
        }
        Commands::Stats(args) => {
            cli::commands::run_stats(args, ctx).await?;
        }
        Commands::Tree(args) => {
            cli::commands::run_tree(args, ctx).await?;
        }
        Commands::Update(args) => {
            cli::commands::run_update(args, ctx).await?;
        }
    }

    Ok(())
}
