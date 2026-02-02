//! Config command arguments.

use clap::{Parser, Subcommand};

/// Config command arguments.
#[derive(Parser, Clone, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

/// Config subcommands.
#[derive(Subcommand, Clone, Debug)]
pub enum ConfigCommand {
    /// Initialize a new configuration file
    Init(InitArgs),
    /// Validate configuration file
    Validate(ValidateArgs),
    /// Migrate old config format to new nested format
    Migrate(MigrateArgs),
    /// List available rsync flag presets
    Presets,
}

/// Validate command arguments.
#[derive(Parser, Clone, Debug)]
pub struct ValidateArgs {
    /// Config file path (defaults to ~/.config/pdb-sync/config.toml)
    #[arg(short, long)]
    pub config: Option<std::path::PathBuf>,

    /// Output validation results in JSON format
    #[arg(long)]
    pub json: bool,

    /// Attempt to fix auto-fixable issues
    #[arg(long)]
    pub fix: bool,
}

/// Migrate command arguments.
#[derive(Parser, Clone, Debug)]
pub struct MigrateArgs {
    /// Config file path (defaults to ~/.config/pdb-sync/config.toml)
    #[arg(short, long)]
    pub config: Option<std::path::PathBuf>,

    /// Dry run - show what would be changed without modifying the file
    #[arg(short = 'n', long)]
    pub dry_run: bool,
}

/// Init command arguments.
#[derive(Parser, Clone, Debug)]
pub struct InitArgs {
    /// Config file path (defaults to ~/.config/pdb-sync/config.toml)
    #[arg(short, long)]
    pub config: Option<std::path::PathBuf>,

    /// Overwrite existing config file
    #[arg(long)]
    pub force: bool,

    /// Generate minimal config (default: full config with comments)
    #[arg(long)]
    pub minimal: bool,

    /// PDB directory path to use in the config
    #[arg(long)]
    pub pdb_dir: Option<std::path::PathBuf>,
}

/// Run config validate command.
pub async fn run_validate(args: ValidateArgs) -> crate::error::Result<()> {
    // If config path is provided, use the new validation
    if args.config.is_some() || !args.fix {
        use crate::cli::commands::config::ConfigCommand;
        let cmd = ConfigCommand::Validate {
            config_path: args.config,
        };
        return crate::cli::commands::config::run_config(cmd).await;
    }

    // Otherwise, use the original validation with fix support
    use crate::config::ConfigLoader;
    use crate::sync::validator::validate_config;

    println!("Validating configuration...");

    let config = ConfigLoader::load()?;
    let validation = validate_config(&config);

    if args.json {
        println!("{}", serde_json::to_string_pretty(&validation)?);
    } else {
        validation.print();
    }

    if validation.has_errors() {
        std::process::exit(1);
    }

    Ok(())
}

/// Run config migrate command.
pub async fn run_migrate(args: MigrateArgs) -> crate::error::Result<()> {
    use crate::cli::commands::config::ConfigCommand;
    let cmd = ConfigCommand::Migrate {
        config_path: args.config,
        dry_run: args.dry_run,
    };
    crate::cli::commands::config::run_config(cmd).await
}

/// Run config presets command.
pub async fn run_presets() -> crate::error::Result<()> {
    use crate::cli::commands::config::ConfigCommand;
    let cmd = ConfigCommand::Presets;
    crate::cli::commands::config::run_config(cmd).await
}

/// Run config init command.
pub async fn run_init(args: InitArgs) -> crate::error::Result<()> {
    use crate::cli::commands::config::ConfigCommand;
    let cmd = ConfigCommand::Init {
        config_path: args.config,
        force: args.force,
        minimal: args.minimal,
        pdb_dir: args.pdb_dir,
    };
    crate::cli::commands::config::run_config(cmd).await
}
