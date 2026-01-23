# Phase 1: Make Config More Writable (書きやすく)

## Goal

Reduce config file verbosity and improve usability by introducing presets and nested structure while maintaining backward compatibility.

## Current Problem

Custom rsync configs are extremely verbose:
```toml
[[sync.custom]]
name = "structures"
url = "rsync.wwpdb.org::ftp_data/structures/"
dest = "data/structures"
rsync_delete = true
rsync_compress = true
rsync_checksum = true
rsync_partial = true
rsync_partial_dir = ".tmp"
rsync_max_size = "5GB"
# ... 15+ more rsync_* fields
```

**Issues:**
- `rsync_` prefix repeated 20+ times
- Hard to remember field names
- High cognitive load
- No common presets

## Solution: Combined Approach

Support three config styles:

### Style 1: Preset-only (Easiest)
```toml
[[sync.custom]]
name = "structures"
url = "rsync.wwpdb.org::ftp_data/structures/"
dest = "data/structures"
preset = "safe"  # safe, fast, minimal, conservative
```

### Style 2: Preset + overrides (Common)
```toml
[[sync.custom]]
name = "structures"
url = "rsync.wwpdb.org::ftp_data/structures/"
dest = "data/structures"
preset = "fast"

[sync.custom.options]
max_size = "5GB"
exclude = ["obsolete/"]
```

### Style 3: Fully custom (Power users)
```toml
[[sync.custom]]
name = "sifts"
url = "rsync.wwpdb.org::ftp_data/sifts/"
dest = "data/sifts"

[sync.custom.options]
delete = true
compress = true
checksum = true
timeout = 300
```

### Style 4: Old format (Backward compat)
```toml
[[sync.custom]]
name = "legacy"
url = "example.org::data"
dest = "data/legacy"
rsync_delete = true
rsync_compress = true
```

## Implementation Tasks

### 1. Schema Changes (`src/config/schema.rs`)

- [ ] Create `RsyncOptionsConfig` struct (no `rsync_` prefix)
- [ ] Add `preset: Option<String>` field to `CustomRsyncConfig`
- [ ] Add `options: Option<RsyncOptionsConfig>` field to `CustomRsyncConfig`
- [ ] Add `#[serde(alias)]` to existing fields for backward compat
  - Example: `#[serde(rename = "rsync_delete", alias = "delete")]`
- [ ] Implement `to_rsync_flags()` with merge priority: `options > preset > legacy`
- [ ] Add `RsyncOptionsConfig::to_rsync_flags()` converter

### 2. Preset System (`src/sync/presets.rs`)

- [ ] Create `RsyncPreset` enum with variants: `Safe`, `Fast`, `Minimal`, `Conservative`
- [ ] Implement `RsyncPreset::to_flags()` for each preset:
  - **Safe**: No delete, compress, checksum, partial, verbose
  - **Fast**: Delete, compress, no checksum, partial, quiet
  - **Minimal**: Bare minimum flags
  - **Conservative**: No delete, compress, checksum, partial, backup, verbose
- [ ] Add `get_rsync_preset(name: &str) -> Option<RsyncFlags>` lookup function

### 3. Merge Logic (`src/sync/flags.rs`)

- [ ] Add `RsyncFlags::merge_with(other: RsyncFlags) -> RsyncFlags` helper
- [ ] Implement field-by-field merge (other overrides self for set fields)

### 4. Migration Command (`src/cli/commands/config.rs`)

- [ ] Create new file for config subcommands
- [ ] Implement `config migrate` subcommand:
  - Detect if rsync flags match a preset → use `preset = "name"`
  - Otherwise → convert to nested `[options]` format
  - Remove `rsync_` prefixes
- [ ] Implement `config validate` to check config syntax
- [ ] Implement `config presets` to list available presets

### 5. CLI Wiring (`src/cli/mod.rs`, `src/cli/args/config.rs`)

- [ ] Add `ConfigCommand` enum with `Migrate`, `Validate`, `Presets` variants
- [ ] Wire up to main CLI

### 6. Documentation Updates

- [ ] Update `README.md`:
  - Add section showing all three config styles
  - Add preset documentation table
  - Add migration guide
  - Update examples to use new format (with backward compat note)
- [ ] Update `CHANGELOG.md` with new feature entry

### 7. Testing

- [ ] Unit tests in `src/config/schema.rs`:
  - Old format still works
  - New nested format works
  - Preset format works
  - Preset + override works
  - Priority order (options > preset > legacy)
- [ ] Integration test (`tests/config_formats.rs`):
  - Load config file with mixed formats
  - Verify CLI override behavior
- [ ] Manual testing checklist:
  - [ ] Old format config loads and works
  - [ ] Preset-only config works
  - [ ] Preset + override config works
  - [ ] Fully nested config works
  - [ ] CLI args override config values
  - [ ] `config migrate` produces valid output

## Critical Files to Modify

1. **`src/config/schema.rs`** (Major) - Add `RsyncOptionsConfig`, update `CustomRsyncConfig`, backward compat
2. **`src/sync/presets.rs`** (Medium) - Add `RsyncPreset` enum and preset lookup
3. **`src/sync/flags.rs`** (Minor) - Add `merge_with()` helper
4. **`src/cli/commands/config.rs`** (New) - Migration/validation commands
5. **`src/cli/mod.rs`** (Minor) - Wire up config subcommands
6. **`README.md`** (Major) - Examples, preset docs, migration guide
7. **`tests/config_formats.rs`** (New) - Integration tests

## Preset Definitions

| Preset | delete | compress | checksum | backup | Use Case |
|--------|--------|----------|----------|--------|----------|
| `safe` | ❌ | ✅ | ✅ | ❌ | First-time sync, cautious users |
| `fast` | ✅ | ✅ | ❌ | ❌ | Regular updates, speed priority |
| `minimal` | ❌ | ❌ | ❌ | ❌ | Bare minimum, full control needed |
| `conservative` | ❌ | ✅ | ✅ | ✅ | Production, maximum safety |

## Verification

### Unit Tests
```bash
cargo test config::schema
cargo test sync::presets
```

### Integration Tests
```bash
cargo test --test config_formats
```

### Manual Verification
```bash
# 1. Create test config with preset
cat > ~/.config/pdb-sync/test-config.toml <<EOF
[[sync.custom]]
name = "test"
url = "rsync.wwpdb.org::ftp_data/structures/"
dest = "data/test"
preset = "safe"
EOF

# 2. Verify it loads
pdb-sync config validate

# 3. Test migration
pdb-sync config migrate --dry-run

# 4. List available presets
pdb-sync config presets
```

### Backward Compatibility Check
```bash
# Use existing config with old format - should still work
pdb-sync sync --custom test
```

---
- [ ] **DONE** - Phase complete
