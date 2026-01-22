//! Global CLI structures and shared argument groups.

use clap::builder::styling::{AnsiColor, Effects};
use clap::builder::Styles;
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use std::path::PathBuf;

use super::env::EnvArgs;
use super::sync::SyncArgs;

// Configures colored help menu colors (similar to uv)
pub const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default());

/// Global CLI structure
#[derive(Parser)]
#[command(name = "pdb-sync")]
#[command(about = "Sync PDB data from configured rsync sources")]
#[command(version)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Override the PDB directory
    #[arg(long, global = true, env = "PDB_DIR")]
    pub pdb_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: SyncCommand,
}

/// Sync command
#[derive(Subcommand)]
pub enum SyncCommand {
    /// Sync from a configured source (runs all if no name specified)
    Sync(SyncArgs),
    /// Environment diagnostics and validation
    Env(EnvArgs),
}

/// Parse CLI with colored styles
pub fn parse_cli() -> Cli {
    let cmd = Cli::command().styles(STYLES).color(clap::ColorChoice::Auto);
    let matches = cmd.get_matches();
    Cli::from_arg_matches(&matches).expect("Failed to parse arguments")
}
