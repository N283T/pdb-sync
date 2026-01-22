//! Initialize the base directory structure for pdb-sync.
//!
//! Creates the recommended directory layout based on mirror rsync structures.
//!
//! ## Directory Structure
//!
//! ### Base (depth=0)
//! - basedir/
//!   ├── pub/        # wwPDB common data (RCSB/PDBj/PDBe)
//!   ├── pdbj/       # PDBj-specific data only
//!   ├── pdbe/       # PDBe-specific data only
//!   └── local/      # User-managed space
//!
//! ### Data Types (depth=1)
//! - pub/pdb/data/:
//!   ├── structures/
//!   ├── assemblies/
//!   ├── biounit/
//!   └── obsolete/
//! - pub/emdb/       # EMDB data (flat)
//! - pub/pdb_ihm/    # PDB-IHM data (flat)
//! - pdbj/:
//!   ├── pdbjplus/
//!   ├── bsma/
//!   └── mmdb/
//! - pdbe/:
//!   ├── assemblies/
//!   ├── foldseek/
//!   ├── fragment_screening/
//!   ├── graphdb/
//!   ├── nmr/
//!   ├── pdb-assemblies-analysis/
//!   ├── pdb_uncompressed/
//!   ├── pdbechem_v2/
//!   ├── sifts/
//!   ├── status/
//!   └── updated_mmcif/
//!
//! ### Layouts (depth=2)
//! - Only pub/pdb/data/ types get divided/all subdirectories:
//!   - structures/divided/, structures/all/
//!   - assemblies/divided/, assemblies/all/
//!   - etc.

use crate::cli::args::InitArgs;
use crate::context::AppContext;
use crate::error::{PdbSyncError, Result};
use crate::tree::{build_tree, render_tree, TreeOptions};
use crate::utils::{header, hint, info, success};
use colored::Colorize;
use std::collections::HashMap;

/// Parse depth string to usize (supports numeric and named values)
fn parse_depth(depth_str: &str) -> Result<usize> {
    match depth_str.to_lowercase().as_str() {
        "base" => Ok(0),
        "types" => Ok(1),
        "layouts" => Ok(2),
        "format" => Ok(3),
        _ => {
            // Try to parse as number
            let depth = depth_str.parse::<usize>().map_err(|_| {
                PdbSyncError::InvalidInput(format!(
                    "Invalid depth '{}'. Use 0-3 or: base, types, layouts, format",
                    depth_str
                ))
            })?;
            // Validate range
            if depth > 3 {
                return Err(PdbSyncError::InvalidInput(format!(
                    "Invalid depth '{}'. Use 0-3 or: base, types, layouts, format",
                    depth_str
                )));
            }
            Ok(depth)
        }
    }
}

/// Subdirectories to create in the base directory (depth=0)
pub const SUBDIRS: &[&str] = &["pub", "pdbj", "pdbe", "local"];

/// Common wwPDB data types (shared across all mirrors)
///
/// Note: structure_factors, nmr_chemical_shifts, nmr_restraints are
/// subdirectories under structures/divided/ (not top-level).
pub fn get_common_data_types() -> Vec<String> {
    vec![
        "structures".to_string(),
        "assemblies".to_string(),
        "biounit".to_string(),
        "obsolete".to_string(),
    ]
}

/// PDBj-specific data types
///
/// Note: These are PDBj-specific directories (emdb/pdb_ihm are now in pub/)
pub fn get_pdbj_data_types() -> Vec<String> {
    vec![
        "pdbjplus".to_string(),
        "bsma".to_string(),
        "mmdb".to_string(),
    ]
}

/// PDBe-specific data types
///
/// Note: Uses actual directory names from ftp.ebi.ac.uk/pub/databases/msd/
pub fn get_pdbe_data_types() -> Vec<String> {
    vec![
        "assemblies".to_string(),
        "foldseek".to_string(),
        "fragment_screening".to_string(),
        "graphdb".to_string(),
        "nmr".to_string(),
        "pdb-assemblies-analysis".to_string(),
        "pdb_uncompressed".to_string(),
        "pdbechem_v2".to_string(),
        "sifts".to_string(),
        "status".to_string(),
        "updated_mmcif".to_string(),
    ]
}

/// Layout subdirectories for depth=2
pub fn get_layout_subdirs() -> Vec<String> {
    vec!["divided".to_string(), "all".to_string()]
}

/// Format subdirectories for depth=3
///
/// Note: "pdb" is lowercase to match actual rsync/HTTP structure.
pub fn get_format_subdirs() -> Vec<String> {
    vec!["mmCIF".to_string(), "pdb".to_string()]
}

/// Build the complete directory tree based on depth
fn build_directory_tree(subdirs: &[String], depth: usize) -> HashMap<String, Vec<String>> {
    let mut tree = HashMap::new();

    for subdir in subdirs {
        let mut paths = Vec::new();

        match subdir.as_str() {
            "pub" => {
                // Common wwPDB data under pub/
                if depth >= 1 {
                    // pub/pdb/data/ - main PDB data with layouts
                    for data_type in get_common_data_types() {
                        let dt_path = format!("pub/pdb/data/{}", data_type);
                        paths.push(dt_path.clone());

                        if depth >= 2 {
                            for layout in get_layout_subdirs() {
                                let layout_path = format!("{}/{}", dt_path, layout);
                                paths.push(layout_path.clone());

                                if depth >= 3 {
                                    for format in get_format_subdirs() {
                                        paths.push(format!("{}/{}", layout_path, format));
                                    }
                                }
                            }
                        }
                    }

                    // pub/emdb/ - flat directory (no layouts)
                    paths.push("pub/emdb".to_string());

                    // pub/pdb_ihm/ - flat directory (no layouts)
                    paths.push("pub/pdb_ihm".to_string());
                }
            }
            "pdbj" => {
                // PDBj-specific data (flat directories, no layouts)
                if depth >= 1 {
                    for data_type in get_pdbj_data_types() {
                        paths.push(format!("pdbj/{}", data_type));
                    }
                }
            }
            "pdbe" => {
                // PDBe-specific data (flat directories, no layouts)
                if depth >= 1 {
                    for data_type in get_pdbe_data_types() {
                        paths.push(format!("pdbe/{}", data_type));
                    }
                }
            }
            "local" => {
                // User-managed, always empty
            }
            _ => {}
        }

        tree.insert(subdir.clone(), paths);
    }

    tree
}

/// Validate a directory name to prevent path traversal and invalid names.
fn validate_dir_name(name: &str) -> Result<()> {
    // Reject empty names
    if name.is_empty() {
        return Err(PdbSyncError::InvalidInput(
            "Directory name cannot be empty".to_string(),
        ));
    }

    // Reject path traversal attempts
    if name.contains("..") {
        return Err(PdbSyncError::InvalidInput(
            "Directory name cannot contain '..'".to_string(),
        ));
    }

    // Reject path separators (should be single directory name only)
    if name.contains('/') || name.contains('\\') {
        return Err(PdbSyncError::InvalidInput(
            "Directory name cannot contain path separators".to_string(),
        ));
    }

    // Reject names starting with dot (hidden directories)
    if name.starts_with('.') {
        return Err(PdbSyncError::InvalidInput(
            "Directory name cannot start with '.'".to_string(),
        ));
    }

    // Reject names that are just whitespace
    if name.trim().is_empty() {
        return Err(PdbSyncError::InvalidInput(
            "Directory name cannot be only whitespace".to_string(),
        ));
    }

    // Reject control characters
    if name.chars().any(|c| c.is_control()) {
        return Err(PdbSyncError::InvalidInput(
            "Directory name cannot contain control characters".to_string(),
        ));
    }

    Ok(())
}

/// Source of the base directory configuration
#[derive(Debug, Clone, Copy)]
enum DirSource {
    CliArg,
    EnvVar,
    Config,
    Default,
}

impl DirSource {
    fn as_str(&self) -> &'static str {
        match self {
            DirSource::CliArg => "CLI argument (--dir)",
            DirSource::EnvVar => "Environment variable (PDB_DIR)",
            DirSource::Config => "Config file",
            DirSource::Default => "Default location",
        }
    }
}

/// Run the init command.
pub async fn run_init(args: InitArgs, ctx: AppContext) -> Result<()> {
    // Parse depth from string (supports numeric and named values)
    let depth = parse_depth(&args.depth)?;

    // Determine base directory and its source
    let (base_dir, dir_source) = if let Some(dir) = args.dir {
        (dir, DirSource::CliArg)
    } else {
        // Check if PDB_DIR is set to distinguish ENV from Config/Default
        if std::env::var("PDB_DIR").is_ok() {
            (ctx.pdb_dir.clone(), DirSource::EnvVar)
        } else if ctx.config.paths.pdb_dir.is_some() {
            (ctx.pdb_dir.clone(), DirSource::Config)
        } else {
            (ctx.pdb_dir.clone(), DirSource::Default)
        }
    };

    // Determine which subdirectories to create
    let (defined_subdirs, custom_subdirs) = if let Some(only) = &args.only {
        // Split into defined (with preset structure) and custom (empty directories)
        let requested: std::collections::HashSet<String> =
            only.iter().map(|s| s.to_lowercase()).collect();

        let mut defined = Vec::new();
        let mut custom = Vec::new();

        for name in &requested {
            if SUBDIRS.contains(&name.as_str()) {
                defined.push(name.clone());
            } else {
                // Validate custom directory name
                validate_dir_name(name)?;
                custom.push(name.clone());
            }
        }

        // Default to all defined subdirs if none specified
        if defined.is_empty() && custom.is_empty() {
            (SUBDIRS.iter().map(|s| s.to_string()).collect(), Vec::new())
        } else {
            (defined, custom)
        }
    } else {
        // Create all defined subdirectories by default
        (SUBDIRS.iter().map(|s| s.to_string()).collect(), Vec::new())
    };

    // Combine all subdirs
    let all_subdirs: Vec<String> = defined_subdirs
        .iter()
        .chain(custom_subdirs.iter())
        .cloned()
        .collect();

    // Build directory tree (custom dirs always have empty structure)
    let tree = build_directory_tree(&defined_subdirs, depth);

    if args.dry_run {
        header("Preview (dry-run)");
        info(&format!(
            "Base: {} [from {}]",
            base_dir.display(),
            dir_source.as_str()
        ));
        for subdir in &all_subdirs {
            println!("  {}/", subdir.cyan());
            if let Some(paths) = tree.get(subdir) {
                for path in paths {
                    println!("    {}", path.dimmed());
                }
            }
        }
        return Ok(());
    }

    // Create base directory if it doesn't exist
    info(&format!(
        "Base directory: {} [from {}]",
        base_dir.display(),
        dir_source.as_str()
    ));
    if !base_dir.exists() {
        println!("Creating base directory...");
        std::fs::create_dir_all(&base_dir)?;
        success("Base directory created");
    } else {
        println!("Base directory already exists.");
    }

    // Create directory structure
    for subdir in &all_subdirs {
        let subdir_path = base_dir.join(subdir);
        if !subdir_path.exists() {
            println!("  Creating: {}/", subdir.cyan());
            std::fs::create_dir(&subdir_path)?;
        } else {
            println!("  Exists: {}/", subdir.dimmed());
        }

        // Only create subdirectories for defined dirs (custom dirs stay empty)
        if let Some(paths) = tree.get(subdir) {
            for path in paths {
                let full_path = base_dir.join(path);
                if !full_path.exists() {
                    println!("    Creating: {}", path.dimmed());
                    std::fs::create_dir_all(&full_path)?;
                } else {
                    println!("    Exists: {}", path.dimmed());
                }
            }
        }
    }

    success(&format!(
        "Directory structure initialized at: {}",
        base_dir.display()
    ));

    // Show environment setup hint if not using CLI arg or ENV
    match dir_source {
        DirSource::CliArg => {}
        DirSource::EnvVar => {}
        DirSource::Config | DirSource::Default => {
            println!();
            hint("Set the PDB_DIR environment variable to avoid specifying --dir:");
            println!("  export PDB_DIR=\"{}\"", base_dir.display());
            println!();
            hint("Or run:");
            println!("  pdb-sync env --export");
        }
    }

    if depth == 0 {
        println!();
        hint("Use --depth 1 to create data type subdirectories:");
        println!("  pdb-sync init --depth 1");
        println!();
        hint("Use --depth 2 to create layout subdirectories (divided/all):");
        println!("  pdb-sync init --depth 2");
        println!();
        hint("Use --depth 3 (or --depth format) to create format subdirectories:");
        println!("  pdb-sync init --depth 3");
        println!("  pdb-sync init --depth format");
        println!();
        hint("You can also specify custom directories:");
        println!("  pdb-sync init --only pub --only myproject");
    } else {
        println!();
        info("You can now use sync commands with --dest to target specific directories:");
        if all_subdirs.contains(&"pub".to_string()) {
            println!(
                "  pdb-sync sync structures --dest {}/pub/pdb/data/structures/divided",
                base_dir.display()
            );
        }
        if all_subdirs.contains(&"pub".to_string()) && depth >= 1 {
            println!(
                "  pdb-sync sync pdbj --type emdb --dest {}/pub/emdb",
                base_dir.display()
            );
        }
        if all_subdirs.contains(&"pdbe".to_string()) && depth >= 1 {
            println!(
                "  pdb-sync sync pdbe --type sifts --dest {}/pdbe/sifts",
                base_dir.display()
            );
        }
    }

    // Show directory tree after creation (not in dry_run mode)
    if !args.dry_run {
        header("Directory structure");
        let tree_options = TreeOptions {
            max_depth: Some(depth + 1), // +1 to show base dir too
            format_filter: None,
            non_empty: false,
        };
        match build_tree(&base_dir, &tree_options).await {
            Ok(tree) => {
                let output = render_tree(
                    &tree,
                    &crate::tree::render::RenderOptions {
                        show_size: false,
                        show_count: false,
                        no_summary: true,
                    },
                );
                print!("{}", output);
            }
            Err(e) => {
                // Tree display is optional, don't fail the command
                eprintln!("Note: Could not display tree: {}", e);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subdirs_const() {
        assert_eq!(SUBDIRS.len(), 4);
        assert!(SUBDIRS.contains(&"pub"));
        assert!(SUBDIRS.contains(&"pdbj"));
        assert!(SUBDIRS.contains(&"pdbe"));
        assert!(SUBDIRS.contains(&"local"));
    }

    #[test]
    fn test_get_common_data_types() {
        let types = get_common_data_types();
        assert_eq!(types.len(), 4);
        assert!(types.contains(&"structures".to_string()));
        assert!(types.contains(&"assemblies".to_string()));
        assert!(types.contains(&"biounit".to_string()));
        assert!(types.contains(&"obsolete".to_string()));
        // structure_factors, nmr_chemical_shifts, nmr_restraints
        // are subdirs under structures/divided/, not top-level
    }

    #[test]
    fn test_get_pdbj_data_types() {
        let types = get_pdbj_data_types();
        assert_eq!(types.len(), 3);
        assert!(types.contains(&"pdbjplus".to_string()));
        assert!(types.contains(&"bsma".to_string()));
        assert!(types.contains(&"mmdb".to_string()));
    }

    #[test]
    fn test_get_pdbe_data_types() {
        let types = get_pdbe_data_types();
        assert_eq!(types.len(), 11);
        assert!(types.contains(&"assemblies".to_string()));
        assert!(types.contains(&"foldseek".to_string()));
        assert!(types.contains(&"fragment_screening".to_string()));
        assert!(types.contains(&"graphdb".to_string()));
        assert!(types.contains(&"nmr".to_string()));
        assert!(types.contains(&"pdb-assemblies-analysis".to_string()));
        assert!(types.contains(&"pdb_uncompressed".to_string()));
        assert!(types.contains(&"pdbechem_v2".to_string()));
        assert!(types.contains(&"sifts".to_string()));
        assert!(types.contains(&"status".to_string()));
        assert!(types.contains(&"updated_mmcif".to_string()));
    }

    #[test]
    fn test_get_layout_subdirs() {
        let layouts = get_layout_subdirs();
        assert_eq!(layouts.len(), 2);
        assert!(layouts.contains(&"divided".to_string()));
        assert!(layouts.contains(&"all".to_string()));
    }

    #[test]
    fn test_build_directory_tree_depth_0() {
        let subdirs = vec!["pub".to_string(), "pdbj".to_string()];
        let tree = build_directory_tree(&subdirs, 0);

        assert_eq!(tree.get("pub"), Some(&vec![]));
        assert_eq!(tree.get("pdbj"), Some(&vec![]));
    }

    #[test]
    fn test_build_directory_tree_depth_1() {
        let subdirs = vec!["pub".to_string()];
        let tree = build_directory_tree(&subdirs, 1);

        let paths = tree.get("pub").unwrap();
        assert!(!paths.is_empty());
        assert!(paths.iter().any(|p| p == "pub/pdb/data/structures"));
        assert!(paths.iter().any(|p| p == "pub/pdb/data/assemblies"));
        assert!(paths.iter().any(|p| p == "pub/emdb"));
        assert!(paths.iter().any(|p| p == "pub/pdb_ihm"));
        assert!(!paths.iter().any(|p| p.contains("divided"))); // depth 1 shouldn't have layouts
    }

    #[test]
    fn test_build_directory_tree_depth_2() {
        let subdirs = vec!["pub".to_string()];
        let tree = build_directory_tree(&subdirs, 2);

        let paths = tree.get("pub").unwrap();
        assert!(paths.iter().any(|p| p == "pub/pdb/data/structures"));
        assert!(paths.iter().any(|p| p == "pub/pdb/data/structures/divided"));
        assert!(paths.iter().any(|p| p == "pub/pdb/data/structures/all"));
        assert!(paths.iter().any(|p| p == "pub/pdb/data/assemblies/divided"));
    }

    #[test]
    fn test_build_directory_tree_local() {
        let subdirs = vec!["local".to_string()];
        let tree = build_directory_tree(&subdirs, 2);

        // local should always be empty
        assert_eq!(tree.get("local"), Some(&vec![]));
    }

    #[test]
    fn test_build_directory_tree_pdbj_depth_2() {
        let subdirs = vec!["pdbj".to_string()];
        let tree = build_directory_tree(&subdirs, 2);

        let paths = tree.get("pdbj").unwrap();
        assert!(paths.iter().any(|p| p == "pdbj/pdbjplus"));
        assert!(paths.iter().any(|p| p == "pdbj/bsma"));
        assert!(paths.iter().any(|p| p == "pdbj/mmdb"));
        // No layouts for PDBj-specific directories (flat structure)
        assert!(!paths.iter().any(|p| p.contains("divided")));
    }

    #[test]
    fn test_build_directory_tree_pdbe_depth_2() {
        let subdirs = vec!["pdbe".to_string()];
        let tree = build_directory_tree(&subdirs, 2);

        let paths = tree.get("pdbe").unwrap();
        assert!(paths.iter().any(|p| p == "pdbe/sifts"));
        assert!(paths.iter().any(|p| p == "pdbe/pdbechem_v2"));
        assert!(paths.iter().any(|p| p == "pdbe/foldseek"));
        assert!(paths.iter().any(|p| p == "pdbe/assemblies"));
        // No layouts for PDBe-specific directories (flat structure)
        assert!(!paths.iter().any(|p| p.contains("divided")));
    }

    #[test]
    fn test_no_duplicate_common_data() {
        // Ensure we don't have wwpdb, rcsb subdirs anymore
        assert!(!SUBDIRS.contains(&"wwpdb"));
        assert!(!SUBDIRS.contains(&"rcsb"));
    }

    #[test]
    fn test_validate_dir_name_valid() {
        // Valid names should pass
        assert!(validate_dir_name("pub").is_ok());
        assert!(validate_dir_name("myproject").is_ok());
        assert!(validate_dir_name("test-dir").is_ok());
        assert!(validate_dir_name("test_dir").is_ok());
        assert!(validate_dir_name("Test123").is_ok());
    }

    #[test]
    fn test_validate_dir_name_empty() {
        // Empty names should fail
        assert!(validate_dir_name("").is_err());
        assert!(validate_dir_name("   ").is_err());
    }

    #[test]
    fn test_validate_dir_name_path_traversal() {
        // Path traversal should fail
        assert!(validate_dir_name("..").is_err());
        assert!(validate_dir_name("test/../data").is_err());
        assert!(validate_dir_name("../test").is_err());
    }

    #[test]
    fn test_validate_dir_name_separators() {
        // Path separators should fail
        assert!(validate_dir_name("test/data").is_err());
        assert!(validate_dir_name("test\\data").is_err());
    }

    #[test]
    fn test_validate_dir_name_hidden() {
        // Names starting with dot should fail
        assert!(validate_dir_name(".hidden").is_err());
        assert!(validate_dir_name(".test").is_err());
    }

    #[test]
    fn test_validate_dir_name_control_chars() {
        // Control characters should fail
        assert!(validate_dir_name("test\n").is_err());
        assert!(validate_dir_name("test\t").is_err());
        assert!(validate_dir_name("test\x00").is_err());
    }

    #[test]
    fn test_parse_depth_numeric() {
        assert_eq!(parse_depth("0").unwrap(), 0);
        assert_eq!(parse_depth("1").unwrap(), 1);
        assert_eq!(parse_depth("2").unwrap(), 2);
        assert_eq!(parse_depth("3").unwrap(), 3);
    }

    #[test]
    fn test_parse_depth_named() {
        assert_eq!(parse_depth("base").unwrap(), 0);
        assert_eq!(parse_depth("BASE").unwrap(), 0);
        assert_eq!(parse_depth("types").unwrap(), 1);
        assert_eq!(parse_depth("TYPES").unwrap(), 1);
        assert_eq!(parse_depth("layouts").unwrap(), 2);
        assert_eq!(parse_depth("LAYOUTS").unwrap(), 2);
        assert_eq!(parse_depth("format").unwrap(), 3);
        assert_eq!(parse_depth("FORMAT").unwrap(), 3);
    }

    #[test]
    fn test_parse_depth_invalid() {
        assert!(parse_depth("invalid").is_err());
        assert!(parse_depth("4").is_err());
        assert!(parse_depth("-1").is_err());
    }

    #[test]
    fn test_build_directory_tree_depth_3() {
        let subdirs = vec!["pub".to_string()];
        let tree = build_directory_tree(&subdirs, 3);

        let paths = tree.get("pub").unwrap();
        // Should have format subdirectories (pdb is lowercase to match rsync)
        assert!(paths.iter().any(|p| p == "pub/pdb/data/structures/divided/mmCIF"));
        assert!(paths.iter().any(|p| p == "pub/pdb/data/structures/divided/pdb"));
        assert!(paths.iter().any(|p| p == "pub/pdb/data/structures/all/mmCIF"));
        assert!(paths.iter().any(|p| p == "pub/pdb/data/structures/all/pdb"));
        assert!(paths.iter().any(|p| p == "pub/pdb/data/assemblies/divided/mmCIF"));
    }

    #[test]
    fn test_get_format_subdirs() {
        let formats = get_format_subdirs();
        assert_eq!(formats.len(), 2);
        assert!(formats.contains(&"mmCIF".to_string()));
        assert!(formats.contains(&"pdb".to_string()));  // lowercase to match rsync
    }
}
