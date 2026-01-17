# Phase 11: Find Command

## Parallel Development Note
This phase is being developed in parallel with other v3 phases.
- Create PR and request review
- Wait for review approval before merging
- Rebase on master if other phases are merged first

## Goal
Add `find` command for searching local PDB files with path output, optimized for scripting and existence checks.

## Difference from `list`

| Feature | `list` | `find` |
|---------|--------|--------|
| Purpose | View collection | Search/locate files |
| Output | Formatted table | Paths (script-friendly) |
| Stats | Yes (`--stats`) | No |
| Existence check | No | Yes (`--exists`, `--missing`) |
| Multiple formats | Filter one | Search across all |

## Usage Examples

```bash
# Check if entry exists locally
pdb-cli find 4hhb
# Output: /data/pdb/mmCIF/hh/4hhb.cif.gz

# Find multiple entries
pdb-cli find 4hhb 1abc 2xyz
# Output:
# /data/pdb/mmCIF/hh/4hhb.cif.gz
# /data/pdb/mmCIF/ab/1abc.cif.gz
# /data/pdb/mmCIF/xy/2xyz.cif.gz

# Glob pattern search
pdb-cli find "1ab*"
pdb-cli find "????a"  # 4-char IDs ending with 'a'

# Find specific data type
pdb-cli find --type assemblies 4hhb
pdb-cli find -t structure-factors 1abc

# Find specific format
pdb-cli find --format pdb 4hhb
pdb-cli find -f cif-gz "1*"

# Check existence (exit code)
pdb-cli find --exists 4hhb && echo "Found"
pdb-cli find --exists 4hhb 1abc 2xyz  # All must exist

# Find missing entries (not in local)
pdb-cli find --missing 4hhb 1abc 2xyz
# Output: entries NOT found locally

# Find from stdin (for piping)
echo -e "4hhb\n1abc" | pdb-cli find --stdin
cat ids.txt | pdb-cli find --stdin --missing

# Find all formats for an entry
pdb-cli find --all-formats 4hhb
# Output:
# /data/pdb/mmCIF/hh/4hhb.cif.gz
# /data/pdb/pdb/hh/pdb4hhb.ent.gz

# Quiet mode (just exit code)
pdb-cli find -q 4hhb  # Exit 0 if found, 1 if not

# Count matches
pdb-cli find "1ab*" --count
# Output: 42

# Use with xargs
pdb-cli find "1ab*" | xargs -I {} cp {} ./my_folder/
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All requested entries found |
| 1 | One or more entries not found |
| 2 | Error (invalid args, IO error, etc.) |

## Implementation Tasks

### 1. Create find module

```rust
// src/find/mod.rs
pub struct FindOptions {
    pub data_type: Option<DataType>,
    pub format: Option<FileFormat>,
    pub all_formats: bool,
    pub exists_mode: bool,
    pub missing_mode: bool,
    pub quiet: bool,
    pub count: bool,
}

pub struct FindResult {
    pub pdb_id: String,
    pub paths: Vec<PathBuf>,
    pub found: bool,
}

pub fn find_entry(
    pdb_dir: &Path,
    pdb_id: &str,
    options: &FindOptions,
) -> FindResult {
    // Search for the entry in various locations
}

pub fn find_pattern(
    pdb_dir: &Path,
    pattern: &str,
    options: &FindOptions,
) -> Vec<FindResult> {
    // Glob search
}
```

### 2. Search logic

```rust
// src/find/search.rs
pub fn search_entry_paths(
    pdb_dir: &Path,
    pdb_id: &str,
    data_type: Option<DataType>,
    format: Option<FileFormat>,
) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let hash = &pdb_id[1..3];  // Middle 2 chars for divided layout

    // Check various locations based on data_type/format
    // structures/divided/mmCIF/{hash}/{id}.cif.gz
    // structures/divided/pdb/{hash}/pdb{id}.ent.gz
    // assemblies/...
    // etc.

    paths
}

pub fn search_pattern(
    pdb_dir: &Path,
    pattern: &str,
    options: &FindOptions,
) -> Vec<(String, Vec<PathBuf>)> {
    // Use glob to find matching files
    // Extract PDB IDs from filenames
    // Return (pdb_id, paths) pairs
}
```

### 3. Add CLI args

```rust
// src/cli/args.rs
#[derive(Args)]
pub struct FindArgs {
    /// PDB IDs or patterns to find
    pub patterns: Vec<String>,

    /// Read patterns from stdin
    #[arg(long)]
    pub stdin: bool,

    /// Data type to search
    #[arg(short, long)]
    pub r#type: Option<DataType>,

    /// File format to search
    #[arg(short, long)]
    pub format: Option<FileFormat>,

    /// Show all formats for each entry
    #[arg(long)]
    pub all_formats: bool,

    /// Check existence (exit code only)
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

### 4. Implement command handler

```rust
// src/cli/commands/find.rs
pub async fn run_find(args: FindArgs, ctx: AppContext) -> Result<()> {
    let patterns = collect_patterns(&args)?;
    let options = FindOptions::from(&args);

    let mut found_count = 0;
    let mut not_found_count = 0;

    for pattern in &patterns {
        let results = if is_glob_pattern(pattern) {
            find_pattern(&ctx.pdb_dir, pattern, &options)
        } else {
            vec![find_entry(&ctx.pdb_dir, pattern, &options)]
        };

        for result in results {
            if result.found {
                found_count += 1;
                if !args.quiet && !args.missing {
                    for path in &result.paths {
                        println!("{}", path.display());
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

    if args.count {
        println!("{}", found_count);
    }

    // Exit code based on results
    if args.exists && not_found_count > 0 {
        std::process::exit(1);
    }

    Ok(())
}
```

## Files to Create/Modify

- `src/find/mod.rs` - New: Find module
- `src/find/search.rs` - New: Search logic
- `src/lib.rs` - Export find module
- `src/cli/args.rs` - Add FindArgs
- `src/cli/commands/find.rs` - New: Find command handler
- `src/cli/commands/mod.rs` - Export find
- `src/main.rs` - Add find command

## Testing

- Test single entry lookup
- Test glob pattern matching
- Test --exists exit codes
- Test --missing mode
- Test --all-formats
- Test stdin input
- Test count mode
- Test with various data types and formats
