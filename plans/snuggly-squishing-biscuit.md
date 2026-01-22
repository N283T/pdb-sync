# Add Comprehensive rsync Options Support

## Summary

Add a curated subset of commonly-used rsync options to `CustomRsyncConfig` and CLI, with per-config defaults and CLI override capability.

## Design Decisions

1. **Curated subset** - Not all rsync options, just the most useful ones for PDB syncing
2. **Flat structure** - Direct fields like `rsync_compress`, not nested structs
3. **DRY with shared struct** - `RsyncFlags` shared between config and CLI
4. **CLI overrides config** - Clear priority: CLI args > config defaults > hardcoded defaults
5. **Full backward compatibility** - Existing configs work without changes

## Critical Files to Modify

| File | Changes |
|------|---------|
| `src/sync/flags.rs` | **NEW** - `RsyncFlags` struct with validation |
| `src/sync/mod.rs` | Export `RsyncFlags` |
| `src/sync/rsync.rs` | Use `RsyncFlags` in `RsyncOptions` and `build_rsync_command()` |
| `src/config/schema.rs` | Add `rsync_*` fields to `CustomRsyncConfig` |
| `src/cli/args/sync.rs` | Add CLI flags and `to_rsync_flags()` |
| `src/cli/commands/sync/wwpdb.rs` | Merge config + CLI flags in `run_custom()` |

## Implementation Steps

### Step 1: Create `src/sync/flags.rs`

```rust
//! Common rsync flag definitions shared between CLI and config.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RsyncFlags {
    // Existing
    pub delete: bool,
    pub bwlimit: Option<u32>,
    pub dry_run: bool,

    // New
    pub compress: bool,
    pub checksum: bool,
    pub partial: bool,
    pub partial_dir: Option<String>,
    pub max_size: Option<String>,
    pub min_size: Option<String>,
    pub timeout: Option<u32>,
    pub contimeout: Option<u32>,
    pub backup: bool,
    pub backup_dir: Option<String>,
    pub chmod: Option<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub include: Vec<String>,
    pub exclude_from: Option<String>,
    pub include_from: Option<String>,
    pub verbose: bool,
    pub quiet: bool,
    pub itemize_changes: bool,
}

impl RsyncFlags {
    pub fn validate(&self) -> Result<(), RsyncFlagError> { ... }
    pub fn merge_with_cli(&self, cli: &RsyncFlags) -> RsyncFlags { ... }
    pub fn apply_to_command(&self, cmd: &mut Command) { ... }
}
```

### Step 2: Update `src/config/schema.rs`

Add to `CustomRsyncConfig`:
```rust
// Rsync options (per-config defaults)
#[serde(rename = "rsync_delete")]
pub rsync_delete: bool,
#[serde(rename = "rsync_compress")]
pub rsync_compress: bool,
#[serde(rename = "rsync_checksum")]
pub rsync_checksum: bool,
// ... etc (all rsync_* fields)

impl CustomRsyncConfig {
    pub fn to_rsync_flags(&self) -> RsyncFlags { ... }
}
```

### Step 3: Update `src/cli/args/sync.rs`

Add CLI args:
```rust
#[arg(long, global = true)]
pub compress: bool,
#[arg(long, global = true)]
pub checksum: bool,
// ... etc

impl SyncArgs {
    pub fn to_rsync_flags(&self) -> RsyncFlags { ... }
}
```

### Step 4: Update `src/sync/rsync.rs`

```rust
pub struct RsyncOptions {
    pub mirror: MirrorId,
    pub data_types: Vec<DataType>,
    pub formats: Vec<FileFormat>,
    pub layout: Layout,
    pub flags: RsyncFlags,  // NEW: replaces individual fields
    pub filters: Vec<String>,
    pub show_progress: bool,
}

impl RsyncRunner {
    fn build_rsync_command(&self, ...) -> Command {
        // Use self.options.flags.apply_to_command()
    }
}
```

### Step 5: Update `src/cli/commands/sync/wwpdb.rs`

```rust
pub async fn run_custom(name: String, args: SyncArgs, ctx: AppContext) -> Result<()> {
    // ...
    let config_flags = custom_config.to_rsync_flags();
    let cli_flags = args.to_rsync_flags();
    let flags = config_flags.merge_with_cli(&cli_flags);
    flags.validate()?;

    // Build rsync command with flags
    let mut cmd = Command::new("rsync");
    cmd.arg("-av");
    flags.apply_to_command(&mut cmd);
    // ...
}
```

## Example Config

```toml
[[sync.custom]]
name = "emdb"
url = "data.pdbj.org::rsync/pub/emdb/"
dest = "pub/emdb"
description = "EMDB data"

# Per-config rsync defaults
rsync_delete = true
rsync_compress = true
rsync_max_size = "2G"
rsync_timeout = 600
rsync_exclude = ["*.tmp", "test/*"]
```

## Example CLI Usage

```bash
# Use config defaults
pdb-sync sync --custom emdb

# Override specific options
pdb-sync sync --custom emdb --no-compress --max-size=5G --verbose

# Combine overrides
pdb-sync sync --custom pdbe-sifts --delete --backup --backup-dir=.bak
```

## Verification

1. Test config parsing with all rsync options
2. Test CLI override behavior
3. Test backward compatibility (old configs without new fields)
4. Run `cargo test`
5. Run `cargo clippy -- -D warnings`
6. Manual sync test with various flag combinations
