# Simplified pdb-sync Init Structure

## Goal

Simplify `pdb-sync init` directory structure to match actual wwPDB mirror layout, removing EMBL template complexity.

## Target Structure

```
basedir/
├── pub/              ← wwPDB共通 (RCSB/PDBj/PDBe)
│   ├── pdb/
│   │   └── data/
│   │       ├── structures/
│   │       ├── assemblies/
│   │       └── biounit/
│   ├── pdb_ihm/
│   └── emdb/
├── pdbj/             ← PDBj固有
│   ├── pdbjplus/
│   ├── bsma/
│   └── (他のPDBj固有ディレクトリ - 必要に応じて追加)
├── pdbe/             ← PDBe固有 (msdの中身)
│   ├── assemblies/
│   ├── foldseek/
│   ├── fragment_screening/
│   ├── graphdb/
│   ├── nmr/
│   ├── pdb-assemblies-analysis/
│   ├── pdb_uncompressed/
│   ├── pdbechem_v2/
│   ├── sifts/
│   ├── status/
│   └── updated_mmcif/
└── local/            ← ユーザースペース
```

## Changes

### 1. `src/cli/commands/init.rs`

| Item | Before | After |
|------|--------|-------|
| `SUBDIRS` | `["data", "pdbj", "pdbe", "local"]` | `["pub", "pdbj", "pdbe", "local"]` |
| Common data path | `data/structures/` | `pub/pdb/data/structures/` |
| PDBj handling | `pdbj/emdb/`, `pdbj/pdb_ihm/` | `pub/emdb/`, `pub/pdb_ihm/` + `pdbj/pdbjplus/` etc |
| Functions to update | `get_pdbj_data_types()` | Change to return PDBj-specific dirs |

**Key changes:**
- Update `SUBDIRS` constant to include `pub` and keep `pdbj`
- Update `get_common_data_types()` → handle `pub/pdb/data/` structure
- Update `get_pdbj_data_types()` → return PDBj-specific dirs (`pdbjplus`, `bsma`, etc)
- Update `get_pdbe_data_types()` → return all PDBe-specific dirs:
  - `assemblies`, `foldseek`, `fragment_screening`, `graphdb`, `nmr`,
  - `pdb-assemblies-analysis`, `pdb_uncompressed`, `pdbechem_v2`,
  - `sifts`, `status`, `updated_mmcif`
- Update `build_directory_tree()` to handle:
  - `pub/pdb/data/` → structures, assemblies, biounit, obsolete
  - `pub/pdb_ihm/` → PDB-IHM data
  - `pub/emdb/` → EMDB data
  - `pdbj/` → PDBj-specific (`pdbjplus`, `bsma`, etc)
  - `pdbe/` → PDBe-specific (all dirs above)

### 2. `src/data_types.rs`

| Item | Before | After |
|------|--------|-------|
| `Template` enum | `Wwpdb, Embl, Full, Minimal, Custom` | `Wwpdb` only (or remove entirely) |
| `EmblDataType` | `Alphafold, Uniprot` | Remove |

**Key changes:**
- Simplify `Template` enum (keep `Wwpdb` as default only)
- Remove `EmblDataType` enum and all related code
- Keep `DataType`, `PdbjDataType`, `PdbeDataType` as-is

### 3. Tests

Update tests in:
- `src/cli/commands/init.rs` tests
- `src/data_types.rs` tests

## Verification

1. Create test directory: `pdb-sync init --dir /tmp/test-pdb-sync`
2. Verify structure:
   ```
   /tmp/test-pdb-sync/
   ├── pub/
   │   ├── pdb/
   │   ├── pdb_ihm/
   │   └── emdb/
   ├── pdbj/
   │   ├── pdbjplus/
   │   ├── bsma/
   │   └── ... (other PDBj-specific)
   ├── pdbe/
   │   ├── assemblies/
   │   ├── foldseek/
   │   ├── fragment_screening/
   │   ├── graphdb/
   │   ├── nmr/
   │   ├── pdb-assemblies-analysis/
   │   ├── pdb_uncompressed/
   │   ├── pdbechem_v2/
   │   ├── sifts/
   │   ├── status/
   │   └── updated_mmcif/
   └── local/
   ```
3. Run tests: `cargo test`

## Notes

- EMBL template complexity removed (user decision: "過剰だし")
- `pub/` matches actual RCSB/PDBj archive structure
- `pub/` includes: pdb/, pdb_ihm/, emdb/ (wwPDB common)
- `pdbj/` includes PDBj-specific: pdbjplus/, bsma/, etc
- `pdbe/` is more user-friendly than `msd/`
- `local/` remains empty for user space
