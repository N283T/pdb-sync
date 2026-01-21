use crate::cli::args::{EnvAction, EnvArgs};
use crate::config::ConfigLoader;
use crate::error::Result;
use crate::utils::{header, warning};
use colored::Colorize;

const ENV_VARS: &[(&str, &str)] = &[
    ("PDB_DIR", "Base directory for PDB files"),
    ("PDB_SYNC_CONFIG", "Path to configuration file"),
    (
        "PDB_SYNC_MIRROR",
        "Default mirror (rcsb, pdbj, pdbe, wwpdb)",
    ),
];

pub async fn run_env(args: EnvArgs, ctx: crate::context::AppContext) -> Result<()> {
    match args.action {
        EnvAction::Show => {
            header("Environment Variables");
            println!();

            for (name, description) in ENV_VARS {
                let value = std::env::var(name).unwrap_or_else(|_| "(not set)".to_string().dimmed().to_string());
                println!("  {} = {}", name.cyan(), value);
                println!("    {}\n", description.dimmed());
            }

            println!("Effective values (after config resolution):");
            println!("  PDB_DIR: {}", ctx.pdb_dir.display().to_string().cyan());
            println!("  Mirror: {}", ctx.mirror.to_string().yellow());
            if let Some(path) = ConfigLoader::config_path() {
                println!("  Config: {}", path.display().to_string().cyan());
            }
        }
        EnvAction::Export => {
            println!("# PDB-SYNC environment variables");
            println!("# Add to your shell profile (~/.bashrc, ~/.zshrc, etc.)\n");

            println!("export PDB_DIR=\"{}\"", ctx.pdb_dir.display());
            println!("export PDB_SYNC_MIRROR=\"{}\"", ctx.mirror);
            if let Some(path) = ConfigLoader::config_path() {
                println!("export PDB_SYNC_CONFIG=\"{}\"", path.display());
            }
        }
        EnvAction::Set { name, value } => {
            // Validate the variable name
            if !ENV_VARS.iter().any(|(n, _)| *n == name) {
                warning(&format!(
                    "'{}' is not a recognized PDB-SYNC environment variable",
                    name
                ));
            }

            println!("# Run this command to set the environment variable:");
            println!("export {}=\"{}\"", name.cyan(), value.yellow());
            println!("\n# Or add it to your shell profile for persistence");
        }
    }

    Ok(())
}
