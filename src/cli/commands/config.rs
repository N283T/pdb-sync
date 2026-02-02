//! Config management commands.
//!
//! This module provides commands for managing pdb-sync configuration files:
//! - **init**: Create a new configuration file with sensible defaults
//! - **migrate**: Convert old `rsync_*` format to new preset/nested format
//! - **validate**: Check config file syntax and preset names
//! - **presets**: List available rsync flag presets
//!
//! # Examples
//!
//! ## Initializing Config
//!
//! ```bash
//! # Create a new config file
//! pdb-sync config init
//!
//! # Create minimal config with custom pdb_dir
//! pdb-sync config init --minimal --pdb-dir /data/pdb
//!
//! # Overwrite existing config (creates backup)
//! pdb-sync config init --force
//! ```
//!
//! ## Migrating Old Config
//!
//! ```bash
//! # Preview migration without modifying file
//! pdb-sync config migrate --dry-run
//!
//! # Actually migrate (creates backup as .toml.bak)
//! pdb-sync config migrate
//! ```
//!
//! ## Validating Config
//!
//! ```bash
//! # Validate default config
//! pdb-sync config validate
//!
//! # Validate specific config file
//! pdb-sync config validate --config-path /path/to/config.toml
//! ```
//!
//! ## Listing Presets
//!
//! ```bash
//! pdb-sync config presets
//! ```

use crate::config::schema::{Config, CustomRsyncConfig, RsyncOptionsConfig};
use crate::error::{PdbSyncError, Result};
use crate::sync::{list_rsync_presets, RsyncFlags, RsyncPreset};
use std::path::PathBuf;

/// Result of a migration attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MigrationType {
    /// Config was migrated to preset format
    ToPreset,
    /// Config was migrated to nested [options] format
    ToNested,
    /// Config is already using new format
    AlreadyNew,
}

/// Config subcommand variants.
#[derive(Debug, Clone)]
pub enum ConfigCommand {
    /// Initialize a new configuration file
    Init {
        /// Config file path (defaults to ~/.config/pdb-sync/config.toml)
        config_path: Option<PathBuf>,
        /// Overwrite existing config file
        force: bool,
        /// Generate minimal config
        minimal: bool,
        /// PDB directory path
        pdb_dir: Option<PathBuf>,
    },
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
        ConfigCommand::Init {
            config_path,
            force,
            minimal,
            pdb_dir,
        } => run_init(config_path, force, minimal, pdb_dir).await,
        ConfigCommand::Migrate {
            config_path,
            dry_run,
        } => run_migrate(config_path, dry_run).await,
        ConfigCommand::Validate { config_path } => run_validate(config_path).await,
        ConfigCommand::Presets => run_presets().await,
    }
}

/// Initialize a new configuration file.
async fn run_init(
    config_path: Option<PathBuf>,
    force: bool,
    minimal: bool,
    pdb_dir: Option<PathBuf>,
) -> Result<()> {
    let config_path = config_path.unwrap_or_else(|| {
        crate::config::ConfigLoader::config_path().unwrap_or_else(|| PathBuf::from("config.toml"))
    });

    // Check if config already exists
    if config_path.exists() && !force {
        return Err(PdbSyncError::Config {
            message: format!(
                "Config file already exists: {}\nUse --force to overwrite.",
                config_path.display()
            ),
            key: None,
            source: None,
        });
    }

    // Create parent directories if needed
    if let Some(parent) = config_path.parent() {
        if !parent.exists() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| PdbSyncError::Config {
                    message: format!("Failed to create config directory: {}", e),
                    key: None,
                    source: Some(Box::new(e)),
                })?;
            println!("Created directory: {}", parent.display());
        }
    }

    // If overwriting, create backup
    if config_path.exists() && force {
        let backup_path = PathBuf::from(format!("{}.bak", config_path.display()));
        tokio::fs::copy(&config_path, &backup_path)
            .await
            .map_err(|e| PdbSyncError::Config {
                message: format!("Failed to create backup: {}", e),
                key: None,
                source: Some(Box::new(e)),
            })?;
        println!("Created backup at: {}", backup_path.display());
    }

    // Generate config content
    let pdb_dir_provided = pdb_dir.is_some();
    let content = if minimal {
        generate_minimal_config(pdb_dir)
    } else {
        generate_full_config(pdb_dir)
    };

    // Write config file
    tokio::fs::write(&config_path, &content)
        .await
        .map_err(|e| PdbSyncError::Config {
            message: format!("Failed to write config file: {}", e),
            key: None,
            source: Some(Box::new(e)),
        })?;

    println!("Created config file: {}", config_path.display());
    println!();
    println!("Next steps:");
    if !pdb_dir_provided {
        println!("  1. Edit the config file to set your pdb_dir");
        println!("  2. Customize sync configurations as needed");
    } else {
        println!("  1. Customize sync configurations as needed");
    }
    println!("  Run: pdb-sync config validate");
    println!("  Run: pdb-sync sync --list");

    Ok(())
}

/// Generate minimal config content.
fn generate_minimal_config(pdb_dir: Option<PathBuf>) -> String {
    let pdb_dir_str = pdb_dir
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "/path/to/pdb".to_string());

    format!(
        r#"[paths]
pdb_dir = "{}"

[sync.custom.structures]
url = "rsync.wwpdb.org::ftp_data/structures/divided/mmCIF/"
dest = "data/structures"
preset = "safe"
"#,
        pdb_dir_str
    )
}

/// Generate full config content with comments.
fn generate_full_config(pdb_dir: Option<PathBuf>) -> String {
    let pdb_dir_str = pdb_dir
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "/path/to/pdb".to_string());

    format!(
        r#"# pdb-sync configuration file
# Documentation: https://github.com/nagaet/pdb-sync

[paths]
# Base directory for PDB data storage (required)
pdb_dir = "{pdb_dir}"

# Global defaults applied to all custom configs (DRY principle)
# Priority: options > preset > defaults
[sync.defaults]
compress = true
timeout = 300
partial = true

# ============================================================
# Custom sync configurations
# ============================================================

# Example: PDB structure files (mmCIF format)
[sync.custom.structures]
url = "rsync.wwpdb.org::ftp_data/structures/divided/mmCIF/"
dest = "data/structures"
description = "PDB structures (mmCIF format, divided layout)"
# Preset options: safe, fast, minimal, conservative
# - safe: No delete, compress, checksum (first-time sync)
# - fast: Delete enabled, no checksum (regular updates)
# - minimal: Bare minimum flags
# - conservative: Maximum safety with backups
preset = "safe"

# Override specific options (takes priority over preset)
# [sync.custom.structures.options]
# delete = true
# max_size = "10G"
# exclude = ["obsolete/"]

# ============================================================
# Additional examples (uncomment to enable)
# ============================================================

# Biological assemblies
# [sync.custom.assemblies]
# url = "rsync.wwpdb.org::ftp_data/structures/divided/assembly/"
# dest = "data/assemblies"
# description = "PDB biological assemblies"
# preset = "fast"

# EMDB (Electron Microscopy Data Bank)
# [sync.custom.emdb]
# url = "ftp.pdbj.org::ftp_data/emdb/"
# dest = "data/emdb"
# description = "EMDB structure maps"
# preset = "safe"
#
# [sync.custom.emdb.options]
# max_size = "5G"

# SIFTS (Structure Integration with Function, Taxonomy and Sequence)
# [sync.custom.sifts]
# url = "rsync.wwpdb.org::ftp/pdb/data/structures/divided/XML/"
# dest = "data/sifts"
# description = "SIFTS mappings"
# preset = "fast"
"#,
        pdb_dir = pdb_dir_str
    )
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
    let mut migrated_count = 0;
    let mut preset_count = 0;
    let mut nested_count = 0;

    println!("\nMigrating custom rsync configs:");
    for (name, custom) in &mut config.sync.custom {
        match try_migrate_custom_config(name, custom) {
            MigrationType::ToPreset => {
                migrated_count += 1;
                preset_count += 1;
            }
            MigrationType::ToNested => {
                migrated_count += 1;
                nested_count += 1;
            }
            MigrationType::AlreadyNew => {
                // Skip
            }
        }
    }

    if migrated_count == 0 {
        println!("No migration needed - config is already using new format or has no custom rsync configs.");
        return Ok(());
    }

    println!("\nMigration summary:");
    println!("  {} configs migrated", migrated_count);
    println!("  {} → preset format", preset_count);
    println!("  {} → nested [options] format", nested_count);

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
        // Create backup before modifying
        let backup_path = config_path.with_extension("toml.bak");
        tokio::fs::copy(&config_path, &backup_path)
            .await
            .map_err(|e| PdbSyncError::Config {
                message: format!("Failed to create backup: {}", e),
                key: None,
                source: Some(Box::new(e)),
            })?;
        println!("Created backup at: {}", backup_path.display());

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
/// Returns the type of migration performed.
fn try_migrate_custom_config(name: &str, custom: &mut CustomRsyncConfig) -> MigrationType {
    // If already using preset or options format, skip
    if custom.preset.is_some() || custom.options.is_some() {
        return MigrationType::AlreadyNew;
    }

    // Check if flags match a preset
    let current_flags = custom.to_rsync_flags(None);

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
            println!("  '{}' → preset = \"{}\"", name, preset_name);
            return MigrationType::ToPreset;
        }
    }

    // If no preset matches, convert to nested options format
    custom.options = Some(RsyncOptionsConfig {
        delete: Some(custom.rsync_delete),
        compress: Some(custom.rsync_compress),
        checksum: Some(custom.rsync_checksum),
        size_only: Some(custom.rsync_size_only),
        ignore_times: Some(custom.rsync_ignore_times),
        modify_window: custom.rsync_modify_window,
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
    println!("  '{}' → [options] nested format", name);
    MigrationType::ToNested
}

/// Check if two RsyncFlags are equivalent (ignoring bwlimit and dry_run).
fn flags_match(a: &RsyncFlags, b: &RsyncFlags) -> bool {
    a.delete == b.delete
        && a.compress == b.compress
        && a.checksum == b.checksum
        && a.size_only == b.size_only
        && a.ignore_times == b.ignore_times
        && a.modify_window == b.modify_window
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
    custom.rsync_size_only = false;
    custom.rsync_ignore_times = false;
    custom.rsync_modify_window = None;
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
    for (name, custom) in &config.sync.custom {
        let flags: RsyncFlags = custom.to_rsync_flags(config.sync.defaults.as_ref());
        flags.validate().map_err(|e| PdbSyncError::Config {
            message: format!("Invalid config for '{}': {}", name, e),
            key: Some(name.clone()),
            source: None,
        })?;

        // Validate preset name if specified
        if let Some(ref preset_name) = custom.preset {
            if crate::sync::get_rsync_preset(preset_name).is_none() {
                return Err(PdbSyncError::Config {
                    message: format!(
                        "Invalid preset '{}' for '{}'. Valid presets: safe, fast, minimal, conservative",
                        preset_name, name
                    ),
                    key: Some(format!("sync.custom.{}.preset", name)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_init_creates_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let result = run_init(Some(config_path.clone()), false, false, None).await;
        assert!(result.is_ok());
        assert!(config_path.exists());

        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("[paths]"));
        assert!(content.contains("pdb_dir"));
    }

    #[tokio::test]
    async fn test_init_minimal() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let result = run_init(Some(config_path.clone()), false, true, None).await;
        assert!(result.is_ok());

        let content = std::fs::read_to_string(&config_path).unwrap();
        // Minimal should not have extensive comments
        assert!(!content.contains("# Documentation:"));
        assert!(content.contains("[paths]"));
    }

    #[tokio::test]
    async fn test_init_with_pdb_dir() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        let pdb_dir = PathBuf::from("/custom/pdb/path");

        let result = run_init(Some(config_path.clone()), false, true, Some(pdb_dir)).await;
        assert!(result.is_ok());

        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("/custom/pdb/path"));
    }

    #[tokio::test]
    async fn test_init_fails_if_exists() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        // Create existing file
        std::fs::write(&config_path, "existing content").unwrap();

        let result = run_init(Some(config_path.clone()), false, false, None).await;
        assert!(result.is_err());

        // Content should be unchanged
        let content = std::fs::read_to_string(&config_path).unwrap();
        assert_eq!(content, "existing content");
    }

    #[tokio::test]
    async fn test_init_force_overwrites() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        // Create existing file
        std::fs::write(&config_path, "existing content").unwrap();

        let result = run_init(Some(config_path.clone()), true, false, None).await;
        assert!(result.is_ok());

        // Content should be new
        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("[paths]"));

        // Backup should exist
        let backup_path = config_path.with_extension("toml.bak");
        assert!(backup_path.exists());
        let backup_content = std::fs::read_to_string(&backup_path).unwrap();
        assert_eq!(backup_content, "existing content");
    }

    #[tokio::test]
    async fn test_init_creates_parent_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("nested/dir/config.toml");

        let result = run_init(Some(config_path.clone()), false, false, None).await;
        assert!(result.is_ok());
        assert!(config_path.exists());
    }

    #[test]
    fn test_generate_minimal_config() {
        let content = generate_minimal_config(None);
        assert!(content.contains("[paths]"));
        assert!(content.contains("pdb_dir = \"/path/to/pdb\""));
        assert!(content.contains("[sync.custom.structures]"));
        assert!(content.contains("preset = \"safe\""));
        // Should not have extensive comments
        assert!(!content.contains("# Documentation:"));
    }

    #[test]
    fn test_generate_full_config() {
        let content = generate_full_config(None);
        assert!(content.contains("[paths]"));
        assert!(content.contains("# Documentation:"));
        assert!(content.contains("[sync.defaults]"));
        assert!(content.contains("[sync.custom.structures]"));
        assert!(content.contains("# [sync.custom.assemblies]"));
    }

    #[test]
    fn test_generate_config_with_custom_pdb_dir() {
        let pdb_dir = PathBuf::from("/my/custom/pdb");
        let content = generate_minimal_config(Some(pdb_dir));
        assert!(content.contains("pdb_dir = \"/my/custom/pdb\""));
    }
}
