# PDB Archive Directory Structure Investigation

## Goal

Investigate and document the actual directory structure of each PDB mirror's archive to determine the correct init layout for pdb-sync.

## Mirror Archive URLs

| Mirror | Docs URL | Archive HTTPS URL |
|--------|----------|-------------------|
| RCSB | https://www.rcsb.org/docs/programmatic-access/file-download-services | https://files.rcsb.org/pub/ |
| PDBj | https://pdbj.org/info/archive | https://files.pdbj.org/pub |
| PDBe | https://www.ebi.ac.uk/pdbe/services/ftp-access | https://ftp.ebi.ac.uk/pub/databases/ |

## Investigation Results

### RCSB (files.rcsb.org)

**Archive URL**: https://files.rcsb.org/pub/

```
pub/
├── emdb/
├── pdb/
└── pdb_ihm/
```

### PDBj (files.pdbj.org)

**Archive URL**: https://files.pdbj.org/pub

```
pub/
├── emdb/
├── pdb/
└── pdb_ihm/
```

**⚠️ SAME STRUCTURE as RCSB!**

### PDBe (ftp.ebi.ac.uk)

**Archive URL**: https://ftp.ebi.ac.uk/pub/databases/

```
databases/ (EMBL-wide, hundreds of databases)
├── msd/          ← PDBe-specific (Macromolecular Structure Database)
│   ├── assemblies/
│   ├── foldseek/
│   ├── pdbechem_v2/
│   └── sifts/
├── pdb/          ← Standard PDB (same as RCSB/PDBj)
├── pdb_ihm/      ← PDB-IHM
├── emdb/         ← EMDB
├── pdb_nextgen/
└── ... (hundreds more)
```

## Key Finding: RCSB and PDBj Have IDENTICAL Structure!

Both RCSB and PDBj use the SAME top-level structure:
```
pub/
├── emdb/
├── pdb/
└── pdb_ihm/
```

Only PDBe is different (embedded in EMBL's `/pub/databases/`).

## Current pdb-sync Structure (BEFORE)

```
pdb/
├── data/       ← wwPDB common
├── pdbj/       ← PDBj-specific
├── pdbe/       ← PDBe-specific
└── local/      ← User space
```

## Full wwPDB Structure (RCSB/PDBj)

```
pub/
├── emdb/
├── pdb/
│   ├── compatible/
│   ├── data/               ← MAIN DATA DIRECTORY
│   │   ├── assemblies/
│   │   ├── biounit/
│   │   ├── bird/
│   │   ├── component-models/
│   │   ├── monomers/
│   │   ├── status/
│   │   └── structures/     ← PRIMARY STRUCTURE DATA
│   ├── derived_data/
│   ├── doc/
│   ├── holdings/
│   ├── refdata/
│   ├── software/
│   └── validation_reports/
└── pdb_ihm/
```

## Key Insight

**The actual wwPDB structure uses `pub/` as base, not `data/`!**

- RCSB: `https://files.rcsb.org/pub/`
- PDBj: `https://files.pdbj.org/pub/`
- Both have: `pub/pdb/data/structures/` as the main path

---

## Proposal: New pdb-sync Init Structure

### Option A: Mirror-First (Recommended) ⭐

**Structure:**
```
pdb/ (or user-defined base)
├── pub/                    ← wwPDB common (RCSB/PDBj)
│   ├── pdb/
│   │   └── data/
│   │       ├── structures/
│   │       ├── assemblies/
│   │       └── biounit/
│   ├── emdb/
│   └── pdb_ihm/
├── pdbe/                   ← PDBe-specific
│   └── msd/
│       ├── sifts/
│       ├── pdbechem_v2/
│       └── foldseek/
└── local/                  ← User space
```

**Rationale:**
- ✅ Matches actual RCSB/PDBj archive structure (`pub/` base)
- ✅ Clear separation: wwPDB common vs PDBe-specific
- ✅ Easy to rsync: `rsync rsync.rcsb.org::pub/pdb/data/structures/ pub/pdb/data/structures/`
- ✅ PDBe-specific data isolated in `pdbe/msd/`

---

### Option B: Simplified pub/

**Structure:**
```
pdb/
├── pub/
│   ├── pdb/
│   │   └── data/
│   │       ├── structures/
│   │       ├── assemblies/
│   │       └── biounit/
│   ├── emdb/
│   ├── pdb_ihm/
│   └── msd/               ← PDBe data (from /pub/databases/msd/)
│       ├── sifts/
│       ├── pdbechem_v2/
│       └── foldseek/
└── local/
```

**Rationale:**
- ✅ Even simpler, everything under `pub/`
- ⚠️ Mixes wwPDB and PDBe data (slightly confusing)

---

### Option C: Current (data/) + Alias

**Structure:**
```
pdb/
├── data/                   ← wwPDB common (current)
│   ├── structures/
│   ├── assemblies/
│   └── biounit/
├── pub/                    ← Symlink to data/ (for compatibility)
├── pdbe/
│   └── msd/
└── local/
```

**Rationale:**
- ✅ Maintains backward compatibility
- ⚠️ Adds complexity (symlinks)

---

## Recommendation: Option A (Mirror-First)

**Reasons:**
1. **Factual**: Matches actual archive structure (`pub/` base)
2. **Clear**: wwPDB common vs PDBe-specific is obvious
3. **Simple**: No symlinks, no confusion
4. **Future-proof**: Easy to add more mirror-specific data

## Implementation Changes

1. Change `SUBDIRS` from `["data", "pdbj", "pdbe", "local"]` to `["pub", "pdbe", "local"]`
2. Update `get_common_data_types()` to reflect `pub/pdb/data/` structure
3. Remove PDBj-specific (it's now part of `pub/`)
4. Keep PDBe-specific (`pdbe/msd/`)

## EMBL Databases Investigation

EMBL FTP (`ftp.ebi.ac.uk/pub/databases/`) contains hundreds of databases. Key ones for PDB users:

```
databases/
├── msd/              ← PDBe (Macromolecular Structure Database)
├── alphafold/        ← AlphaFold predictions
├── pdb/              ← Standard PDB (same as RCSB/PDBj)
├── pdb_ihm/          ← PDB-IHM
├── emdb/             ← EMDB
├── uniprot/          ← UniProt protein sequences
├── ensembl/          ← Genome data
├── chebi/            ← Chemical entities
├── pdbe-kb/          ← PDBe-KB
└── ... (200+ more)
```

## New Proposals Based on User Feedback

### Option 1: Simplified (msd → pdbe)

**Structure:**
```
basedir/
├── pub/              ← wwPDB common (RCSB/PDBj)
│   ├── pdb/
│   ├── pdb_ihm/
│   └── emdb/
├── pdbe/             ← Renamed from msd
│   ├── sifts/
│   ├── pdbechem_v2/
│   └── foldseek/
└── local/
```

**Rationale:**
- ✅ Simple, clear
- ✅ `pdbe/` is more user-friendly than `msd/`
- ⚠️ Loses connection to "EMBL" context

---

### Option 2: EMBL-First (Template System)

**Structure (with --template embl):**
```
basedir/
├── embl/             ← EMBL databases
│   ├── pdbe/
│   │   └── msd/
│   ├── alphafold/
│   ├── uniprot/
│   ├── ensembl/
│   └── ...
├── pdbj/             ← Optional (--template pdbj)
│   └── (PDBj-specific data)
└── local/
```

**Templates:**
- `--template wwpdb` → pub/ structure (RCSB/PDBj/PDBe common)
- `--template embl` → embl/ structure (EMBL databases)
- `--template full` → Both

**Rationale:**
- ✅ Flexible, user chooses their use case
- ✅ Clear separation: wwPDB vs EMBL
- ⚠️ More complex to implement

---

### Option 3: Hybrid (User's Latest Idea)

**Structure:**
```
basedir/
├── pub/              ← wwPDB common (RCSB/PDBj/PDBe)
│   ├── pdb/
│   ├── pdb_ihm/
│   └── emdb/
├── embl/             ← EMBL EXCEPT wwPDB duplicates
│   ├── alphafold/
│   ├── uniprot/
│   ├── ensembl/
│   ├── chebi/
│   └── ...
└── local/
```

**Rationale:**
- ✅ `pub/` = wwPDB standard (all 3 mirrors)
- ✅ `embl/` = EMBL-specific (alphafold, uniprot, etc.)
- ⚠️ `pdb_ihm` exists in both places

## Final Decision: Template System ⭐

### Template Options

| Template | Description | Creates |
|----------|-------------|---------|
| `wwpdb` | wwPDB standard (default) | `pub/` |
| `embl` | EMBL databases | `embl/` |
| `full` | Both wwPDB and EMBL | `pub/` + `embl/` |
| `minimal` | Minimal (structures only) | `pub/pdb/data/structures/` |
| `custom` | User selects specific | User choice |

### Template Structures

#### `wwpdb` (default)
```
basedir/
├── pub/
│   ├── pdb/
│   │   └── data/
│   │       ├── structures/
│   │       ├── assemblies/
│   │       └── biounit/
│   ├── pdb_ihm/
│   └── emdb/
└── local/
```

#### `embl`
```
basedir/
├── embl/
│   ├── alphafold/
│   │   └── latest/
│   ├── uniprot/
│   │   └── knowledgebase/
│   └── pdbe/
│       └── msd/
│           ├── sifts/
│           ├── pdbechem_v2/
│           └── foldseek/
└── local/
```

#### `full`
```
basedir/
├── pub/              ← wwpdb
├── embl/             ← embl
└── local/
```

### Usage Examples

```bash
# Default (wwpdb)
pdb-sync init

# EMBL databases only
pdb-sync init --template embl

# Both wwPDB and EMBL
pdb-sync init --template full

# Minimal (structures only)
pdb-sync init --template minimal

# Custom selection
pdb-sync init --template custom --only alphafold --only uniprot --only sifts
```

## EMBL Database Structures (Investigation Results)

### alphafold/
```
alphafold/
├── latest/           ← Latest predictions (v4)
├── v4/
├── v3/
├── v2/
├── v1/
├── sequences.fasta   ← All sequences (92GB!)
├── accession_ids.csv
└── README.txt
```
**Key for PDB users:** Compare AlphaFold predictions with experimental structures

### uniprot/
```
uniprot/
├── current_release/
├── knowledgebase/
├── uniref/           ← UniRef clusters (UniRef50, UniRef90, UniRef100)
├── previous_releases/
└── README
```
**Key for PDB users:** Cross-reference PDB structures with UniProt annotations

### chebi/
```
chebi/
├── Flat_file_tab_delimited/
├── SDF/              ← Structure-data files
├── chebi-2/          ← ChEBI v2 (current)
├── ontology/
├── archive/
└── README.txt
```
**Key for PDB users:** Chemical components and ligands in PDB structures

### pdbe-kb/ (not yet investigated)
**Note:** PDBe-KB API is more commonly used than FTP for this data

## EMBL Data Types for pdb-sync

Based on investigation, recommended EMBL data types for templates:

| Data Type | Directory | Description | PDB Relevance |
|-----------|-----------|-------------|----------------|
| **alphafold** | `alphafold/latest/` | AlphaFold v4 predictions | ⭐⭐⭐ Structure comparison |
| **uniprot** | `uniprot/knowledgebase/` | UniProtKB | ⭐⭐⭐ Annotations |

**Note:** `chebi` removed (less relevant for most PDB users).

## TODO

- [x] Investigate alphafold structure
- [x] Investigate uniprot structure
- [x] Investigate chebi structure
- [ ] Define EMBL data types enum
- [ ] Implement template-based init command
- [ ] Update data_types.rs with new structures
- [ ] Test all templates
