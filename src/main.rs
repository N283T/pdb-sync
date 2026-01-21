mod api;
mod cli;
mod config;
mod context;
mod convert;
mod download;
mod error;
mod files;
mod history;
mod jobs;
mod mirrors;
mod stats;
mod sync;
mod tree;
mod update;
mod utils;
mod validation;
mod watch;

// Re-export from library crate
pub use pdb_sync::data_types;

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

    // Check if running as a background job
    if let Some(job_id) = &cli.job_id {
        return run_as_background_job(&cli, job_id).await;
    }

    // Check if --bg flag is set for supported commands
    if should_run_background(&cli) {
        return spawn_background_job();
    }

    // Check for first-run setup
    if cli::commands::needs_setup() {
        cli::commands::run_setup()?;
    }

    // Load context
    let ctx = AppContext::new().await?.with_overrides(cli.pdb_dir, None);

    // Dispatch to command handlers
    match cli.command {
        Commands::Init(args) => {
            cli::commands::init::run_init(args, ctx).await?;
        }
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
                if matches!(e, error::PdbSyncError::EntriesNotFound(_, _)) {
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
        Commands::Jobs(args) => {
            cli::commands::run_jobs(args).await?;
        }
    }

    Ok(())
}

/// Check if the command should run in background
fn should_run_background(cli: &Cli) -> bool {
    match &cli.command {
        Commands::Sync(args) => args.bg,
        Commands::Download(args) => args.bg,
        _ => false,
    }
}

/// Spawn a background job and exit immediately
fn spawn_background_job() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let (job_id, job_dir) = jobs::spawn::spawn_background(&args)?;

    println!("Job started: {}", job_id);
    println!("Log: {}", job_dir.join("stdout.log").display());
    println!();
    println!("Use 'pdb-sync jobs' to check status");
    println!(
        "Use 'pdb-sync jobs log {} --follow' to tail the output",
        job_id
    );

    Ok(())
}

/// Run as a background job (called when --_job-id is set)
async fn run_as_background_job(cli: &Cli, job_id: &str) -> anyhow::Result<()> {
    // Run the actual command
    let result = run_command(cli).await;

    // Finalize job with exit code
    let exit_code = if result.is_ok() { 0 } else { 1 };
    jobs::spawn::finalize_job(job_id, exit_code)?;

    result
}

/// Run the command (without background handling)
async fn run_command(cli: &Cli) -> anyhow::Result<()> {
    // Check for first-run setup
    if cli::commands::needs_setup() {
        cli::commands::run_setup()?;
    }

    // Load context
    let ctx = AppContext::new()
        .await?
        .with_overrides(cli.pdb_dir.clone(), None);

    // Dispatch to command handlers
    match &cli.command {
        Commands::Init(args) => {
            cli::commands::init::run_init(args.clone(), ctx).await?;
        }
        Commands::Sync(args) => {
            cli::commands::run_sync(args.clone(), ctx).await?;
        }
        Commands::Download(args) => {
            cli::commands::run_download(args.clone(), ctx).await?;
        }
        Commands::Copy(args) => {
            cli::commands::run_copy(args.clone(), ctx).await?;
        }
        Commands::List(args) => {
            cli::commands::run_list(args.clone(), ctx).await?;
        }
        Commands::Find(args) => {
            if let Err(e) = cli::commands::run_find(args.clone(), ctx).await {
                if matches!(e, error::PdbSyncError::EntriesNotFound(_, _)) {
                    std::process::exit(1);
                }
                return Err(e.into());
            }
        }
        Commands::Config(args) => {
            cli::commands::run_config(args.clone(), ctx).await?;
        }
        Commands::Env(args) => {
            cli::commands::run_env(args.clone(), ctx).await?;
        }
        Commands::Info(args) => {
            cli::commands::run_info(args.clone(), ctx).await?;
        }
        Commands::Validate(args) => {
            cli::commands::run_validate(args.clone(), ctx).await?;
        }
        Commands::Watch(args) => {
            cli::commands::run_watch(args.clone(), ctx).await?;
        }
        Commands::Convert(args) => {
            cli::commands::run_convert(args.clone(), ctx).await?;
        }
        Commands::Stats(args) => {
            cli::commands::run_stats(args.clone(), ctx).await?;
        }
        Commands::Tree(args) => {
            cli::commands::run_tree(args.clone(), ctx).await?;
        }
        Commands::Update(args) => {
            cli::commands::run_update(args.clone(), ctx).await?;
        }
        Commands::Jobs(args) => {
            cli::commands::run_jobs(args.clone()).await?;
        }
    }

    Ok(())
}
