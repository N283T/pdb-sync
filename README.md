# pdb-sync

Simple CLI tool for syncing PDB (Protein Data Bank) data from rsync mirrors.

## Features

- **Custom rsync configs**: Define multiple sync sources in config file
- **Batch sync**: Run all configured syncs with a single command
- **Parallel execution**: Run multiple sync operations concurrently with `--parallel`
- **Automatic retry**: Retry on transient failures with exponential backoff
- **Plan mode**: Preview changes before executing with `--plan`
- **Built-in presets**: Quick-start profiles for common PDB sources
- **Rsync flag presets**: 4 presets (safe, fast, minimal, conservative) for common scenarios
- **Flexible config formats**: Support 4 config styles from preset-only to fully custom
- **Config migration**: Automatic conversion from old `rsync_*` format to new nested format
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
mirror = "rcsb"

# Global defaults for all custom configs (DRY)
[sync.defaults]
compress = true
timeout = 300
partial = true

# Full custom configuration (RECOMMENDED)
[sync.custom.structures]
url = "rsync.wwpdb.org::ftp_data/structures/divided/mmCIF/"
dest = "data/structures/divided/mmCIF"
description = "PDB structures (mmCIF format, divided layout)"

[sync.custom.structures.options]
delete = true
max_size = "10G"

# You can also use presets for quick setup
[sync.custom.emdb]
url = "data.pdbj.org::rsync/pub/emdb/"
dest = "data/emdb"
description = "EMDB (Electron Microscopy Data Bank)"
preset = "safe"  # fast, safe, minimal, or conservative

[sync.custom.emdb.options]
max_size = "5G"
exclude = ["obsolete/"]
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

### Sync Command

```
pdb-sync sync [NAME] [OPTIONS]

Arguments:
  [NAME]  Name of custom sync config to run (runs all if not specified)

Options:
  --all                     Run all custom sync configs
  -d, --dest <DIR>          Override destination directory
  --list                    List available custom sync configs
  --fail-fast               Stop on first failure when syncing all configs
  -n, --dry-run             Dry run without changes
  --plan                    Plan mode - show what would change without executing
  --parallel <N>            Maximum number of concurrent sync operations

  # Built-in profiles
  --profile-list            List available profile presets
  --profile-add <NAME>      Add a profile preset to config
  --profile-dry-run         Dry-run for profile add (show what would be added)

  # Retry on failure
  --retry <COUNT>           Number of retry attempts on failure (0 = no retry, default: 0)
  --retry-delay <SECONDS>   Delay between retries in seconds (default: exponential backoff)

  # rsync options
  --delete                  Delete files not present on remote
  --no-delete               Do not delete files (overrides --delete)
  --bwlimit <KBPS>          Bandwidth limit in KB/s
  -z, --compress            Compress data during transfer
  --no-compress             Do not compress (overrides -z/--compress)
  -c, --checksum            Use checksum for file comparison
  --no-checksum             Do not use checksum (overrides -c/--checksum)
  --size-only               Compare by size only, ignore timestamps
  --no-size-only            Do not use size-only comparison
  --ignore-times            Always transfer files, ignoring timestamps
  --no-ignore-times         Do not ignore timestamps
  --modify-window <SECONDS> Timestamp tolerance in seconds
  --partial                 Keep partially transferred files
  --no-partial              Do not keep partial files
  --partial-dir <DIR>       Directory for partial files
  --max-size <SIZE>         Maximum file size to transfer
  --min-size <SIZE>         Minimum file size to transfer
  --timeout <SECONDS>       I/O timeout in seconds
  --contimeout <SECONDS>    Connection timeout in seconds
  --backup                  Create backups
  --no-backup               Do not create backups
  --backup-dir <DIR>        Backup directory
  --chmod <FLAGS>           Change permission flags
  --exclude <PATTERN>       Exclude patterns (repeatable)
  --include <PATTERN>       Include patterns (repeatable)
  --exclude-from <FILE>     File with exclude patterns
  --include-from <FILE>     File with include patterns
  --rsync-verbose           Verbose rsync output
  --no-rsync-verbose        Do not enable verbose output
  --rsync-quiet             Quiet rsync output
  --no-rsync-quiet          Do not enable quiet output
  --itemize-changes         Itemize changes
  --no-itemize-changes      Do not itemize changes

  -v, --verbose             Enable verbose output
  -h, --help                Print help
```

### Config Command

Manage configuration files and presets:

```bash
# Validate config file
pdb-sync config validate

# Migrate old format to new nested format
pdb-sync config migrate

# Show migration preview (dry-run)
pdb-sync config migrate --dry-run

# List available rsync flag presets
pdb-sync config presets
```

### Quick Start with Built-in Profiles

```bash
# List available profile presets
pdb-sync sync --profile-list

# Add a preset to your config (dry-run first)
pdb-sync sync --profile-add structures --profile-dry-run

# Add a preset to your config
pdb-sync sync --profile-add structures
```

### Parallel Execution

```bash
# Run all configs with up to 4 concurrent operations
pdb-sync sync --all --parallel 4

# Combine with retry for robust syncing
pdb-sync sync --all --parallel 4 --retry 3
```

### Plan Mode

```bash
# Preview what would change
pdb-sync sync structures --plan

# Preview changes for all configs
pdb-sync sync --all --plan
```

### Retry on Failure

```bash
# Retry up to 3 times with exponential backoff (1s, 2s, 4s)
pdb-sync sync structures --retry 3

# Retry with fixed delay of 5 seconds
pdb-sync sync structures --retry 3 --retry-delay 5
```

## Configuration

Config file location: `~/.config/pdb-sync/config.toml`

📖 **[Complete Configuration Reference](docs/config-reference.md)** - Detailed documentation for all config options

### Custom Sync Configs

Each custom sync config defines an rsync source. `pdb-sync` supports three configuration styles:

#### Style 1: Preset-only (Easiest)

Use a built-in preset for common rsync flag combinations:

```toml
[sync.custom.structures]
url = "rsync.wwpdb.org::ftp_data/structures/"
dest = "data/structures"
preset = "safe"  # safe, fast, minimal, or conservative
```

#### Style 2: Preset + Overrides (Recommended)

Start with a preset and override specific options:

```toml
[sync.custom.structures]
url = "rsync.wwpdb.org::ftp_data/structures/"
dest = "data/structures"
preset = "fast"

[sync.custom.structures.options]
max_size = "5GB"
exclude = ["obsolete/"]
```

#### Style 3: Fully Custom

Define all options explicitly:

```toml
[sync.custom.sifts]
url = "rsync.wwpdb.org::ftp_data/sifts/"
dest = "data/sifts"

[sync.custom.sifts.options]
delete = true
compress = true
checksum = true
timeout = 300
```

#### Style 4: Legacy Format (Backward Compatible)

The old format with `rsync_` prefix is still supported:

```toml
[sync.custom.legacy]
url = "example.org::data"
dest = "data/legacy"
rsync_delete = true
rsync_compress = true
rsync_checksum = true
```

### Rsync Flag Presets

| Preset | delete | compress | checksum | backup | Use Case |
|--------|--------|----------|----------|--------|----------|
| `safe` | ❌ | ✅ | ✅ | ❌ | First-time sync, cautious users |
| `fast` | ✅ | ✅ | ❌ | ❌ | Regular updates, speed priority |
| `minimal` | ❌ | ❌ | ❌ | ❌ | Bare minimum, full control needed |
| `conservative` | ❌ | ✅ | ✅ | ✅ | Production, maximum safety |

List available presets:

```bash
pdb-sync config presets
```

### Config Priority

When using multiple config styles, priority is: **options > preset > legacy**

```toml
[sync.custom.test]
url = "example.org::data"
dest = "data/test"

# Legacy: delete=false
rsync_delete = false

# Preset "fast": delete=true
preset = "fast"

# Options: delete=false (highest priority)
[sync.custom.test.options]
delete = false
```

Result: `delete = false` (from options)

### Migrating Old Configs

Convert old `rsync_*` format to new nested format:

```bash
# Dry run (show what would change)
pdb-sync config migrate --dry-run

# Actually migrate
pdb-sync config migrate
```

The migration tool will:
1. Detect if flags match a preset → use `preset = "name"`
2. Otherwise → convert to nested `[options]` format
3. Remove `rsync_` prefixes

### Available rsync Options

| Config Field | CLI Flag | Description |
|--------------|----------|-------------|
| `rsync_delete` | --delete / --no-delete | Delete files not present on remote |
| `rsync_compress` | -z, --compress / --no-compress | Compress data during transfer |
| `rsync_checksum` | -c, --checksum / --no-checksum | Use checksum for file comparison |
| `rsync_size_only` | --size-only / --no-size-only | Compare by size only, ignore timestamps |
| `rsync_ignore_times` | --ignore-times / --no-ignore-times | Always transfer files, ignoring timestamps |
| `rsync_modify_window` | --modify-window | Timestamp tolerance in seconds |
| `rsync_partial` | --partial / --no-partial | Keep partially transferred files |
| `rsync_partial_dir` | --partial-dir | Directory for partial files |
| `rsync_max_size` | --max-size | Maximum file size to transfer |
| `rsync_min_size` | --min-size | Minimum file size to transfer |
| `rsync_timeout` | --timeout | I/O timeout in seconds |
| `rsync_contimeout` | --contimeout | Connection timeout in seconds |
| `rsync_backup` | --backup / --no-backup | Create backups |
| `rsync_backup_dir` | --backup-dir | Backup directory |
| `rsync_chmod` | --chmod | Change permission flags |
| `rsync_exclude` | --exclude | Exclude patterns (array) |
| `rsync_include` | --include | Include patterns (array) |
| `rsync_exclude_from` | --exclude-from | File with exclude patterns |
| `rsync_include_from` | --include-from | File with include patterns |
| `rsync_verbose` | --rsync-verbose / --no-rsync-verbose | Verbose output |
| `rsync_quiet` | --rsync-quiet / --no-rsync-quiet | Quiet mode |
| `rsync_itemize_changes` | --itemize-changes / --no-itemize-changes | Itemize changes |
| `rsync_dry_run` | -n, --dry-run | Dry run without changes |

### Example Configs

```toml
[sync]
mirror = "rcsb"

# Standard PDB structures (using preset)
[sync.custom.structures]
url = "rsync.wwpdb.org::ftp_data/structures/divided/mmCIF/"
dest = "data/structures/divided/mmCIF"
description = "PDB structures (mmCIF format, divided layout)"
preset = "fast"

[sync.custom.structures.options]
bwlimit = 5000

# Biological assemblies (using preset with exclusions)
[sync.custom.assemblies]
url = "rsync.wwpdb.org::ftp_data/structures/divided/assemblies/mmCIF/"
dest = "data/assemblies/divided/mmCIF"
description = "Biological assemblies (mmCIF format)"
preset = "fast"

# EMDB data (fully custom)
[sync.custom.emdb]
url = "data.pdbj.org::rsync/pub/emdb/"
dest = "data/emdb"
description = "Electron Microscopy Data Bank"

[sync.custom.emdb.options]
delete = true
compress = true
exclude = ["*.tmp", "test/*"]

# SIFTS data (using preset)
[sync.custom.pdbj-sifts]
url = "ftp.pdbj.org::pub/pdbj/data/sifts/"
dest = "pdbj/sifts"
description = "SIFTS data from PDBj"
preset = "safe"

# Other PDBj directories
[sync.custom.pdbj-bsma]
url = "data.pdbj.org::rsync/pdbj/bsma/"
dest = "pdbj/bsma"
preset = "fast"

[sync.custom.pdbj-mmjson]
url = "data.pdbj.org::rsync/pdbjplus/data/cc/mmjson/"
dest = "pdbj/mmjson"
preset = "fast"

[sync.custom.pdbj-pdbe]
url = "data.pdbj.org::rsync/pdbjplus/data/cc/pdbe/"
dest = "pdbj/pdbe"
preset = "fast"
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
