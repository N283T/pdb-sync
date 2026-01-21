use crate::config::{Config, ConfigLoader};
use crate::error::Result;
use crate::mirrors::MirrorId;
use crate::utils::{header, hint, info, success, warning};
use colored::Colorize;
use std::io::{self, Write};
use std::path::PathBuf;

/// Check if first-run setup is needed
pub fn needs_setup() -> bool {
    ConfigLoader::config_path()
        .map(|p| !p.exists())
        .unwrap_or(false)
}

/// Run the interactive setup wizard
pub fn run_setup() -> Result<()> {
    header("Welcome to pdb-sync!");
    println!("Let's set up your configuration.\n");

    let mut config = Config::default();

    // Select mirror
    config.sync.mirror = prompt_mirror()?;

    // Set PDB directory
    config.paths.pdb_dir = prompt_pdb_dir()?;

    // Set default format
    config.download.default_format = prompt_default_format()?;

    // Auto decompress
    config.download.auto_decompress = prompt_yes_no(
        "Automatically decompress downloaded files?",
        config.download.auto_decompress,
    )?;

    // Save config
    ConfigLoader::save(&config)?;

    let config_path = ConfigLoader::config_path().unwrap();
    println!();
    success(&format!(
        "Configuration saved to: {}",
        config_path.display()
    ));
    hint("You can modify these settings anytime with 'pdb-sync config set <key> <value>'");
    println!();

    Ok(())
}

fn prompt_mirror() -> Result<MirrorId> {
    info("Select your preferred mirror:");
    println!("  1) rcsb  - RCSB PDB (US)");
    println!("  2) pdbj  - PDBj (Japan)");
    println!("  3) pdbe  - PDBe (Europe)");
    println!("  4) wwpdb - wwPDB (Global)");
    print!("\nChoice [1-4, default: 1]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    let mirror = match input {
        "" | "1" | "rcsb" => MirrorId::Rcsb,
        "2" | "pdbj" => MirrorId::Pdbj,
        "3" | "pdbe" => MirrorId::Pdbe,
        "4" | "wwpdb" => MirrorId::Wwpdb,
        _ => {
            warning("Invalid choice, using default (rcsb)");
            MirrorId::Rcsb
        }
    };

    println!("Selected: {}\n", mirror.to_string().green());
    Ok(mirror)
}

fn prompt_pdb_dir() -> Result<Option<PathBuf>> {
    let default_dir = directories::UserDirs::new()
        .map(|d| d.home_dir().join("pdb"))
        .unwrap_or_else(|| PathBuf::from("./pdb"));

    print!(
        "PDB files directory [default: {}]: ",
        default_dir.display().to_string().cyan()
    );
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    let path = if input.is_empty() {
        None // Use default from context
    } else {
        Some(PathBuf::from(input))
    };

    if let Some(ref p) = path {
        println!("PDB directory: {}\n", p.display().to_string().cyan());
    } else {
        println!(
            "PDB directory: {} (default)\n",
            default_dir.display().to_string().cyan()
        );
    }

    Ok(path)
}

fn prompt_default_format() -> Result<String> {
    info("Select default download format:");
    println!("  1) mmcif   - mmCIF format (.cif)");
    println!("  2) pdb     - Legacy PDB format (.pdb)");
    println!("  3) bcif    - BinaryCIF format (.bcif)");
    println!("  4) cif-gz  - Compressed mmCIF (.cif.gz)");
    print!("\nChoice [1-4, default: 1]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    let format = match input {
        "" | "1" | "mmcif" => "mmcif",
        "2" | "pdb" => "pdb",
        "3" | "bcif" => "bcif",
        "4" | "cif-gz" => "cif-gz",
        _ => {
            warning("Invalid choice, using default (mmcif)");
            "mmcif"
        }
    };

    println!("Selected: {}\n", format.yellow());
    Ok(format.to_string())
}

fn prompt_yes_no(question: &str, default: bool) -> Result<bool> {
    let default_str = if default { "Y/n".green() } else { "y/N".red() };
    print!("{} [{}]: ", question, default_str);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    let result = match input.as_str() {
        "" => default,
        "y" | "yes" => true,
        "n" | "no" => false,
        _ => default,
    };

    println!("{}\n", if result { "Yes".green() } else { "No".red() });
    Ok(result)
}
