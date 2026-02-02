use crate::config::Config;
use crate::error::Result;
use directories::{ProjectDirs, UserDirs};
use std::path::PathBuf;

pub struct ConfigLoader;

impl ConfigLoader {
    /// Get the config file path
    ///
    /// Priority:
    /// 1. PDB_SYNC_CONFIG environment variable
    /// 2. XDG_CONFIG_HOME/pdb-sync/config.toml (if exists)
    /// 3. ~/.config/pdb-sync/config.toml (XDG default, if exists)
    /// 4. Platform-specific directory (directories crate)
    pub fn config_path() -> Option<PathBuf> {
        // 1. Explicit environment variable (highest priority)
        if let Ok(path) = std::env::var("PDB_SYNC_CONFIG") {
            return Some(PathBuf::from(path));
        }

        // 2. XDG_CONFIG_HOME (if set and config exists)
        if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
            let path = PathBuf::from(xdg_config).join("pdb-sync/config.toml");
            if path.exists() {
                return Some(path);
            }
        }

        // 3. XDG default (~/.config/pdb-sync/config.toml, if exists)
        if let Some(user_dirs) = UserDirs::new() {
            let xdg_default = user_dirs.home_dir().join(".config/pdb-sync/config.toml");
            if xdg_default.exists() {
                return Some(xdg_default);
            }
        }

        // 4. Platform-specific directory (directories crate)
        Self::platform_config_dir().map(|dir| dir.join("config.toml"))
    }

    /// Get the platform-specific config directory (without XDG fallback)
    fn platform_config_dir() -> Option<PathBuf> {
        ProjectDirs::from("", "", "pdb-sync").map(|dirs| dirs.config_dir().to_path_buf())
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
}
