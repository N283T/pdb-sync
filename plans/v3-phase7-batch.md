# Phase 7: Batch Processing

## Parallel Development Note
This phase is being developed in parallel with other v3 phases.
- Create PR and request review
- Wait for review approval before merging
- Rebase on master if other phases are merged first

## Goal
Enable pipeline-style batch processing with stdin/stdout support for composing commands.

## Usage Examples

```bash
# Pipe search results to download
pdb-cli search --resolution "<1.5" -o ids | pdb-cli download --stdin

# Pipe list to validate
pdb-cli list -o ids | pdb-cli validate --stdin

# Pipe search to copy
pdb-cli search --method nmr -o ids | pdb-cli copy --stdin --dest ./nmr_structures

# Chain with standard Unix tools
pdb-cli search --text "kinase" -o ids | head -100 | pdb-cli download --stdin

# Read from file (existing feature, but consistent)
pdb-cli download -l ids.txt
cat ids.txt | pdb-cli download --stdin

# Output IDs for piping
pdb-cli list -o ids > local_ids.txt
pdb-cli validate --errors-only -o ids | pdb-cli download --stdin --overwrite

# Complex pipeline
pdb-cli search --organism "E. coli" --resolution "<2.0" -o ids \
  | pdb-cli download --stdin \
  && pdb-cli validate -P
```

## Implementation Tasks

### 1. Add --stdin flag to relevant commands

```rust
// src/cli/args.rs
#[derive(Args)]
pub struct DownloadArgs {
    /// PDB IDs to download
    pub pdb_ids: Vec<String>,

    /// Read PDB IDs from file
    #[arg(short, long)]
    pub list: Option<PathBuf>,

    /// Read PDB IDs from stdin
    #[arg(long)]
    pub stdin: bool,

    // ... other args
}

// Similarly for: ValidateArgs, CopyArgs, InfoArgs
```

### 2. Create ID reader utility

```rust
// src/utils/id_reader.rs
pub struct IdSource {
    pub ids: Vec<String>,
}

impl IdSource {
    /// Collect IDs from all sources (args, file, stdin)
    pub fn collect(
        args_ids: Vec<String>,
        list_file: Option<&Path>,
        use_stdin: bool,
    ) -> Result<Self> {
        let mut ids = args_ids;

        if let Some(path) = list_file {
            ids.extend(read_ids_from_file(path)?);
        }

        if use_stdin {
            ids.extend(read_ids_from_stdin()?);
        }

        // Deduplicate and validate
        let ids: Vec<String> = ids
            .into_iter()
            .filter(|id| !id.is_empty())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        Ok(Self { ids })
    }
}

fn read_ids_from_stdin() -> Result<Vec<String>> {
    use std::io::{self, BufRead};

    let stdin = io::stdin();
    let ids: Vec<String> = stdin
        .lock()
        .lines()
        .filter_map(|line| line.ok())
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();

    Ok(ids)
}
```

### 3. Add --output ids format

Ensure all commands that output PDB IDs support `-o ids`:

```rust
// src/cli/args.rs
#[derive(ValueEnum, Clone)]
pub enum OutputFormat {
    Text,
    Json,
    Csv,
    Ids,  // One ID per line, for piping
}
```

### 4. Update commands to use IdSource

```rust
// src/cli/commands/download.rs
pub async fn run_download(args: DownloadArgs, ctx: AppContext) -> Result<()> {
    let source = IdSource::collect(
        args.pdb_ids,
        args.list.as_deref(),
        args.stdin,
    )?;

    if source.ids.is_empty() {
        return Err(PdbCliError::NoIdsProvided);
    }

    // ... rest of download logic
}
```

### 5. Detect if stdin is a TTY

```rust
// Don't wait for stdin if it's a terminal and --stdin not specified
fn should_read_stdin(explicit_stdin: bool) -> bool {
    if explicit_stdin {
        return true;
    }
    // Auto-detect piped input
    !atty::is(atty::Stream::Stdin)
}
```

### 6. Add batch-specific options

```rust
// src/cli/args.rs
#[derive(Args)]
pub struct BatchOptions {
    /// Continue on errors (don't stop on first failure)
    #[arg(long)]
    pub continue_on_error: bool,

    /// Maximum items to process
    #[arg(long)]
    pub max_items: Option<usize>,

    /// Skip first N items
    #[arg(long)]
    pub skip: Option<usize>,
}
```

## Commands to Update

| Command | Add --stdin | Add -o ids | Notes |
|---------|-------------|------------|-------|
| download | ✅ | N/A | Primary batch target |
| validate | ✅ | ✅ | Output failed IDs |
| copy | ✅ | N/A | |
| info | ✅ | N/A | Multiple ID queries |
| list | N/A | ✅ | Source for pipelines |
| search | N/A | ✅ | Source for pipelines |
| update | ✅ | ✅ | Output outdated IDs |

## Files to Create/Modify

- `src/utils/id_reader.rs` - New: ID collection utility
- `src/utils/mod.rs` - Export id_reader
- `src/cli/args.rs` - Add --stdin, -o ids to commands
- `src/cli/commands/download.rs` - Use IdSource
- `src/cli/commands/validate.rs` - Use IdSource, add -o ids
- `src/cli/commands/copy.rs` - Use IdSource
- `src/cli/commands/list.rs` - Add -o ids
- `src/cli/commands/search.rs` - Add -o ids

## Dependencies

```toml
# Cargo.toml
[dependencies]
atty = "0.2"  # For TTY detection
```

## Testing

- Test reading from stdin
- Test piping between commands
- Test deduplication
- Test empty input handling
- Test TTY detection
- Test --continue-on-error
