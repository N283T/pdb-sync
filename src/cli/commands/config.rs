use crate::cli::args::{ConfigAction, ConfigArgs};
use crate::config::{ConfigLoader, MergedConfig};
use crate::context::AppContext;
use crate::data_types::{DataType, Layout};
use crate::error::{PdbSyncError, Result};
use crate::mirrors::{print_mirror_latencies, MirrorId};
use crate::utils::{header, info, success};
use colored::Colorize;
use std::env;
use std::path::PathBuf;

pub async fn run_config(args: ConfigArgs, ctx: crate::context::AppContext) -> Result<()> {
    match args.action {
        ConfigAction::Init => {
            let path = ConfigLoader::init()?;
            success(&format!("Configuration initialized at: {}", path.display()));
        }
        ConfigAction::Show => {
            let config = ConfigLoader::load()?;
            let toml_str = toml::to_string_pretty(&config)?;
            println!("{}", toml_str);

            if let Some(path) = ConfigLoader::config_path() {
                println!();
                info(&format!("Config file: {}", path.display()));
            }
        }
        ConfigAction::Get { key } => {
            let config = ConfigLoader::load()?;
            let value = get_config_value(&config, &key)?;
            println!("{}", value);
        }
        ConfigAction::Set { key, value } => {
            let mut config = ConfigLoader::load()?;
            set_config_value(&mut config, &key, &value)?;
            ConfigLoader::save(&config)?;
            success(&format!("Set {} = {}", key.cyan(), value.yellow()));
        }
        ConfigAction::TestMirrors => {
            header("Testing Mirror Latencies");
            println!();
            print_mirror_latencies().await;
        }
        ConfigAction::Sources => {
            run_sources(ctx)?;
        }
    }

    Ok(())
}

/// Show where each configuration value is coming from.
fn run_sources(_ctx: AppContext) -> Result<()> {
    header("Configuration Sources");

    // Get CLI values (these would normally come from args, but for now we use None)
    let cli_pdb_dir = None;
    let cli_mirror = None;

    // Get environment values
    let env_pdb_dir = env::var("PDB_DIR").ok().map(PathBuf::from);
    let env_mirror = env::var("PDB_SYNC_MIRROR")
        .ok()
        .and_then(|s| s.parse().ok());

    // Get config file values
    let config = ConfigLoader::load().ok();

    // Default values
    let default_pdb_dir = env::var("HOME").ok().map(|p| PathBuf::from(p).join(".pdb"));
    let default_mirror = MirrorId::Rcsb;

    // Merge with source tracking
    let merged = MergedConfig::merge(
        cli_pdb_dir,
        cli_mirror,
        env_pdb_dir,
        env_mirror,
        config,
        default_pdb_dir,
        default_mirror,
    );

    // Display sources
    println!("{}", merged.display_sources());

    // Show legend
    println!();
    info("Priority order: command-line > environment variable > config file > default");

    Ok(())
}

fn get_config_value(config: &crate::config::Config, key: &str) -> Result<String> {
    match key {
        // paths.*
        "paths.pdb_dir" => Ok(config
            .paths
            .pdb_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default()),

        // sync.*
        "sync.mirror" => Ok(config.sync.mirror.to_string()),
        "sync.bwlimit" => Ok(config.sync.bwlimit.to_string()),
        "sync.delete" => Ok(config.sync.delete.to_string()),
        "sync.layout" => Ok(config.sync.layout.to_string()),
        "sync.data_types" => Ok(config.sync.data_types.join(",")),

        // download.*
        "download.auto_decompress" => Ok(config.download.auto_decompress.to_string()),
        "download.parallel" => Ok(config.download.parallel.to_string()),
        "download.default_format" => Ok(config.download.default_format.clone()),
        "download.retry_count" => Ok(config.download.retry_count.to_string()),

        // mirror_selection.*
        "mirror_selection.auto_select" => Ok(config.mirror_selection.auto_select.to_string()),
        "mirror_selection.preferred_region" => Ok(config
            .mirror_selection
            .preferred_region
            .clone()
            .unwrap_or_default()),
        "mirror_selection.latency_cache_ttl" => {
            Ok(config.mirror_selection.latency_cache_ttl.to_string())
        }

        _ => Err(PdbSyncError::Config {
            message: format!("Unknown config key: {}", key),
            key: Some(key.to_string()),
            source: None,
        }),
    }
}

fn set_config_value(config: &mut crate::config::Config, key: &str, value: &str) -> Result<()> {
    match key {
        // paths.*
        "paths.pdb_dir" => {
            config.paths.pdb_dir = if value.is_empty() {
                None
            } else {
                Some(std::path::PathBuf::from(value))
            };
        }

        // sync.*
        "sync.mirror" => {
            config.sync.mirror = value.parse()?;
        }
        "sync.bwlimit" => {
            config.sync.bwlimit = value.parse().map_err(|_| PdbSyncError::Config {
                message: "Invalid bwlimit value".to_string(),
                key: Some(key.to_string()),
                source: None,
            })?;
        }
        "sync.delete" => {
            config.sync.delete = value.parse().map_err(|_| PdbSyncError::Config {
                message: "Invalid boolean value".to_string(),
                key: Some(key.to_string()),
                source: None,
            })?;
        }
        "sync.layout" => {
            config.sync.layout = match value.to_lowercase().as_str() {
                "divided" => Layout::Divided,
                "all" => Layout::All,
                _ => {
                    return Err(PdbSyncError::Config {
                        message: format!("Invalid layout: {}. Use 'divided' or 'all'", value),
                        key: Some(key.to_string()),
                        source: None,
                    })
                }
            };
        }
        "sync.data_types" => {
            let valid_types: Vec<String> =
                DataType::all().iter().map(|dt| dt.to_string()).collect();
            let parsed: Vec<String> = value
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            for dt in &parsed {
                if !valid_types.contains(dt) {
                    return Err(PdbSyncError::Config {
                        message: format!(
                            "Unknown data type: '{}'. Valid types: {}",
                            dt,
                            valid_types.join(", ")
                        ),
                        key: Some(key.to_string()),
                        source: None,
                    });
                }
            }
            config.sync.data_types = parsed;
        }

        // download.*
        "download.auto_decompress" => {
            config.download.auto_decompress = value.parse().map_err(|_| PdbSyncError::Config {
                message: "Invalid boolean value".to_string(),
                key: Some(key.to_string()),
                source: None,
            })?;
        }
        "download.parallel" => {
            config.download.parallel = value.parse().map_err(|_| PdbSyncError::Config {
                message: "Invalid parallel value".to_string(),
                key: Some(key.to_string()),
                source: None,
            })?;
        }
        "download.default_format" => {
            config.download.default_format = value.to_string();
        }
        "download.retry_count" => {
            config.download.retry_count = value.parse().map_err(|_| PdbSyncError::Config {
                message: "Invalid retry_count value".to_string(),
                key: Some(key.to_string()),
                source: None,
            })?;
        }

        // mirror_selection.*
        "mirror_selection.auto_select" => {
            config.mirror_selection.auto_select =
                value.parse().map_err(|_| PdbSyncError::Config {
                    message: "Invalid boolean value".to_string(),
                    key: Some(key.to_string()),
                    source: None,
                })?;
        }
        "mirror_selection.preferred_region" => {
            config.mirror_selection.preferred_region = if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            };
        }
        "mirror_selection.latency_cache_ttl" => {
            config.mirror_selection.latency_cache_ttl =
                value.parse().map_err(|_| PdbSyncError::Config {
                    message: "Invalid latency_cache_ttl value".to_string(),
                    key: Some(key.to_string()),
                    source: None,
                })?;
        }

        _ => {
            return Err(PdbSyncError::Config {
                message: format!("Unknown config key: {}", key),
                key: Some(key.to_string()),
                source: None,
            })
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn test_get_config_value_new_keys() {
        let config = Config::default();

        // sync.layout
        assert_eq!(get_config_value(&config, "sync.layout").unwrap(), "divided");

        // sync.data_types
        assert_eq!(
            get_config_value(&config, "sync.data_types").unwrap(),
            "structures"
        );

        // download.retry_count
        assert_eq!(
            get_config_value(&config, "download.retry_count").unwrap(),
            "3"
        );

        // mirror_selection.*
        assert_eq!(
            get_config_value(&config, "mirror_selection.auto_select").unwrap(),
            "false"
        );
        assert_eq!(
            get_config_value(&config, "mirror_selection.preferred_region").unwrap(),
            ""
        );
        assert_eq!(
            get_config_value(&config, "mirror_selection.latency_cache_ttl").unwrap(),
            "3600"
        );
    }

    #[test]
    fn test_set_config_value_new_keys() {
        let mut config = Config::default();

        // sync.layout
        set_config_value(&mut config, "sync.layout", "all").unwrap();
        assert_eq!(config.sync.layout, Layout::All);

        set_config_value(&mut config, "sync.layout", "divided").unwrap();
        assert_eq!(config.sync.layout, Layout::Divided);

        // sync.data_types
        set_config_value(&mut config, "sync.data_types", "structures,assemblies").unwrap();
        assert_eq!(
            config.sync.data_types,
            vec!["structures".to_string(), "assemblies".to_string()]
        );

        // download.retry_count
        set_config_value(&mut config, "download.retry_count", "5").unwrap();
        assert_eq!(config.download.retry_count, 5);

        // mirror_selection.*
        set_config_value(&mut config, "mirror_selection.auto_select", "true").unwrap();
        assert!(config.mirror_selection.auto_select);

        set_config_value(&mut config, "mirror_selection.preferred_region", "jp").unwrap();
        assert_eq!(
            config.mirror_selection.preferred_region,
            Some("jp".to_string())
        );

        set_config_value(&mut config, "mirror_selection.latency_cache_ttl", "7200").unwrap();
        assert_eq!(config.mirror_selection.latency_cache_ttl, 7200);
    }

    #[test]
    fn test_set_config_value_invalid_layout() {
        let mut config = Config::default();
        let result = set_config_value(&mut config, "sync.layout", "invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_set_config_value_invalid_data_type() {
        let mut config = Config::default();
        let result = set_config_value(&mut config, "sync.data_types", "invalid_type");
        assert!(result.is_err());
    }

    #[test]
    fn test_set_config_value_invalid_retry_count() {
        let mut config = Config::default();
        let result = set_config_value(&mut config, "download.retry_count", "not_a_number");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_config_value_unknown_key() {
        let config = Config::default();
        let result = get_config_value(&config, "unknown.key");
        assert!(result.is_err());
    }

    #[test]
    fn test_set_config_value_unknown_key() {
        let mut config = Config::default();
        let result = set_config_value(&mut config, "unknown.key", "value");
        assert!(result.is_err());
    }
}
