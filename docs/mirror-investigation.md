# PDB Mirror Investigation

Investigation date: 2026-01-17 (Updated)

## Archive Statistics

| Metric | Value |
|--------|-------|
| Total PDB Entries | ~248,000 structures |
| Computed Structure Models | ~1,068,577 |
| Total Files | ~3,000,000+ |
| Total Archive Size | ~1,086 GB (1 TB+) |
| Weekly Releases | ~275 new structures |
| Growth Rate | Doubling every 6-8 years |

Source: [RCSB PDB Statistics](https://www.rcsb.org/stats/)

## Directory Structure (wwPDB Standard)

All mirrors follow the wwPDB standard structure:

```
ftp_data/
├── structures/
│   ├── all/                    # Flat structure (all files in one dir)
│   │   ├── mmCIF/              # {id}.cif.gz
│   │   ├── pdb/                # pdb{id}.ent.gz
│   │   ├── XML/
│   │   ├── XML-noatom/
│   │   ├── XML-extatom/
│   │   ├── structure_factors/  # r{id}sf.ent.gz
│   │   ├── nmr_chemical_shifts/
│   │   ├── nmr_data/
│   │   ├── nmr_restraints/
│   │   └── nmr_restraints_v2/
│   ├── divided/                # Divided by middle 2 chars of PDB ID
│   │   ├── mmCIF/{hash}/       # {id}.cif.gz
│   │   ├── pdb/{hash}/         # pdb{id}.ent.gz
│   │   └── ...
│   ├── models/                 # Computed structure models
│   │   ├── current/pdb/
│   │   ├── obsolete/
│   │   └── index/
│   └── obsolete/               # Obsolete entries (same structure as divided)
├── assemblies/
│   └── mmCIF/
│       ├── all/
│       └── divided/{hash}/     # {id}-assembly{N}.cif.gz
├── biounit/
│   └── coordinates/
│       ├── all/
│       └── divided/{hash}/     # {id}.pdb{N}.gz (legacy PDB format)
├── bird/
├── component-models/
├── monomers/
└── status/
```

### Hash Directory Convention

The 2-character hash is the **middle two characters** of the 4-character PDB ID:
- PDB ID `1abc` → hash `ab` → stored in `{hash}/ab/`
- PDB ID `100d` → hash `00` → stored in `{hash}/00/`

## rsync Endpoints

| Mirror | URL | Port | Status |
|--------|-----|------|--------|
| RCSB | `rsync://rsync.rcsb.org/` | 33444 | Active |
| PDBj | `rsync://rsync.pdbj.org/` | 873 | Active |
| PDBe | `rsync://rsync.ebi.ac.uk/pub/databases/pdb/` | 873 | Active |
| wwPDB | `rsync://rsync.wwpdb.org/` | 873 | Active |

### rsync Modules (All Mirrors)

| Module | Path | Description |
|--------|------|-------------|
| `ftp` | `/pub/pdb` | Top level of PDB archive |
| `ftp_data` | `/pub/pdb/data` | Data directory (structures, etc.) |
| `ftp_derived` | `/pub/pdb/derived_data` | Derived data |
| `ftp_doc` | `/pub/pdb/doc` | Documentation |
| `emdb` | `/pub/emdb` | EMDB data |
| `pdb_ihm` | `/pub/pdb_ihm` | PDB-IHM (integrative/hybrid methods) |

### rsync URL Patterns

```bash
# mmCIF coordinate files
rsync://rsync.rcsb.org:33444/ftp_data/structures/divided/mmCIF/{hash}/{id}.cif.gz

# PDB format coordinate files
rsync://rsync.rcsb.org:33444/ftp_data/structures/divided/pdb/{hash}/pdb{id}.ent.gz

# Structure factors
rsync://rsync.rcsb.org:33444/ftp_data/structures/divided/structure_factors/{hash}/r{id}sf.ent.gz

# Biological assemblies (mmCIF)
rsync://rsync.rcsb.org:33444/ftp_data/assemblies/mmCIF/divided/{hash}/{id}-assembly{N}.cif.gz

# Biological assemblies (legacy PDB format)
rsync://rsync.rcsb.org:33444/ftp_data/biounit/coordinates/divided/{hash}/{id}.pdb{N}.gz

# Flat structure (all in one directory)
rsync://rsync.rcsb.org:33444/ftp_data/structures/all/mmCIF/
```

## HTTPS Endpoints

| Endpoint | URL Pattern | Compressed | Notes |
|----------|-------------|------------|-------|
| RCSB mmCIF | `https://files.rcsb.org/download/{id}.cif` | No | Case insensitive, ~60ms |
| RCSB PDB | `https://files.rcsb.org/download/{id}.pdb` | No | Case insensitive |
| RCSB mmCIF.gz | `https://files.rcsb.org/download/{id}.cif.gz` | Yes | Gzipped version |
| RCSB BinaryCIF | `https://models.rcsb.org/{id}.bcif` | Binary | Different server! |
| PDBj | `https://pdbj.org/rest/downloadPDBfile?format=mmcif&id={id}` | Yes | 302 redirect, gzipped |
| PDBe | `https://www.ebi.ac.uk/pdbe/entry-files/{id}.cif` | No | Case sensitive (lowercase) |
| wwPDB | `https://files.wwpdb.org/pub/pdb/data/structures/divided/mmCIF/{hash}/{id}.cif.gz` | Yes | Full path required |

### HTTPS Response Details (1abc.cif)

| Mirror | Status | Content-Type | Size | Response Time |
|--------|--------|--------------|------|---------------|
| RCSB | 200 | chemical/x-cif | 69,506 | ~60ms |
| RCSB .gz | 200 | application/gzip | 17,580 | ~60ms |
| PDBj | 302→200 | application/gzip | 17,580 | ~90ms |
| PDBe | 200 | text/plain | 69,506 | ~2.6s |
| wwPDB | 200 | application/gzip | 17,580 | ~580ms |

## FTP Endpoints

| Mirror | URL | Status |
|--------|-----|--------|
| PDBj | `ftp://ftp.pdbj.org/pub/pdb/` | **Active** |
| PDBe/EBI | `ftp://ftp.ebi.ac.uk/pub/databases/pdb/` | **Active** |
| RCSB | `ftp://ftp.rcsb.org/` | Not available (DNS fails) |
| wwPDB | `ftp://ftp.wwpdb.org/` | Not available (DNS fails) |

## File Formats

### Coordinate Files

| Format | Directory | Filename Pattern | Notes |
|--------|-----------|------------------|-------|
| mmCIF | `mmCIF/` | `{id}.cif.gz` | Standard format, recommended |
| PDB | `pdb/` | `pdb{id}.ent.gz` | Legacy format |
| PDBML/XML | `XML/` | `{id}.xml.gz` | XML format |
| XML-noatom | `XML-noatom/` | `{id}-noatom.xml.gz` | Without atom coords |
| XML-extatom | `XML-extatom/` | `{id}-extatom.xml.gz` | Extended atom info |

### Experimental Data

| Format | Directory | Filename Pattern | Notes |
|--------|-----------|------------------|-------|
| Structure factors | `structure_factors/` | `r{id}sf.ent.gz` | X-ray diffraction data |
| NMR restraints | `nmr_restraints/` | `{id}_mr.str.gz` | NMR restraint data |
| NMR shifts | `nmr_chemical_shifts/` | `{id}_cs.str.gz` | Chemical shift data |

### Special Formats (RCSB Only)

| Format | Source | Notes |
|--------|--------|-------|
| BinaryCIF (.bcif) | `models.rcsb.org` | Efficient binary format for visualization |

## Key Findings

1. **wwPDB is the standard** - All mirrors follow identical structure
2. **BinaryCIF is RCSB-only** - Served from `models.rcsb.org`
3. **rsync uses compressed files** - All `.gz` format
4. **HTTPS varies by mirror**:
   - RCSB: Fastest, supports both compressed/uncompressed
   - PDBe: Slowest from Asia
   - wwPDB: Requires full path with hash
   - PDBj: Uses redirect, always gzipped
5. **FTP partially available** - Only PDBj and EBI
6. **Archive is large** - ~1TB total, ~248K structures

## Recommended Implementation

### Primary Sources
- **rsync**: Use RCSB or PDBj for bulk sync
- **HTTPS**: Use RCSB as primary, PDBe/wwPDB as fallback
- **FTP**: Use PDBj if FTP required

### Default Settings
- Format: mmCIF (`.cif.gz`)
- Layout: divided (by hash)
- Mirror: RCSB (US) or auto-select by region

### Storage Estimates

| Sync Type | Estimated Size |
|-----------|----------------|
| mmCIF only (divided) | ~150-200 GB |
| PDB only (divided) | ~100-150 GB |
| Both formats | ~300-400 GB |
| Full archive | ~1 TB+ |
