//! Sync command arguments.

use crate::context::AppContext;
use crate::error::Result;
use crate::sync::RsyncFlagOverrides;
use clap::{ArgAction, Parser};

/// Sync command arguments.
#[derive(Parser, Clone, Debug)]
pub struct SyncArgs {
    /// Name of the custom sync config to run (optional - runs all if not specified)
    #[arg(value_name = "NAME")]
    pub name: Option<String>,

    /// Run all custom sync configs
    #[arg(long)]
    pub all: bool,

    /// Override destination directory
    #[arg(short, long)]
    pub dest: Option<std::path::PathBuf>,

    /// List available custom sync configs
    #[arg(long)]
    pub list: bool,

    /// Stop on first failure when running all configs
    #[arg(long)]
    pub fail_fast: bool,

    /// Dry run - show the rsync command without executing
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Delete files not present on remote
    #[arg(long)]
    pub delete: bool,

    /// Do not delete files not present on remote
    #[arg(long, action = ArgAction::SetTrue, overrides_with = "delete")]
    pub no_delete: bool,

    /// Compress data during transfer
    #[arg(short = 'z', long)]
    pub compress: bool,

    /// Do not compress data during transfer
    #[arg(long, action = ArgAction::SetTrue, overrides_with = "compress")]
    pub no_compress: bool,

    /// Use checksum for file comparison
    #[arg(short = 'c', long)]
    pub checksum: bool,

    /// Do not use checksum for file comparison
    #[arg(long, action = ArgAction::SetTrue, overrides_with = "checksum")]
    pub no_checksum: bool,

    /// Keep partially transferred files
    #[arg(long)]
    pub partial: bool,

    /// Do not keep partially transferred files
    #[arg(long, action = ArgAction::SetTrue, overrides_with = "partial")]
    pub no_partial: bool,

    /// Directory for partial files
    #[arg(long)]
    pub partial_dir: Option<String>,

    /// Maximum file size to transfer
    #[arg(long)]
    pub max_size: Option<String>,

    /// Minimum file size to transfer
    #[arg(long)]
    pub min_size: Option<String>,

    /// I/O timeout in seconds
    #[arg(long)]
    pub timeout: Option<u32>,

    /// Connection timeout in seconds
    #[arg(long)]
    pub contimeout: Option<u32>,

    /// Create backups
    #[arg(long)]
    pub backup: bool,

    /// Do not create backups
    #[arg(long, action = ArgAction::SetTrue, overrides_with = "backup")]
    pub no_backup: bool,

    /// Backup directory
    #[arg(long)]
    pub backup_dir: Option<String>,

    /// Change permission flags
    #[arg(long)]
    pub chmod: Option<String>,

    /// Exclude patterns
    #[arg(long, value_name = "PATTERN", action = ArgAction::Append)]
    pub exclude: Option<Vec<String>>,

    /// Include patterns
    #[arg(long, value_name = "PATTERN", action = ArgAction::Append)]
    pub include: Option<Vec<String>>,

    /// File with exclude patterns
    #[arg(long)]
    pub exclude_from: Option<String>,

    /// File with include patterns
    #[arg(long)]
    pub include_from: Option<String>,

    /// Bandwidth limit in KB/s
    #[arg(long)]
    pub bwlimit: Option<u32>,

    /// Verbose rsync output
    #[arg(long)]
    pub rsync_verbose: bool,

    /// Quiet rsync output
    #[arg(long)]
    pub rsync_quiet: bool,

    /// Do not enable rsync verbose output
    #[arg(long, action = ArgAction::SetTrue, overrides_with = "rsync_verbose")]
    pub no_rsync_verbose: bool,

    /// Do not enable rsync quiet output
    #[arg(long, action = ArgAction::SetTrue, overrides_with = "rsync_quiet")]
    pub no_rsync_quiet: bool,

    /// Itemize changes
    #[arg(long)]
    pub itemize_changes: bool,

    /// Do not itemize changes
    #[arg(long, action = ArgAction::SetTrue, overrides_with = "itemize_changes")]
    pub no_itemize_changes: bool,
}

impl SyncArgs {
    /// Convert args to rsync flags (for CLI override functionality)
    pub fn to_rsync_overrides(&self) -> RsyncFlagOverrides {
        let delete = if self.no_delete {
            Some(false)
        } else if self.delete {
            Some(true)
        } else {
            None
        };

        let compress = if self.no_compress {
            Some(false)
        } else if self.compress {
            Some(true)
        } else {
            None
        };

        let checksum = if self.no_checksum {
            Some(false)
        } else if self.checksum {
            Some(true)
        } else {
            None
        };

        let partial = if self.no_partial {
            Some(false)
        } else if self.partial {
            Some(true)
        } else {
            None
        };

        let backup = if self.no_backup {
            Some(false)
        } else if self.backup {
            Some(true)
        } else {
            None
        };

        let verbose = if self.no_rsync_verbose {
            Some(false)
        } else if self.rsync_verbose {
            Some(true)
        } else {
            None
        };

        let quiet = if self.no_rsync_quiet {
            Some(false)
        } else if self.rsync_quiet {
            Some(true)
        } else {
            None
        };

        let itemize_changes = if self.no_itemize_changes {
            Some(false)
        } else if self.itemize_changes {
            Some(true)
        } else {
            None
        };

        RsyncFlagOverrides {
            delete,
            compress,
            checksum,
            partial,
            backup,
            verbose,
            quiet,
            itemize_changes,
            dry_run: if self.dry_run { Some(true) } else { None },
            bwlimit: self.bwlimit,
            partial_dir: self.partial_dir.clone(),
            max_size: self.max_size.clone(),
            min_size: self.min_size.clone(),
            timeout: self.timeout,
            contimeout: self.contimeout,
            backup_dir: self.backup_dir.clone(),
            chmod: self.chmod.clone(),
            exclude: self.exclude.clone(),
            include: self.include.clone(),
            exclude_from: self.exclude_from.clone(),
            include_from: self.include_from.clone(),
        }
    }
}

/// Run sync based on arguments.
pub async fn run_sync(args: SyncArgs, ctx: AppContext) -> Result<()> {
    use crate::cli::commands::sync::wwpdb::{run_custom, run_custom_all};

    if args.list {
        crate::cli::commands::sync::wwpdb::list_custom(&ctx);
        return Ok(());
    }

    if args.all {
        run_custom_all(args, ctx).await
    } else if let Some(ref name) = args.name {
        run_custom(name.clone(), args, ctx).await
    } else {
        // No name specified, run all
        run_custom_all(args, ctx).await
    }
}
