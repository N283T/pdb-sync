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

    /// Plan mode - show what would change without executing
    #[arg(long)]
    pub plan: bool,

    /// List available profile presets
    #[arg(long)]
    pub profile_list: bool,

    /// Add a profile preset to config
    #[arg(long, value_name = "NAME")]
    pub profile_add: Option<String>,

    /// Dry-run for profile add (show what would be added without modifying config)
    #[arg(long)]
    pub profile_dry_run: bool,

    /// Maximum number of concurrent sync operations
    #[arg(long, value_name = "N")]
    pub parallel: Option<usize>,

    /// Number of retry attempts on failure (0 = no retry)
    #[arg(long, default_value = "0")]
    pub retry: u32,

    /// Delay between retries in seconds (default: exponential backoff)
    #[arg(long, value_name = "SECONDS")]
    pub retry_delay: Option<u32>,
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

    /// Validate sync arguments.
    pub fn validate(&self) -> Result<()> {
        if let Some(p) = self.parallel {
            if p == 0 {
                return Err(crate::error::PdbSyncError::InvalidInput(
                    "parallel must be greater than 0".to_string(),
                ));
            }
            if p > 100 {
                return Err(crate::error::PdbSyncError::InvalidInput(
                    "parallel cannot exceed 100 (to prevent resource exhaustion)".to_string(),
                ));
            }
        }

        // Validate retry count
        if self.retry > 100 {
            return Err(crate::error::PdbSyncError::InvalidInput(
                "retry count cannot exceed 100 (to prevent excessively long operations)"
                    .to_string(),
            ));
        }

        // Warn when --retry is combined with --delete
        // (Retrying with delete can cause unexpected file loss)
        let delete_enabled = self.delete && !self.no_delete;
        if self.retry > 0 && delete_enabled {
            eprintln!(
                "Warning: Using --retry with --delete may cause unexpected file loss on transient failures. \
                 Consider using --retry without --delete, or --fail-fast with --all to stop on first error."
            );
        }

        Ok(())
    }
}

/// Run sync based on arguments.
pub async fn run_sync(args: SyncArgs, ctx: AppContext) -> Result<()> {
    use crate::cli::commands::sync::wwpdb::{run_custom, run_custom_all};

    // Validate arguments
    args.validate()?;

    // Handle profile list
    if args.profile_list {
        crate::sync::presets::list_presets();
        return Ok(());
    }

    // Handle profile add
    if let Some(ref profile_name) = args.profile_add {
        use crate::sync::presets;

        let preset = presets::get_preset(profile_name).ok_or_else(|| {
            crate::error::PdbSyncError::InvalidInput(format!(
                "Profile preset '{}' not found. Use --profile-list to see available presets.",
                profile_name
            ))
        })?;

        if args.profile_dry_run {
            println!("Dry-run mode - would add the following profile to config:");
            println!();
            println!("Name: {}", preset.name);
            println!("  URL: {}", preset.url);
            println!("  Destination: {}", preset.dest);
            println!("  Description: {}", preset.description);
            return Ok(());
        }

        // Add preset to config with file locking held throughout
        // (read-modify-write in single atomic operation to prevent TOCTOU race)
        let preset_name = preset.name.clone();
        let preset_name_for_print = preset.name.clone();
        let config_path = tokio::task::spawn_blocking(move || {
            use fs2::FileExt;
            use std::fs::OpenOptions;
            use std::io::Write;

            let config_path: std::path::PathBuf = crate::config::ConfigLoader::config_path()
                .ok_or_else(|| crate::error::PdbSyncError::Config {
                    message: "Could not determine config file path".to_string(),
                    key: None,
                    source: None,
                })?;

            // Create config directory if it doesn't exist
            if let Some(parent) = config_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    crate::error::PdbSyncError::Config {
                        message: format!("Failed to create config directory: {}", e),
                        key: None,
                        source: None,
                    }
                })?;
            }

            // Open file with read+write mode (creates if doesn't exist)
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(config_path.as_path())
                .map_err(|e| crate::error::PdbSyncError::Config {
                    message: format!("Failed to open config file: {}", e),
                    key: None,
                    source: None,
                })?;

            // Acquire exclusive lock - HELD THROUGHOUT ENTIRE OPERATION
            file.lock_exclusive()
                .map_err(|e| crate::error::PdbSyncError::Config {
                    message: format!("Failed to lock config file: {}", e),
                    key: None,
                    source: None,
                })?;

            // Read existing config or use default
            let mut config = if config_path.exists() {
                let metadata = file
                    .metadata()
                    .map_err(|e| crate::error::PdbSyncError::Config {
                        message: format!("Failed to get file metadata: {}", e),
                        key: None,
                        source: None,
                    })?;

                if metadata.len() > 0 {
                    let mut content = String::new();
                    use std::io::Read;
                    file.read_to_string(&mut content).map_err(|e| {
                        crate::error::PdbSyncError::Config {
                            message: format!("Failed to read config file: {}", e),
                            key: None,
                            source: None,
                        }
                    })?;
                    toml::from_str(&content).map_err(|e| crate::error::PdbSyncError::Config {
                        message: format!("Failed to parse config file: {}", e),
                        key: None,
                        source: None,
                    })?
                } else {
                    crate::config::Config::default()
                }
            } else {
                crate::config::Config::default()
            };

            // Check for conflicting name (while lock is still held!)
            if config.sync.custom.contains_key(&preset_name) {
                return Err(crate::error::PdbSyncError::InvalidInput(format!(
                    "A config with name '{}' already exists. Please remove it first or choose a different name.",
                    preset_name
                )));
            }

            // Add preset to config (while lock is still held!)
            config
                .sync
                .custom
                .insert(preset.name.clone(), crate::config::schema::CustomRsyncConfig {
                    url: preset.url.clone(),
                    dest: preset.dest.clone(),
                    description: Some(preset.description.clone()),
                    preset: None,
                    options: None,
                    rsync_delete: false,
                    rsync_compress: false,
                    rsync_checksum: false,
                    rsync_partial: false,
                    rsync_partial_dir: None,
                    rsync_max_size: None,
                    rsync_min_size: None,
                    rsync_timeout: None,
                    rsync_contimeout: None,
                    rsync_backup: false,
                    rsync_backup_dir: None,
                    rsync_chmod: None,
                    rsync_exclude: Vec::new(),
                    rsync_include: Vec::new(),
                    rsync_exclude_from: None,
                    rsync_include_from: None,
                    rsync_verbose: false,
                    rsync_quiet: false,
                    rsync_itemize_changes: false,
                });

            // Truncate and write new config (while lock is still held!)
            file.set_len(0).map_err(|e| crate::error::PdbSyncError::Config {
                message: format!("Failed to truncate config file: {}", e),
                key: None,
                source: None,
            })?;

            let toml_string = toml::to_string_pretty(&config).map_err(|e| {
                crate::error::PdbSyncError::Config {
                    message: format!("Failed to serialize config: {}", e),
                    key: None,
                    source: None,
                }
            })?;

            file.write_all(toml_string.as_bytes()).map_err(|e| {
                crate::error::PdbSyncError::Config {
                    message: format!("Failed to write config file: {}", e),
                    key: None,
                    source: None,
                }
            })?;

            // Sync to ensure data is written to disk
            file.sync_all()
                .map_err(|e| crate::error::PdbSyncError::Config {
                    message: format!("Failed to sync config file: {}", e),
                    key: None,
                    source: None,
                })?;

            // Lock is released when file is dropped here
            Ok::<std::path::PathBuf, crate::error::PdbSyncError>(config_path)
        })
        .await
        .map_err(|e| crate::error::PdbSyncError::Job(format!("Failed to add profile: {}", e)))??;

        println!(
            "Added profile '{}' to config file: {}",
            preset_name_for_print,
            config_path.display()
        );
        return Ok(());
    }

    if args.list {
        crate::cli::commands::sync::wwpdb::list_custom(&ctx);
        return Ok(());
    }

    // Warn if --parallel is set for single config sync
    if args.parallel.is_some() && args.name.is_some() {
        eprintln!("Warning: --parallel is ignored when syncing a single config (use --all or omit NAME to run multiple configs in parallel)");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_retry_too_high() {
        let args = SyncArgs {
            retry: 101,
            name: None,
            all: false,
            dest: None,
            list: false,
            fail_fast: false,
            dry_run: false,
            delete: false,
            no_delete: false,
            compress: false,
            no_compress: false,
            checksum: false,
            no_checksum: false,
            partial: false,
            no_partial: false,
            partial_dir: None,
            max_size: None,
            min_size: None,
            timeout: None,
            contimeout: None,
            backup: false,
            no_backup: false,
            backup_dir: None,
            chmod: None,
            exclude: None,
            include: None,
            exclude_from: None,
            include_from: None,
            bwlimit: None,
            rsync_verbose: false,
            rsync_quiet: false,
            no_rsync_verbose: false,
            no_rsync_quiet: false,
            itemize_changes: false,
            no_itemize_changes: false,
            plan: false,
            profile_list: false,
            profile_add: None,
            profile_dry_run: false,
            parallel: None,
            retry_delay: None,
        };
        assert!(args.validate().is_err());
    }

    #[test]
    fn test_validate_retry_max_allowed() {
        let args = SyncArgs {
            retry: 100,
            name: None,
            all: false,
            dest: None,
            list: false,
            fail_fast: false,
            dry_run: false,
            delete: false,
            no_delete: false,
            compress: false,
            no_compress: false,
            checksum: false,
            no_checksum: false,
            partial: false,
            no_partial: false,
            partial_dir: None,
            max_size: None,
            min_size: None,
            timeout: None,
            contimeout: None,
            backup: false,
            no_backup: false,
            backup_dir: None,
            chmod: None,
            exclude: None,
            include: None,
            exclude_from: None,
            include_from: None,
            bwlimit: None,
            rsync_verbose: false,
            rsync_quiet: false,
            no_rsync_verbose: false,
            no_rsync_quiet: false,
            itemize_changes: false,
            no_itemize_changes: false,
            plan: false,
            profile_list: false,
            profile_add: None,
            profile_dry_run: false,
            parallel: None,
            retry_delay: None,
        };
        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_validate_retry_zero_ok() {
        let args = SyncArgs {
            retry: 0,
            name: None,
            all: false,
            dest: None,
            list: false,
            fail_fast: false,
            dry_run: false,
            delete: false,
            no_delete: false,
            compress: false,
            no_compress: false,
            checksum: false,
            no_checksum: false,
            partial: false,
            no_partial: false,
            partial_dir: None,
            max_size: None,
            min_size: None,
            timeout: None,
            contimeout: None,
            backup: false,
            no_backup: false,
            backup_dir: None,
            chmod: None,
            exclude: None,
            include: None,
            exclude_from: None,
            include_from: None,
            bwlimit: None,
            rsync_verbose: false,
            rsync_quiet: false,
            no_rsync_verbose: false,
            no_rsync_quiet: false,
            itemize_changes: false,
            no_itemize_changes: false,
            plan: false,
            profile_list: false,
            profile_add: None,
            profile_dry_run: false,
            parallel: None,
            retry_delay: None,
        };
        assert!(args.validate().is_ok());
    }
}
