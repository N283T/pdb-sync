# Phase 9: Watch Command

## Parallel Development Note
This phase is being developed in parallel with other v3 phases.
- Create PR and request review
- Wait for review approval before merging
- Rebase on master if other phases are merged first

## Goal
Add `watch` command to monitor for new PDB entries and automatically download them.

## Usage Examples

```bash
# Watch for new entries and download
pdb-cli watch

# Watch with specific interval
pdb-cli watch --interval 1h
pdb-cli watch --interval 30m
pdb-cli watch -i 6h

# Watch with filters
pdb-cli watch --method xray --resolution "<2.0"
pdb-cli watch --organism "Homo sapiens"

# Watch specific data types
pdb-cli watch --type structures --type assemblies

# Watch and run in background
pdb-cli watch --bg

# Dry run (show what would be downloaded)
pdb-cli watch --dry-run

# Watch with notification (desktop/email)
pdb-cli watch --notify desktop
pdb-cli watch --notify email --email user@example.com

# Watch with custom hook script
pdb-cli watch --on-new ./my_script.sh
# Script receives: PDB_ID, FILE_PATH as arguments

# Output:
# [2024-01-15 10:30:00] Watching for new PDB entries...
# [2024-01-15 10:30:00] Checking for new entries since 2024-01-14
# [2024-01-15 10:30:05] Found 23 new entries
# [2024-01-15 10:30:05] Downloading: 8abc, 8def, 8ghi, ...
# [2024-01-15 10:35:00] Downloaded 23 new entries
# [2024-01-15 11:30:00] Checking for new entries since 2024-01-15
# [2024-01-15 11:30:02] No new entries found
```

## How It Works

1. **Initial State**: Record last check timestamp or use last known entry
2. **Poll Loop**:
   - Query RCSB API for entries released after last check
   - Apply user filters
   - Download new entries
   - Update last check timestamp
3. **Repeat**: Wait for interval, then check again

## Implementation Tasks

### 1. Create watch module

```rust
// src/watch/mod.rs
pub struct WatchConfig {
    pub interval: Duration,
    pub filters: SearchFilters,
    pub data_types: Vec<DataType>,
    pub notify: Option<NotifyConfig>,
    pub on_new_hook: Option<PathBuf>,
    pub dry_run: bool,
}

pub struct Watcher {
    config: WatchConfig,
    last_check: DateTime<Utc>,
    state_file: PathBuf,
}

impl Watcher {
    pub async fn run(&mut self) -> Result<()> {
        loop {
            let new_entries = self.check_new_entries().await?;

            if !new_entries.is_empty() {
                if self.config.dry_run {
                    self.print_would_download(&new_entries);
                } else {
                    self.download_entries(&new_entries).await?;
                    self.run_hooks(&new_entries).await?;
                    self.notify(&new_entries).await?;
                }
            }

            self.save_state()?;
            tokio::time::sleep(self.config.interval).await;
        }
    }

    async fn check_new_entries(&self) -> Result<Vec<String>> {
        // Query RCSB API for entries released after last_check
        // Apply filters
        // Return list of PDB IDs
    }
}
```

### 2. Query new entries from RCSB

```rust
// src/watch/rcsb.rs
pub async fn get_new_entries(
    since: DateTime<Utc>,
    filters: &SearchFilters,
) -> Result<Vec<String>> {
    // Build search query with:
    // - rcsb_accession_info.initial_release_date > since
    // - User filters (method, resolution, etc.)

    let query = SearchQuery {
        released_after: Some(since.date_naive()),
        ..filters.clone().into()
    };

    search(query).await
}
```

### 3. State persistence

```rust
// src/watch/state.rs
#[derive(Serialize, Deserialize)]
pub struct WatchState {
    pub last_check: DateTime<Utc>,
    pub last_entry_id: Option<String>,
    pub downloaded_ids: HashSet<String>,
}

impl WatchState {
    pub fn load(path: &Path) -> Result<Self> { ... }
    pub fn save(&self, path: &Path) -> Result<()> { ... }
}

// Store in: ~/.cache/pdb-cli/watch_state.json
```

### 4. Add CLI args

```rust
// src/cli/args.rs
#[derive(Args)]
pub struct WatchArgs {
    /// Check interval (e.g., "1h", "30m", "6h")
    #[arg(short, long, default_value = "1h")]
    pub interval: String,

    /// Experimental method filter
    #[arg(long)]
    pub method: Option<ExperimentalMethod>,

    /// Resolution filter (e.g., "<2.0")
    #[arg(long)]
    pub resolution: Option<String>,

    /// Organism filter
    #[arg(long)]
    pub organism: Option<String>,

    /// Data types to download
    #[arg(short, long)]
    pub r#type: Vec<DataType>,

    /// Run in background
    #[arg(long)]
    pub bg: bool,

    /// Dry run
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Notification method
    #[arg(long)]
    pub notify: Option<NotifyMethod>,

    /// Hook script to run on new entries
    #[arg(long)]
    pub on_new: Option<PathBuf>,
}

#[derive(ValueEnum, Clone)]
pub enum NotifyMethod {
    Desktop,
    Email,
}
```

### 5. Notification support

```rust
// src/watch/notify.rs
pub async fn send_notification(
    method: NotifyMethod,
    entries: &[String],
    config: &NotifyConfig,
) -> Result<()> {
    match method {
        NotifyMethod::Desktop => send_desktop_notification(entries),
        NotifyMethod::Email => send_email_notification(entries, config),
    }
}

fn send_desktop_notification(entries: &[String]) -> Result<()> {
    // Use notify-rust crate
    Notification::new()
        .summary("PDB-CLI: New Entries")
        .body(&format!("Downloaded {} new PDB entries", entries.len()))
        .show()?;
    Ok(())
}
```

### 6. Hook script support

```rust
// src/watch/hooks.rs
pub async fn run_hook(
    script: &Path,
    pdb_id: &str,
    file_path: &Path,
) -> Result<()> {
    Command::new(script)
        .arg(pdb_id)
        .arg(file_path)
        .status()
        .await?;
    Ok(())
}
```

## Files to Create/Modify

- `src/watch/mod.rs` - New: Watch orchestration
- `src/watch/rcsb.rs` - New: RCSB new entry queries
- `src/watch/state.rs` - New: State persistence
- `src/watch/notify.rs` - New: Notifications
- `src/watch/hooks.rs` - New: Hook script support
- `src/lib.rs` - Export watch module
- `src/cli/args.rs` - Add WatchArgs
- `src/cli/commands/watch.rs` - New: Watch command handler
- `src/cli/commands/mod.rs` - Export watch
- `src/main.rs` - Add watch command

## Dependencies

```toml
# Cargo.toml
[dependencies]
notify-rust = "4"  # Desktop notifications
humantime = "2"    # Parse duration strings like "1h", "30m"
```

## Testing

- Test new entry detection
- Test state persistence
- Test filter application
- Test hook execution
- Test interval parsing
- Integration test with RCSB API
