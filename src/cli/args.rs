use crate::files::FileFormat;
use crate::mirrors::MirrorId;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "pdb-cli")]
#[command(about = "CLI tool for managing Protein Data Bank files")]
#[command(version)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Override the PDB directory
    #[arg(long, global = true, env = "PDB_DIR")]
    pub pdb_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Sync files from a PDB mirror using rsync
    Sync(SyncArgs),

    /// Download individual files via HTTPS
    Download(DownloadArgs),

    /// Copy local PDB files
    Copy(CopyArgs),

    /// Manage configuration
    Config(ConfigArgs),

    /// Manage environment variables
    Env(EnvArgs),
}

#[derive(Parser)]
pub struct SyncArgs {
    /// Mirror to sync from
    #[arg(short, long, value_enum)]
    pub mirror: Option<MirrorId>,

    /// File format to sync
    #[arg(short, long, value_enum, default_value = "mmcif")]
    pub format: SyncFormat,

    /// Destination directory
    #[arg(short, long)]
    pub dest: Option<PathBuf>,

    /// Delete files not present on the remote
    #[arg(long)]
    pub delete: bool,

    /// Bandwidth limit in KB/s
    #[arg(long)]
    pub bwlimit: Option<u32>,

    /// Perform a dry run without making changes
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Filter patterns (PDB ID prefixes)
    #[arg(trailing_var_arg = true)]
    pub filters: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum SyncFormat {
    Pdb,
    Mmcif,
    Both,
}

impl SyncFormat {
    pub fn to_file_formats(self) -> Vec<FileFormat> {
        match self {
            SyncFormat::Pdb => vec![FileFormat::Pdb],
            SyncFormat::Mmcif => vec![FileFormat::Mmcif],
            SyncFormat::Both => vec![FileFormat::Pdb, FileFormat::Mmcif],
        }
    }
}

#[derive(Parser)]
pub struct DownloadArgs {
    /// PDB IDs to download
    #[arg(required = true)]
    pub pdb_ids: Vec<String>,

    /// File format to download
    #[arg(short, long, value_enum, default_value = "mmcif")]
    pub format: FileFormat,

    /// Destination directory
    #[arg(short, long)]
    pub dest: Option<PathBuf>,

    /// Decompress downloaded files
    #[arg(long)]
    pub decompress: bool,

    /// Mirror to download from
    #[arg(short, long, value_enum)]
    pub mirror: Option<MirrorId>,

    /// Overwrite existing files
    #[arg(long)]
    pub overwrite: bool,
}

#[derive(Parser)]
pub struct CopyArgs {
    /// Source file or directory
    pub source: PathBuf,

    /// Destination directory
    pub dest: PathBuf,

    /// Flatten directory structure
    #[arg(long)]
    pub flatten: bool,

    /// Create symbolic links instead of copying
    #[arg(long)]
    pub symlink: bool,
}

#[derive(Parser)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Initialize configuration file
    Init,
    /// Show current configuration
    Show,
    /// Set a configuration value
    Set {
        /// Configuration key (e.g., sync.mirror)
        key: String,
        /// Value to set
        value: String,
    },
    /// Get a configuration value
    Get {
        /// Configuration key (e.g., sync.mirror)
        key: String,
    },
}

#[derive(Parser)]
pub struct EnvArgs {
    #[command(subcommand)]
    pub action: EnvAction,
}

#[derive(Subcommand)]
pub enum EnvAction {
    /// Show relevant environment variables
    Show,
    /// Export environment variables as shell commands
    Export,
    /// Set an environment variable (prints the command)
    Set {
        /// Variable name
        name: String,
        /// Value
        value: String,
    },
}
