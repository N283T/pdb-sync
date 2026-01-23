# Add sync.defaults and Clean Up SyncConfig

## Goal

1. **Add `sync.defaults`** - Global default rsync options for all custom configs (DRY)
2. **Remove unused SyncConfig fields** - Clean up `bwlimit`, `delete`, `layout`, `data_types`
3. **Update priority chain** - From `legacy → preset → options` to `legacy → defaults → preset → options`
4. **Update documentation** - De-emphasize presets, emphasize full custom options

## Current State

### SyncConfig Fields (src/config/schema.rs:377-406)

```rust
pub struct SyncConfig {
    pub mirror: MirrorId,              // ✅ USED (context.rs)
    pub bwlimit: u32,                  // ❌ UNUSED (tests only)
    pub delete: bool,                  // ❌ UNUSED (tests only)
    pub layout: Layout,                // ❌ UNUSED (tests only)
    pub data_types: Vec<String>,       // ❌ UNUSED (tests only)
    pub custom: HashMap<String, CustomRsyncConfig>,  // ✅ USED
}
```

### Current Priority Chain

```
legacy fields → preset → options → CLI args
```

## Desired State

### New SyncConfig

```rust
pub struct SyncConfig {
    pub mirror: MirrorId,              // KEEP: Used in context.rs
    pub defaults: Option<RsyncOptionsConfig>,  // NEW: Global defaults
    pub custom: HashMap<String, CustomRsyncConfig>,  // KEEP
    // REMOVE: bwlimit, delete, layout, data_types
}
```

### New Priority Chain

```
legacy fields → defaults → preset → options → CLI args
```

### Example Configuration

```toml
# Global defaults (NEW - DRY configuration)
[sync.defaults]
compress = true
timeout = 300
partial = true

# Custom configs
[sync.custom.structures]
url = "rsync.wwpdb.org::ftp_data/structures/"
dest = "data/structures"

# Full custom (MAIN USAGE - emphasized in docs)
[sync.custom.structures.options]
delete = true
max_size = "10G"
exclude = ["obsolete/"]

[sync.custom.emdb]
url = "data.pdbj.org::rsync/pub/emdb/"
dest = "data/emdb"
preset = "safe"  # Preset still available (but de-emphasized)
```

## Implementation Tasks

### Phase 1: Schema Changes

#### 1.1 Update SyncConfig (`src/config/schema.rs:377-406`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncConfig {
    #[serde(with = "mirror_id_serde")]
    pub mirror: MirrorId,

    // NEW: Global defaults
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defaults: Option<RsyncOptionsConfig>,

    #[serde(default)]
    pub custom: HashMap<String, CustomRsyncConfig>,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            mirror: MirrorId::Rcsb,
            defaults: None,
            custom: HashMap::new(),
        }
    }
}
```

**Remove**: `bwlimit`, `delete`, `layout`, `data_types`, `default_data_types()`

#### 1.2 Update CustomRsyncConfig::to_rsync_flags() (`src/config/schema.rs:262-372`)

**Current signature**:
```rust
pub fn to_rsync_flags(&self) -> crate::sync::RsyncFlags
```

**New signature**:
```rust
pub fn to_rsync_flags(&self, global_defaults: Option<&RsyncOptionsConfig>) -> crate::sync::RsyncFlags
```

**Implementation** (insert after line 296, before preset merge):

```rust
// Apply global defaults if specified (NEW)
if let Some(defaults) = global_defaults {
    if let Some(delete) = defaults.delete {
        flags.delete = delete;
    }
    if let Some(compress) = defaults.compress {
        flags.compress = compress;
    }
    if let Some(checksum) = defaults.checksum {
        flags.checksum = checksum;
    }
    if let Some(size_only) = defaults.size_only {
        flags.size_only = size_only;
    }
    if let Some(ignore_times) = defaults.ignore_times {
        flags.ignore_times = ignore_times;
    }
    if defaults.modify_window.is_some() {
        flags.modify_window = defaults.modify_window;
    }
    if let Some(partial) = defaults.partial {
        flags.partial = partial;
    }
    if defaults.partial_dir.is_some() {
        flags.partial_dir = defaults.partial_dir.clone();
    }
    if defaults.max_size.is_some() {
        flags.max_size = defaults.max_size.clone();
    }
    if defaults.min_size.is_some() {
        flags.min_size = defaults.min_size.clone();
    }
    if defaults.timeout.is_some() {
        flags.timeout = defaults.timeout;
    }
    if defaults.contimeout.is_some() {
        flags.contimeout = defaults.contimeout;
    }
    if let Some(backup) = defaults.backup {
        flags.backup = backup;
    }
    if defaults.backup_dir.is_some() {
        flags.backup_dir = defaults.backup_dir.clone();
    }
    if defaults.chmod.is_some() {
        flags.chmod = defaults.chmod.clone();
    }
    if !defaults.exclude.is_empty() {
        flags.exclude = defaults.exclude.clone();
    }
    if !defaults.include.is_empty() {
        flags.include = defaults.include.clone();
    }
    if defaults.exclude_from.is_some() {
        flags.exclude_from = defaults.exclude_from.clone();
    }
    if defaults.include_from.is_some() {
        flags.include_from = defaults.include_from.clone();
    }
    if let Some(verbose) = defaults.verbose {
        flags.verbose = verbose;
    }
    if let Some(quiet) = defaults.quiet {
        flags.quiet = quiet;
    }
    if let Some(itemize_changes) = defaults.itemize_changes {
        flags.itemize_changes = itemize_changes;
    }
}
```

#### 1.3 Update Call Sites

Find and update all `to_rsync_flags()` calls:

```bash
rg "to_rsync_flags\(\)" src/ -t rust
```

**Pattern**:
```rust
// Before:
let flags = custom_config.to_rsync_flags();

// After:
let flags = custom_config.to_rsync_flags(ctx.config.sync.defaults.as_ref());
```

**Expected files**:
- `src/cli/commands/sync/wwpdb.rs`
- Possibly others from grep results

### Phase 2: Test Updates

#### 2.1 Remove Tests for Deleted Fields (`src/config/schema.rs`)

**Line 461** - `test_default_config`:
- Remove: `assert_eq!(config.sync.layout, Layout::Divided);`

**Lines 474-498** - `test_config_deserialization`:
- Remove from TOML: `bwlimit = 1000`, `layout = "all"`
- Remove assertions for these fields

**Lines 500-515** - `test_backward_compatibility`:
- Remove from TOML: `bwlimit = 1000`
- Remove: `assert_eq!(config.sync.layout, Layout::Divided);`

**Lines 518-521** - `test_default_data_types`:
- **DELETE ENTIRE TEST** - field removed

#### 2.2 Add New Tests for sync.defaults

Add after line 723:

```rust
#[test]
fn test_sync_defaults_basic() {
    let toml_str = r#"
        [sync.defaults]
        delete = true
        compress = true
        timeout = 300

        [sync.custom.test]
        url = "example.org::data"
        dest = "data/test"
    "#;
    let config: Config = toml::from_str(toml_str).unwrap();

    let defaults = config.sync.defaults.as_ref().unwrap();
    assert_eq!(defaults.delete, Some(true));
    assert_eq!(defaults.compress, Some(true));
    assert_eq!(defaults.timeout, Some(300));

    let custom = config.sync.custom.get("test").unwrap();
    let flags = custom.to_rsync_flags(config.sync.defaults.as_ref());
    assert!(flags.delete);
    assert!(flags.compress);
    assert_eq!(flags.timeout, Some(300));
}

#[test]
fn test_sync_defaults_with_preset() {
    // Test priority: preset > defaults
    let toml_str = r#"
        [sync.defaults]
        delete = true
        compress = true

        [sync.custom.test]
        url = "example.org::data"
        dest = "data/test"
        preset = "safe"
    "#;
    let config: Config = toml::from_str(toml_str).unwrap();
    let custom = config.sync.custom.get("test").unwrap();
    let flags = custom.to_rsync_flags(config.sync.defaults.as_ref());

    assert!(!flags.delete);  // safe preset overrides defaults
    assert!(flags.compress);
}

#[test]
fn test_sync_defaults_with_options() {
    // Test priority: options > preset > defaults
    let toml_str = r#"
        [sync.defaults]
        delete = true
        compress = true

        [sync.custom.test]
        url = "example.org::data"
        dest = "data/test"
        preset = "safe"

        [sync.custom.test.options]
        delete = true
        max_size = "1G"
    "#;
    let config: Config = toml::from_str(toml_str).unwrap();
    let custom = config.sync.custom.get("test").unwrap();
    let flags = custom.to_rsync_flags(config.sync.defaults.as_ref());

    assert!(flags.delete);  // options override
    assert!(flags.compress);
    assert_eq!(flags.max_size, Some("1G".to_string()));
}

#[test]
fn test_sync_defaults_priority_chain() {
    // Full chain: legacy → defaults → preset → options
    let toml_str = r#"
        [sync.defaults]
        compress = true
        timeout = 300

        [sync.custom.test]
        url = "example.org::data"
        dest = "data/test"
        rsync_delete = false
        rsync_verbose = true
        preset = "fast"

        [sync.custom.test.options]
        timeout = 600
    "#;
    let config: Config = toml::from_str(toml_str).unwrap();
    let custom = config.sync.custom.get("test").unwrap();
    let flags = custom.to_rsync_flags(config.sync.defaults.as_ref());

    assert!(flags.delete);  // preset > defaults > legacy
    assert!(flags.compress);
    assert!(!flags.verbose);  // preset quiet overrides legacy verbose
    assert_eq!(flags.timeout, Some(600));  // options > defaults
}

#[test]
fn test_sync_defaults_none() {
    // Backward compat: configs without defaults work
    let toml_str = r#"
        [sync.custom.test]
        url = "example.org::data"
        dest = "data/test"

        [sync.custom.test.options]
        delete = true
    "#;
    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(config.sync.defaults.is_none());

    let custom = config.sync.custom.get("test").unwrap();
    let flags = custom.to_rsync_flags(None);
    assert!(flags.delete);
}
```

#### 2.3 Update Existing Priority Test

**Line 673-705** - `test_custom_config_priority_order`:

Update to test 4-level priority chain with defaults:

```rust
#[test]
fn test_custom_config_priority_order() {
    let toml_str = r#"
        [sync.defaults]
        compress = true
        checksum = false

        [sync.custom.test]
        url = "example.org::data"
        dest = "data/test"
        rsync_delete = false
        rsync_compress = false
        preset = "fast"

        [sync.custom.test.options]
        delete = false
        max_size = "1G"
    "#;
    let config: Config = toml::from_str(toml_str).unwrap();
    let custom = config.sync.custom.get("test").unwrap();
    let flags = custom.to_rsync_flags(config.sync.defaults.as_ref());

    assert!(!flags.delete);  // options override
    assert!(flags.compress);  // preset > defaults > legacy
    assert_eq!(flags.max_size, Some("1G".to_string()));
}
```

### Phase 3: Documentation Updates

#### 3.1 README.md

**Quick Start section** (lines 27-50):

Update example to show `sync.defaults` and emphasize `options`:

```toml
[sync]
mirror = "rcsb"

# Global defaults for all custom configs
[sync.defaults]
compress = true
timeout = 300
partial = true

[sync.custom.structures]
url = "rsync.wwpdb.org::ftp_data/structures/divided/mmCIF/"
dest = "data/structures/divided/mmCIF"
description = "PDB structures (mmCIF format, divided layout)"

# Full custom options (RECOMMENDED)
[sync.custom.structures.options]
delete = true
max_size = "10G"

[sync.custom.emdb]
url = "data.pdbj.org::rsync/pub/emdb/"
dest = "data/emdb"
description = "EMDB"

[sync.custom.emdb.options]
max_size = "5G"
exclude = ["obsolete/"]
```

**Remove sections**: Any mention of `sync.bwlimit`, `sync.delete`, `sync.layout`, `sync.data_types`

#### 3.2 README.ja.md

Same changes as README.md in Japanese.

#### 3.3 docs/config-reference.md

**Basic Structure section** (line 33-55):

Remove: `bwlimit`, `delete`, `layout`, `data_types`
Add: `defaults`

```toml
[paths]
pdb_dir = "/data/pdb"

[sync]
mirror = "rcsb"

# Global defaults for all custom configs
[sync.defaults]
compress = true
timeout = 300

[sync.custom.structures]
url = "rsync.wwpdb.org::ftp_data/structures/"
dest = "data/structures"

[sync.custom.structures.options]
delete = true
max_size = "10G"
```

**sync Section** (around line 88-165):

**REMOVE subsections**:
- `bwlimit`
- `delete`
- `layout`
- `data_types`

**ADD new subsection** after `mirror`:

```markdown
### `defaults`

**Type**: Table (RsyncOptionsConfig)
**Default**: None
**Description**: Global default rsync options for all custom configs

Set common options once instead of repeating in every config.

```toml
[sync.defaults]
compress = true
timeout = 300
partial = true
```

**Priority**: `options > preset > defaults > legacy`

All fields are optional. See [sync.custom.NAME.options](#synccustomnameoptions-section) for available fields.
```

**Priority Rules section** (around line 630-683):

Update from 3-level to 4-level priority chain:

```markdown
## Priority Rules

When multiple configuration sources specify the same option:

```
options > preset > defaults > legacy fields
```

**Example**:

```toml
[sync.defaults]
compress = true
timeout = 300

[sync.custom.test]
url = "example.org::data"
dest = "data/test"
rsync_delete = false  # legacy
preset = "fast"       # preset: delete=true

[sync.custom.test.options]
delete = false  # options: highest priority
timeout = 600   # override defaults
```

**Result**:
- `delete = false` (options)
- `compress = true` (preset/defaults)
- `timeout = 600` (options > defaults)
```

**Configuration Examples section** (around line 684-827):

Update all examples to:
1. Show `sync.defaults` usage
2. Emphasize `options` over `preset`

#### 3.4 docs/config-reference.ja.md

Same changes as English version in Japanese.

#### 3.5 Module Docstring (`src/config/schema.rs:1-44`)

Update to show `sync.defaults` in examples:

```rust
//! Configuration schema for pdb-sync.
//!
//! Priority order: options > preset > defaults > legacy > built-in defaults
//!
//! # Examples
//!
//! ## Global Defaults + Options (RECOMMENDED)
//!
//! ```toml
//! [sync.defaults]
//! compress = true
//! timeout = 300
//!
//! [sync.custom.structures]
//! url = "rsync.wwpdb.org::ftp_data/structures/"
//! dest = "data/structures"
//!
//! [sync.custom.structures.options]
//! delete = true
//! max_size = "10G"
//! ```
```

### Phase 4: Migration Support (Optional)

#### 4.1 Add Warnings for Removed Fields (`src/cli/commands/config.rs`)

In `migrate` or `validate` command, detect removed fields:

```rust
// Warn about removed fields (if found in TOML)
// This requires manual TOML parsing to detect unknown fields
// Optional - can skip if not worth the complexity
```

## Critical Files

1. **`src/config/schema.rs`** - Remove 4 fields, add `defaults`, update method, add 6 tests
2. **`src/cli/commands/sync/wwpdb.rs`** - Update `to_rsync_flags()` call sites
3. **`README.md`** - Update examples, remove old fields
4. **`README.ja.md`** - Japanese version of README updates
5. **`docs/config-reference.md`** - Full documentation update
6. **`docs/config-reference.ja.md`** - Japanese version of docs

## Verification

### Build and Test

```bash
# Format
cargo fmt

# Lint
cargo clippy -- -D warnings

# Test
cargo test

# Specific tests
cargo test config::schema::tests
```

### Manual Testing

```bash
# Create test config
cat > ~/.config/pdb-sync/test-config.toml <<'EOF'
[sync.defaults]
compress = true
timeout = 300

[sync.custom.test]
url = "rsync.wwpdb.org::ftp_data/structures/"
dest = "data/test"

[sync.custom.test.options]
delete = true
EOF

# Verify it loads
pdb-sync sync --list

# Dry run
pdb-sync sync test --dry-run
```

### Test Coverage

- [ ] SyncConfig fields removed (bwlimit, delete, layout, data_types)
- [ ] `sync.defaults` field added
- [ ] `sync.defaults` serializes/deserializes correctly
- [ ] Priority chain works: legacy → defaults → preset → options
- [ ] Backward compat: configs without defaults still work
- [ ] All existing tests pass (with updates)
- [ ] 6 new tests added for defaults
- [ ] Documentation updated (4 files)

## Migration Path for Users

| Old | New |
|-----|-----|
| `sync.bwlimit = 5000` | Use CLI: `--bwlimit 5000` |
| `sync.delete = true` | `sync.defaults.delete = true` or per-config `options.delete` |
| `sync.layout = "divided"` | Not needed (removed) |
| `sync.data_types = [...]` | Define explicit custom configs |

---
- [x] **DONE** - Implementation complete
