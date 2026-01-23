use crate::config::{Config, ConfigLoader};
use crate::error::Result;
use std::path::PathBuf;

/// Application context that combines configuration, environment variables, and CLI arguments
#[derive(Clone)]
pub struct AppContext {
    pub config: Config,
    pub pdb_dir: PathBuf,
}

impl AppContext {
    pub async fn new() -> Result<Self> {
        let config = ConfigLoader::load()?;

        // Priority: ENV > config > default
        let pdb_dir = std::env::var("PDB_DIR")
            .map(PathBuf::from)
            .ok()
            .or_else(|| config.paths.pdb_dir.clone())
            .unwrap_or_else(|| {
                directories::UserDirs::new()
                    .map(|d| d.home_dir().join("pdb"))
                    .unwrap_or_else(|| PathBuf::from("./pdb"))
            });

        Ok(Self { config, pdb_dir })
    }

    pub fn with_overrides(mut self, pdb_dir: Option<PathBuf>) -> Self {
        if let Some(dir) = pdb_dir {
            self.pdb_dir = dir;
        }
        self
    }
}
