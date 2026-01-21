//! Individual command argument structures.

use crate::data_types::DataType;
use crate::download::EngineType;
use crate::files::FileFormat;
use crate::mirrors::MirrorId;
use crate::tree::SortBy;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use super::enums::{ExperimentalMethod, NotifyMethod, SortField};
use super::{PdbDirArgs, MirrorArgs, ProgressArgs, DryRunArgs};

/// Arguments for the init command.
#[derive(Parser, Clone)]
pub struct InitArgs {
    /// Base directory path (default: $PDB_DIR, or config file, or ~/pdb)
    #[arg(short, long)]
    pub dir: Option<PathBuf>,

    /// Create only specific subdirectories (can be specified multiple times)
    #[arg(short, long, value_name = "DIR")]
    pub only: Option<Vec<String>>,

    /// Directory depth to create (0-3, or: base, types, layouts, format)
    #[arg(long, default_value = "0")]
    pub depth: String,

    /// Show what would be created without creating
    #[arg(long, short = 'n')]
    pub dry_run: bool,
}

#[derive(Parser, Clone)]
pub struct DownloadArgs {
    /// PDB directory and destination
    #[command(flatten)]
    pub pdb_dir: PdbDirArgs,

    /// Mirror selection
    #[command(flatten)]
    pub mirror: MirrorArgs,

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

    /// Decompress downloaded files
    #[arg(long)]
    pub decompress: bool,

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

    /// Download engine to use
    #[arg(long, value_enum)]
    pub engine: Option<EngineType>,

    /// Number of connections per server (aria2c only)
    #[arg(long, default_value = "4")]
    pub connections: u32,

    /// Number of splits per download (aria2c only)
    #[arg(long, default_value = "1")]
    pub split: u32,

    /// Export aria2c input file to stdout instead of downloading
    #[arg(long)]
    pub export_aria2c: bool,

    /// Run in background
    #[arg(long)]
    pub bg: bool,
}

#[derive(Parser, Clone)]
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

#[derive(Parser, Clone)]
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
    pub output: super::enums::OutputFormat,

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


#[derive(Parser, Clone)]
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

#[derive(Parser, Clone)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Subcommand, Clone)]
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

#[derive(Parser, Clone)]
pub struct EnvArgs {
    #[command(subcommand)]
    pub action: EnvAction,
}

#[derive(Subcommand, Clone)]
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

#[derive(Parser, Clone)]
pub struct InfoArgs {
    /// PDB ID to query
    pub pdb_id: String,

    /// Show local file info only (no network request)
    #[arg(long)]
    pub local: bool,

    /// Output format
    #[arg(long, value_enum, default_value = "text")]
    pub output: super::enums::OutputFormat,

    /// Show all available fields
    #[arg(short, long)]
    pub all: bool,
}

#[derive(Parser, Clone)]
pub struct ValidateArgs {
    /// Mirror selection
    #[command(flatten)]
    pub mirror: MirrorArgs,

    /// Progress and output arguments
    #[command(flatten)]
    pub progress: ProgressArgs,

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

    /// Show only errors (skip valid files)
    #[arg(long)]
    pub errors_only: bool,

    /// Read PDB IDs from a file (one per line)
    #[arg(short, long)]
    pub list: Option<PathBuf>,

    /// Read PDB IDs from stdin (one per line)
    #[arg(long)]
    pub stdin: bool,

    /// Output format
    #[arg(short, long, value_enum, default_value = "text")]
    pub output: super::enums::OutputFormat,
}

/// Validate resolution filter (must be in range 0.0-100.0)
fn validate_resolution(s: &str) -> Result<f64, String> {
    let value: f64 = s.parse().map_err(|_| format!("Invalid number: {}", s))?;
    if !(0.0..=100.0).contains(&value) {
        return Err(format!(
            "Resolution must be between 0.0 and 100.0, got {}",
            value
        ));
    }
    Ok(value)
}

/// Validate organism filter string (max 200 chars, alphanumeric + basic punctuation)
fn validate_organism(s: &str) -> Result<String, String> {
    const MAX_LEN: usize = 200;
    if s.len() > MAX_LEN {
        return Err(format!(
            "Organism name too long ({} chars, max {})",
            s.len(),
            MAX_LEN
        ));
    }
    // Allow alphanumeric, spaces, hyphens, periods, parentheses
    if s.chars()
        .all(|c| c.is_alphanumeric() || " -._()".contains(c))
    {
        Ok(s.to_string())
    } else {
        Err(
            "Organism name contains invalid characters (allowed: alphanumeric, space, -._())"
                .into(),
        )
    }
}

#[derive(Parser, Clone)]
pub struct WatchArgs {
    /// Check interval (e.g., "1h", "30m", "1d")
    #[arg(short, long, default_value = "1h")]
    pub interval: String,

    /// Filter by experimental method
    #[arg(long, value_enum)]
    pub method: Option<ExperimentalMethod>,

    /// Filter by maximum resolution (Å), range: 0.0-100.0
    #[arg(long, value_parser = validate_resolution)]
    pub resolution: Option<f64>,

    /// Filter by source organism (scientific name), max 200 characters
    #[arg(long, value_parser = validate_organism)]
    pub organism: Option<String>,

    /// Data types to download (can specify multiple times)
    #[arg(short = 't', long = "type", value_enum)]
    pub data_types: Vec<DataType>,

    /// File format to download
    #[arg(short, long, value_enum, default_value = "mmcif")]
    pub format: FileFormat,

    /// Dry run (don't download, just show what would be downloaded)
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Send notification on new entries
    #[arg(long, value_enum)]
    pub notify: Option<NotifyMethod>,

    /// Email address for notifications (requires --notify email)
    #[arg(long, requires = "notify")]
    pub email: Option<String>,

    /// Script to run on each new entry (receives PDB_ID and PDB_FILE as env vars)
    #[arg(long)]
    pub on_new: Option<PathBuf>,

    /// Destination directory for downloads
    #[arg(short, long)]
    pub dest: Option<PathBuf>,

    /// Mirror to download from
    #[arg(short, long, value_enum)]
    pub mirror: Option<MirrorId>,

    /// Run once and exit (don't loop)
    #[arg(long)]
    pub once: bool,

    /// Start watching from this date (YYYY-MM-DD), default: 7 days ago or last check
    #[arg(long)]
    pub since: Option<String>,
}


#[derive(Parser, Clone)]
pub struct ConvertArgs {
    /// Files to convert (paths or glob patterns)
    pub files: Vec<String>,

    /// Decompress .gz files
    #[arg(long, conflicts_with = "compress")]
    pub decompress: bool,

    /// Compress files to .gz format
    #[arg(long, conflicts_with = "decompress")]
    pub compress: bool,

    /// Target format (requires gemmi for format conversion)
    #[arg(long, value_enum)]
    pub to: Option<FileFormat>,

    /// Source format filter for batch mode
    #[arg(long, value_enum)]
    pub from: Option<FileFormat>,

    /// Destination directory
    #[arg(short, long)]
    pub dest: Option<PathBuf>,

    /// Replace original files (delete source after conversion)
    #[arg(long)]
    pub in_place: bool,

    /// Read paths from stdin (one per line)
    #[arg(long)]
    pub stdin: bool,

    /// Number of parallel conversions (1-64)
    #[arg(short, long, default_value = "4", value_parser = clap::value_parser!(u8).range(1..=64))]
    pub parallel: u8,
}

#[derive(Parser, Clone)]
pub struct StatsArgs {
    /// Show detailed statistics (size distribution, oldest/newest files)
    #[arg(long)]
    pub detailed: bool,

    /// Compare with remote PDB archive
    #[arg(long)]
    pub compare_remote: bool,

    /// Filter by file format
    #[arg(short, long, value_enum)]
    pub format: Option<FileFormat>,

    /// Filter by data type
    #[arg(short = 't', long = "type", value_enum)]
    pub data_type: Option<DataType>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "text")]
    pub output: super::enums::OutputFormat,
}

/// Arguments for the tree command.
///
/// # Examples
///
/// Show full tree with default options:
/// ```bash
/// pdb-sync tree
/// ```
///
/// Limit depth and filter by format:
/// ```bash
/// pdb-sync tree --depth 2 --format cif-gz
/// ```
///
/// Show top 10 directories by size:
/// ```bash
/// pdb-sync tree --top 10 --sort-by size
/// ```
#[derive(Parser, Clone)]
#[command(after_help = "Examples:
  pdb-sync tree                           Show full tree
  pdb-sync tree --depth 2                 Limit depth to 2
  pdb-sync tree --format cif-gz           Filter by mmCIF format
  pdb-sync tree --top 10                  Top 10 directories by count
  pdb-sync tree --top 10 --sort-by size   Top 10 directories by size
  pdb-sync tree -o json                   Output as JSON")]
pub struct TreeArgs {
    /// Maximum depth to display (0 = root only)
    #[arg(short, long)]
    pub depth: Option<usize>,

    /// Filter by file format
    #[arg(short, long, value_enum)]
    pub format: Option<FileFormat>,

    /// Show file sizes
    #[arg(short, long)]
    pub size: bool,

    /// Show file counts
    #[arg(short, long)]
    pub count: bool,

    /// Hide summary line
    #[arg(long)]
    pub no_summary: bool,

    /// Show only non-empty directories
    #[arg(long)]
    pub non_empty: bool,

    /// Show top N directories (use with --sort-by)
    #[arg(long)]
    pub top: Option<usize>,

    /// Sort field for --top mode
    #[arg(long, value_enum, default_value = "count")]
    pub sort_by: SortBy,

    /// Output format
    #[arg(short, long, value_enum, default_value = "text")]
    pub output: super::enums::OutputFormat,
}

#[derive(Parser, Clone)]
pub struct UpdateArgs {
    /// Mirror selection
    #[command(flatten)]
    pub mirror: MirrorArgs,

    /// Progress and dry run arguments
    #[command(flatten)]
    pub progress: ProgressArgs,

    /// Dry run arguments
    #[command(flatten)]
    pub dry_run: DryRunArgs,

    /// PDB IDs to check/update (empty = all local files)
    pub pdb_ids: Vec<String>,

    /// Check only, don't download updates
    #[arg(short, long)]
    pub check: bool,

    /// Use checksums for verification (slower but accurate)
    #[arg(long)]
    pub verify: bool,

    /// Force update even if files appear up-to-date
    #[arg(long)]
    pub force: bool,

    /// File format to check
    #[arg(short, long, value_enum)]
    pub format: Option<FileFormat>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "text")]
    pub output: super::enums::OutputFormat,

    /// Number of parallel checks
    #[arg(short = 'j', long, default_value = "10")]
    pub parallel: usize,
}

#[derive(Parser, Clone)]
pub struct JobsArgs {
    /// Show all jobs (including old completed ones)
    #[arg(short, long)]
    pub all: bool,

    /// Show only running jobs
    #[arg(long)]
    pub running: bool,

    #[command(subcommand)]
    pub action: Option<JobsAction>,
}

#[derive(Subcommand, Clone)]
pub enum JobsAction {
    /// Show status of a specific job
    Status {
        /// Job ID
        job_id: String,
    },
    /// Show logs for a job
    Log {
        /// Job ID
        job_id: String,
        /// Follow log output (like tail -f)
        #[arg(short, long)]
        follow: bool,
    },
    /// Cancel a running job
    Cancel {
        /// Job ID
        job_id: String,
    },
    /// Clean up old job directories
    Clean {
        /// Remove jobs older than this duration (e.g., "7d", "24h")
        #[arg(long, default_value = "7d")]
        older_than: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_resolution() {
        // Valid resolutions
        assert!(validate_resolution("0.0").is_ok());
        assert!(validate_resolution("1.5").is_ok());
        assert!(validate_resolution("100.0").is_ok());

        // Invalid resolutions
        assert!(validate_resolution("-0.1").is_err());
        assert!(validate_resolution("100.1").is_err());
        assert!(validate_resolution("abc").is_err());
    }

    #[test]
    fn test_validate_organism() {
        // Valid organisms
        assert!(validate_organism("Homo sapiens").is_ok());
        assert!(validate_organism("Escherichia coli").is_ok());
        assert!(validate_organism("Mus musculus (mouse)").is_ok());

        // Too long
        let long_name = "a".repeat(201);
        let result = validate_organism(&long_name);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("201 chars"));

        // Invalid characters
        assert!(validate_organism("test@invalid").is_err());
        assert!(validate_organism("test;injection").is_err());
    }
}
