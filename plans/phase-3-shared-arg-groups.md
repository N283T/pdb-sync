# Refactor Phase 3: Shared Argument Groups

## Overview
Use `#[command(flatten)]` for shared argument groups (GlobalArgs, PdbDirArgs) to reduce duplication across commands.

**Effort**: Short (1-2 days)

## Requirements
- Create reusable argument group structs
- Apply #[command(flatten)] to commands
- Ensure no behavioral changes
- Keep tests passing

## Architecture Changes

### File: src/cli/args/global.rs (MODIFY/CREATE)

```rust
/// Global options shared across all commands
#[derive(Parser, Clone, Debug)]
pub struct GlobalArgs {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Override the PDB directory
    #[arg(long, global = true, env = "PDB_DIR")]
    pub pdb_dir: Option<PathBuf>,
}

/// PDB directory specification (for commands that need path context)
#[derive(Parser, Clone, Debug)]
pub struct PdbDirArgs {
    /// Override the PDB directory
    #[arg(long, env = "PDB_DIR")]
    pub pdb_dir: Option<PathBuf>,

    /// Destination directory (alias for pdb_dir)
    #[arg(short, long)]
    pub dest: Option<PathBuf>,
}

/// Mirror selection arguments
#[derive(Parser, Clone, Debug)]
pub struct MirrorArgs {
    /// Mirror to use
    #[arg(short, long, value_enum)]
    pub mirror: Option<MirrorId>,
}

/// Progress/output arguments
#[derive(Parser, Clone, Debug)]
pub struct ProgressArgs {
    /// Show progress bar
    #[arg(short = 'P', long)]
    pub progress: bool,

    /// Quiet mode (suppress output)
    #[arg(short, long)]
    pub quiet: bool,
}

/// Dry run arguments
#[derive(Parser, Clone, Debug)]
pub struct DryRunArgs {
    /// Perform a dry run without making changes
    #[arg(short = 'n', long)]
    pub dry_run: bool,
}
```

### File: src/cli/args/commands.rs (MODIFY)

Apply #[command(flatten)] to relevant commands:

```rust
#[derive(Parser, Clone)]
pub struct DownloadArgs {
    // Flatten common args
    #[command(flatten)]
    pub pdb_dir: PdbDirArgs,

    #[command(flatten)]
    pub mirror: MirrorArgs,

    #[command(flatten)]
    pub progress: ProgressArgs,

    // Command-specific args
    pub pdb_ids: Vec<String>,
    pub data_type: DataType,
    // ... rest of DownloadArgs
}
```

## Implementation Steps

### Step 1: Define shared arg groups
- [ ] Add GlobalArgs struct
- [ ] Add PdbDirArgs struct
- [ ] Add MirrorArgs struct
- [ ] Add ProgressArgs struct
- [ ] Add DryRunArgs struct

### Step 2: Update command structs
- [ ] Apply flatten to DownloadArgs
- [ ] Apply flatten to SyncArgs
- [ ] Apply flatten to CopyArgs
- [ ] Apply flatten to ValidateArgs
- [ ] Apply flatten to UpdateArgs

### Step 3: Update command handlers
- [ ] Update access to `pdb_dir` (via `.pdb_dir.pdb_dir`)
- [ ] Update access to `mirror` (via `.mirror.mirror`)
- [ ] Update access to `progress` (via `.progress.progress`)

### Step 4: Update tests
- [ ] Update test assertions to use flattened paths
- [ ] Verify all clap tests still pass

## Verification

```bash
# Build and test
cargo build
cargo test

# Test help output shows options correctly
cargo run -- download --help
cargo run -- sync --help

# Test commands still work
cargo run -- download 1ABC --mirror pdbj
cargo run -- sync structures --dry-run
```

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Access path changes break handlers | Medium | Update all handlers |
| Tests fail due to path changes | Low | Update test assertions |
| Help output changes slightly | Low | Acceptable (same options) |

## Success Criteria

- [ ] All shared arg groups defined
- [ ] At least 4 commands use flattened args
- [ ] All tests pass
- [ ] Help output correct
- [ ] No clippy warnings

---
- [x] **DONE** - Phase 3 complete (2025-01-21)
