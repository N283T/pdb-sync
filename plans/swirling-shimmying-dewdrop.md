# Phase 1: Sync Command Restructuring - Implementation Plan

## Overview

Restructure the `sync` command to support subcommands for wwPDB standard data and mirror-specific data (PDBj, PDBe).

## New Command Structure

```
pdb-cli sync
  ├── [wwpdb]           # Default subcommand (can omit)
  │   └── --type: structures, assemblies, structure-factors, etc.
  ├── structures        # Shortcut: sync wwpdb --type structures
  ├── assemblies        # Shortcut: sync wwpdb --type assemblies
  ├── pdbj              # PDBj-specific data only
  │   └── --type: emdb, pdb-ihm, derived
  └── pdbe              # PDBe-specific data only
      └── --type: sifts, pdbechem, foldseek
```

## Implementation Steps

### Step 1: Add Mirror-Specific Data Types

**File: `src/data_types.rs`**

Add two new enums for mirror-specific data:

```rust
/// PDBj-specific data types (available only from PDBj mirror)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum, Serialize, Deserialize)]
pub enum PdbjDataType {
    /// EMDB (EM Data Bank) - rsync://rsync.pdbj.org/emdb/
    Emdb,
    /// PDB-IHM (Integrative/Hybrid Methods) - rsync://rsync.pdbj.org/pdb_ihm/
    PdbIhm,
    /// Derived data - rsync://rsync.pdbj.org/ftp_derived/
    Derived,
}

/// PDBe-specific data types (available only from PDBe mirror)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum, Serialize, Deserialize)]
pub enum PdbeDataType {
    /// SIFTS (Structure Integration with Function, Taxonomy and Sequences)
    Sifts,
    /// PDBeChem v2 (chemical component dictionary)
    Pdbechem,
    /// Foldseek database
    Foldseek,
}
```

### Step 2: Update Mirror Registry

**File: `src/mirrors/registry.rs`**

Add methods to build mirror-specific rsync URLs:

```rust
impl Mirror {
    /// Get PDBj-specific rsync URL (only valid for PDBj mirror)
    pub fn pdbj_rsync_url(&self, data_type: PdbjDataType) -> Option<String> {
        if self.id != MirrorId::Pdbj {
            return None;
        }
        let module = match data_type {
            PdbjDataType::Emdb => "emdb",
            PdbjDataType::PdbIhm => "pdb_ihm",
            PdbjDataType::Derived => "ftp_derived",
        };
        Some(format!("rsync://{}/{}/", self.rsync_host.trim_start_matches("rsync://"), module))
    }

    /// Get PDBe-specific rsync URL (only valid for PDBe mirror)
    pub fn pdbe_rsync_url(&self, data_type: PdbeDataType) -> Option<String> {
        if self.id != MirrorId::Pdbe {
            return None;
        }
        let path = match data_type {
            PdbeDataType::Sifts => "pub/databases/msd/sifts/",
            PdbeDataType::Pdbechem => "pub/databases/msd/pdbechem_v2/",
            PdbeDataType::Foldseek => "pub/databases/msd/foldseek/",
        };
        Some(format!("rsync://{}/{}", self.rsync_host.trim_start_matches("rsync://"), path))
    }
}
```

### Step 3: Restructure CLI Arguments

**File: `src/cli/args.rs`**

Replace flat `SyncArgs` with `SyncCommand` enum:

```rust
#[derive(Parser)]
pub struct SyncArgs {
    #[command(subcommand)]
    pub command: Option<SyncCommand>,

    // Shared options (for backward compatibility when no subcommand)
    #[arg(short, long, value_enum)]
    pub mirror: Option<MirrorId>,
    #[arg(short = 't', long = "type", value_enum)]
    pub data_types: Vec<DataType>,
    // ... other common options
}

#[derive(Subcommand)]
pub enum SyncCommand {
    /// Sync wwPDB standard data (default)
    Wwpdb(WwpdbSyncArgs),
    /// Shortcut: sync structures
    Structures(ShortcutSyncArgs),
    /// Shortcut: sync assemblies
    Assemblies(ShortcutSyncArgs),
    /// Sync PDBj-specific data
    Pdbj(PdbjSyncArgs),
    /// Sync PDBe-specific data
    Pdbe(PdbeSyncArgs),
}

#[derive(Parser)]
pub struct WwpdbSyncArgs {
    #[arg(short, long, value_enum)]
    pub mirror: Option<MirrorId>,
    #[arg(short = 't', long = "type", value_enum)]
    pub data_types: Vec<DataType>,
    // ... other options
}

#[derive(Parser)]
pub struct ShortcutSyncArgs {
    #[arg(short, long, value_enum)]
    pub mirror: Option<MirrorId>,
    // ... common options (no --type needed)
}

#[derive(Parser)]
pub struct PdbjSyncArgs {
    #[arg(short = 't', long = "type", value_enum, required = true)]
    pub data_types: Vec<PdbjDataType>,
    // ... other options
}

#[derive(Parser)]
pub struct PdbeSyncArgs {
    #[arg(short = 't', long = "type", value_enum, required = true)]
    pub data_types: Vec<PdbeDataType>,
    // ... other options
}
```

### Step 4: Convert Sync Command to Module

**Directory: `src/cli/commands/sync/`**

Convert single file to module structure:

```
src/cli/commands/sync/
├── mod.rs          # Entry point, routes to subcommands
├── wwpdb.rs        # wwPDB standard sync handler
├── pdbj.rs         # PDBj-specific sync handler
├── pdbe.rs         # PDBe-specific sync handler
└── common.rs       # Shared utilities (print_summary, human_bytes)
```

**`src/cli/commands/sync/mod.rs`:**
```rust
mod common;
mod pdbj;
mod pdbe;
mod wwpdb;

pub async fn run_sync(args: SyncArgs, ctx: AppContext) -> Result<()> {
    match args.command {
        Some(SyncCommand::Wwpdb(sub)) => wwpdb::run(sub, ctx).await,
        Some(SyncCommand::Structures(sub)) => wwpdb::run_structures(sub, ctx).await,
        Some(SyncCommand::Assemblies(sub)) => wwpdb::run_assemblies(sub, ctx).await,
        Some(SyncCommand::Pdbj(sub)) => pdbj::run(sub, ctx).await,
        Some(SyncCommand::Pdbe(sub)) => pdbe::run(sub, ctx).await,
        None => {
            // Backward compatibility: treat as wwpdb
            wwpdb::run_legacy(args, ctx).await
        }
    }
}
```

### Step 5: Update Main Entry Point

**File: `src/main.rs`**

No changes needed - `Commands::Sync(args)` still works, just with restructured `SyncArgs`.

### Step 6: Update Exports

**File: `src/cli/commands/mod.rs`**

Change from:
```rust
pub mod sync;
pub use sync::run_sync;
```

To (no change needed - module path stays the same).

## Files to Modify

| File | Change |
|------|--------|
| `src/data_types.rs` | Add `PdbjDataType`, `PdbeDataType` enums |
| `src/mirrors/registry.rs` | Add `pdbj_rsync_url()`, `pdbe_rsync_url()` methods |
| `src/cli/args.rs` | Restructure `SyncArgs`, add `SyncCommand` enum |
| `src/cli/commands/sync.rs` | Convert to `sync/mod.rs` module directory |
| `src/cli/commands/mod.rs` | Update if needed |

## New Files

| File | Purpose |
|------|---------|
| `src/cli/commands/sync/mod.rs` | Entry point, subcommand routing |
| `src/cli/commands/sync/wwpdb.rs` | wwPDB standard sync handler |
| `src/cli/commands/sync/pdbj.rs` | PDBj-specific sync handler |
| `src/cli/commands/sync/pdbe.rs` | PDBe-specific sync handler |
| `src/cli/commands/sync/common.rs` | Shared utilities |

## Test Plan

### Unit Tests

1. **Data type tests** (`src/data_types.rs`)
   - `PdbjDataType` rsync path generation
   - `PdbeDataType` rsync path generation
   - Serialization/deserialization

2. **Mirror registry tests** (`src/mirrors/registry.rs`)
   - `pdbj_rsync_url()` returns correct URLs for PDBj
   - `pdbj_rsync_url()` returns `None` for other mirrors
   - `pdbe_rsync_url()` returns correct URLs for PDBe
   - `pdbe_rsync_url()` returns `None` for other mirrors

3. **CLI argument parsing tests**
   - Parse `sync wwpdb --type structures`
   - Parse `sync structures` (shortcut)
   - Parse `sync pdbj --type emdb`
   - Parse `sync pdbe --type sifts`
   - Parse `sync --type structures` (backward compat)

### Integration Tests

1. **Backward compatibility**
   ```bash
   pdb-cli sync --type structures --mirror pdbj --dry-run
   ```
   Should work exactly as before.

2. **New subcommands (dry-run)**
   ```bash
   pdb-cli sync wwpdb --type structures --mirror pdbj --dry-run
   pdb-cli sync structures --mirror pdbj --dry-run
   pdb-cli sync pdbj --type emdb --dry-run
   pdb-cli sync pdbe --type sifts --dry-run
   ```

### Verification Commands

```bash
# Quality checks
cargo fmt --check
cargo clippy -- -D warnings
cargo test

# Manual dry-run tests
cargo run -- sync --type structures --dry-run
cargo run -- sync wwpdb --type structures --dry-run
cargo run -- sync structures --dry-run
cargo run -- sync pdbj --type emdb --dry-run
cargo run -- sync pdbe --type sifts --dry-run
```

## Implementation Order (TDD)

1. **Red**: Write tests for new data types
2. **Green**: Implement `PdbjDataType`, `PdbeDataType` in `src/data_types.rs`
3. **Red**: Write tests for mirror URL builders
4. **Green**: Add `pdbj_rsync_url()`, `pdbe_rsync_url()` in `src/mirrors/registry.rs`
5. **Red**: Write tests for CLI argument parsing
6. **Green**: Restructure `SyncArgs` and add `SyncCommand` in `src/cli/args.rs`
7. **Refactor**: Convert `sync.rs` to `sync/` module directory
8. **Green**: Implement subcommand handlers
9. **Verify**: Run all tests and dry-run commands
