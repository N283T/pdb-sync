use crate::cli::args::{ConfigAction, ConfigArgs};
use crate::config::ConfigLoader;
use crate::data_types::Layout;
use crate::error::{PdbCliError, Result};
use crate::mirrors::print_mirror_latencies;

pub async fn run_config(args: ConfigArgs, _ctx: crate::context::AppContext) -> Result<()> {
    match args.action {
        ConfigAction::Init => {
            let path = ConfigLoader::init()?;
            println!("Configuration initialized at: {}", path.display());
        }
        ConfigAction::Show => {
            let config = ConfigLoader::load()?;
            let toml_str = toml::to_string_pretty(&config)?;
            println!("{}", toml_str);

            if let Some(path) = ConfigLoader::config_path() {
                println!("\nConfig file: {}", path.display());
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
            println!("Set {} = {}", key, value);
        }
        ConfigAction::TestMirrors => {
            print_mirror_latencies().await;
        }
    }

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

        _ => Err(PdbCliError::Config(format!("Unknown config key: {}", key))),
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
            config.sync.bwlimit = value
                .parse()
                .map_err(|_| PdbCliError::Config("Invalid bwlimit value".into()))?;
        }
        "sync.delete" => {
            config.sync.delete = value
                .parse()
                .map_err(|_| PdbCliError::Config("Invalid boolean value".into()))?;
        }
        "sync.layout" => {
            config.sync.layout = match value.to_lowercase().as_str() {
                "divided" => Layout::Divided,
                "all" => Layout::All,
                _ => {
                    return Err(PdbCliError::Config(format!(
                        "Invalid layout: {}. Use 'divided' or 'all'",
                        value
                    )))
                }
            };
        }
        "sync.data_types" => {
            config.sync.data_types = value
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }

        // download.*
        "download.auto_decompress" => {
            config.download.auto_decompress = value
                .parse()
                .map_err(|_| PdbCliError::Config("Invalid boolean value".into()))?;
        }
        "download.parallel" => {
            config.download.parallel = value
                .parse()
                .map_err(|_| PdbCliError::Config("Invalid parallel value".into()))?;
        }
        "download.default_format" => {
            config.download.default_format = value.to_string();
        }
        "download.retry_count" => {
            config.download.retry_count = value
                .parse()
                .map_err(|_| PdbCliError::Config("Invalid retry_count value".into()))?;
        }

        // mirror_selection.*
        "mirror_selection.auto_select" => {
            config.mirror_selection.auto_select = value
                .parse()
                .map_err(|_| PdbCliError::Config("Invalid boolean value".into()))?;
        }
        "mirror_selection.preferred_region" => {
            config.mirror_selection.preferred_region = if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            };
        }
        "mirror_selection.latency_cache_ttl" => {
            config.mirror_selection.latency_cache_ttl = value
                .parse()
                .map_err(|_| PdbCliError::Config("Invalid latency_cache_ttl value".into()))?;
        }

        _ => return Err(PdbCliError::Config(format!("Unknown config key: {}", key))),
    }
    Ok(())
}
