# pdb-cli

CLI tool for managing Protein Data Bank (PDB) files. Supports rsync synchronization from PDB mirrors, HTTPS downloads, and local file management.

## Features

- **sync**: Synchronize PDB files from mirrors using rsync
- **download**: Download individual files via HTTPS with progress bar
- **copy**: Copy local PDB files with flatten/symlink options
- **config**: Manage configuration settings
- **env**: Manage environment variables

## Installation

```bash
cargo install --path .
```

## Usage

```bash
# Show help
pdb-cli --help

# Sync mmCIF files from PDBj mirror (dry-run)
pdb-cli sync --mirror pdbj --dry-run

# Download a structure file
pdb-cli download 4hhb --dest ./structures

# Download multiple files in PDB format
pdb-cli download 1abc 2xyz 3def --format pdb --dest ./structures

# Show current configuration
pdb-cli config show

# Initialize configuration file
pdb-cli config init

# Show environment variables
pdb-cli env show
```

## Commands

### sync

Synchronize files from a PDB mirror using rsync.

```bash
pdb-cli sync [OPTIONS] [FILTER]...

Options:
  -m, --mirror <MIRROR>   Mirror: rcsb, pdbj, pdbe, wwpdb
  -f, --format <FORMAT>   Format: pdb, mmcif, both [default: mmcif]
  -d, --dest <DIR>        Destination directory
  --delete                Delete files not present on remote
  --bwlimit <KBPS>        Bandwidth limit in KB/s
  -n, --dry-run           Perform dry run without changes
```

### download

Download individual files via HTTPS.

```bash
pdb-cli download [OPTIONS] <PDB_ID>...

Options:
  -f, --format <FORMAT>   Format: pdb, mmcif, bcif [default: mmcif]
  -d, --dest <DIR>        Destination directory
  -m, --mirror <MIRROR>   Mirror to use
  --decompress            Decompress downloaded files
  --overwrite             Overwrite existing files
```

### copy

Copy local PDB files.

```bash
pdb-cli copy [OPTIONS] <SOURCE> <DEST>

Options:
  --flatten    Flatten directory structure
  --symlink    Create symbolic links instead of copying
```

### config

Manage configuration.

```bash
pdb-cli config init          # Initialize config file
pdb-cli config show          # Show current config
pdb-cli config get <KEY>     # Get a config value
pdb-cli config set <KEY> <VALUE>  # Set a config value
```

### env

Manage environment variables.

```bash
pdb-cli env show    # Show environment variables
pdb-cli env export  # Export as shell commands
pdb-cli env set <NAME> <VALUE>  # Print set command
```

## Configuration

Configuration file location: `~/.config/pdb-cli/config.toml`

```toml
[paths]
pdb_dir = "/data/pdb"

[sync]
mirror = "rcsb"
bwlimit = 0
delete = false

[download]
auto_decompress = true
parallel = 4
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `PDB_DIR` | Base directory for PDB files |
| `PDB_CLI_CONFIG` | Path to configuration file |
| `PDB_CLI_MIRROR` | Default mirror (rcsb, pdbj, pdbe, wwpdb) |

## Supported Mirrors

| ID | Region | Description |
|----|--------|-------------|
| rcsb | US | RCSB PDB |
| pdbj | Japan | PDBj |
| pdbe | Europe | PDBe (EBI) |
| wwpdb | Global | wwPDB |

## License

MIT
