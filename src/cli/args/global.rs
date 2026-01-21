//! Global CLI structures and shared argument groups.

use clap::builder::styling::{AnsiColor, Effects};
use clap::builder::Styles;
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use std::path::PathBuf;

use super::commands::*;
use super::sync::SyncArgs;
use crate::mirrors::MirrorId;

// Configures colored help menu colors (similar to uv)
pub const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default());

/// Global options shared across all commands
#[derive(Parser, Clone, Debug)]
pub struct GlobalArgs {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

/// PDB directory arguments.
///
/// This arg group is flattened into commands that need PDB directory or
/// destination path specification:
/// - `download`: For choosing download destination
/// - `copy`: For choosing copy destination
#[derive(Parser, Clone, Debug)]
pub struct PdbDirArgs {
    /// Override the PDB directory
    #[arg(long, env = "PDB_DIR")]
    pub pdb_dir: Option<PathBuf>,

    /// Destination directory
    #[arg(short, long)]
    pub dest: Option<PathBuf>,
}

/// Mirror selection arguments.
///
/// This arg group is flattened into commands that need mirror selection:
/// - `download`: For choosing download source
/// - `validate`: For fetching checksums
/// - `update`: For checking remote versions
/// - `watch`: For choosing download source
#[derive(Parser, Clone, Debug)]
pub struct MirrorArgs {
    /// Mirror to use
    #[arg(short, long, value_enum)]
    pub mirror: Option<MirrorId>,
}

/// Progress bar arguments.
///
/// This arg group is flattened into commands that support progress display:
/// - `validate`: For showing validation progress
/// - `update`: For showing update check progress
#[derive(Parser, Clone, Debug)]
pub struct ProgressArgs {
    /// Show progress bar
    #[arg(short = 'P', long)]
    pub progress: bool,
}

/// Dry run arguments.
///
/// This arg group is flattened into commands that support dry-run mode:
/// - `sync`: For previewing rsync operations
/// - `update`: For previewing updates without downloading
#[derive(Parser, Clone, Debug)]
pub struct DryRunArgs {
    /// Perform a dry run without making changes
    #[arg(short = 'n', long)]
    pub dry_run: bool,
}

/// Main CLI structure
#[derive(Parser)]
#[command(name = "pdb-sync")]
#[command(about = "CLI tool for managing Protein Data Bank files")]
#[command(version)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Override the PDB directory
    #[arg(long, global = true, env = "PDB_DIR")]
    pub pdb_dir: Option<PathBuf>,

    /// Internal: Job ID for background execution (hidden)
    #[arg(long = "_job-id", hide = true)]
    pub job_id: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

/// Available subcommands
#[derive(Subcommand)]
pub enum Commands {
    /// Initialize the base directory structure
    Init(InitArgs),

    /// Sync files from a PDB mirror using rsync
    Sync(SyncArgs),

    /// Download individual files via HTTPS
    #[command(visible_alias = "dl")]
    Download(DownloadArgs),

    /// Copy local PDB files
    Copy(CopyArgs),

    /// List local PDB files
    List(ListArgs),

    /// Find local PDB files (path output for scripting)
    Find(FindArgs),

    /// Manage configuration
    #[command(visible_alias = "cfg")]
    Config(ConfigArgs),

    /// Manage environment variables
    Env(EnvArgs),

    /// Show information about a PDB entry
    Info(InfoArgs),

    /// Validate local PDB files against checksums
    #[command(visible_alias = "val")]
    Validate(ValidateArgs),

    /// Watch for new PDB entries and download automatically
    Watch(WatchArgs),

    /// Convert file formats (compression, decompression, format conversion)
    Convert(ConvertArgs),

    /// Show statistics about the local PDB collection
    Stats(StatsArgs),

    /// Show directory tree of local PDB mirror
    Tree(TreeArgs),

    /// Check for and download updates to local files
    Update(UpdateArgs),

    /// Manage background jobs
    Jobs(JobsArgs),
}

/// Parse CLI with colored styles
pub fn parse_cli() -> Cli {
    let cmd = Cli::command().styles(STYLES).color(clap::ColorChoice::Auto);
    let matches = cmd.get_matches();
    Cli::from_arg_matches(&matches).expect("Failed to parse arguments")
}
