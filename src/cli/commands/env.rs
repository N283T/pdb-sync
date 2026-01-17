use crate::cli::args::{EnvAction, EnvArgs};
use crate::config::ConfigLoader;
use crate::error::Result;

const ENV_VARS: &[(&str, &str)] = &[
    ("PDB_DIR", "Base directory for PDB files"),
    ("PDB_CLI_CONFIG", "Path to configuration file"),
    ("PDB_CLI_MIRROR", "Default mirror (rcsb, pdbj, pdbe, wwpdb)"),
];

pub async fn run_env(args: EnvArgs, ctx: crate::context::AppContext) -> Result<()> {
    match args.action {
        EnvAction::Show => {
            println!("Environment Variables:\n");

            for (name, description) in ENV_VARS {
                let value = std::env::var(name).unwrap_or_else(|_| "(not set)".to_string());
                println!("  {} = {}", name, value);
                println!("    {}\n", description);
            }

            println!("Effective values (after config resolution):");
            println!("  PDB_DIR: {}", ctx.pdb_dir.display());
            println!("  Mirror: {}", ctx.mirror);
            if let Some(path) = ConfigLoader::config_path() {
                println!("  Config: {}", path.display());
            }
        }
        EnvAction::Export => {
            println!("# PDB-CLI environment variables");
            println!("# Add to your shell profile (~/.bashrc, ~/.zshrc, etc.)\n");

            println!("export PDB_DIR=\"{}\"", ctx.pdb_dir.display());
            println!("export PDB_CLI_MIRROR=\"{}\"", ctx.mirror);
            if let Some(path) = ConfigLoader::config_path() {
                println!("export PDB_CLI_CONFIG=\"{}\"", path.display());
            }
        }
        EnvAction::Set { name, value } => {
            // Validate the variable name
            if !ENV_VARS.iter().any(|(n, _)| *n == name) {
                eprintln!(
                    "Warning: '{}' is not a recognized PDB-CLI environment variable",
                    name
                );
            }

            println!("# Run this command to set the environment variable:");
            println!("export {}=\"{}\"", name, value);
            println!("\n# Or add it to your shell profile for persistence");
        }
    }

    Ok(())
}
