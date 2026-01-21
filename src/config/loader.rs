use crate::config::Config;
use crate::error::{PdbSyncError, Result};
use directories::ProjectDirs;
use std::path::PathBuf;

pub struct ConfigLoader;

impl ConfigLoader {
    /// Get the default config directory path
    pub fn config_dir() -> Option<PathBuf> {
        if let Ok(path) = std::env::var("PDB_SYNC_CONFIG") {
            let path = PathBuf::from(path);
            return path.parent().map(|p| p.to_path_buf());
        }

        ProjectDirs::from("", "", "pdb-sync").map(|dirs| dirs.config_dir().to_path_buf())
    }

    /// Get the config file path
    pub fn config_path() -> Option<PathBuf> {
        if let Ok(path) = std::env::var("PDB_SYNC_CONFIG") {
            return Some(PathBuf::from(path));
        }

        Self::config_dir().map(|dir| dir.join("config.toml"))
    }

    /// Load config from file, or return default if not found
    pub fn load() -> Result<Config> {
        let path = match Self::config_path() {
            Some(p) => p,
            None => return Ok(Config::default()),
        };

        if !path.exists() {
            return Ok(Config::default());
        }

        let content = std::fs::read_to_string(&path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save config to file
    pub fn save(config: &Config) -> Result<()> {
        let path = Self::config_path()
            .ok_or_else(|| PdbSyncError::Config("Cannot determine config path".into()))?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(config)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Initialize config file with defaults
    pub fn init() -> Result<PathBuf> {
        let path = Self::config_path()
            .ok_or_else(|| PdbSyncError::Config("Cannot determine config path".into()))?;

        if path.exists() {
            return Err(PdbSyncError::Config(format!(
                "Config file already exists at {}",
                path.display()
            )));
        }

        let config = Config::default();
        Self::save(&config)?;
        Ok(path)
    }
}
