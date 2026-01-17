# Phase 11: Find Command Implementation Plan

## Summary

Implement a `find` command for searching local PDB files with path output, optimized for scripting and existence checks. This differs from `list` by outputting paths (script-friendly) instead of formatted tables, and supporting existence checking with exit codes.

## Files to Create/Modify

### New Files
1. `src/cli/commands/find.rs` - Command handler (main implementation)

### Modified Files
1. `src/cli/args.rs` - Add `FindArgs` struct and `Find` command variant
2. `src/cli/commands/mod.rs` - Export find module
3. `src/main.rs` - Add Find command dispatch

## Implementation Steps

### Step 1: Add FindArgs to args.rs

Add after line 241 (after `ListArgs`):

```rust
#[derive(Parser)]
pub struct FindArgs {
    /// PDB IDs or patterns to find
    pub patterns: Vec<String>,

    /// Read patterns from stdin
    #[arg(long)]
    pub stdin: bool,

    /// Data type to search (default: structures)
    #[arg(short = 't', long = "type", value_enum)]
    pub data_type: Option<DataType>,

    /// File format to search
    #[arg(short, long, value_enum)]
    pub format: Option<FileFormat>,

    /// Show all formats for each entry
    #[arg(long)]
    pub all_formats: bool,

    /// Check existence (exit code only, all must exist for 0)
    #[arg(long)]
    pub exists: bool,

    /// Show entries NOT found locally
    #[arg(long)]
    pub missing: bool,

    /// Quiet mode (no output, just exit code)
    #[arg(short, long)]
    pub quiet: bool,

    /// Count matches only
    #[arg(long)]
    pub count: bool,
}
```

Add to `Commands` enum after `List`:
```rust
/// Find local PDB files (path output for scripting)
Find(FindArgs),
```

### Step 2: Create find.rs command handler

Create `src/cli/commands/find.rs`:

```rust
//! Find command - searches local PDB files with path output.

use crate::cli::args::FindArgs;
use crate::context::AppContext;
use crate::data_types::DataType;
use crate::error::{PdbCliError, Result};
use crate::files::{build_full_path, FileFormat, PdbId};
use glob::Pattern;
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
            "No PDB IDs or patterns provided. Use positional arguments or --stdin.".to_string()
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
                    for path in &result.paths {
                        if args.count {
                            all_paths.push(path.clone());
                        } else {
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
    if args.count {
        println!("{}", all_paths.len());
    }

    // Exit code based on results
    if args.exists || args.missing {
        if args.exists && not_found_count > 0 {
            std::process::exit(1);
        }
        if args.missing && not_found_count == 0 {
            // --missing with nothing missing is "success" (exit 0)
        }
    } else if found_count == 0 && not_found_count > 0 {
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
            let line = line.map_err(|e| PdbCliError::Io(e))?;
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
async fn find_by_pattern(pdb_dir: &Path, pattern: &str, args: &FindArgs) -> Result<Vec<FindResult>> {
    let glob_pattern = Pattern::new(&pattern.to_lowercase())
        .map_err(|e| PdbCliError::InvalidInput(format!("Invalid pattern '{}': {}", pattern, e)))?;

    let mut results = Vec::new();
    let formats = get_formats_to_search(args);

    for format in &formats {
        let format_dir = pdb_dir.join(format.subdir());
        if !format_dir.exists() {
            continue;
        }

        // Scan hash directories
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
    } else if args.all_formats {
        vec![FileFormat::CifGz, FileFormat::PdbGz, FileFormat::BcifGz]
    } else {
        // Default: search all compressed formats, return first found
        vec![FileFormat::CifGz, FileFormat::PdbGz, FileFormat::BcifGz]
    }
}

/// Extract PDB ID from filename based on format
fn extract_pdb_id_from_filename(filename: &str, format: &FileFormat) -> Option<String> {
    match format {
        FileFormat::Mmcif | FileFormat::CifGz => {
            filename.strip_suffix(".cif.gz")
                .or_else(|| filename.strip_suffix(".cif"))
                .map(|s| s.to_string())
        }
        FileFormat::Pdb | FileFormat::PdbGz => {
            // PDB format: pdb{id}.ent.gz for classic, {id}.ent.gz for extended
            filename.strip_suffix(".ent.gz")
                .or_else(|| filename.strip_suffix(".pdb"))
                .and_then(|s| {
                    if s.starts_with("pdb") && s.len() == 7 {
                        // Classic: pdb1abc -> 1abc
                        Some(s[3..].to_string())
                    } else if s.starts_with("pdb_") {
                        // Extended: pdb_00001abc
                        Some(s.to_string())
                    } else {
                        None
                    }
                })
        }
        FileFormat::Bcif | FileFormat::BcifGz => {
            filename.strip_suffix(".bcif.gz")
                .or_else(|| filename.strip_suffix(".bcif"))
                .map(|s| s.to_string())
        }
    }
}

/// Deduplicate results by PDB ID, keeping first occurrence
fn deduplicate_results(results: Vec<FindResult>) -> Vec<FindResult> {
    let mut seen = std::collections::HashSet::new();
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
    }
}
```

### Step 3: Update commands/mod.rs

Add after line 6:
```rust
pub mod find;
```

Add to exports after line 16:
```rust
pub use find::run_find;
```

### Step 4: Update main.rs

Add match arm in the command dispatch (after line 55, the List arm):
```rust
Commands::Find(args) => {
    cli::commands::run_find(args, ctx).await?;
}
```

## Testing

### Unit Tests (in find.rs)
- `test_is_glob_pattern` - Verify glob detection
- `test_extract_pdb_id_mmcif` - Extract IDs from mmCIF filenames
- `test_extract_pdb_id_pdb` - Extract IDs from PDB filenames
- `test_extract_pdb_id_bcif` - Extract IDs from BCIF filenames

### Integration Tests (manual or automated)
1. **Single entry lookup**: `pdb-cli find 4hhb`
2. **Multiple entries**: `pdb-cli find 4hhb 1abc`
3. **Glob pattern**: `pdb-cli find "1ab*"`
4. **Format filter**: `pdb-cli find --format cif-gz 4hhb`
5. **All formats**: `pdb-cli find --all-formats 4hhb`
6. **Exists check**: `pdb-cli find --exists 4hhb && echo "Found"`
7. **Missing mode**: `pdb-cli find --missing 4hhb nonexistent`
8. **Quiet mode**: `pdb-cli find -q 4hhb`
9. **Count mode**: `pdb-cli find "1*" --count`
10. **Stdin input**: `echo "4hhb" | pdb-cli find --stdin`

### Exit Code Tests
- Exit 0 when all entries found
- Exit 1 when one or more entries not found
- Exit 2 on errors (invalid args, IO errors)

## Verification

```bash
# Build and check for errors
cargo build
cargo clippy -- -D warnings

# Run tests
cargo test find

# Format code
cargo fmt

# Manual smoke tests (if local mirror exists)
pdb-cli find --help
pdb-cli find 4hhb
pdb-cli find --exists 4hhb && echo "Found"
pdb-cli find --missing 4hhb nonexistent_id
```
