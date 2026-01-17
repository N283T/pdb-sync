# pdb-cli

CLI tool for managing Protein Data Bank (PDB) files. Supports rsync synchronization from PDB mirrors, HTTPS downloads, local file management, and validation.

## Features

- **sync**: Synchronize PDB files from mirrors using rsync with data type and layout selection
- **download**: Download files via HTTPS with parallel downloads and retry support
- **list**: List and search local PDB files with filtering and statistics
- **info**: Query PDB entry metadata from RCSB API
- **validate**: Verify file integrity using checksums with auto-repair option
- **copy**: Copy local PDB files with flatten/symlink options
- **config**: Manage configuration with automatic mirror selection
- **env**: Manage environment variables

## Installation

```bash
cargo install --path .
```

## Quick Start

```bash
# First-time setup (interactive)
pdb-cli config init

# Download structure files
pdb-cli download 4hhb 1abc 2xyz

# List local files
pdb-cli list

# Get entry information
pdb-cli info 4hhb

# Validate local files
pdb-cli validate --progress
```

## Commands

### sync

Synchronize files from a PDB mirror using rsync.

```bash
pdb-cli sync [OPTIONS] [FILTERS]...

Options:
  -m, --mirror <MIRROR>     Mirror: rcsb, pdbj, pdbe, wwpdb
  -t, --type <DATA_TYPE>    Data type (can specify multiple times):
                            structures, assemblies, biounit, structure-factors,
                            nmr-chemical-shifts, nmr-restraints, obsolete
  -f, --format <FORMAT>     Format: pdb, mmcif, both [default: mmcif]
  -l, --layout <LAYOUT>     Layout: divided, all [default: divided]
  -d, --dest <DIR>          Destination directory
  --delete                  Delete files not present on remote
  --bwlimit <KBPS>          Bandwidth limit in KB/s
  -n, --dry-run             Perform dry run without changes
  -P, --progress            Show detailed progress
```

Examples:
```bash
# Sync mmCIF structures from PDBj (dry-run)
pdb-cli sync --mirror pdbj --dry-run

# Sync multiple data types
pdb-cli sync -t structures -t assemblies --mirror rcsb

# Sync with bandwidth limit
pdb-cli sync --mirror wwpdb --bwlimit 10000
```

### download

Download individual files via HTTPS with parallel downloads.

```bash
pdb-cli download [OPTIONS] <PDB_IDS>...

Options:
  -t, --type <DATA_TYPE>    Data type [default: structures]
  -f, --format <FORMAT>     Format: pdb, mmcif, bcif [default: mmcif]
  -a, --assembly <NUM>      Assembly number (for assemblies type)
  -d, --dest <DIR>          Destination directory
  -m, --mirror <MIRROR>     Mirror to use
  -p, --parallel <NUM>      Parallel downloads [default: 4]
  --retry <NUM>             Retry attempts [default: 3]
  --decompress              Decompress downloaded files
  --overwrite               Overwrite existing files
  -l, --list <FILE>         Read PDB IDs from file
```

Examples:
```bash
# Download multiple structures
pdb-cli download 4hhb 1abc 2xyz --dest ./structures

# Download in PDB format with decompression
pdb-cli download 4hhb --format pdb --decompress

# Download biological assemblies
pdb-cli download 4hhb -t assemblies -a 1

# Download structure factors
pdb-cli download 1abc -t structure-factors

# Download from file list with 8 parallel connections
pdb-cli download -l pdb_ids.txt -p 8
```

### list

List local PDB files with filtering and statistics.

```bash
pdb-cli list [OPTIONS] [PATTERN]

Options:
  -f, --format <FORMAT>     File format to list
  -s, --size                Show file sizes
  --time                    Show modification times
  -o, --output <FORMAT>     Output: text, json, csv [default: text]
  --stats                   Show statistics only
  --sort <FIELD>            Sort by: name, size, time [default: name]
  -r, --reverse             Reverse sort order
```

Examples:
```bash
# List all local files
pdb-cli list

# List files matching pattern
pdb-cli list "1ab*"

# Show statistics only
pdb-cli list --stats

# List with sizes, sorted by size
pdb-cli list -s --sort size -r

# Export as JSON
pdb-cli list -o json > files.json
```

### info

Show information about a PDB entry from RCSB API.

```bash
pdb-cli info [OPTIONS] <PDB_ID>

Options:
  --local                   Show local file info only
  --output <FORMAT>         Output: text, json, csv [default: text]
  -a, --all                 Show all available fields
```

Examples:
```bash
# Get entry info
pdb-cli info 4hhb

# Get full details
pdb-cli info 4hhb --all

# Get as JSON
pdb-cli info 4hhb -o json

# Check local file only
pdb-cli info 4hhb --local
```

### validate

Validate local PDB files against checksums.

```bash
pdb-cli validate [OPTIONS] [PDB_IDS]...

Options:
  -t, --type <DATA_TYPE>    Data type to validate
  -f, --format <FORMAT>     File format to validate
  -m, --mirror <MIRROR>     Mirror for checksums
  --fix                     Re-download corrupted files
  -P, --progress            Show progress bar
  --errors-only             Show only errors
```

Examples:
```bash
# Validate all local files
pdb-cli validate -P

# Validate specific IDs
pdb-cli validate 1abc 2xyz 3def

# Validate and fix corrupted files
pdb-cli validate --fix -P

# Show only errors
pdb-cli validate --errors-only
```

### copy

Copy local PDB files.

```bash
pdb-cli copy [OPTIONS] <PDB_IDS>... --dest <DIR>

Options:
  -f, --format <FORMAT>     File format [default: cif-gz]
  -d, --dest <DIR>          Destination directory (required)
  --keep-structure          Keep directory structure
  -s, --symlink             Create symbolic links
  -l, --list <FILE>         Read PDB IDs from file
```

### config

Manage configuration settings.

```bash
pdb-cli config init              # Initialize config file
pdb-cli config show              # Show current config
pdb-cli config get <KEY>         # Get a config value
pdb-cli config set <KEY> <VALUE> # Set a config value
pdb-cli config test-mirrors      # Test mirror latencies
```

### env

Manage environment variables.

```bash
pdb-cli env show                 # Show environment variables
pdb-cli env export               # Export as shell commands
pdb-cli env set <NAME> <VALUE>   # Print set command
```

## Configuration

Configuration file: `~/.config/pdb-cli/config.toml`

```toml
[paths]
pdb_dir = "/data/pdb"

[sync]
mirror = "rcsb"
bwlimit = 0
delete = false
layout = "divided"
data_types = ["structures"]

[download]
default_format = "mmcif"
auto_decompress = true
parallel = 4
retry_count = 3

[mirror_selection]
auto_select = false
preferred_region = "us"
latency_cache_ttl = 3600
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `PDB_DIR` | Base directory for PDB files |
| `PDB_CLI_CONFIG` | Path to configuration file |
| `PDB_CLI_MIRROR` | Default mirror |

## Supported Mirrors

| ID | Region | Description |
|----|--------|-------------|
| rcsb | US | RCSB PDB (Research Collaboratory for Structural Bioinformatics) |
| pdbj | Japan | PDBj (Protein Data Bank Japan) |
| pdbe | Europe | PDBe (Protein Data Bank in Europe) |
| wwpdb | Global | wwPDB (Worldwide Protein Data Bank) |

## Data Types

| Type | Description |
|------|-------------|
| structures | Coordinate files (mmCIF/PDB format) |
| assemblies | Biological assemblies |
| biounit | Legacy biounit format |
| structure-factors | X-ray diffraction data |
| nmr-chemical-shifts | NMR chemical shifts |
| nmr-restraints | NMR restraints |
| obsolete | Obsolete entries |

## Extended PDB ID Support

Supports both classic (4-char) and extended PDB ID formats:

- Classic: `1abc`, `4hhb`
- Extended: `pdb_00001abc` (12-char format for future expansion)

## License

MIT
