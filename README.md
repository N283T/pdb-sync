# pdb-sync

Simple CLI tool for syncing PDB (Protein Data Bank) data from rsync mirrors.

## Features

- **Custom rsync configs**: Define multiple sync sources in config file
- **Batch sync**: Run all configured syncs with a single command
- **Flexible rsync options**: Per-config rsync flag defaults with CLI override support
- **Real-time progress**: rsync progress output (`--info=progress2`) is always enabled

## Installation

```bash
cargo install --path .
```

## Quick Start

1. Create config file at `~/.config/pdb-sync/config.toml`:

```toml
[sync]
mirror = "rcsb"  # Default mirror (not used for custom configs)

# Define custom rsync syncs
[[sync.custom]]
name = "structures"
url = "rsync.wwpdb.org::ftp_data/structures/divided/mmCIF/"
dest = "data/structures/divided/mmCIF"
description = "PDB structures (mmCIF format, divided layout)"

[[sync.custom]]
name = "emdb"
url = "data.pdbj.org::rsync/pub/emdb/"
dest = "data/emdb"
description = "EMDB (Electron Microscopy Data Bank)"
```

2. Run sync:

```bash
# Run all configured syncs
pdb-sync sync

# Run specific config by name
pdb-sync sync emdb

# Explicitly run all configs
pdb-sync sync --all
```

## Usage

```
pdb-sync sync [NAME] [OPTIONS]

Arguments:
  [NAME]  Name of custom sync config to run (runs all if not specified)

Options:
  -a, --all           Run all custom sync configs
  -d, --dest <DIR>    Override destination directory
  --list              List available custom sync configs
  --fail-fast         Stop on first failure when syncing all configs
  -n, --dry-run        Dry run without changes
  --delete            Delete files not present on remote
  --bwlimit <KBPS>    Bandwidth limit in KB/s
  -z, --compress      Compress data during transfer
  -c, --checksum      Use checksum for file comparison
  --exclude <PATTERN> Exclude patterns (repeatable)
  --include <PATTERN> Include patterns (repeatable)
  --rsync-verbose     Verbose rsync output
  --rsync-quiet       Quiet rsync output
  -v, --verbose       Enable verbose output
  -h, --help          Print help
```

## Configuration

Config file location: `~/.config/pdb-sync/config.toml`

### Custom Sync Configs

Each custom sync config defines an rsync source:

```toml
[[sync.custom]]
name = "my-sync"              # Required: unique identifier
url = "host::module/path"      # Required: rsync URL
dest = "local/path"            # Required: destination subdirectory
description = "Description"    # Optional: human-readable description

# Optional rsync flags (per-config defaults)
rsync_delete = true
rsync_compress = true
rsync_bwlimit = 1000           # KB/s
rsync_timeout = 600            # seconds
rsync_exclude = ["*.tmp", "test/*"]
```

### Available rsync Options

| Config Field | CLI Flag | Description |
|--------------|----------|-------------|
| `rsync_delete` | --delete | Delete files not present on remote |
| `rsync_compress` | -z | Compress data during transfer |
| `rsync_checksum` | -c | Use checksum for file comparison |
| `rsync_partial` | --partial | Keep partially transferred files |
| `rsync_partial_dir` | --partial-dir | Directory for partial files |
| `rsync_max_size` | --max-size | Maximum file size to transfer |
| `rsync_min_size` | --min-size | Minimum file size to transfer |
| `rsync_timeout` | --timeout | I/O timeout in seconds |
| `rsync_contimeout` | --contimeout | Connection timeout in seconds |
| `rsync_backup` | --backup | Create backups |
| `rsync_backup_dir` | --backup-dir | Backup directory |
| `rsync_chmod` | --chmod | Change permission flags |
| `rsync_exclude` | --exclude | Exclude patterns (array) |
| `rsync_include` | --include | Include patterns (array) |
| `rsync_verbose` | --rsync-verbose | Verbose output |
| `rsync_quiet` | --rsync-quiet | Quiet mode |
| `rsync_dry_run` | -n, --dry-run | Dry run without changes |

### Example Configs

```toml
[sync]
mirror = "rcsb"

# Standard PDB structures
[[sync.custom]]
name = "structures"
url = "rsync.wwpdb.org::ftp_data/structures/divided/mmCIF/"
dest = "data/structures/divided/mmCIF"
description = "PDB structures (mmCIF format, divided layout)"
rsync_delete = true
rsync_bwlimit = 5000

# Biological assemblies
[[sync.custom]]
name = "assemblies"
url = "rsync.wwpdb.org::ftp_data/structures/divided/assemblies/mmCIF/"
dest = "data/assemblies/divided/mmCIF"
description = "Biological assemblies (mmCIF format)"
rsync_delete = true

# EMDB data
[[sync.custom]]
name = "emdb"
url = "data.pdbj.org::rsync/pub/emdb/"
dest = "data/emdb"
description = "Electron Microscopy Data Bank"
rsync_delete = true
rsync_exclude = ["*.tmp", "test/*"]

# PDBj additional directories
[[sync.custom]]
name = "pdbj-sifts"
url = "ftp.pdbj.org::pub/pdbj/data/sifts/"
dest = "pdbj/sifts"
description = "SIFTS data from PDBj"
rsync_delete = true

[[sync.custom]]
name = "pdbj-bsma"
url = "data.pdbj.org::rsync/pdbj/bsma/"
dest = "pdbj/bsma"
description = "BSM-Arc (Binding Site Matrix archive)"
rsync_delete = true

[[sync.custom]]
name = "pdbj-mmjson"
url = "data.pdbj.org::rsync/pdbjplus/data/cc/mmjson/"
dest = "pdbj/mmjson"
description = "mmJSON format from PDBj+"
rsync_delete = true

[[sync.custom]]
name = "pdbj-pdbe"
url = "data.pdbj.org::rsync/pdbjplus/data/cc/pdbe/"
dest = "pdbj/pdbe"
description = "PDBE data from PDBj+"
rsync_delete = true
```

## Examples

```bash
# Run all configured syncs
pdb-sync sync

# Run specific sync
pdb-sync sync emdb

# Override destination
pdb-sync sync structures --dest /mnt/c/pdb

# Run all syncs
pdb-sync sync --all

# Verbose mode
pdb-sync sync -v --all
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `PDB_DIR` | Base directory for PDB files |
| `PDB_SYNC_CONFIG` | Path to configuration file |

## License

MIT
