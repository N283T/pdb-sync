use crate::mirrors::MirrorId;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub paths: PathsConfig,
    pub sync: SyncConfig,
    pub download: DownloadConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PathsConfig {
    pub pdb_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncConfig {
    #[serde(with = "mirror_id_serde")]
    pub mirror: MirrorId,
    pub bwlimit: u32,
    pub delete: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            mirror: MirrorId::Rcsb,
            bwlimit: 0,
            delete: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DownloadConfig {
    pub auto_decompress: bool,
    pub parallel: u8,
    pub default_format: String,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            auto_decompress: true,
            parallel: 4,
            default_format: "mmcif".to_string(),
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
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("mirror = \"rcsb\""));
    }

    #[test]
    fn test_config_deserialization() {
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
        assert_eq!(config.paths.pdb_dir, Some(PathBuf::from("/data/pdb")));
        assert_eq!(config.sync.mirror, MirrorId::Pdbj);
        assert_eq!(config.sync.bwlimit, 1000);
        assert!(!config.download.auto_decompress);
        assert_eq!(config.download.parallel, 8);
    }
}
