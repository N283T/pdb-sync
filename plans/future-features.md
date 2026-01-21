# Future Features

This document tracks planned features that are not yet implemented.

## Priority: Low

These are enhancement features that can be implemented when needed.

---

## 1. Extended PDB ID Support

### Status: Not Started
**Priority**: Low (wwPDB has not yet introduced extended IDs)

### Description
Support for extended PDB ID format (12 characters: `pdb_00001abc`) in addition to classic 4-character IDs.

### Implementation Outline
- Create `PdbId` enum with `Classic(String)` and `Extended(String)` variants
- Update path building logic to handle both formats
- Update URL construction for download mirrors
- Add regex validation for both formats

### Files to Modify
- `src/files/pdb_id.rs` - Add PdbId enum
- `src/files/paths.rs` - Update path building
- `src/download/https.rs` - Update URL construction

### Note
Extended PDB IDs are not yet in production by wwPDB. Implementation should wait until official specification is finalized.

---

## 2. Additional Command Aliases

### Status: Partially Implemented
**Priority**: Low (nice-to-have usability improvement)

### Already Implemented
- `download` → `dl`
- `config` → `cfg`
- `validate` → `val`

### Remaining Aliases

#### DataType Aliases
| Full Name | Proposed Aliases |
|-----------|-----------------|
| `structures` | `st`, `struct` |
| `assemblies` | `asm`, `assembly` |
| `structure-factors` | `sf`, `xray` |
| `nmr-chemical-shifts` | `nmr-cs`, `cs` |
| `nmr-restraints` | `nmr-r` |

#### Format Aliases
| Full Name | Proposed Aliases |
|-----------|-----------------|
| `mmcif` | `cif` |

#### Mirror Aliases (verify existing)
| Full Name | Aliases |
|-----------|---------|
| `rcsb` | `us` |
| `pdbj` | `jp` |
| `pdbe` | `uk`, `eu` |
| `wwpdb` | `global` |

### Files to Modify
- `src/data_types.rs` - Add DataType value aliases
- `src/files/mod.rs` - Add FileFormat aliases
- `src/mirrors/registry.rs` - Verify mirror aliases

---

## 3. aria2c Download Engine

### Status: Not Started
**Priority**: Low (built-in downloader works well)

### Description
Add aria2c as an optional download engine for faster parallel downloads with multi-connection support.

### Implementation Outline
- Create `DownloadEngine` trait
- Refactor existing downloader as `BuiltinEngine`
- Implement `Aria2cEngine` wrapper
- Add CLI args for engine selection
- Add config option for default engine

### Usage Examples
```bash
pdb-sync download 4hhb --engine aria2c
pdb-sync config set download.engine aria2c
```

### Files to Create/Modify
- `src/download/engine.rs` - DownloadEngine trait
- `src/download/builtin.rs` - Refactored built-in engine
- `src/download/aria2c.rs` - aria2c wrapper
- `src/cli/args/commands.rs` - Add engine options
- `src/config/schema.rs` - Add download.engine config

---

## 4. Storage Management Command

### Status: Partially Implemented (convert command exists)
**Priority**: Low (convert handles compression/decompression)

### Description
The `convert` command already handles compression/decompression for individual files. A future `storage` command could provide bulk collection management:

- `storage status` - Show collection compression statistics
- `storage compress` - Bulk compress all uncompressed files
- `storage decompress` - Bulk decompress all compressed files
- `storage dedupe` - Remove duplicate compressed/uncompressed pairs

### Note
This is a convenience enhancement. The same functionality can be achieved with:
```bash
# Find and compress
pdb-sync list --format pdb --uncompressed | pdb-sync convert --compress

# Find and decompress
pdb-sync list --format cif.gz | pdb-sync convert --decompress
```

---

## Completed Features

The following features from the original v3 plan have been implemented:

- ✅ Phase 1-5: CLI refactoring (args modularization, error handling, shared args, custom parsers, flag tracking)
- ✅ Phase 4 (v3): Update command
- ✅ Phase 5 (v3): Background jobs
- ✅ Phase 8 (v3): Stats command
- ✅ Phase 9 (v3): Watch command
- ✅ Phase 11 (v3): Find command
- ✅ Phase 12 (v3): Tree command
- ✅ Batch processing: stdin support in download, validate, copy, etc.
- ✅ All commands from original Japanese phase1-7 plans
