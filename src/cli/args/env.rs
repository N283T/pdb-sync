//! Environment command arguments.

use crate::context::AppContext;
use crate::error::Result;
use clap::{Parser, Subcommand};

/// Environment command arguments.
#[derive(Parser, Clone, Debug)]
pub struct EnvArgs {
    #[command(subcommand)]
    pub command: EnvCommand,
}

/// Environment subcommands.
#[derive(Subcommand, Clone, Debug)]
pub enum EnvCommand {
    /// Run environment diagnostics
    Doctor,
}

/// Run env based on arguments.
pub fn run_env(args: EnvArgs, ctx: AppContext) -> Result<()> {
    match args.command {
        EnvCommand::Doctor => crate::cli::commands::env::run_doctor(ctx),
    }
}
