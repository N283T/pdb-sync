# Phase 4: Update Command

## Parallel Development Note
This phase is being developed in parallel with other v3 phases.
- Create PR and request review
- Wait for review approval before merging
- Rebase on master if other phases are merged first

## Goal
Add `update` command to check if local files are outdated compared to mirror and optionally update them.

## Usage Examples

```bash
# Check all local files for updates
pdb-cli update --check
pdb-cli update -c

# Check specific PDB IDs
pdb-cli update --check 4hhb 1abc 2xyz

# Check and show details
pdb-cli update --check --verbose

# Actually update outdated files
pdb-cli update
pdb-cli update 4hhb 1abc

# Update with progress
pdb-cli update -P

# Dry run (show what would be updated)
pdb-cli update --dry-run
pdb-cli update -n

# Update only specific format
pdb-cli update --format cif-gz

# Force update (re-download even if up-to-date)
pdb-cli update --force 4hhb

# Output as JSON (for scripting)
pdb-cli update --check -o json
```

## How It Works

### Checking for Updates

1. **Option A: Use HTTP HEAD requests**
   - Check `Last-Modified` header from mirror
   - Compare with local file's mtime
   - Fast but not 100% reliable

2. **Option B: Use checksums**
   - Fetch CHECKSUMS file from mirror
   - Compare with local file's MD5
   - More reliable but slower

3. **Option C: Use RCSB API**
   - Query `rcsb_accession_info.revision_date`
   - Compare with local file's mtime
   - Good for individual entries

### Recommended Approach
- For bulk checking: Use HTTP HEAD (fast)
- For verification: Use checksums (accurate)
- Add `--verify` flag to use checksums instead of mtime

## Implementation Tasks

### 1. Add update detection logic

```rust
// src/update/mod.rs
pub enum UpdateStatus {
    UpToDate,
    Outdated { local_time: DateTime, remote_time: DateTime },
    Missing,
    Unknown,
}

pub async fn check_update_status(
    pdb_id: &str,
    local_path: &Path,
    mirror: MirrorId,
) -> Result<UpdateStatus> {
    // HEAD request to get Last-Modified
    // Compare with local mtime
}

pub async fn check_update_status_checksum(
    pdb_id: &str,
    local_path: &Path,
    mirror: MirrorId,
) -> Result<UpdateStatus> {
    // Fetch checksum from mirror
    // Compare with local file's checksum
}
```

### 2. Add CLI args

```rust
// src/cli/args.rs
#[derive(Args)]
pub struct UpdateArgs {
    /// PDB IDs to check/update (empty = all local files)
    pub pdb_ids: Vec<String>,

    /// Check only, don't update
    #[arg(short, long)]
    pub check: bool,

    /// Dry run
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Use checksums for verification (slower but accurate)
    #[arg(long)]
    pub verify: bool,

    /// Force update even if up-to-date
    #[arg(long)]
    pub force: bool,

    /// File format to check
    #[arg(short, long)]
    pub format: Option<FileFormat>,

    /// Mirror to check against
    #[arg(short, long)]
    pub mirror: Option<MirrorId>,

    /// Show progress
    #[arg(short = 'P', long)]
    pub progress: bool,

    /// Output format
    #[arg(short, long, default_value = "text")]
    pub output: OutputFormat,
}
```

### 3. Implement command handler

```rust
// src/cli/commands/update.rs
pub async fn run_update(args: UpdateArgs, ctx: AppContext) -> Result<()> {
    // 1. Get list of files to check
    // 2. Check update status for each
    // 3. If --check, just report
    // 4. Otherwise, download outdated files
}
```

### 4. Parallel checking with rate limiting

```rust
// Check multiple files in parallel
let semaphore = Arc::new(Semaphore::new(10));
let futures = pdb_ids.iter().map(|id| {
    let permit = semaphore.clone().acquire_owned();
    async move {
        let _permit = permit.await;
        check_update_status(id, ...).await
    }
});
```

## Files to Create/Modify

- `src/update/mod.rs` - New: Update detection logic
- `src/lib.rs` - Export update module
- `src/cli/args.rs` - Add UpdateArgs
- `src/cli/commands/update.rs` - New: Update command handler
- `src/cli/commands/mod.rs` - Export update
- `src/main.rs` - Add update command

## Testing

- Test update detection with mocked HTTP responses
- Test checksum-based verification
- Test with various file states (up-to-date, outdated, missing)
- Test parallel checking
- Test output formats
