use crate::cli::args::{ConfigAction, ConfigArgs};
use crate::config::ConfigLoader;
use crate::error::{PdbCliError, Result};

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
    }

    Ok(())
}

fn get_config_value(config: &crate::config::Config, key: &str) -> Result<String> {
    match key {
        "paths.pdb_dir" => Ok(config
            .paths
            .pdb_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default()),
        "sync.mirror" => Ok(config.sync.mirror.to_string()),
        "sync.bwlimit" => Ok(config.sync.bwlimit.to_string()),
        "sync.delete" => Ok(config.sync.delete.to_string()),
        "download.auto_decompress" => Ok(config.download.auto_decompress.to_string()),
        "download.parallel" => Ok(config.download.parallel.to_string()),
        _ => Err(PdbCliError::Config(format!("Unknown config key: {}", key))),
    }
}

fn set_config_value(
    config: &mut crate::config::Config,
    key: &str,
    value: &str,
) -> Result<()> {
    match key {
        "paths.pdb_dir" => {
            config.paths.pdb_dir = if value.is_empty() {
                None
            } else {
                Some(std::path::PathBuf::from(value))
            };
        }
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
        _ => return Err(PdbCliError::Config(format!("Unknown config key: {}", key))),
    }
    Ok(())
}
