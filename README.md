# pdb-sync

CLI tool for managing Protein Data Bank (PDB) files. Supports rsync synchronization from PDB mirrors, HTTPS downloads, local file management, validation, and automatic updates.

## Features

- **sync**: Synchronize PDB files from mirrors using rsync with data type and layout selection
- **download**: Download files via HTTPS with parallel downloads and retry support
- **list**: List and search local PDB files with filtering and statistics
- **find**: Find PDB files with path output for scripting
- **info**: Query PDB entry metadata from RCSB API
- **validate**: Verify file integrity using checksums with auto-repair option
- **update**: Check for and download updates to local files
- **watch**: Monitor RCSB for new entries and auto-download
- **stats**: Show statistics about local PDB collection
- **tree**: Display directory structure visualization
- **convert**: Convert between formats (compress/decompress/format conversion)
- **copy**: Copy local PDB files with flatten/symlink options
- **config**: Manage configuration with automatic mirror selection
- **jobs**: Manage background jobs
- **env**: Manage environment variables

## Installation

```bash
cargo install --path .
```

## Quick Start

```bash
# First-time setup (interactive)
pdb-sync config init

# Download structure files
pdb-sync download 4hhb 1abc 2xyz

# List local files
pdb-sync list

# Get entry information
pdb-sync info 4hhb

# Validate local files
pdb-sync validate --progress

# Check for updates
pdb-sync update --check

# Watch for new entries
pdb-sync watch --once --dry-run
```

## Commands

### sync

Synchronize files from a PDB mirror using rsync. Supports subcommands for different data sources.

```bash
pdb-sync sync [OPTIONS] [COMMAND]

Subcommands:
  wwpdb       Sync wwPDB standard data (structures, assemblies, etc.)
  structures  Shortcut for `wwpdb --type structures`
  assemblies  Shortcut for `wwpdb --type assemblies`
  pdbj        Sync PDBj-specific data (EMDB, PDB-IHM, derived data)
  pdbe        Sync PDBe-specific data (SIFTS, PDBeChem, Foldseek)

Options:
  -m, --mirror <MIRROR>     Mirror: rcsb, pdbj, pdbe, wwpdb
  -t, --type <DATA_TYPE>    Data type (can specify multiple times)
  -f, --format <FORMAT>     Format: pdb, mmcif, both [default: mmcif]
  -l, --layout <LAYOUT>     Layout: divided, all [default: divided]
  -d, --dest <DIR>          Destination directory
  --delete                  Delete files not present on remote
  --bwlimit <KBPS>          Bandwidth limit in KB/s
  -n, --dry-run             Perform dry run without changes
  -P, --progress            Show detailed progress
  --bg                      Run in background
```

Examples:
```bash
# Sync mmCIF structures from PDBj (dry-run)
pdb-sync sync --mirror pdbj --dry-run

# Sync structures using shortcut
pdb-sync sync structures --mirror rcsb

# Sync multiple data types
pdb-sync sync wwpdb -t structures -t assemblies --mirror rcsb

# Sync PDBj-specific data (EMDB)
pdb-sync sync pdbj --type emdb

# Sync PDBe Foldseek database
pdb-sync sync pdbe --type foldseek

# Run sync in background
pdb-sync sync --mirror wwpdb --bg
```

### download

Download individual files via HTTPS with parallel downloads.

```bash
pdb-sync download [OPTIONS] <PDB_IDS>...

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
  --bg                      Run in background
```

Examples:
```bash
# Download multiple structures
pdb-sync download 4hhb 1abc 2xyz --dest ./structures

# Download in PDB format with decompression
pdb-sync download 4hhb --format pdb --decompress

# Download biological assemblies
pdb-sync download 4hhb -t assemblies -a 1

# Download structure factors
pdb-sync download 1abc -t structure-factors

# Download from file list with 8 parallel connections
pdb-sync download -l pdb_ids.txt -p 8

# Run download in background
pdb-sync download -l large_list.txt --bg
```

### list

List local PDB files with filtering and statistics.

```bash
pdb-sync list [OPTIONS] [PATTERN]

Options:
  -f, --format <FORMAT>     File format to list
  -s, --size                Show file sizes
  --time                    Show modification times
  -o, --output <FORMAT>     Output: text, json, csv, ids [default: text]
  --stats                   Show statistics only
  --sort <FIELD>            Sort by: name, size, time [default: name]
  -r, --reverse             Reverse sort order
```

Examples:
```bash
# List all local files
pdb-sync list

# List files matching pattern
pdb-sync list "1ab*"

# Show statistics only
pdb-sync list --stats

# List with sizes, sorted by size
pdb-sync list -s --sort size -r

# Export as JSON
pdb-sync list -o json > files.json

# Get just IDs for piping
pdb-sync list -o ids | head -10
```

### find

Find PDB files with path output optimized for scripting.

```bash
pdb-sync find [OPTIONS] [PATTERNS]...

Options:
  -f, --format <FORMAT>     File format to search
  --all-formats             Show all formats for each entry
  --exists                  Check existence (exit code only)
  --missing                 Show entries NOT found locally
  -q, --quiet               Quiet mode (no output, just exit code)
  --count                   Count matches only
  --stdin                   Read patterns from stdin
```

Examples:
```bash
# Find specific entries
pdb-sync find 4hhb 1abc

# Find all formats for an entry
pdb-sync find 4hhb --all-formats

# Check if files exist (for scripting)
pdb-sync find 4hhb --exists && echo "Found"

# Find missing entries from a list
cat ids.txt | pdb-sync find --stdin --missing

# Use with xargs
pdb-sync find "1ab*" | xargs -I{} cp {} ./output/
```

### info

Show information about a PDB entry from RCSB API.

```bash
pdb-sync info [OPTIONS] <PDB_ID>

Options:
  --local                   Show local file info only
  --output <FORMAT>         Output: text, json, csv [default: text]
  -a, --all                 Show all available fields
```

Examples:
```bash
# Get entry info
pdb-sync info 4hhb

# Get full details
pdb-sync info 4hhb --all

# Get as JSON
pdb-sync info 4hhb -o json

# Check local file only
pdb-sync info 4hhb --local
```

### validate

Validate local PDB files against checksums.

```bash
pdb-sync validate [OPTIONS] [PDB_IDS]...

Options:
  -f, --format <FORMAT>     File format to validate
  -m, --mirror <MIRROR>     Mirror for checksums
  --fix                     Re-download corrupted files
  -P, --progress            Show progress bar
  --errors-only             Show only errors
  -o, --output <FORMAT>     Output: text, json, csv, ids [default: text]
```

Examples:
```bash
# Validate all local files
pdb-sync validate -P

# Validate specific IDs
pdb-sync validate 1abc 2xyz 3def

# Validate and fix corrupted files
pdb-sync validate --fix -P

# Get list of invalid IDs for piping
pdb-sync validate -o ids | pdb-sync download -l -
```

### update

Check for and download updates to local files.

```bash
pdb-sync update [OPTIONS] [PDB_IDS]...

Options:
  -c, --check               Check only, don't download updates
  -n, --dry-run             Show what would be updated
  --verify                  Use checksums for verification (slower)
  --force                   Force update even if up-to-date
  -f, --format <FORMAT>     File format to check
  -m, --mirror <MIRROR>     Mirror to check against
  -P, --progress            Show progress bar
  -o, --output <FORMAT>     Output: text, json, csv, ids [default: text]
  -j, --parallel <NUM>      Parallel checks [default: 10]
```

Examples:
```bash
# Check all files for updates
pdb-sync update --check -P

# Update specific files
pdb-sync update 4hhb 1abc

# Dry run to see what would be updated
pdb-sync update --dry-run

# Force update with checksum verification
pdb-sync update --force --verify

# Get list of outdated IDs
pdb-sync update --check -o ids
```

### watch

Watch for new PDB entries and download automatically.

```bash
pdb-sync watch [OPTIONS]

Options:
  -i, --interval <INTERVAL> Check interval (e.g., "1h", "30m") [default: 1h]
  --method <METHOD>         Filter: xray, nmr, em, neutron
  --resolution <NUM>        Max resolution in Angstroms
  --organism <NAME>         Filter by source organism
  -t, --type <DATA_TYPE>    Data types to download
  -f, --format <FORMAT>     File format [default: mmcif]
  -n, --dry-run             Don't download, just show matches
  --notify <TYPE>           Notification: desktop, email
  --email <ADDR>            Email for notifications
  --on-new <SCRIPT>         Run script on each new entry
  -m, --mirror <MIRROR>     Mirror to download from
  --once                    Run once and exit
  --since <DATE>            Start date (YYYY-MM-DD)
```

Examples:
```bash
# Watch for new entries (runs continuously)
pdb-sync watch

# Check once for new high-resolution X-ray structures
pdb-sync watch --once --method xray --resolution 2.0

# Watch with desktop notifications
pdb-sync watch --notify desktop

# Run custom script on new entries
pdb-sync watch --on-new ./process_new.sh

# Dry run to see recent entries
pdb-sync watch --once --dry-run --since 2024-01-01
```

### stats

Show statistics about the local PDB collection.

```bash
pdb-sync stats [OPTIONS]

Options:
  --detailed                Show size distribution, oldest/newest files
  --compare-remote          Compare with remote PDB archive
  -f, --format <FORMAT>     Filter by file format
  -t, --type <DATA_TYPE>    Filter by data type
  -o, --output <FORMAT>     Output: text, json, csv [default: text]
```

Examples:
```bash
# Show basic statistics
pdb-sync stats

# Show detailed statistics
pdb-sync stats --detailed

# Compare with remote archive
pdb-sync stats --compare-remote

# Stats for specific format
pdb-sync stats -f cif-gz

# Export as JSON
pdb-sync stats -o json
```

### tree

Display directory tree of local PDB mirror.

```bash
pdb-sync tree [OPTIONS]

Options:
  -d, --depth <NUM>         Maximum depth to display
  -f, --format <FORMAT>     Filter by file format
  -s, --size                Show file sizes
  -c, --count               Show file counts
  --no-summary              Hide summary line
  --non-empty               Show only non-empty directories
  --top <NUM>               Show top N directories
  --sort-by <FIELD>         Sort: count, size [default: count]
  -o, --output <FORMAT>     Output: text, json, csv [default: text]
```

Examples:
```bash
# Show full tree
pdb-sync tree

# Limit depth
pdb-sync tree --depth 2

# Show top 10 directories by size
pdb-sync tree --top 10 --sort-by size

# Show with counts and sizes
pdb-sync tree -c -s

# Export as JSON
pdb-sync tree -o json
```

### convert

Convert file formats (compression, decompression, format conversion).

```bash
pdb-sync convert [OPTIONS] [FILES]...

Options:
  --decompress              Decompress .gz files
  --compress                Compress files to .gz
  --to <FORMAT>             Target format (requires gemmi)
  --from <FORMAT>           Source format filter
  -d, --dest <DIR>          Destination directory
  --in-place                Replace original files
  --stdin                   Read paths from stdin
  -p, --parallel <NUM>      Parallel conversions [default: 4]
```

Examples:
```bash
# Decompress files
pdb-sync convert --decompress *.cif.gz

# Compress files
pdb-sync convert --compress *.cif

# Convert mmCIF to PDB format (requires gemmi)
pdb-sync convert --to pdb --from cif-gz -d ./pdb_files/

# In-place decompression
pdb-sync convert --decompress --in-place ./data/*.gz

# Batch convert from stdin
find ./data -name "*.cif.gz" | pdb-sync convert --stdin --decompress
```

### copy

Copy local PDB files.

```bash
pdb-sync copy [OPTIONS] <PDB_IDS>... --dest <DIR>

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
pdb-sync config init              # Initialize config file
pdb-sync config show              # Show current config
pdb-sync config get <KEY>         # Get a config value
pdb-sync config set <KEY> <VALUE> # Set a config value
pdb-sync config test-mirrors      # Test mirror latencies
```

### jobs

Manage background jobs.

```bash
pdb-sync jobs [OPTIONS] [COMMAND]

Commands:
  status <ID>     Show status of a specific job
  log <ID>        Show logs for a job
  cancel <ID>     Cancel a running job
  clean           Clean up old job directories

Options:
  -a, --all                 Show all jobs (including old)
  --running                 Show only running jobs
```

Examples:
```bash
# List all jobs
pdb-sync jobs

# Show running jobs only
pdb-sync jobs --running

# Check job status
pdb-sync jobs status abc123

# View job logs
pdb-sync jobs log abc123

# Cancel a running job
pdb-sync jobs cancel abc123

# Clean up old jobs
pdb-sync jobs clean
```

### env

Manage environment variables.

```bash
pdb-sync env show                 # Show environment variables
pdb-sync env export               # Export as shell commands
pdb-sync env set <NAME> <VALUE>   # Print set command
```

## Configuration

Configuration file: `~/.config/pdb-sync/config.toml`

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
| `PDB_SYNC_CONFIG` | Path to configuration file |
| `PDB_SYNC_MIRROR` | Default mirror |

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

## Aliases

Short aliases are available for commonly used commands and options.

### Command Aliases

| Full Command | Alias |
|--------------|-------|
| `download` | `dl` |
| `validate` | `val` |
| `config` | `cfg` |

### Option Value Aliases

#### Data Types (`--type` / `-t`)

| Full Name | Aliases |
|-----------|---------|
| `structures` | `st`, `struct` |
| `assemblies` | `asm`, `assembly` |
| `structure-factors` | `sf`, `xray` |
| `nmr-chemical-shifts` | `nmr-cs`, `cs` |
| `nmr-restraints` | `nmr-r` |

#### Formats (`--format` / `-f`)

| Full Name | Alias |
|-----------|-------|
| `mmcif` | `cif` |

#### Mirrors (`--mirror` / `-m`)

| Full Name | Aliases |
|-----------|---------|
| `rcsb` | `us` |
| `pdbj` | `jp` |
| `pdbe` | `uk`, `eu` |
| `wwpdb` | `global` |

### Example Usage

```bash
# Before (full names)
pdb-sync download 4hhb --type structures --format mmcif

# After (with aliases)
pdb-sync dl 4hhb -t st -f cif

# Validate shorthand
pdb-sync val --fix -P

# Config shorthand
pdb-sync cfg show
```

## Piping and Scripting

Commands support `-o ids` output for piping:

```bash
# Find outdated files and update them
pdb-sync update --check -o ids | pdb-sync download -l -

# Validate and re-download corrupted files
pdb-sync validate -o ids | pdb-sync download -l - --overwrite

# Find missing entries and download
cat wanted.txt | pdb-sync find --stdin --missing | pdb-sync download -l -
```

## License

MIT
