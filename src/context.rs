use crate::config::{Config, ConfigLoader};
use crate::error::Result;
use crate::mirrors::{select_best_mirror, MirrorId};
use std::path::PathBuf;
use std::time::Duration;

/// Application context that combines configuration, environment variables, and CLI arguments
#[derive(Clone)]
pub struct AppContext {
    pub config: Config,
    pub pdb_dir: PathBuf,
    pub mirror: MirrorId,
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

        // Mirror selection priority: ENV > auto-select > config
        let mirror = if let Ok(env_mirror) = std::env::var("PDB_SYNC_MIRROR") {
            env_mirror.parse().unwrap_or(config.sync.mirror)
        } else if config.mirror_selection.auto_select {
            let ttl = Duration::from_secs(config.mirror_selection.latency_cache_ttl);
            let preferred = config.mirror_selection.preferred_region.as_deref();
            select_best_mirror(preferred, ttl).await
        } else {
            config.sync.mirror
        };

        Ok(Self {
            config,
            pdb_dir,
            mirror,
        })
    }

    pub fn with_overrides(mut self, pdb_dir: Option<PathBuf>, mirror: Option<MirrorId>) -> Self {
        if let Some(dir) = pdb_dir {
            self.pdb_dir = dir;
        }
        if let Some(m) = mirror {
            self.mirror = m;
        }
        self
    }
}
