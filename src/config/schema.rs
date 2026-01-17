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
    pub download: DownloadConfig,
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
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DownloadConfig {
    pub auto_decompress: bool,
    pub parallel: usize,
    pub default_format: String,
    /// Number of retry attempts for failed downloads
    pub retry_count: u32,
    /// Download engine: "builtin" or "aria2c"
    pub engine: String,
    /// Number of connections per server for aria2c (-x flag)
    pub aria2c_connections: u32,
    /// Number of splits per download for aria2c (-s flag)
    pub aria2c_split: u32,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            auto_decompress: true,
            parallel: 4,
            default_format: "mmcif".to_string(),
            retry_count: 3,
            engine: "builtin".to_string(),
            aria2c_connections: 4,
            aria2c_split: 1,
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
        assert_eq!(config.download.parallel, 4);
        assert_eq!(config.sync.layout, Layout::Divided);
        assert_eq!(config.download.retry_count, 3);
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

            [download]
            auto_decompress = false
            parallel = 8
            retry_count = 5

            [mirror_selection]
            auto_select = true
            preferred_region = "jp"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.paths.pdb_dir, Some(PathBuf::from("/data/pdb")));
        assert_eq!(config.sync.mirror, MirrorId::Pdbj);
        assert_eq!(config.sync.bwlimit, 1000);
        assert_eq!(config.sync.layout, Layout::All);
        assert!(!config.download.auto_decompress);
        assert_eq!(config.download.parallel, 8);
        assert_eq!(config.download.retry_count, 5);
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

            [download]
            auto_decompress = false
            parallel = 8
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        // New fields should have defaults
        assert_eq!(config.sync.layout, Layout::Divided);
        assert_eq!(config.download.retry_count, 3);
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
}
