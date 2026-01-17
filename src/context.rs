use crate::config::{Config, ConfigLoader};
use crate::error::Result;
use crate::mirrors::MirrorId;
use std::path::PathBuf;

/// Application context that combines configuration, environment variables, and CLI arguments
pub struct AppContext {
    pub config: Config,
    pub pdb_dir: PathBuf,
    pub mirror: MirrorId,
}

impl AppContext {
    pub fn new() -> Result<Self> {
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

        let mirror = std::env::var("PDB_CLI_MIRROR")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(config.sync.mirror);

        Ok(Self {
            config,
            pdb_dir,
            mirror,
        })
    }

    pub fn with_overrides(
        mut self,
        pdb_dir: Option<PathBuf>,
        mirror: Option<MirrorId>,
    ) -> Self {
        if let Some(dir) = pdb_dir {
            self.pdb_dir = dir;
        }
        if let Some(m) = mirror {
            self.mirror = m;
        }
        self
    }
}
