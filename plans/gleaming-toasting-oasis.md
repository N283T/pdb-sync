# Phase 8: Stats Command - Implementation Plan

## Overview

Add a `stats` command to show statistics about the local PDB collection with optional remote comparison.

## Files to Modify/Create

### New Files
- `src/utils/mod.rs` - Shared utilities module
- `src/utils/format.rs` - `human_bytes()`, number formatting, CSV escaping
- `src/stats/mod.rs` - Stats module exports
- `src/stats/types.rs` - Statistics data structures
- `src/stats/collector.rs` - Local statistics collection
- `src/stats/remote.rs` - Remote comparison (RCSB API)
- `src/history/mod.rs` - History module exports
- `src/history/tracker.rs` - Operation history tracking
- `src/cli/commands/stats.rs` - Command handler

### Modified Files
- `Cargo.toml` - Add `thousands` crate
- `src/main.rs` - Add `utils`, `stats`, `history` modules; add Stats command dispatch
- `src/cli/args.rs` - Add `StatsArgs` struct and `Commands::Stats` variant
- `src/cli/commands/mod.rs` - Export stats module
- `src/api/rcsb.rs` - Add `get_total_entry_count()` for remote comparison
- `src/cli/commands/list.rs` - Refactor to use shared utils (optional)

## Implementation Steps

### Step 1: Add Dependency

Add to `Cargo.toml`:
```toml
thousands = "0.2"
```

### Step 2: Create Shared Utilities Module

**`src/utils/mod.rs`**:
```rust
pub mod format;
pub use format::*;
```

**`src/utils/format.rs`**:
- Extract `human_bytes()` from `list.rs`
- Add `format_with_commas()` using thousands crate
- Add `escape_csv_field()` from `list.rs`

### Step 3: Create Stats Types

**`src/stats/types.rs`**:
```rust
pub struct LocalStats {
    pub total_entries: usize,
    pub unique_pdb_ids: usize,
    pub total_size: u64,
    pub by_format: BTreeMap<String, FormatStats>,
    pub by_data_type: BTreeMap<String, DataTypeStats>,
    pub detailed: Option<DetailedStats>,
    pub last_sync: Option<DateTime<Utc>>,
    pub last_download: Option<DateTime<Utc>>,
}

pub struct FormatStats { pub count: usize, pub size: u64 }
pub struct DataTypeStats { pub count: usize, pub size: u64, pub unique_pdb_ids: usize }

pub struct DetailedStats {
    pub min_size: u64,
    pub max_size: u64,
    pub avg_size: f64,
    pub oldest_file: Option<FileInfo>,
    pub newest_file: Option<FileInfo>,
    pub size_distribution: SizeDistribution,
}

pub struct FileInfo { pub pdb_id: String, pub path: PathBuf, pub size: u64, pub modified: DateTime<Local> }

pub struct SizeDistribution {
    pub under_1kb: usize,
    pub kb_1_10: usize,
    pub kb_10_100: usize,
    pub kb_100_1mb: usize,
    pub mb_1_10: usize,
    pub over_10mb: usize,
}

pub struct RemoteStats { pub total_entries: usize, pub fetched_at: DateTime<Utc> }
pub struct Comparison { pub local_count: usize, pub remote_count: usize, pub coverage_percent: f64 }
```

### Step 4: Create Stats Collector

**`src/stats/collector.rs`**:
- `LocalStatsCollector::new(pdb_dir: &Path)`
- `collect(&self, detailed: bool, format_filter: Option<FileFormat>, type_filter: Option<DataType>) -> Result<LocalStats>`
- Scan directories: mmCIF/, pdb/, bcif/ for structures; other directories for other data types
- Use DataType enum for directory mapping

### Step 5: Add Remote Stats

**`src/api/rcsb.rs`** - Add method:
```rust
pub async fn get_total_entry_count(&self) -> Result<usize> {
    // POST to https://search.rcsb.org/rcsbsearch/v2/query
    // with minimal query returning just total_count
}
```

**`src/stats/remote.rs`**:
- `RemoteStatsProvider::new()`
- `fetch_stats() -> Result<RemoteStats>`
- `compare(local_count: usize, remote_stats: &RemoteStats) -> Comparison`

### Step 6: Create History Tracker

**`src/history/tracker.rs`**:
```rust
pub struct HistoryTracker { path: PathBuf, history: OperationHistory }

pub struct OperationHistory {
    pub last_sync: Option<DateTime<Utc>>,
    pub last_download: Option<DateTime<Utc>>,
}

impl HistoryTracker {
    pub fn load() -> Result<Self>  // From ~/.cache/pdb-cli/history.json
    pub fn save(&self) -> Result<()>
    pub fn record_sync(&mut self, timestamp: DateTime<Utc>)
    pub fn record_download(&mut self, timestamp: DateTime<Utc>)
}
```

### Step 7: Add CLI Arguments

**`src/cli/args.rs`**:
```rust
#[derive(Parser)]
pub struct StatsArgs {
    #[arg(long)]
    pub detailed: bool,

    #[arg(long)]
    pub compare_remote: bool,

    #[arg(short, long, value_enum)]
    pub format: Option<FileFormat>,

    #[arg(short = 't', long = "type", value_enum)]
    pub data_type: Option<DataType>,

    #[arg(short, long, value_enum, default_value = "text")]
    pub output: OutputFormat,
}

// Add to Commands enum:
Stats(StatsArgs),
```

### Step 8: Create Command Handler

**`src/cli/commands/stats.rs`**:
```rust
pub async fn run_stats(args: StatsArgs, ctx: AppContext) -> Result<()> {
    let collector = LocalStatsCollector::new(&ctx.pdb_dir);
    let stats = collector.collect(args.detailed, args.format, args.data_type).await?;

    let comparison = if args.compare_remote {
        let client = RcsbClient::new();
        let remote = RemoteStatsProvider::fetch_stats(&client).await?;
        Some(RemoteStatsProvider::compare(stats.total_entries, &remote))
    } else {
        None
    };

    // Load history for last_sync/last_download timestamps
    let history = HistoryTracker::load().ok();

    match args.output {
        OutputFormat::Text => print_text(&stats, comparison.as_ref(), history.as_ref(), args.detailed),
        OutputFormat::Json => print_json(&stats, comparison.as_ref())?,
        OutputFormat::Csv => print_csv(&stats, comparison.as_ref()),
    }
    Ok(())
}
```

### Step 9: Update Main and Modules

**`src/main.rs`**:
```rust
mod utils;
mod stats;
mod history;

// In match cli.command:
Commands::Stats(args) => cli::commands::run_stats(args, ctx).await?,
```

**`src/cli/commands/mod.rs`**:
```rust
pub mod stats;
pub use stats::run_stats;
```

## Output Examples

### Basic Text Output
```
Local PDB Collection Statistics
===============================
Total entries: 12,345
Total size: 45.6 GB

By format:
  mmCIF (.cif.gz): 10,234 files (38.2 GB)
  PDB (.ent.gz):    2,111 files (7.4 GB)

By data type:
  structures:       10,234
  assemblies:        1,500
  structure-factors:   611

Last sync: 2024-01-15 10:30:00
```

### With --compare-remote
```
...
Coverage: 5.2% of PDB archive (12,345 / 235,000)
```

### With --detailed
```
...
Size Distribution:
  < 1 KB:        120 files
  1-10 KB:     1,234 files
  10-100 KB:   8,500 files
  100 KB-1 MB: 4,500 files
  1-10 MB:     1,000 files
  > 10 MB:        78 files

Smallest: 234 B  (9xyz.cif.gz)
Largest:  45 MB (8abc.cif.gz)
Average:  820 KB

Oldest: 1abc.cif.gz (2024-01-01 10:00)
Newest: 9xyz.cif.gz (2025-01-18 14:30)
```

## Test Plan

### Unit Tests
1. `utils/format.rs`: Test `human_bytes()`, `format_with_commas()`, `escape_csv_field()`
2. `stats/types.rs`: Test serialization of all structs
3. `stats/collector.rs`: Test with mock filesystem using tempfile crate
4. `stats/remote.rs`: Test comparison calculations
5. `history/tracker.rs`: Test load/save cycle

### Integration Tests
1. Create temp directory with mock PDB files
2. Run `pdb-cli stats` and verify output
3. Test `--format cif-gz` filter
4. Test `--type structures` filter
5. Test `--detailed` output
6. Test `-o json` and `-o csv` output formats

### Network Tests (ignored by default)
1. Test `--compare-remote` with real RCSB API

## Verification

After implementation:
```bash
# Build and check
cargo fmt --check
cargo clippy -- -D warnings
cargo test

# Manual testing
cargo run -- stats
cargo run -- stats --detailed
cargo run -- stats --format cif-gz
cargo run -- stats --type structures
cargo run -- stats -o json
cargo run -- stats --compare-remote
```
