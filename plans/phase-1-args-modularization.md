# Refactor Phase 1: Args.rs Modularization

## Overview
Split the 1140+ line `src/cli/args.rs` into smaller, focused modules following uv's pattern of organizing CLI arguments.

**Effort**: Medium (3-4 days)

## Requirements
- Break args.rs into focused modules under 400 lines each
- Maintain backward compatibility for all imports
- Preserve all existing tests
- No changes to CLI behavior

## Architecture Changes

### New Directory Structure
```
src/cli/args/
├── mod.rs         # Public re-exports, parse_cli()
├── enums.rs       # ValueEnum types
├── global.rs      # GlobalArgs, shared arg groups
├── sync.rs        # Sync subcommand args
└── commands.rs    # Individual command args
```

### File: src/cli/args/mod.rs (NEW)
- Module declarations
- Public re-exports for backward compatibility
- `parse_cli()` function (moved from args.rs)

### File: src/cli/args/enums.rs (NEW)
- `OutputFormat` enum
- `SortField` enum
- `NotifyMethod` enum
- `ExperimentalMethod` enum
- All impl blocks

### File: src/cli/args/global.rs (NEW)
- `STYLES` constant
- `Cli` struct (main entry point)
- `Commands` enum
- `GlobalArgs` struct (for #[command(flatten)])
- `PdbDirArgs` struct (for #[command(flatten)])

### File: src/cli/args/sync.rs (NEW)
- `SyncFormat` enum
- `SyncArgs` struct
- `SyncCommand` enum
- `WwpdbSyncArgs`
- `ShortcutSyncArgs`
- `PdbjSyncArgs`
- `PdbeSyncArgs`

### File: src/cli/args/commands.rs (NEW)
- `InitArgs`
- `DownloadArgs`
- `CopyArgs`
- `ListArgs`
- `FindArgs`
- `ConfigArgs`, `ConfigAction`
- `EnvArgs`, `EnvAction`
- `InfoArgs`
- `ValidateArgs`
- `WatchArgs`
- `ConvertArgs`
- `StatsArgs`
- `TreeArgs`
- `UpdateArgs`
- `JobsArgs`, `JobsAction`
- `validate_resolution()` function
- `validate_organism()` function

### File: src/cli/args/tests.rs (NEW)
- All test modules from args.rs

### File: src/cli/mod.rs (MODIFY)
- Update imports from `args` module

## Implementation Steps

### Step 1: Create module structure
- [x] Create `src/cli/args/` directory
- [x] Create empty `mod.rs` with module declarations

### Step 2: Extract enums
- [x] Create `enums.rs` with all ValueEnum types
- [x] Move impl blocks for each enum
- [x] Verify compiles

### Step 3: Extract global args
- [x] Create `global.rs` with Cli, Commands, STYLES
- [x] Add GlobalArgs and PdbDirArgs structs
- [x] Move parse_cli() function

### Step 4: Extract sync args
- [x] Create `sync.rs` with all sync-related structs
- [x] Move SyncFormat enum and impl

### Step 5: Extract command args
- [x] Create `commands.rs` with individual command structs
- [x] Move validator functions

### Step 6: Extract tests
- [x] Create `tests.rs` with all test modules (N/A - no tests in args module)
- [x] Update imports to reference new modules

### Step 7: Update exports
- [x] Add `pub use` re-exports in `mod.rs`
- [x] Update `src/cli/mod.rs` if needed
- [x] Verify all external imports still work

## Verification

```bash
# Build passes
cargo build

# All tests pass
cargo test

# Clippy clean
cargo clippy -- -D warnings

# Help output unchanged
cargo run -- --help

# Test each command
cargo run -- sync --help
cargo run -- download --help
cargo run -- config --help
```

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Import path changes break consumers | High | Use `pub use` re-exports |
| Test modules broken | Medium | Update imports in tests.rs |
| Circular dependencies | Medium | Keep enums independent |

## Success Criteria

- [x] `src/cli/args.rs` removed/replaced with directory
- [x] Each file under 400 lines
- [x] All existing tests pass
- [x] CLI behavior unchanged
- [x] No clippy warnings

---
- [x] **DONE** - Phase 1 complete (2025-01-21)
