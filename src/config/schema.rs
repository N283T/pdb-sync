//! Configuration schema for pdb-sync.
//!
//! This module defines the TOML configuration structure with support for:
//! - **Preset-based configs**: Use built-in presets like "safe", "fast"
//! - **Nested options**: Override specific flags with `[sync.custom.options]`
//! - **Legacy format**: Backward compatible with old `rsync_*` fields
//!
//! # Examples
//!
//! ## Preset-Only Config
//!
//! ```toml
//! [[sync.custom]]
//! name = "structures"
//! url = "rsync.wwpdb.org::ftp_data/structures/"
//! dest = "data/structures"
//! preset = "fast"
//! ```
//!
//! ## Preset + Override
//!
//! ```toml
//! [[sync.custom]]
//! name = "structures"
//! url = "rsync.wwpdb.org::ftp_data/structures/"
//! dest = "data/structures"
//! preset = "fast"
//!
//! [sync.custom.options]
//! max_size = "5GB"
//! exclude = ["obsolete/"]
//! ```
//!
//! ## Fully Custom
//!
//! ```toml
//! [[sync.custom]]
//! name = "sifts"
//! url = "rsync.wwpdb.org::ftp_data/sifts/"
//! dest = "data/sifts"
//!
//! [sync.custom.options]
//! delete = true
//! compress = true
//! checksum = true
//! timeout = 300
//! ```

use crate::data_types::Layout;
use crate::mirrors::MirrorId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub paths: PathsConfig,
    pub sync: SyncConfig,
    pub mirror_selection: MirrorSelectionConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PathsConfig {
    pub pdb_dir: Option<PathBuf>,
    /// Per-data-type directories (e.g., "structures" -> "/data/pdb/structures")
    #[serde(default)]
    pub data_type_dirs: HashMap<String, PathBuf>,
}

/// Rsync options configuration (nested format, no `rsync_` prefix).
///
/// This is the new, cleaner format for rsync options.
/// Uses Option<bool> to distinguish "not set" from "explicitly set to false".
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RsyncOptionsConfig {
    /// Delete files that don't exist on the remote
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<bool>,
    /// Compress data during transfer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compress: Option<bool>,
    /// Use checksum for file comparison
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<bool>,
    /// Keep partially transferred files
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial: Option<bool>,
    /// Directory for partial files
    pub partial_dir: Option<String>,
    /// Maximum file size to transfer
    pub max_size: Option<String>,
    /// Minimum file size to transfer
    pub min_size: Option<String>,
    /// I/O timeout in seconds
    pub timeout: Option<u32>,
    /// Connection timeout in seconds
    pub contimeout: Option<u32>,
    /// Create backups
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup: Option<bool>,
    /// Backup directory
    pub backup_dir: Option<String>,
    /// Change permission flags
    pub chmod: Option<String>,
    /// Exclude patterns
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Include patterns
    #[serde(default)]
    pub include: Vec<String>,
    /// File with exclude patterns
    pub exclude_from: Option<String>,
    /// File with include patterns
    pub include_from: Option<String>,
    /// Verbose output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbose: Option<bool>,
    /// Quiet mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quiet: Option<bool>,
    /// Itemize changes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub itemize_changes: Option<bool>,
}

impl RsyncOptionsConfig {
    /// Convert to RsyncFlags for use in rsync operations.
    ///
    /// Only sets fields that are explicitly provided (Some).
    /// Fields that are None will be left at their default values in RsyncFlags.
    #[allow(dead_code)] // Used in production code via CustomRsyncConfig
    pub fn to_rsync_flags(&self) -> crate::sync::RsyncFlags {
        crate::sync::RsyncFlags {
            delete: self.delete.unwrap_or(false),
            compress: self.compress.unwrap_or(false),
            checksum: self.checksum.unwrap_or(false),
            partial: self.partial.unwrap_or(false),
            partial_dir: self.partial_dir.clone(),
            max_size: self.max_size.clone(),
            min_size: self.min_size.clone(),
            timeout: self.timeout,
            contimeout: self.contimeout,
            backup: self.backup.unwrap_or(false),
            backup_dir: self.backup_dir.clone(),
            chmod: self.chmod.clone(),
            exclude: self.exclude.clone(),
            include: self.include.clone(),
            exclude_from: self.exclude_from.clone(),
            include_from: self.include_from.clone(),
            verbose: self.verbose.unwrap_or(false),
            quiet: self.quiet.unwrap_or(false),
            itemize_changes: self.itemize_changes.unwrap_or(false),
            // bwlimit and dry_run are handled separately (from CLI args)
            bwlimit: None,
            dry_run: false,
        }
    }
}

/// Custom rsync configuration for user-defined sync targets.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomRsyncConfig {
    /// rsync URL (e.g., "data.pdbj.org::rsync/pub/emdb/" or "rsync://rsync.ebi.ac.uk/pub/databases/msd/sifts/")
    pub url: String,
    /// Destination directory relative to pdb_dir (e.g., "pub/emdb" or "pdbe/sifts")
    pub dest: String,
    /// Optional description
    #[serde(default)]
    pub description: Option<String>,

    // === New format: preset-based ===
    /// Preset name (safe, fast, minimal, conservative)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,

    /// Nested rsync options (new format without rsync_ prefix)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<RsyncOptionsConfig>,

    // === Legacy format: flat rsync_* fields (for backward compatibility) ===
    /// Delete files that don't exist on the remote
    #[serde(rename = "rsync_delete", alias = "delete")]
    pub rsync_delete: bool,
    /// Compress data during transfer
    #[serde(rename = "rsync_compress", alias = "compress")]
    pub rsync_compress: bool,
    /// Use checksum for file comparison
    #[serde(rename = "rsync_checksum", alias = "checksum")]
    pub rsync_checksum: bool,
    /// Keep partially transferred files
    #[serde(rename = "rsync_partial", alias = "partial")]
    pub rsync_partial: bool,
    /// Directory for partial files
    #[serde(rename = "rsync_partial_dir", alias = "partial_dir")]
    pub rsync_partial_dir: Option<String>,
    /// Maximum file size to transfer
    #[serde(rename = "rsync_max_size", alias = "max_size")]
    pub rsync_max_size: Option<String>,
    /// Minimum file size to transfer
    #[serde(rename = "rsync_min_size", alias = "min_size")]
    pub rsync_min_size: Option<String>,
    /// I/O timeout in seconds
    #[serde(rename = "rsync_timeout", alias = "timeout")]
    pub rsync_timeout: Option<u32>,
    /// Connection timeout in seconds
    #[serde(rename = "rsync_contimeout", alias = "contimeout")]
    pub rsync_contimeout: Option<u32>,
    /// Create backups
    #[serde(rename = "rsync_backup", alias = "backup")]
    pub rsync_backup: bool,
    /// Backup directory
    #[serde(rename = "rsync_backup_dir", alias = "backup_dir")]
    pub rsync_backup_dir: Option<String>,
    /// Change permission flags
    #[serde(rename = "rsync_chmod", alias = "chmod")]
    pub rsync_chmod: Option<String>,
    /// Exclude patterns
    #[serde(rename = "rsync_exclude", alias = "exclude", default)]
    pub rsync_exclude: Vec<String>,
    /// Include patterns
    #[serde(rename = "rsync_include", alias = "include", default)]
    pub rsync_include: Vec<String>,
    /// File with exclude patterns
    #[serde(rename = "rsync_exclude_from", alias = "exclude_from")]
    pub rsync_exclude_from: Option<String>,
    /// File with include patterns
    #[serde(rename = "rsync_include_from", alias = "include_from")]
    pub rsync_include_from: Option<String>,
    /// Verbose output
    #[serde(rename = "rsync_verbose", alias = "verbose")]
    pub rsync_verbose: bool,
    /// Quiet mode
    #[serde(rename = "rsync_quiet", alias = "quiet")]
    pub rsync_quiet: bool,
    /// Itemize changes
    #[serde(rename = "rsync_itemize_changes", alias = "itemize_changes")]
    pub rsync_itemize_changes: bool,
}

impl CustomRsyncConfig {
    /// Convert to RsyncFlags for use in rsync operations.
    ///
    /// Priority order: options > preset > legacy fields
    /// 1. Start with legacy fields (for backward compatibility)
    /// 2. If preset is specified, merge preset flags
    /// 3. If options is specified, apply options (field-by-field override)
    pub fn to_rsync_flags(&self) -> crate::sync::RsyncFlags {
        // Start with legacy fields (backward compatibility)
        let mut flags = crate::sync::RsyncFlags {
            delete: self.rsync_delete,
            compress: self.rsync_compress,
            checksum: self.rsync_checksum,
            partial: self.rsync_partial,
            partial_dir: self.rsync_partial_dir.clone(),
            max_size: self.rsync_max_size.clone(),
            min_size: self.rsync_min_size.clone(),
            timeout: self.rsync_timeout,
            contimeout: self.rsync_contimeout,
            backup: self.rsync_backup,
            backup_dir: self.rsync_backup_dir.clone(),
            chmod: self.rsync_chmod.clone(),
            exclude: self.rsync_exclude.clone(),
            include: self.rsync_include.clone(),
            exclude_from: self.rsync_exclude_from.clone(),
            include_from: self.rsync_include_from.clone(),
            verbose: self.rsync_verbose,
            quiet: self.rsync_quiet,
            itemize_changes: self.rsync_itemize_changes,
            // bwlimit and dry_run are handled separately (from CLI args)
            bwlimit: None,
            dry_run: false,
        };

        // Apply preset if specified
        if let Some(ref preset_name) = self.preset {
            if let Some(preset_flags) = crate::sync::get_rsync_preset(preset_name) {
                flags = flags.merge_with(&preset_flags);
            }
        }

        // Apply options if specified (highest priority)
        // Use field-by-field override to respect Option<bool> semantics
        if let Some(ref options) = self.options {
            if let Some(delete) = options.delete {
                flags.delete = delete;
            }
            if let Some(compress) = options.compress {
                flags.compress = compress;
            }
            if let Some(checksum) = options.checksum {
                flags.checksum = checksum;
            }
            if let Some(partial) = options.partial {
                flags.partial = partial;
            }
            if let Some(backup) = options.backup {
                flags.backup = backup;
            }
            if let Some(verbose) = options.verbose {
                flags.verbose = verbose;
            }
            if let Some(quiet) = options.quiet {
                flags.quiet = quiet;
            }
            if let Some(itemize_changes) = options.itemize_changes {
                flags.itemize_changes = itemize_changes;
            }

            // Option fields
            if options.partial_dir.is_some() {
                flags.partial_dir = options.partial_dir.clone();
            }
            if options.max_size.is_some() {
                flags.max_size = options.max_size.clone();
            }
            if options.min_size.is_some() {
                flags.min_size = options.min_size.clone();
            }
            if options.timeout.is_some() {
                flags.timeout = options.timeout;
            }
            if options.contimeout.is_some() {
                flags.contimeout = options.contimeout;
            }
            if options.backup_dir.is_some() {
                flags.backup_dir = options.backup_dir.clone();
            }
            if options.chmod.is_some() {
                flags.chmod = options.chmod.clone();
            }
            if options.exclude_from.is_some() {
                flags.exclude_from = options.exclude_from.clone();
            }
            if options.include_from.is_some() {
                flags.include_from = options.include_from.clone();
            }

            // Vec fields: non-empty overrides
            if !options.exclude.is_empty() {
                flags.exclude = options.exclude.clone();
            }
            if !options.include.is_empty() {
                flags.include = options.include.clone();
            }
        }

        flags
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncConfig {
    #[serde(with = "mirror_id_serde")]
    pub mirror: MirrorId,
    pub bwlimit: u32,
    pub delete: bool,
    /// Default layout for synced files
    pub layout: Layout,
    /// Data types to sync by default
    #[serde(default = "default_data_types")]
    pub data_types: Vec<String>,
    /// Custom rsync configurations
    #[serde(default)]
    pub custom: HashMap<String, CustomRsyncConfig>,
}

fn default_data_types() -> Vec<String> {
    vec!["structures".to_string()]
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            mirror: MirrorId::Rcsb,
            bwlimit: 0,
            delete: false,
            layout: Layout::default(),
            data_types: default_data_types(),
            custom: HashMap::new(),
        }
    }
}

/// Configuration for automatic mirror selection based on latency.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MirrorSelectionConfig {
    /// Enable automatic mirror selection based on latency
    pub auto_select: bool,
    /// Preferred region (e.g., "us", "jp", "europe")
    /// If set, prefer mirrors in this region within 2x latency tolerance
    pub preferred_region: Option<String>,
    /// TTL for latency cache in seconds
    pub latency_cache_ttl: u64,
}

impl Default for MirrorSelectionConfig {
    fn default() -> Self {
        Self {
            auto_select: false,
            preferred_region: None,
            latency_cache_ttl: 3600,
        }
    }
}

mod mirror_id_serde {
    use super::*;
    use serde::{Deserializer, Serializer};
    use std::str::FromStr;

    pub fn serialize<S>(mirror: &MirrorId, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&mirror.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<MirrorId, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        MirrorId::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.sync.mirror, MirrorId::Rcsb);
        assert_eq!(config.sync.layout, Layout::Divided);
        assert!(!config.mirror_selection.auto_select);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("mirror = \"rcsb\""));
        assert!(toml_str.contains("layout = \"divided\""));
    }

    #[test]
    fn test_config_deserialization() {
        let toml_str = r#"
            [paths]
            pdb_dir = "/data/pdb"

            [sync]
            mirror = "pdbj"
            bwlimit = 1000
            layout = "all"

            [mirror_selection]
            auto_select = true
            preferred_region = "jp"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.paths.pdb_dir, Some(PathBuf::from("/data/pdb")));
        assert_eq!(config.sync.mirror, MirrorId::Pdbj);
        assert_eq!(config.sync.bwlimit, 1000);
        assert_eq!(config.sync.layout, Layout::All);
        assert!(config.mirror_selection.auto_select);
        assert_eq!(
            config.mirror_selection.preferred_region,
            Some("jp".to_string())
        );
    }

    #[test]
    fn test_backward_compatibility() {
        // Old config format should still work
        let toml_str = r#"
            [paths]
            pdb_dir = "/data/pdb"

            [sync]
            mirror = "pdbj"
            bwlimit = 1000
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        // New fields should have defaults
        assert_eq!(config.sync.layout, Layout::Divided);
        assert!(!config.mirror_selection.auto_select);
    }

    #[test]
    fn test_default_data_types() {
        let config = Config::default();
        assert_eq!(config.sync.data_types, vec!["structures".to_string()]);
    }

    #[test]
    fn test_custom_rsync_config() {
        let toml_str = r#"
            [sync.custom.pdbj-emdb]
            url = "data.pdbj.org::rsync/pub/emdb/"
            dest = "pub/emdb"
            description = "EMDB data"

            [sync.custom.pdbe-sifts]
            url = "rsync://rsync.ebi.ac.uk/pub/databases/msd/sifts/"
            dest = "pdbe/sifts"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.sync.custom.len(), 2);

        let emdb = config.sync.custom.get("pdbj-emdb").unwrap();
        assert_eq!(emdb.url, "data.pdbj.org::rsync/pub/emdb/");
        assert_eq!(emdb.dest, "pub/emdb");
        assert_eq!(emdb.description, Some("EMDB data".to_string()));

        let sifts = config.sync.custom.get("pdbe-sifts").unwrap();
        assert_eq!(
            sifts.url,
            "rsync://rsync.ebi.ac.uk/pub/databases/msd/sifts/"
        );
        assert_eq!(sifts.dest, "pdbe/sifts");
        assert_eq!(sifts.description, None);
    }

    // === New format tests ===

    #[test]
    fn test_custom_config_old_format_backward_compat() {
        // Old format with rsync_ prefix should still work
        let toml_str = r#"
            [sync.custom.legacy]
            url = "example.org::data"
            dest = "data/legacy"
            rsync_delete = true
            rsync_compress = true
            rsync_checksum = true
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.sync.custom.len(), 1);

        let custom = config.sync.custom.get("legacy").unwrap();
        assert!(custom.rsync_delete);
        assert!(custom.rsync_compress);
        assert!(custom.rsync_checksum);

        let flags = custom.to_rsync_flags();
        assert!(flags.delete);
        assert!(flags.compress);
        assert!(flags.checksum);
    }

    #[test]
    fn test_custom_config_preset_only() {
        // New format: preset only
        let toml_str = r#"
            [sync.custom.structures]
            url = "rsync.wwpdb.org::ftp_data/structures/"
            dest = "data/structures"
            preset = "safe"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.sync.custom.len(), 1);

        let custom = config.sync.custom.get("structures").unwrap();
        assert_eq!(custom.preset, Some("safe".to_string()));

        let flags = custom.to_rsync_flags();
        // Safe preset: no delete, compress, checksum, partial, verbose
        assert!(!flags.delete);
        assert!(flags.compress);
        assert!(flags.checksum);
        assert!(flags.partial);
        assert!(flags.verbose);
    }

    #[test]
    fn test_custom_config_preset_with_override() {
        // New format: preset + override in [options]
        let toml_str = r#"
            [sync.custom.structures]
            url = "rsync.wwpdb.org::ftp_data/structures/"
            dest = "data/structures"
            preset = "fast"

            [sync.custom.structures.options]
            max_size = "5GB"
            exclude = ["obsolete/"]
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.sync.custom.len(), 1);

        let custom = config.sync.custom.get("structures").unwrap();
        assert_eq!(custom.preset, Some("fast".to_string()));
        assert!(custom.options.is_some());

        let options = custom.options.as_ref().unwrap();
        assert_eq!(options.max_size, Some("5GB".to_string()));
        assert_eq!(options.exclude, vec!["obsolete/".to_string()]);

        let flags = custom.to_rsync_flags();
        // Fast preset: delete, compress, no checksum, partial, quiet
        assert!(flags.delete);
        assert!(flags.compress);
        assert!(!flags.checksum);
        assert!(flags.partial);
        assert!(flags.quiet);
        // Options override
        assert_eq!(flags.max_size, Some("5GB".to_string()));
        assert_eq!(flags.exclude, vec!["obsolete/".to_string()]);
    }

    #[test]
    fn test_custom_config_nested_options_only() {
        // New format: fully custom with nested options (no preset)
        let toml_str = r#"
            [sync.custom.sifts]
            url = "rsync.wwpdb.org::ftp_data/sifts/"
            dest = "data/sifts"

            [sync.custom.sifts.options]
            delete = true
            compress = true
            checksum = true
            timeout = 300
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.sync.custom.len(), 1);

        let custom = config.sync.custom.get("sifts").unwrap();
        assert!(custom.preset.is_none());
        assert!(custom.options.is_some());

        let options = custom.options.as_ref().unwrap();
        assert_eq!(options.delete, Some(true));
        assert_eq!(options.compress, Some(true));
        assert_eq!(options.checksum, Some(true));
        assert_eq!(options.timeout, Some(300));

        let flags = custom.to_rsync_flags();
        assert!(flags.delete);
        assert!(flags.compress);
        assert!(flags.checksum);
        assert_eq!(flags.timeout, Some(300));
    }

    #[test]
    fn test_custom_config_priority_order() {
        // Test priority: options > preset > legacy
        let toml_str = r#"
            [sync.custom.test]
            url = "example.org::data"
            dest = "data/test"

            # Legacy: delete=false, compress=false
            rsync_delete = false
            rsync_compress = false

            # Preset "fast": delete=true, compress=true
            preset = "fast"

            # Options: delete override to false (highest priority)
            [sync.custom.test.options]
            delete = false
            max_size = "1G"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        let custom = config.sync.custom.get("test").unwrap();

        let flags = custom.to_rsync_flags();

        // Priority test:
        // - delete: options (false) > preset (true) > legacy (false) = false
        // - compress: preset (true) > legacy (false) = true
        // - max_size: options (1G) = 1G
        assert!(!flags.delete); // options override
        assert!(flags.compress); // preset applies
        assert_eq!(flags.max_size, Some("1G".to_string())); // options adds new field
    }

    #[test]
    fn test_rsync_options_config_to_flags() {
        let options = RsyncOptionsConfig {
            delete: Some(true),
            compress: Some(true),
            max_size: Some("1G".to_string()),
            exclude: vec!["*.tmp".to_string()],
            ..Default::default()
        };

        let flags = options.to_rsync_flags();
        assert!(flags.delete);
        assert!(flags.compress);
        assert_eq!(flags.max_size, Some("1G".to_string()));
        assert_eq!(flags.exclude, vec!["*.tmp".to_string()]);
    }
}
