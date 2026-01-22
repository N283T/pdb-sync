use crate::data_types::{DataType, Layout};
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

impl PathsConfig {
    /// Get the directory for a specific data type, if configured.
    #[allow(dead_code)]
    pub fn dir_for(&self, data_type: &DataType) -> Option<&PathBuf> {
        let key = data_type.to_string();
        self.data_type_dirs.get(&key)
    }
}

/// Custom rsync configuration for user-defined sync targets.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomRsyncConfig {
    /// Unique name for this custom sync config
    pub name: String,
    /// rsync URL (e.g., "data.pdbj.org::rsync/pub/emdb/" or "rsync://rsync.ebi.ac.uk/pub/databases/msd/sifts/")
    pub url: String,
    /// Destination directory relative to pdb_dir (e.g., "pub/emdb" or "pdbe/sifts")
    pub dest: String,
    /// Optional description
    #[serde(default)]
    pub description: Option<String>,

    // === Per-config rsync option defaults ===
    /// Delete files that don't exist on the remote
    #[serde(rename = "rsync_delete")]
    pub rsync_delete: bool,
    /// Compress data during transfer
    #[serde(rename = "rsync_compress")]
    pub rsync_compress: bool,
    /// Use checksum for file comparison
    #[serde(rename = "rsync_checksum")]
    pub rsync_checksum: bool,
    /// Keep partially transferred files
    #[serde(rename = "rsync_partial")]
    pub rsync_partial: bool,
    /// Directory for partial files
    #[serde(rename = "rsync_partial_dir")]
    pub rsync_partial_dir: Option<String>,
    /// Maximum file size to transfer
    #[serde(rename = "rsync_max_size")]
    pub rsync_max_size: Option<String>,
    /// Minimum file size to transfer
    #[serde(rename = "rsync_min_size")]
    pub rsync_min_size: Option<String>,
    /// I/O timeout in seconds
    #[serde(rename = "rsync_timeout")]
    pub rsync_timeout: Option<u32>,
    /// Connection timeout in seconds
    #[serde(rename = "rsync_contimeout")]
    pub rsync_contimeout: Option<u32>,
    /// Create backups
    #[serde(rename = "rsync_backup")]
    pub rsync_backup: bool,
    /// Backup directory
    #[serde(rename = "rsync_backup_dir")]
    pub rsync_backup_dir: Option<String>,
    /// Change permission flags
    #[serde(rename = "rsync_chmod")]
    pub rsync_chmod: Option<String>,
    /// Exclude patterns
    #[serde(rename = "rsync_exclude", default)]
    pub rsync_exclude: Vec<String>,
    /// Include patterns
    #[serde(rename = "rsync_include", default)]
    pub rsync_include: Vec<String>,
    /// File with exclude patterns
    #[serde(rename = "rsync_exclude_from")]
    pub rsync_exclude_from: Option<String>,
    /// File with include patterns
    #[serde(rename = "rsync_include_from")]
    pub rsync_include_from: Option<String>,
    /// Verbose output
    #[serde(rename = "rsync_verbose")]
    pub rsync_verbose: bool,
    /// Quiet mode
    #[serde(rename = "rsync_quiet")]
    pub rsync_quiet: bool,
    /// Itemize changes
    #[serde(rename = "rsync_itemize_changes")]
    pub rsync_itemize_changes: bool,
}

impl CustomRsyncConfig {
    /// Convert to RsyncFlags for use in rsync operations.
    pub fn to_rsync_flags(&self) -> crate::sync::RsyncFlags {
        crate::sync::RsyncFlags {
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
        }
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
    pub custom: Vec<CustomRsyncConfig>,
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
            custom: Vec::new(),
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
    fn test_data_type_dirs() {
        let toml_str = r#"
            [paths]
            pdb_dir = "/data/pdb"

            [paths.data_type_dirs]
            structures = "/data/pdb/structures"
            assemblies = "/data/pdb/assemblies"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.paths.dir_for(&DataType::Structures),
            Some(&PathBuf::from("/data/pdb/structures"))
        );
        assert_eq!(
            config.paths.dir_for(&DataType::Assemblies),
            Some(&PathBuf::from("/data/pdb/assemblies"))
        );
        assert_eq!(config.paths.dir_for(&DataType::Biounit), None);
    }

    #[test]
    fn test_default_data_types() {
        let config = Config::default();
        assert_eq!(config.sync.data_types, vec!["structures".to_string()]);
    }

    #[test]
    fn test_custom_rsync_config() {
        let toml_str = r#"
            [[sync.custom]]
            name = "pdbj-emdb"
            url = "data.pdbj.org::rsync/pub/emdb/"
            dest = "pub/emdb"
            description = "EMDB data"

            [[sync.custom]]
            name = "pdbe-sifts"
            url = "rsync://rsync.ebi.ac.uk/pub/databases/msd/sifts/"
            dest = "pdbe/sifts"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.sync.custom.len(), 2);
        assert_eq!(config.sync.custom[0].name, "pdbj-emdb");
        assert_eq!(config.sync.custom[0].url, "data.pdbj.org::rsync/pub/emdb/");
        assert_eq!(config.sync.custom[0].dest, "pub/emdb");
        assert_eq!(
            config.sync.custom[0].description,
            Some("EMDB data".to_string())
        );
        assert_eq!(config.sync.custom[1].name, "pdbe-sifts");
        assert_eq!(
            config.sync.custom[1].url,
            "rsync://rsync.ebi.ac.uk/pub/databases/msd/sifts/"
        );
        assert_eq!(config.sync.custom[1].dest, "pdbe/sifts");
        assert_eq!(config.sync.custom[1].description, None);
    }
}
