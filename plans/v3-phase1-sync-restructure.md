# Phase 1: Sync Command Restructuring

## Parallel Development Note
This phase is being developed in parallel with other v3 phases.
- Create PR and request review
- Wait for review approval before merging
- Rebase on master if other phases are merged first

## Goal
Restructure `sync` command with subcommands for wwPDB standard data and mirror-specific data.

## New Command Structure

```
pdb-cli sync
  ├── [wwpdb]           # Default, can omit. --mirror selectable
  │   └── --type: structures, assemblies, structure-factors, etc.
  ├── structures        # Shortcut for: sync wwpdb --type structures
  ├── assemblies        # Shortcut for: sync wwpdb --type assemblies
  ├── pdbj              # PDBj-specific data only
  │   └── --type: emdb, pdb-ihm, derived
  └── pdbe              # PDBe-specific data only
      └── --type: sifts, pdbechem, foldseek
```

## Usage Examples

```bash
# wwPDB standard (backward compatible)
pdb-cli sync --type structures --mirror pdbj
pdb-cli sync wwpdb --type structures --mirror pdbj

# Shortcuts for common data types
pdb-cli sync structures --mirror pdbj
pdb-cli sync assemblies --mirror rcsb

# Multiple types (requires wwpdb subcommand)
pdb-cli sync wwpdb -t structures -t assemblies --mirror pdbj

# PDBj-specific
pdb-cli sync pdbj --type emdb --progress
pdb-cli sync pdbj -t pdb-ihm -t derived

# PDBe-specific
pdb-cli sync pdbe --type sifts
pdb-cli sync pdbe -t pdbechem -t foldseek
```

## Implementation Tasks

### 1. Define new data types for mirror-specific data

```rust
// src/data_types.rs - Add PDBj-specific types
pub enum PdbjDataType {
    Emdb,           // rsync://rsync.pdbj.org/emdb/
    PdbIhm,         // rsync://rsync.pdbj.org/pdb_ihm/
    Derived,        // rsync://rsync.pdbj.org/ftp_derived/
}

// PDBe-specific types
pub enum PdbeDataType {
    Sifts,          // pub/databases/msd/sifts/
    Pdbechem,       // pub/databases/msd/pdbechem_v2/
    Foldseek,       // pub/databases/msd/foldseek/
    GraphDb,        // pub/databases/msd/graphdb/
    Assemblies,     // pub/databases/msd/pdb-assemblies-analysis/
}
```

### 2. Restructure CLI args

```rust
// src/cli/args.rs
#[derive(Subcommand)]
pub enum SyncCommand {
    /// Sync wwPDB standard data (default)
    #[command(name = "wwpdb")]
    Wwpdb(WwpdbSyncArgs),

    /// Shortcut: sync structures
    Structures(StructuresSyncArgs),

    /// Shortcut: sync assemblies
    Assemblies(AssembliesSyncArgs),

    /// Sync PDBj-specific data
    Pdbj(PdbjSyncArgs),

    /// Sync PDBe-specific data
    Pdbe(PdbeSyncArgs),
}
```

### 3. Update mirror registry with new paths

```rust
// src/mirrors/registry.rs
impl Mirror {
    pub fn pdbj_rsync_url(&self, data_type: PdbjDataType) -> String { ... }
    pub fn pdbe_rsync_url(&self, data_type: PdbeDataType) -> String { ... }
}
```

### 4. Implement sync handlers for each subcommand

- `src/cli/commands/sync/wwpdb.rs`
- `src/cli/commands/sync/pdbj.rs`
- `src/cli/commands/sync/pdbe.rs`

### 5. Backward compatibility

When no subcommand is given, default to `wwpdb` behavior:
```rust
// If user runs: pdb-cli sync --type structures
// Treat as: pdb-cli sync wwpdb --type structures
```

## Files to Modify

- `src/cli/args.rs` - Add SyncCommand enum with subcommands
- `src/cli/commands/sync.rs` → `src/cli/commands/sync/mod.rs` - Restructure as module
- `src/data_types.rs` - Add PdbjDataType, PdbeDataType
- `src/mirrors/registry.rs` - Add mirror-specific URL builders
- `src/main.rs` - Update command dispatch

## Testing

- Test backward compatibility with existing sync commands
- Test each subcommand (wwpdb, pdbj, pdbe)
- Test shortcuts (structures, assemblies)
- Dry-run tests for new data types
