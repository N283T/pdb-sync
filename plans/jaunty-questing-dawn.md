# Rename Project: pdb-cli → pdb-sync

## Overview
Rename the project from `pdb-cli` to `pdb-sync` to better reflect its primary purpose (synchronization features) and provide a more intuitive name.

## Files to Modify

### 1. Core Configuration
| File | Change |
|------|--------|
| `Cargo.toml` | `name = "pdb-sync"` |
| `src/lib.rs` | Update crate documentation comment |

### 2. CLI Command Name
| File | Change |
|------|--------|
| `src/cli/args.rs:85` | `#[command(name = "pdb-sync")]` |

### 3. Library Imports
| File | Change |
|------|--------|
| `src/main.rs:21` | `pub use pdb_sync::data_types;` |

### 4. Error Types
| File | Change |
|------|--------|
| `src/error.rs` | `PdbCliError` → `PdbSyncError` |
| All files using `PdbCliError` | Update to `PdbSyncError` |

### 5. Environment Variables
| Current | New |
|---------|-----|
| `PDB_CLI_CONFIG` | `PDB_SYNC_CONFIG` |
| `PDB_CLI_MIRROR` | `PDB_SYNC_MIRROR` |

Files to update:
- `src/context.rs`
- `src/config/loader.rs`
- `src/cli/commands/env.rs`

### 6. Documentation
| File | Changes |
|------|---------|
| `README.md` | Replace all `pdb-cli` references with `pdb-sync` |
| `README.ja.md` | Replace all `pdb-cli` references with `pdb-sync` |

## Implementation Steps

1. **Update Cargo.toml** - Change package name
2. **Update lib.rs** - Update crate documentation
3. **Update cli/args.rs** - Change CLI command name
4. **Update main.rs** - Update library import
5. **Update error.rs** - Rename error enum
6. **Update all error references** - Replace `PdbCliError` with `PdbSyncError`
7. **Update environment variable names** - Change `PDB_CLI_*` to `PDB_SYNC_*`
8. **Update README.md** - Replace project name references
9. **Update README.ja.md** - Replace project name references

## Verification

After changes:
1. Build the project: `cargo build`
2. Run tests: `cargo test`
3. Verify CLI help: `cargo run -- --help` (should show `pdb-sync`)
4. Test environment variable behavior
5. Verify documentation accuracy

## Notes

- This is a breaking change for users who have configured `PDB_CLI_*` environment variables
- The binary name will change from `pdb-cli` to `pdb-sync`
- Consider adding a migration note or backward compatibility if needed
