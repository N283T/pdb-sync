//! Merged configuration from multiple sources with source tracking.
//!
//! This module handles merging configuration values from different sources
//! (defaults, config file, environment variables, CLI arguments) while
//! tracking where each value originated.

#![allow(dead_code)]

use crate::config::source::SourcedValue;
use crate::config::Config;
use crate::mirrors::MirrorId;
use std::path::PathBuf;
///
/// This struct represents the final configuration after merging values from
/// all sources (defaults, config file, environment variables, and CLI arguments),
/// with each field tracking its origin.
#[derive(Debug, Clone)]
pub struct MergedConfig {
    /// PDB directory path
    pub pdb_dir: SourcedValue<Option<PathBuf>>,
    /// Mirror selection
    pub mirror: SourcedValue<Option<MirrorId>>,
}

impl MergedConfig {
    /// Create a new merged config from the given sources.
    ///
    /// Priority order (highest to lowest):
    /// 1. CLI arguments
    /// 2. Environment variables
    /// 3. Config file
    /// 4. Default values
    pub fn merge(
        cli_pdb_dir: Option<PathBuf>,
        cli_mirror: Option<MirrorId>,
        env_pdb_dir: Option<PathBuf>,
        env_mirror: Option<MirrorId>,
        config: Option<Config>,
        default_pdb_dir: Option<PathBuf>,
        default_mirror: MirrorId,
    ) -> Self {
        // Merge pdb_dir with priority: CLI > Env > Config > Default
        let pdb_dir = if let Some(val) = cli_pdb_dir {
            SourcedValue::from_cli(Some(val))
        } else if let Some(val) = env_pdb_dir {
            SourcedValue::from_env(Some(val))
        } else if let Some(cfg) = &config {
            SourcedValue::from_config(cfg.paths.pdb_dir.clone())
        } else {
            SourcedValue::with_default(default_pdb_dir)
        };

        // Merge mirror with priority: CLI > Env > Config > Default
        let mirror = if let Some(val) = cli_mirror {
            SourcedValue::from_cli(Some(val))
        } else if let Some(val) = env_mirror {
            SourcedValue::from_env(Some(val))
        } else if let Some(cfg) = &config {
            SourcedValue::from_config(Some(cfg.sync.mirror))
        } else {
            SourcedValue::with_default(Some(default_mirror))
        };

        Self { pdb_dir, mirror }
    }

    /// Get the PDB directory path.
    #[allow(dead_code)]
    pub fn get_pdb_dir(&self) -> Option<&PathBuf> {
        self.pdb_dir.value.as_ref()
    }

    /// Get the mirror selection.
    #[allow(dead_code)]
    pub fn get_mirror(&self) -> Option<&MirrorId> {
        self.mirror.value.as_ref()
    }

    /// Display all configuration sources for debugging.
    pub fn display_sources(&self) -> String {
        let mut output = String::new();

        output.push_str("Configuration Sources:\n");

        // PDB directory
        output.push_str(&format!(
            "  pdb_dir: {:?} (source: {})\n",
            self.pdb_dir.value, self.pdb_dir.source
        ));

        // Mirror
        output.push_str(&format!(
            "  mirror: {:?} (source: {})\n",
            self.mirror.value, self.mirror.source
        ));

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::source::FlagSource;

    #[test]
    fn test_merge_cli_priority() {
        let merged = MergedConfig::merge(
            Some(PathBuf::from("/cli")),
            Some(MirrorId::Pdbe),
            Some(PathBuf::from("/env")),
            Some(MirrorId::Pdbj),
            None,
            Some(PathBuf::from("/default")),
            MirrorId::Rcsb,
        );

        // CLI args should win
        assert_eq!(merged.get_pdb_dir(), Some(&PathBuf::from("/cli")));
        assert_eq!(merged.get_mirror(), Some(&MirrorId::Pdbe));
        assert_eq!(merged.pdb_dir.source, FlagSource::CliArg);
        assert_eq!(merged.mirror.source, FlagSource::CliArg);
    }

    #[test]
    fn test_merge_env_priority() {
        let merged = MergedConfig::merge(
            None, // No CLI
            None,
            Some(PathBuf::from("/env")),
            Some(MirrorId::Pdbj),
            None, // No config
            Some(PathBuf::from("/default")),
            MirrorId::Rcsb,
        );

        // Env vars should win
        assert_eq!(merged.get_pdb_dir(), Some(&PathBuf::from("/env")));
        assert_eq!(merged.get_mirror(), Some(&MirrorId::Pdbj));
        assert_eq!(merged.pdb_dir.source, FlagSource::Env);
        assert_eq!(merged.mirror.source, FlagSource::Env);
    }

    #[test]
    fn test_merge_config_priority() {
        let config = Config {
            sync: crate::config::schema::SyncConfig {
                mirror: MirrorId::Wwpdb,
                ..Default::default()
            },
            paths: crate::config::schema::PathsConfig {
                pdb_dir: Some(PathBuf::from("/config")),
                ..Default::default()
            },
            ..Default::default()
        };

        let merged = MergedConfig::merge(
            None, // No CLI
            None,
            None, // No env
            None,
            Some(config),
            Some(PathBuf::from("/default")),
            MirrorId::Rcsb,
        );

        // Config should win
        assert_eq!(merged.get_pdb_dir(), Some(&PathBuf::from("/config")));
        assert_eq!(merged.get_mirror(), Some(&MirrorId::Wwpdb));
        assert_eq!(merged.pdb_dir.source, FlagSource::Config);
        assert_eq!(merged.mirror.source, FlagSource::Config);
    }

    #[test]
    fn test_merge_default_fallback() {
        let merged = MergedConfig::merge(
            None, // No CLI
            None,
            None, // No env
            None,
            None, // No config
            Some(PathBuf::from("/default")),
            MirrorId::Rcsb,
        );

        // Defaults should be used
        assert_eq!(merged.get_pdb_dir(), Some(&PathBuf::from("/default")));
        assert_eq!(merged.get_mirror(), Some(&MirrorId::Rcsb));
        assert_eq!(merged.pdb_dir.source, FlagSource::Default);
        assert_eq!(merged.mirror.source, FlagSource::Default);
    }

    #[test]
    fn test_display_sources() {
        let merged = MergedConfig::merge(
            Some(PathBuf::from("/cli")),
            Some(MirrorId::Rcsb),
            None,
            None,
            None,
            Some(PathBuf::from("/default")),
            MirrorId::Pdbe,
        );

        let output = merged.display_sources();
        assert!(output.contains("pdb_dir"));
        assert!(output.contains("mirror"));
        assert!(output.contains("command-line argument"));
    }
}
