# PDB Mirror Investigation

Investigation date: 2026-01-17

## rsync Structure (Common across all mirrors)

All mirrors follow the wwPDB standard structure:

```
ftp_data/structures/
├── all/                    # Flat structure (all files in one dir)
│   ├── mmCIF/              # *.cif.gz
│   ├── pdb/                # pdb*.ent.gz
│   ├── XML/
│   ├── XML-noatom/
│   ├── XML-extatom/
│   ├── nmr_chemical_shifts/
│   ├── nmr_data/
│   ├── nmr_restraints/
│   └── structure_factors/
├── divided/                # Divided by middle 2 chars of PDB ID
│   ├── mmCIF/{ab}/         # {id}.cif.gz
│   ├── pdb/{ab}/           # pdb{id}.ent.gz
│   └── ...
├── models/                 # Model structures
└── obsolete/               # Obsolete entries
```

## Mirror Endpoints

### rsync

| Mirror | URL | Port | Notes |
|--------|-----|------|-------|
| RCSB | `rsync://rsync.rcsb.org/` | 33444 | US |
| PDBj | `rsync://rsync.pdbj.org/` | 873 (default) | Japan |
| PDBe | `rsync://rsync.ebi.ac.uk/pub/databases/pdb/` | 873 | Europe |
| wwPDB | `rsync://rsync.wwpdb.org/` | 873 | Global |

### HTTPS

| Mirror | Endpoint | Format | Notes |
|--------|----------|--------|-------|
| RCSB | `https://files.rcsb.org/download/{id}.cif` | Uncompressed | Direct download |
| RCSB | `https://files.rcsb.org/download/{id}.pdb` | Uncompressed | Legacy PDB |
| RCSB bcif | `https://models.rcsb.org/{id}.bcif` | BinaryCIF | Different server! |
| PDBj | `https://pdbj.org/rest/downloadPDBfile?format=mmcif&id={id}` | Uncompressed | Redirects |
| PDBe | `https://www.ebi.ac.uk/pdbe/entry-files/download/{id}.cif` | Uncompressed | |
| wwPDB | `https://files.wwpdb.org/pub/pdb/data/structures/divided/mmCIF/{mid}/{id}.cif.gz` | Compressed | Full path required |

## File Formats

### Available via rsync (all mirrors)

| Format | Directory | Filename Pattern | Notes |
|--------|-----------|------------------|-------|
| mmCIF | `mmCIF/` | `{id}.cif.gz` | Standard format |
| PDB | `pdb/` | `pdb{id}.ent.gz` | Legacy format |
| PDBML/XML | `XML/` | `{id}.xml.gz` | XML format |
| XML-noatom | `XML-noatom/` | `{id}-noatom.xml.gz` | Without atom coords |
| XML-extatom | `XML-extatom/` | `{id}-extatom.xml.gz` | Extended atom info |
| Structure factors | `structure_factors/` | `r{id}sf.ent.gz` | X-ray data |
| NMR restraints | `nmr_restraints/` | `{id}_mr.str.gz` | NMR data |
| NMR shifts | `nmr_chemical_shifts/` | `{id}_cs.str.gz` | Chemical shifts |

### Available via HTTPS only

| Format | Source | Notes |
|--------|--------|-------|
| BinaryCIF (.bcif) | RCSB only (`models.rcsb.org`) | Not available on other mirrors |

## Key Findings

1. **wwPDB is the standard** - All mirrors follow wwPDB structure
2. **BinaryCIF is RCSB-only** - `models.rcsb.org` is a separate service
3. **rsync uses compressed files** - All `.gz` format
4. **HTTPS varies by mirror**:
   - RCSB/PDBe: Uncompressed direct download
   - wwPDB: Compressed with full path
   - PDBj: Uses redirect

## Recommended Implementation

Focus on wwPDB standard:
- rsync: Use `ftp_data/structures/divided/` path
- HTTPS: Support both compressed (wwPDB) and uncompressed (RCSB)
- Formats: mmCIF (.cif.gz) as default, PDB (.ent.gz) as legacy
- BinaryCIF: Optional, RCSB-only feature

## rsync Module Mapping

| Module | Path |
|--------|------|
| `ftp` | `/pub/pdb` |
| `ftp_data` | `/pub/pdb/data` |
| `ftp_derived` | `/pub/pdb/derived_data` |
| `ftp_doc` | `/pub/pdb/doc` |
| `emdb` | `/pub/emdb` |
| `pdb_ihm` | `/pub/pdb_ihm` |
