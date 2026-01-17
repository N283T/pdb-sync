//! Find command - searches local PDB files with path output.

use crate::cli::args::FindArgs;
use crate::context::AppContext;
use crate::error::{PdbCliError, Result};
use crate::files::{build_full_path, FileFormat, PdbId};
use glob::Pattern;
use std::collections::HashSet;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Result of searching for a single PDB entry
struct FindResult {
    pdb_id: String,
    paths: Vec<PathBuf>,
    found: bool,
}

/// Main entry point for the find command
pub async fn run_find(args: FindArgs, ctx: AppContext) -> Result<()> {
    let pdb_dir = &ctx.pdb_dir;

    if !pdb_dir.exists() {
        return Err(PdbCliError::Path(format!(
            "PDB directory does not exist: {}",
            pdb_dir.display()
        )));
    }

    // Collect patterns from args and/or stdin
    let patterns = collect_patterns(&args)?;

    if patterns.is_empty() {
        return Err(PdbCliError::InvalidInput(
            "No PDB IDs or patterns provided. Use positional arguments or --stdin.".to_string(),
        ));
    }

    let mut found_count = 0usize;
    let mut not_found_count = 0usize;
    let mut all_paths: Vec<PathBuf> = Vec::new();

    for pattern in &patterns {
        let results = if is_glob_pattern(pattern) {
            find_by_pattern(pdb_dir, pattern, &args).await?
        } else {
            vec![find_single_entry(pdb_dir, pattern, &args).await]
        };

        for result in results {
            if result.found {
                found_count += 1;
                if !args.quiet && !args.missing {
                    if args.count {
                        all_paths.extend(result.paths);
                    } else {
                        for path in &result.paths {
                            println!("{}", path.display());
                        }
                    }
                }
            } else {
                not_found_count += 1;
                if !args.quiet && args.missing {
                    println!("{}", result.pdb_id);
                }
            }
        }
    }

    // Handle count mode
    if args.count && !args.quiet {
        println!("{}", all_paths.len());
    }

    // Exit code based on results
    if args.exists && not_found_count > 0 {
        std::process::exit(1);
    }

    if !args.exists && !args.missing && found_count == 0 && not_found_count > 0 {
        // Normal mode: exit 1 if nothing found
        std::process::exit(1);
    }

    Ok(())
}

/// Collect patterns from command line args and/or stdin
fn collect_patterns(args: &FindArgs) -> Result<Vec<String>> {
    let mut patterns = args.patterns.clone();

    if args.stdin {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let line = line.map_err(PdbCliError::Io)?;
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                patterns.push(trimmed.to_string());
            }
        }
    }

    Ok(patterns)
}

/// Check if a pattern contains glob wildcards
fn is_glob_pattern(pattern: &str) -> bool {
    pattern.contains('*') || pattern.contains('?') || pattern.contains('[')
}

/// Find a single PDB entry by exact ID
async fn find_single_entry(pdb_dir: &Path, id: &str, args: &FindArgs) -> FindResult {
    let pdb_id = match PdbId::new(id) {
        Ok(id) => id,
        Err(_) => {
            return FindResult {
                pdb_id: id.to_string(),
                paths: Vec::new(),
                found: false,
            };
        }
    };

    let paths = search_entry_paths(pdb_dir, &pdb_id, args).await;
    let found = !paths.is_empty();

    FindResult {
        pdb_id: pdb_id.as_str().to_string(),
        paths,
        found,
    }
}

/// Find entries matching a glob pattern
async fn find_by_pattern(
    pdb_dir: &Path,
    pattern: &str,
    args: &FindArgs,
) -> Result<Vec<FindResult>> {
    let glob_pattern = Pattern::new(&pattern.to_lowercase())
        .map_err(|e| PdbCliError::InvalidInput(format!("Invalid pattern '{}': {}", pattern, e)))?;

    let mut results = Vec::new();
    let formats = get_formats_to_search(args);

    for format in &formats {
        let format_dir = pdb_dir.join(format.subdir());
        if !format_dir.exists() {
            continue;
        }

        // Scan hash directories (divided layout)
        let mut hash_entries = fs::read_dir(&format_dir).await?;
        while let Some(hash_entry) = hash_entries.next_entry().await? {
            let hash_path = hash_entry.path();
            if !hash_path.is_dir() {
                continue;
            }

            // Scan files in hash directory
            let mut file_entries = fs::read_dir(&hash_path).await?;
            while let Some(file_entry) = file_entries.next_entry().await? {
                let file_path = file_entry.path();
                let file_name = match file_path.file_name().and_then(|n| n.to_str()) {
                    Some(n) => n,
                    None => continue,
                };

                // Extract PDB ID from filename
                let pdb_id = match extract_pdb_id_from_filename(file_name, format) {
                    Some(id) => id,
                    None => continue,
                };

                // Check if pattern matches
                if glob_pattern.matches(&pdb_id.to_lowercase()) {
                    results.push(FindResult {
                        pdb_id: pdb_id.clone(),
                        paths: vec![file_path],
                        found: true,
                    });
                }
            }
        }
    }

    // Deduplicate by PDB ID if --all-formats is not set
    if !args.all_formats {
        results = deduplicate_results(results);
    }

    Ok(results)
}

/// Search for paths of a specific PDB entry
async fn search_entry_paths(pdb_dir: &Path, pdb_id: &PdbId, args: &FindArgs) -> Vec<PathBuf> {
    let formats = get_formats_to_search(args);
    let mut paths = Vec::new();

    for format in &formats {
        let path = build_full_path(pdb_dir, pdb_id, *format);
        if path.exists() {
            paths.push(path);
            if !args.all_formats {
                break; // Return first found unless --all-formats
            }
        }
    }

    paths
}

/// Get the list of formats to search based on args
fn get_formats_to_search(args: &FindArgs) -> Vec<FileFormat> {
    if let Some(format) = args.format {
        vec![format]
    } else {
        // Default: search all compressed formats
        vec![FileFormat::CifGz, FileFormat::PdbGz, FileFormat::BcifGz]
    }
}

/// Extract PDB ID from filename based on format
fn extract_pdb_id_from_filename(filename: &str, format: &FileFormat) -> Option<String> {
    match format {
        FileFormat::Mmcif | FileFormat::CifGz => {
            // Format: {pdb_id}.cif.gz or {pdb_id}.cif
            filename
                .strip_suffix(".cif.gz")
                .or_else(|| filename.strip_suffix(".cif"))
                .map(|s| s.to_string())
        }
        FileFormat::Pdb | FileFormat::PdbGz => {
            // Classic format: pdb{id}.ent.gz or pdb{id}.pdb
            // Extended format: {id}.ent.gz or {id}.pdb (no extra prefix)
            filename
                .strip_suffix(".ent.gz")
                .or_else(|| filename.strip_suffix(".pdb"))
                .and_then(|s| {
                    if s.starts_with("pdb") && s.len() == 7 {
                        // Classic: pdb1abc -> 1abc
                        Some(s[3..].to_string())
                    } else if s.starts_with("pdb_") && s.len() == 12 {
                        // Extended: pdb_00001abc (full ID)
                        Some(s.to_string())
                    } else {
                        None
                    }
                })
        }
        FileFormat::Bcif | FileFormat::BcifGz => {
            // Format: {pdb_id}.bcif.gz or {pdb_id}.bcif
            filename
                .strip_suffix(".bcif.gz")
                .or_else(|| filename.strip_suffix(".bcif"))
                .map(|s| s.to_string())
        }
    }
}

/// Deduplicate results by PDB ID, keeping first occurrence
fn deduplicate_results(results: Vec<FindResult>) -> Vec<FindResult> {
    let mut seen = HashSet::new();
    results
        .into_iter()
        .filter(|r| seen.insert(r.pdb_id.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_glob_pattern() {
        assert!(is_glob_pattern("1ab*"));
        assert!(is_glob_pattern("*abc"));
        assert!(is_glob_pattern("1?bc"));
        assert!(is_glob_pattern("[12]abc"));
        assert!(!is_glob_pattern("1abc"));
        assert!(!is_glob_pattern("pdb_00001abc"));
    }

    #[test]
    fn test_extract_pdb_id_mmcif() {
        assert_eq!(
            extract_pdb_id_from_filename("1abc.cif.gz", &FileFormat::CifGz),
            Some("1abc".to_string())
        );
        assert_eq!(
            extract_pdb_id_from_filename("pdb_00001abc.cif.gz", &FileFormat::CifGz),
            Some("pdb_00001abc".to_string())
        );
    }

    #[test]
    fn test_extract_pdb_id_pdb() {
        assert_eq!(
            extract_pdb_id_from_filename("pdb1abc.ent.gz", &FileFormat::PdbGz),
            Some("1abc".to_string())
        );
        assert_eq!(
            extract_pdb_id_from_filename("pdb_00001abc.ent.gz", &FileFormat::PdbGz),
            Some("pdb_00001abc".to_string())
        );
    }

    #[test]
    fn test_extract_pdb_id_bcif() {
        assert_eq!(
            extract_pdb_id_from_filename("1abc.bcif.gz", &FileFormat::BcifGz),
            Some("1abc".to_string())
        );
        assert_eq!(
            extract_pdb_id_from_filename("pdb_00001abc.bcif.gz", &FileFormat::BcifGz),
            Some("pdb_00001abc".to_string())
        );
    }

    #[test]
    fn test_extract_pdb_id_uncompressed() {
        // mmCIF uncompressed
        assert_eq!(
            extract_pdb_id_from_filename("1abc.cif", &FileFormat::Mmcif),
            Some("1abc".to_string())
        );
        // BCIF uncompressed
        assert_eq!(
            extract_pdb_id_from_filename("1abc.bcif", &FileFormat::Bcif),
            Some("1abc".to_string())
        );
    }
}
