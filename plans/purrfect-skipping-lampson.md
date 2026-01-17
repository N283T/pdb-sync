# Phase 9: Watch Command Implementation Plan

## Overview

Implement the `watch` command to monitor RCSB for new PDB entries and automatically download them with configurable filters, notifications, and hook scripts.

## Files to Create

### 1. `src/watch/mod.rs` - Watch Orchestration
- `WatchConfig` struct - parsed config from CLI args
- `Watcher` struct - main orchestrator with watch loop
- `parse_interval()` - parse duration strings ("1h", "30m")
- Graceful shutdown handling with Ctrl+C

### 2. `src/watch/rcsb.rs` - RCSB Search API Client
- `SearchFilters` struct - method, resolution, organism filters
- `RcsbSearchClient` - POST to `https://search.rcsb.org/rcsbsearch/v2/query`
- `search_new_entries(since: NaiveDate, filters)` - query new releases
- Build JSON query with date filter + optional filters

### 3. `src/watch/state.rs` - State Persistence
- `WatchState` struct - last_check timestamp, downloaded_ids set
- Load from `~/.cache/pdb-cli/watch_state.json`
- `save()` / `load_or_init()` / `mark_downloaded()` / `is_downloaded()`

### 4. `src/watch/notify.rs` - Notifications
- `NotifyConfig` struct
- `NotificationSender::notify()` - dispatch to desktop/email
- Feature-gated: `desktop-notify`, `email-notify`

### 5. `src/watch/hooks.rs` - Hook Script Execution
- `HookRunner::run(pdb_id, file_path)` - execute user script
- Pass PDB_ID and PDB_FILE as args and env vars

### 6. `src/cli/commands/watch.rs` - Command Handler
- `run_watch(args: WatchArgs, ctx: AppContext) -> Result<()>`

## Files to Modify

### 1. `src/cli/args.rs`
Add:
```rust
/// Notification method
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum NotifyMethod { Desktop, Email }

/// Experimental method filter
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
pub enum ExperimentalMethod { Xray, Nmr, Em, Neutron, Other }

#[derive(Parser)]
pub struct WatchArgs {
    #[arg(short, long, default_value = "1h")]
    pub interval: String,
    #[arg(long)]
    pub method: Option<ExperimentalMethod>,
    #[arg(long)]
    pub resolution: Option<f64>,
    #[arg(long)]
    pub organism: Option<String>,
    #[arg(short = 't', long = "type", value_enum)]
    pub data_types: Vec<DataType>,
    #[arg(short, long, value_enum, default_value = "mmcif")]
    pub format: FileFormat,
    #[arg(short = 'n', long)]
    pub dry_run: bool,
    #[arg(long)]
    pub notify: Option<NotifyMethod>,
    #[arg(long, requires = "notify")]
    pub email: Option<String>,
    #[arg(long)]
    pub on_new: Option<PathBuf>,
    #[arg(short, long)]
    pub dest: Option<PathBuf>,
    #[arg(short, long, value_enum)]
    pub mirror: Option<MirrorId>,
    #[arg(long)]
    pub once: bool,
    #[arg(long)]
    pub since: Option<String>,
}

// Add to Commands enum:
Watch(WatchArgs),
```

### 2. `src/cli/commands/mod.rs`
```rust
pub mod watch;
pub use watch::run_watch;
```

### 3. `src/main.rs`
```rust
Commands::Watch(args) => {
    cli::commands::run_watch(args, ctx).await?;
}
```

### 4. `src/error.rs`
Add error variants:
```rust
#[error("Watch error: {0}")]
Watch(String),
#[error("Search API error: {0}")]
SearchApi(String),
#[error("State persistence error: {0}")]
StatePersistence(String),
#[error("Hook execution failed: {0}")]
HookExecution(String),
#[error("Notification error: {0}")]
Notification(String),
#[error("Invalid interval: {0}")]
InvalidInterval(String),
```

### 5. `Cargo.toml`
```toml
[dependencies]
humantime = "2"

[dependencies.notify-rust]
version = "4"
optional = true

[dependencies.lettre]
version = "0.11"
features = ["tokio1-rustls-tls"]
optional = true

[features]
default = []
desktop-notify = ["notify-rust"]
email-notify = ["lettre"]
```

## Implementation Sequence

1. **Dependencies & Errors** - Update Cargo.toml, error.rs
2. **CLI Args** - Add WatchArgs, enums, Commands::Watch
3. **State Module** - Implement state.rs with persistence
4. **Search Client** - Implement rcsb.rs with RCSB Search API
5. **Watch Core** - Implement mod.rs with Watcher loop
6. **Hooks** - Implement hooks.rs
7. **Notifications** - Implement notify.rs (feature-gated)
8. **Command Handler** - Implement watch.rs, update main.rs
9. **Tests** - Unit tests for each module

## Testing Strategy

### Unit Tests
- `state.rs`: Save/load roundtrip, mark_downloaded, case-insensitivity
- `rcsb.rs`: Query building (date-only, with filters, combined)
- `hooks.rs`: Script not found error, successful execution (Unix)
- `mod.rs`: Interval parsing ("1h", "30m", invalid inputs)

### Integration Tests
- RCSB API call with recent date (network test, ignored by default)
- Dry-run mode (no downloads)
- --once mode (exits after one check)

## Verification

Run these commands to verify the implementation:

```bash
# Build
cargo build

# Run quality checks
cargo fmt --check
cargo clippy -- -D warnings

# Run tests
cargo test

# Test CLI help
cargo run -- watch --help

# Test dry-run (network test)
cargo run -- watch --once --dry-run --since 2025-01-01

# Test with filters
cargo run -- watch --once --dry-run --method xray --resolution 2.0 --since 2025-01-01
```

## Key Design Decisions

1. **Feature Flags**: Desktop/email notifications are optional to avoid unnecessary dependencies
2. **Graceful Shutdown**: Use `tokio::select!` with `ctrl_c` to save state on interrupt
3. **State Location**: `~/.cache/pdb-cli/watch_state.json` following XDG conventions
4. **Default Behavior**: Start from 7 days ago if no state exists
5. **Reuse Existing**: Use `HttpsDownloader` for downloads, `AppContext` for config
