# Phase 8: Stats Command

## Parallel Development Note
This phase is being developed in parallel with other v3 phases.
- Create PR and request review
- Wait for review approval before merging
- Rebase on master if other phases are merged first

## Goal
Add `stats` command to show statistics about the local PDB collection.

## Usage Examples

```bash
# Show overall statistics
pdb-cli stats

# Output:
# Local PDB Collection Statistics
# ===============================
# Total entries: 12,345
# Total size: 45.6 GB
#
# By format:
#   mmCIF (.cif.gz): 10,234 files (38.2 GB)
#   PDB (.ent.gz):    2,111 files (7.4 GB)
#
# By data type:
#   structures:      10,234
#   assemblies:       1,500
#   structure-factors:  611
#
# Coverage: 5.2% of PDB archive (12,345 / 235,000)
# Last sync: 2024-01-15 10:30:00
# Last download: 2024-01-16 14:20:00

# Detailed statistics
pdb-cli stats --detailed

# Output format
pdb-cli stats -o json
pdb-cli stats -o csv

# Stats for specific format
pdb-cli stats --format cif-gz

# Stats for specific data type
pdb-cli stats --type structures

# Compare with remote (requires network)
pdb-cli stats --compare-remote
```

## Statistics to Collect

### Basic Stats
- Total number of entries
- Total size on disk
- Breakdown by format (cif, pdb, bcif, etc.)
- Breakdown by data type (structures, assemblies, etc.)

### Detailed Stats
- Size distribution (histogram)
- Oldest/newest files by mtime
- Average file size
- Files per hash directory (for divided layout)

### Comparison Stats (--compare-remote)
- Total entries in PDB archive
- Coverage percentage
- Missing entries count
- Outdated entries count

## Implementation Tasks

### 1. Create stats collection module

```rust
// src/stats/mod.rs
pub struct LocalStats {
    pub total_entries: u64,
    pub total_size: u64,
    pub by_format: HashMap<FileFormat, FormatStats>,
    pub by_data_type: HashMap<DataType, u64>,
    pub oldest_file: Option<FileInfo>,
    pub newest_file: Option<FileInfo>,
    pub last_sync: Option<DateTime<Utc>>,
    pub last_download: Option<DateTime<Utc>>,
}

pub struct FormatStats {
    pub count: u64,
    pub total_size: u64,
}

pub struct FileInfo {
    pub pdb_id: String,
    pub path: PathBuf,
    pub size: u64,
    pub mtime: DateTime<Utc>,
}

impl LocalStats {
    pub async fn collect(pdb_dir: &Path) -> Result<Self> {
        // Walk directory and collect stats
    }
}
```

### 2. Add remote comparison

```rust
// src/stats/remote.rs
pub struct RemoteStats {
    pub total_entries: u64,
    pub last_updated: DateTime<Utc>,
}

pub async fn fetch_remote_stats() -> Result<RemoteStats> {
    // Query RCSB API for total entry count
    // GET https://data.rcsb.org/rest/v1/holdings/current/entry_ids
    // Or use search API with count
}

pub struct ComparisonStats {
    pub local: LocalStats,
    pub remote: RemoteStats,
    pub coverage_percent: f64,
    pub missing_count: u64,
}
```

### 3. Add CLI args

```rust
// src/cli/args.rs
#[derive(Args)]
pub struct StatsArgs {
    /// Show detailed statistics
    #[arg(long)]
    pub detailed: bool,

    /// Compare with remote PDB archive
    #[arg(long)]
    pub compare_remote: bool,

    /// Filter by format
    #[arg(short, long)]
    pub format: Option<FileFormat>,

    /// Filter by data type
    #[arg(short, long)]
    pub r#type: Option<DataType>,

    /// Output format
    #[arg(short, long, default_value = "text")]
    pub output: OutputFormat,
}
```

### 4. Implement command handler

```rust
// src/cli/commands/stats.rs
pub async fn run_stats(args: StatsArgs, ctx: AppContext) -> Result<()> {
    let stats = LocalStats::collect(&ctx.pdb_dir).await?;

    let comparison = if args.compare_remote {
        Some(fetch_comparison(&stats).await?)
    } else {
        None
    };

    match args.output {
        OutputFormat::Text => print_text_stats(&stats, comparison, args.detailed),
        OutputFormat::Json => print_json_stats(&stats, comparison),
        OutputFormat::Csv => print_csv_stats(&stats, comparison),
    }
}
```

### 5. Track sync/download history

```rust
// src/history/mod.rs
pub struct OperationHistory {
    pub last_sync: Option<OperationRecord>,
    pub last_download: Option<OperationRecord>,
}

pub struct OperationRecord {
    pub timestamp: DateTime<Utc>,
    pub operation: String,
    pub count: u64,
}

// Store in: ~/.cache/pdb-cli/history.json
```

### 6. Pretty text output

```rust
fn print_text_stats(stats: &LocalStats, comparison: Option<ComparisonStats>, detailed: bool) {
    println!("Local PDB Collection Statistics");
    println!("===============================");
    println!("Total entries: {}", stats.total_entries.separate_with_commas());
    println!("Total size: {}", human_bytes(stats.total_size));
    println!();

    println!("By format:");
    for (format, fs) in &stats.by_format {
        println!("  {:15} {:>8} files ({:>8})",
            format.to_string(),
            fs.count.separate_with_commas(),
            human_bytes(fs.total_size));
    }

    if let Some(comp) = comparison {
        println!();
        println!("Coverage: {:.1}% of PDB archive ({} / {})",
            comp.coverage_percent,
            stats.total_entries.separate_with_commas(),
            comp.remote.total_entries.separate_with_commas());
    }
}
```

## Files to Create/Modify

- `src/stats/mod.rs` - New: Stats collection
- `src/stats/remote.rs` - New: Remote comparison
- `src/history/mod.rs` - New: Operation history tracking
- `src/lib.rs` - Export stats, history modules
- `src/cli/args.rs` - Add StatsArgs
- `src/cli/commands/stats.rs` - New: Stats command handler
- `src/cli/commands/mod.rs` - Export stats
- `src/main.rs` - Add stats command

## Dependencies

```toml
# Cargo.toml
[dependencies]
thousands = "0.2"  # For number formatting with separators
```

## Testing

- Test stats collection on mock directory
- Test remote stats fetching
- Test comparison calculations
- Test output formats
- Test filtering by format/type
