use crate::data_types::{DataType, Layout};
use crate::files::FileFormat;
use crate::mirrors::MirrorId;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Output format for list command
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum OutputFormat {
    /// Plain text output
    #[default]
    Text,
    /// JSON output
    Json,
    /// CSV output
    Csv,
    /// One ID per line (for piping)
    Ids,
}

/// Sort field for list command
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum SortField {
    /// Sort by PDB ID (alphabetical)
    #[default]
    Name,
    /// Sort by file size
    Size,
    /// Sort by modification time
    Time,
}

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
}

#[derive(Parser)]
pub struct SyncArgs {
    /// Mirror to sync from
    #[arg(short, long, value_enum)]
    pub mirror: Option<MirrorId>,

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

    /// Show detailed progress
    #[arg(short = 'P', long)]
    pub progress: bool,

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
    #[arg(required_unless_present_any = ["list", "stdin"])]
    pub pdb_ids: Vec<String>,

    /// Data type to download
    #[arg(short = 't', long = "type", value_enum, default_value = "structures")]
    pub data_type: DataType,

    /// File format to download (for structures)
    #[arg(short, long, value_enum, default_value = "mmcif")]
    pub format: FileFormat,

    /// Assembly number (for assemblies type, 0 = try all 1-60)
    #[arg(short, long)]
    pub assembly: Option<u8>,

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

    /// Number of parallel downloads
    #[arg(short, long, default_value = "4")]
    pub parallel: u8,

    /// Number of retry attempts
    #[arg(long, default_value = "3")]
    pub retry: u32,

    /// Read PDB IDs from a file (one per line)
    #[arg(short, long)]
    pub list: Option<PathBuf>,

    /// Read PDB IDs from stdin (one per line)
    #[arg(long)]
    pub stdin: bool,
}

#[derive(Parser)]
pub struct CopyArgs {
    /// PDB IDs to copy from local mirror
    #[arg(required_unless_present_any = ["list", "stdin"])]
    pub pdb_ids: Vec<String>,

    /// Destination directory
    #[arg(short, long)]
    pub dest: PathBuf,

    /// File format
    #[arg(short, long, value_enum, default_value = "cif-gz")]
    pub format: FileFormat,

    /// Keep directory structure from mirror (default: flat)
    #[arg(long)]
    pub keep_structure: bool,

    /// Create symbolic links instead of copying
    #[arg(short, long)]
    pub symlink: bool,

    /// Read PDB IDs from a file (one per line)
    #[arg(short, long)]
    pub list: Option<PathBuf>,

    /// Read PDB IDs from stdin (one per line)
    #[arg(long)]
    pub stdin: bool,
}

#[derive(Parser)]
pub struct ListArgs {
    /// Pattern to filter PDB IDs (supports glob patterns like "1ab*", "*xyz")
    pub pattern: Option<String>,

    /// File format to list
    #[arg(short, long, value_enum)]
    pub format: Option<FileFormat>,

    /// Show file sizes
    #[arg(short, long)]
    pub size: bool,

    /// Show modification times
    #[arg(long)]
    pub time: bool,

    /// Output format
    #[arg(short, long, value_enum, default_value = "text")]
    pub output: OutputFormat,

    /// Show statistics only (no file list)
    #[arg(long)]
    pub stats: bool,

    /// Sort field
    #[arg(long, value_enum, default_value = "name")]
    pub sort: SortField,

    /// Reverse sort order
    #[arg(short, long)]
    pub reverse: bool,
}

#[derive(Parser)]
pub struct FindArgs {
    /// PDB IDs or patterns to find
    pub patterns: Vec<String>,

    /// Read patterns from stdin
    #[arg(long)]
    pub stdin: bool,

    /// File format to search
    #[arg(short, long, value_enum)]
    pub format: Option<FileFormat>,

    /// Show all formats for each entry
    #[arg(long)]
    pub all_formats: bool,

    /// Check existence (exit code only, all must exist for 0)
    #[arg(long)]
    pub exists: bool,

    /// Show entries NOT found locally
    #[arg(long)]
    pub missing: bool,

    /// Quiet mode (no output, just exit code)
    #[arg(short, long)]
    pub quiet: bool,

    /// Count matches only
    #[arg(long)]
    pub count: bool,
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
    /// Test mirror latencies
    TestMirrors,
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

#[derive(Parser)]
pub struct InfoArgs {
    /// PDB ID to query
    pub pdb_id: String,

    /// Show local file info only (no network request)
    #[arg(long)]
    pub local: bool,

    /// Output format
    #[arg(long, value_enum, default_value = "text")]
    pub output: OutputFormat,

    /// Show all available fields
    #[arg(short, long)]
    pub all: bool,
}

#[derive(Parser)]
pub struct ValidateArgs {
    /// PDB IDs to validate (empty = all local files)
    pub pdb_ids: Vec<String>,

    /// Data type to validate
    #[arg(short = 't', long = "type", value_enum)]
    pub data_type: Option<DataType>,

    /// File format to validate
    #[arg(short, long, value_enum)]
    pub format: Option<FileFormat>,

    /// Re-download corrupted files
    #[arg(long)]
    pub fix: bool,

    /// Show progress bar
    #[arg(short = 'P', long)]
    pub progress: bool,

    /// Show only errors (skip valid files)
    #[arg(long)]
    pub errors_only: bool,

    /// Mirror to use for checksums (and --fix downloads)
    #[arg(short, long, value_enum)]
    pub mirror: Option<MirrorId>,

    /// Read PDB IDs from a file (one per line)
    #[arg(short, long)]
    pub list: Option<PathBuf>,

    /// Read PDB IDs from stdin (one per line)
    #[arg(long)]
    pub stdin: bool,

    /// Output format
    #[arg(short, long, value_enum, default_value = "text")]
    pub output: OutputFormat,
}
