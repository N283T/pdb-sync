//! Config management commands.

use crate::config::schema::{Config, CustomRsyncConfig, RsyncOptionsConfig};
use crate::error::{PdbSyncError, Result};
use crate::sync::{list_rsync_presets, RsyncFlags, RsyncPreset};
use std::path::PathBuf;

/// Config subcommand variants.
#[derive(Debug, Clone)]
pub enum ConfigCommand {
    /// Migrate old config format to new nested format
    Migrate {
        /// Config file path (defaults to ~/.config/pdb-sync/config.toml)
        config_path: Option<PathBuf>,
        /// Dry run - show what would be changed without modifying the file
        dry_run: bool,
    },
    /// Validate config file syntax
    Validate {
        /// Config file path (defaults to ~/.config/pdb-sync/config.toml)
        config_path: Option<PathBuf>,
    },
    /// List available rsync flag presets
    Presets,
}

/// Run the config command.
pub async fn run_config(cmd: ConfigCommand) -> Result<()> {
    match cmd {
        ConfigCommand::Migrate {
            config_path,
            dry_run,
        } => run_migrate(config_path, dry_run).await,
        ConfigCommand::Validate { config_path } => run_validate(config_path).await,
        ConfigCommand::Presets => run_presets().await,
    }
}

/// Migrate old config format to new nested format.
async fn run_migrate(config_path: Option<PathBuf>, dry_run: bool) -> Result<()> {
    let config_path = config_path.unwrap_or_else(|| {
        crate::config::ConfigLoader::config_path().unwrap_or_else(|| PathBuf::from("config.toml"))
    });

    println!("Loading config from: {}", config_path.display());

    // Load existing config
    let content =
        tokio::fs::read_to_string(&config_path)
            .await
            .map_err(|e| PdbSyncError::Config {
                message: format!("Failed to read config file: {}", e),
                key: None,
                source: Some(Box::new(e)),
            })?;

    let mut config: Config = toml::from_str(&content).map_err(|e| PdbSyncError::Config {
        message: format!("Failed to parse config: {}", e),
        key: None,
        source: Some(Box::new(e)),
    })?;

    // Migrate custom rsync configs
    let mut migrated = false;
    for custom in &mut config.sync.custom {
        if try_migrate_custom_config(custom) {
            migrated = true;
        }
    }

    if !migrated {
        println!("No migration needed - config is already using new format or has no custom rsync configs.");
        return Ok(());
    }

    // Serialize migrated config
    let new_content = toml::to_string_pretty(&config).map_err(|e| PdbSyncError::Config {
        message: format!("Failed to serialize config: {}", e),
        key: None,
        source: Some(Box::new(e)),
    })?;

    if dry_run {
        println!("\n=== DRY RUN - Migrated config (not written to file) ===\n");
        println!("{}", new_content);
    } else {
        // Write back to file
        tokio::fs::write(&config_path, &new_content)
            .await
            .map_err(|e| PdbSyncError::Config {
                message: format!("Failed to write config file: {}", e),
                key: None,
                source: Some(Box::new(e)),
            })?;
        println!("Config migrated successfully to: {}", config_path.display());
    }

    Ok(())
}

/// Try to migrate a custom rsync config to new format.
///
/// Returns true if migration was performed.
fn try_migrate_custom_config(custom: &mut CustomRsyncConfig) -> bool {
    // If already using preset or options format, skip
    if custom.preset.is_some() || custom.options.is_some() {
        return false;
    }

    // Check if flags match a preset
    let current_flags = custom.to_rsync_flags();

    let presets = [
        (RsyncPreset::Safe, "safe"),
        (RsyncPreset::Fast, "fast"),
        (RsyncPreset::Minimal, "minimal"),
        (RsyncPreset::Conservative, "conservative"),
    ];

    for (preset_enum, preset_name) in &presets {
        let preset_flags = preset_enum.to_flags();
        if flags_match(&current_flags, &preset_flags) {
            // Use preset
            custom.preset = Some(preset_name.to_string());
            clear_legacy_fields(custom);
            println!("  {} → preset = \"{}\"", custom.name, preset_name);
            return true;
        }
    }

    // If no preset matches, convert to nested options format
    custom.options = Some(RsyncOptionsConfig {
        delete: Some(custom.rsync_delete),
        compress: Some(custom.rsync_compress),
        checksum: Some(custom.rsync_checksum),
        partial: Some(custom.rsync_partial),
        partial_dir: custom.rsync_partial_dir.clone(),
        max_size: custom.rsync_max_size.clone(),
        min_size: custom.rsync_min_size.clone(),
        timeout: custom.rsync_timeout,
        contimeout: custom.rsync_contimeout,
        backup: Some(custom.rsync_backup),
        backup_dir: custom.rsync_backup_dir.clone(),
        chmod: custom.rsync_chmod.clone(),
        exclude: custom.rsync_exclude.clone(),
        include: custom.rsync_include.clone(),
        exclude_from: custom.rsync_exclude_from.clone(),
        include_from: custom.rsync_include_from.clone(),
        verbose: Some(custom.rsync_verbose),
        quiet: Some(custom.rsync_quiet),
        itemize_changes: Some(custom.rsync_itemize_changes),
    });

    clear_legacy_fields(custom);
    println!("  {} → [options] nested format", custom.name);
    true
}

/// Check if two RsyncFlags are equivalent (ignoring bwlimit and dry_run).
fn flags_match(a: &RsyncFlags, b: &RsyncFlags) -> bool {
    a.delete == b.delete
        && a.compress == b.compress
        && a.checksum == b.checksum
        && a.partial == b.partial
        && a.backup == b.backup
        && a.verbose == b.verbose
        && a.quiet == b.quiet
        && a.itemize_changes == b.itemize_changes
        && a.partial_dir == b.partial_dir
        && a.max_size == b.max_size
        && a.min_size == b.min_size
        && a.timeout == b.timeout
        && a.contimeout == b.contimeout
        && a.backup_dir == b.backup_dir
        && a.chmod == b.chmod
        && a.exclude == b.exclude
        && a.include == b.include
        && a.exclude_from == b.exclude_from
        && a.include_from == b.include_from
}

/// Clear legacy rsync_* fields (set to defaults).
fn clear_legacy_fields(custom: &mut CustomRsyncConfig) {
    custom.rsync_delete = false;
    custom.rsync_compress = false;
    custom.rsync_checksum = false;
    custom.rsync_partial = false;
    custom.rsync_partial_dir = None;
    custom.rsync_max_size = None;
    custom.rsync_min_size = None;
    custom.rsync_timeout = None;
    custom.rsync_contimeout = None;
    custom.rsync_backup = false;
    custom.rsync_backup_dir = None;
    custom.rsync_chmod = None;
    custom.rsync_exclude = Vec::new();
    custom.rsync_include = Vec::new();
    custom.rsync_exclude_from = None;
    custom.rsync_include_from = None;
    custom.rsync_verbose = false;
    custom.rsync_quiet = false;
    custom.rsync_itemize_changes = false;
}

/// Validate config file syntax.
async fn run_validate(config_path: Option<PathBuf>) -> Result<()> {
    let config_path = config_path.unwrap_or_else(|| {
        crate::config::ConfigLoader::config_path().unwrap_or_else(|| PathBuf::from("config.toml"))
    });

    println!("Validating config: {}", config_path.display());

    // Load and parse config
    let content =
        tokio::fs::read_to_string(&config_path)
            .await
            .map_err(|e| PdbSyncError::Config {
                message: format!("Failed to read config file: {}", e),
                key: None,
                source: Some(Box::new(e)),
            })?;

    let config: Config = toml::from_str(&content).map_err(|e| PdbSyncError::Config {
        message: format!("Failed to parse config: {}", e),
        key: None,
        source: Some(Box::new(e)),
    })?;

    // Validate each custom rsync config
    for custom in &config.sync.custom {
        let flags: RsyncFlags = custom.to_rsync_flags();
        flags.validate().map_err(|e| PdbSyncError::Config {
            message: format!("Invalid config for '{}': {}", custom.name, e),
            key: Some(custom.name.clone()),
            source: None,
        })?;

        // Validate preset name if specified
        if let Some(ref preset_name) = custom.preset {
            if crate::sync::get_rsync_preset(preset_name).is_none() {
                return Err(PdbSyncError::Config {
                    message: format!(
                        "Invalid preset '{}' for '{}'. Valid presets: safe, fast, minimal, conservative",
                        preset_name, custom.name
                    ),
                    key: Some(format!("sync.custom[{}].preset", custom.name)),
                    source: None,
                });
            }
        }
    }

    println!("✓ Config is valid");
    println!("  {} custom rsync configs", config.sync.custom.len());

    Ok(())
}

/// List available rsync flag presets.
async fn run_presets() -> Result<()> {
    list_rsync_presets();
    Ok(())
}
