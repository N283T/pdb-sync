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
    /// Validate configuration file
    Validate(ValidateArgs),
}

/// Validate command arguments.
#[derive(Parser, Clone, Debug)]
pub struct ValidateArgs {
    /// Output validation results in JSON format
    #[arg(long)]
    pub json: bool,

    /// Attempt to fix auto-fixable issues
    #[arg(long)]
    pub fix: bool,
}

/// Run config validate command.
pub async fn run_validate(args: ValidateArgs) -> crate::error::Result<()> {
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
