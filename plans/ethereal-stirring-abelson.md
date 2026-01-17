# Phase 7: Configuration Improvements - Implementation Plan

## Overview

Phase 7 adds enhanced configuration options including per-data-type directories, default layout settings, and automatic mirror selection based on latency.

## Implementation Steps

### Step 1: Update Config Schema (`src/config/schema.rs`)

**Changes:**
1. Add imports: `use crate::data_types::Layout;` and `use std::collections::HashMap;`
2. Add `mirror_selection: MirrorSelectionConfig` field to `Config`
3. Update `PathsConfig`:
   - Add `data_type_dirs: HashMap<String, PathBuf>`
   - Add `dir_for(&self, data_type: &DataType) -> Option<&PathBuf>` method
4. Update `SyncConfig`:
   - Add `layout: Layout` (use native serde, NOT custom module - Layout already has `#[derive(Serialize, Deserialize)]`)
   - Add `data_types: Vec<String>` with `#[serde(default = "default_data_types")]`
5. Update `DownloadConfig`:
   - Change `parallel: u8` to `parallel: usize`
   - Add `retry_count: u32` (default: 3)
6. Add new `MirrorSelectionConfig` struct:
   - `auto_select: bool` (default: false)
   - `preferred_region: Option<String>`
   - `latency_cache_ttl: u64` (default: 3600)

**Note:** Layout already has `#[derive(Serialize, Deserialize)]` with `#[serde(rename_all = "kebab-case")]` in `src/data_types.rs`, so no custom serde module needed.

### Step 2: Create Auto-Select Module (`src/mirrors/auto_select.rs`)

**New file with:**
- `LatencyCache` struct with RwLock for thread-safe caching
- `select_best_mirror(preferred_region: Option<&str>, cache_ttl: Duration) -> MirrorId` (async)
- `test_all_mirrors() -> HashMap<MirrorId, Duration>` (async)
- `test_mirror_latency(mirror_id: MirrorId) -> Option<Duration>` (async, HEAD request)
- `find_best_from_results()` - respect preferred region within 2x latency tolerance
- `print_mirror_latencies()` - diagnostic output (async)
- Unit tests for `find_best_from_results`

### Step 3: Update Mirrors Module (`src/mirrors/mod.rs`)

```rust
pub mod auto_select;
pub mod registry;

pub use auto_select::{print_mirror_latencies, select_best_mirror};
pub use registry::{Mirror, MirrorId};
```

### Step 4: Update AppContext (`src/context.rs`)

- Change `new()` from sync to `pub async fn new() -> Result<Self>`
- Add auto-selection logic when `config.mirror_selection.auto_select` is true
- Priority: ENV > auto-select > config

### Step 5: Update Main (`src/main.rs`)

- Change line 40 from `AppContext::new()?` to `AppContext::new().await?`

### Step 6: Update CLI Args (`src/cli/args.rs`)

Add `TestMirrors` variant to `ConfigAction` enum:
```rust
/// Test mirror latencies
TestMirrors,
```

### Step 7: Update Config Command (`src/cli/commands/config.rs`)

1. Add import: `use crate::mirrors::print_mirror_latencies;`
2. Add `ConfigAction::TestMirrors` handler
3. Update `get_config_value()` with new keys:
   - `sync.layout`, `sync.data_types`
   - `download.retry_count`, `download.default_format`
   - `mirror_selection.auto_select`, `mirror_selection.preferred_region`, `mirror_selection.latency_cache_ttl`
4. Update `set_config_value()` with same keys

## Critical Files

| File | Action |
|------|--------|
| `src/config/schema.rs` | Modify - core schema changes |
| `src/mirrors/auto_select.rs` | Create - latency testing |
| `src/mirrors/mod.rs` | Modify - export auto_select |
| `src/context.rs` | Modify - async new(), auto-select |
| `src/main.rs` | Modify - await AppContext::new() |
| `src/cli/args.rs` | Modify - add TestMirrors |
| `src/cli/commands/config.rs` | Modify - new keys + test-mirrors |

## Dependencies

No new dependencies required:
- `tokio` already has `sync` feature for `RwLock`
- `reqwest` already available for HEAD requests
- `futures-util` already available for `join_all`

## Backward Compatibility

All new fields use `#[serde(default)]` - existing config files will work unchanged.

## Verification

```bash
# Build and test
cargo build
cargo test

# Test config commands
cargo run -- config show
cargo run -- config set sync.layout divided
cargo run -- config set download.retry_count 5
cargo run -- config set mirror_selection.auto_select true
cargo run -- config test-mirrors

# Verify old config loads (create minimal config, load it)
```

## Config Example (after implementation)

```toml
[paths]
pdb_dir = "/data/pdb"

[paths.data_type_dirs]
structures = "/data/pdb/structures"
assemblies = "/data/pdb/assemblies"

[sync]
mirror = "pdbj"
bwlimit = 10000
delete = false
layout = "divided"
data_types = ["structures", "assemblies"]

[download]
default_format = "mmcif"
auto_decompress = true
parallel = 8
retry_count = 3

[mirror_selection]
auto_select = true
preferred_region = "jp"
latency_cache_ttl = 3600
```
