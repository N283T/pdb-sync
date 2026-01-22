# Dead Code Removal Plan for pdb-sync

## Summary

After the refactoring to sync-only (commit 565a7d8), several modules and functions remain that are no longer used by the current sync-only functionality. This plan identifies and safely removes dead code while preserving the public API.

## Dead Code Identified

### 1. **`src/utils/id_reader.rs` - Entire module (228 lines)**
- **Status**: Completely unused
- **Reason**: `IdSource` was used for batch ID collection in the removed download command
- **Safe to remove**: Yes - no references in the codebase

### 2. **`src/cli/commands/sync/common.rs` - Most of the file (~100 lines)**
- **Status**: Most functions are marked `#[allow(dead_code)]` and only used in tests
- **Functions to remove**:
  - `SyncResult` struct - never used outside tests
  - `print_summary()` - never called
  - `print_mirror_summary()` - never called
  - `parse_rsync_output()` - only used in tests
- **Keep**: `validate_subpath()` - IS used in `wwpdb.rs` (line 38)
- **Keep**: `human_bytes` re-export - not currently used but part of utils module

### 3. **`src/files/paths.rs` - Several unused functions**
- **Functions to remove**:
  - `FileFormat::extension()` - marked `#[allow(dead_code)]`, never used
  - `FileFormat::subdir()` - only used in `SyncResult` (dead) and tests
  - `FileFormat::is_compressed()` - only used in tests
  - `build_relative_path()` - marked `#[allow(dead_code)]`, never used
  - `build_full_path()` - marked `#[allow(dead_code)]`, never used
- **Keep**: `FileFormat::base_format()` - IS used in `mirrors/registry.rs:115`

### 4. **`src/files/pdb_id.rs` - Unused methods**
- **Methods to remove**:
  - `PdbId::is_extended()` - marked `#[allow(dead_code)]`, never used
  - `PdbId::short_form()` - marked `#[allow(dead_code)]`, never used

### 5. **`src/config/schema.rs` - Unused method**
- **Method to remove**: `PathsConfig::dir_for()` - marked `#[allow(dead_code)]`, never used
- **Note**: `data_type_dirs` HashMap field is still used for config deserialization, so keep that

## Keep (NOT removing - Public API)

### `src/data_types.rs`
- `DataType` and `Layout` are exported via `lib.rs` - part of public API
- Methods like `rsync_subpath()`, `filename_pattern()`, `description()` are public API
- Even if not used internally, these should be kept for library users

## Implementation Steps

### Phase 1: Remove id_reader module
1. Remove `src/utils/id_reader.rs`
2. Remove `mod id_reader;` and `pub use id_reader::IdSource;` from `src/utils/mod.rs`

### Phase 2: Clean up common.rs
1. Remove `SyncResult` struct
2. Remove `print_summary()`, `print_mirror_summary()`, `parse_rsync_output()` functions
3. Keep `validate_subpath()` - it IS used
4. Remove `pub use crate::utils::human_bytes;` if not needed elsewhere

### Phase 3: Clean up files/paths.rs
1. Remove `FileFormat::extension()`
2. Remove `FileFormat::subdir()`
3. Remove `FileFormat::is_compressed()`
4. Remove `build_relative_path()`
5. Remove `build_full_path()`
6. Keep `FileFormat::base_format()` - used in mirrors/registry.rs

### Phase 4: Clean up files/pdb_id.rs
1. Remove `PdbId::is_extended()`
2. Remove `PdbId::short_form()`

### Phase 5: Clean up config/schema.rs
1. Remove `PathsConfig::dir_for()` method
2. Keep `data_type_dirs` HashMap field

## Verification

After each phase, run:
```bash
cargo build --release
cargo test
cargo clippy -- -D warnings
cargo fmt --all -- --check
```

## Files to Modify

1. `src/utils/id_reader.rs` - DELETE entire file
2. `src/utils/mod.rs` - Remove id_reader references
3. `src/cli/commands/sync/common.rs` - Remove dead functions
4. `src/files/paths.rs` - Remove dead functions
5. `src/files/pdb_id.rs` - Remove dead methods
6. `src/config/schema.rs` - Remove dir_for() method

## Estimated Lines Removed

- `id_reader.rs`: ~228 lines (entire file)
- `common.rs`: ~80 lines (keeping validate_subpath)
- `paths.rs`: ~70 lines (including dead FileFormat methods)
- `pdb_id.rs`: ~20 lines
- `schema.rs`: ~10 lines

**Total: ~408 lines of dead code**
