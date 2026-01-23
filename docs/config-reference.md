# config.toml Reference

Complete reference documentation for pdb-sync configuration file `config.toml`.

## Table of Contents

- [File Location](#file-location)
- [Basic Structure](#basic-structure)
- [paths Section](#paths-section)
- [sync Section](#sync-section)
- [sync.custom.NAME Section](#synccustomname-section)
- [sync.custom.NAME.options Section](#synccustomnameoptions-section)
- [mirror_selection Section](#mirror_selection-section)
- [Preset Reference](#preset-reference)
- [Priority Rules](#priority-rules)
- [Configuration Examples](#configuration-examples)
- [Environment Variables](#environment-variables)

---

## File Location

Default: `~/.config/pdb-sync/config.toml`

Can be overridden with `PDB_SYNC_CONFIG` environment variable:
```bash
export PDB_SYNC_CONFIG=/path/to/custom/config.toml
pdb-sync sync
```

---

## Basic Structure

```toml
[paths]
pdb_dir = "/data/pdb"

# Global defaults for all custom configs
[sync.defaults]
compress = true
timeout = 300

[sync.custom.structures]
url = "rsync.wwpdb.org::ftp_data/structures/divided/mmCIF/"
dest = "data/structures"
preset = "fast"

[mirror_selection]
auto_select = false
preferred_region = "us"
latency_cache_ttl = 3600
```

---

## paths Section

Configure where PDB data is stored.

### `pdb_dir`

**Type**: String (path)
**Default**: None (required)
**Description**: Base directory for PDB data storage

```toml
[paths]
pdb_dir = "/mnt/data/pdb"
```

### `data_type_dirs`

**Type**: HashMap<String, String>
**Default**: `{}`
**Description**: Specify different directories per data type

```toml
[paths.data_type_dirs]
structures = "/mnt/ssd/pdb/structures"
assemblies = "/mnt/hdd/pdb/assemblies"
```

---

## sync Section

General sync configuration.

### `defaults`

**Type**: Table (RsyncOptionsConfig)
**Default**: None
**Description**: Global default rsync options for all custom configs

Set common options once instead of repeating in every config (DRY principle).

**Priority**: `options > preset > defaults > legacy fields`

```toml
[sync.defaults]
compress = true
timeout = 300
partial = true
delete = false
max_size = "10G"
```

All fields are optional. See [sync.custom.NAME.options](#synccustomnameoptions-section) for available fields.

---

## sync.custom.NAME Section

Define custom rsync configurations. Uses **HashMap format**, so the `name` field is not needed.

### Basic Fields

#### `url`

**Type**: String
**Required**: ✅
**Description**: rsync URL

Supported formats:
- `rsync://` protocol: `rsync://rsync.ebi.ac.uk/pub/databases/msd/sifts/`
- `::` format: `data.pdbj.org::rsync/pub/emdb/`

```toml
[sync.custom.emdb]
url = "data.pdbj.org::rsync/pub/emdb/"
```

#### `dest`

**Type**: String
**Required**: ✅
**Description**: Relative path from `pdb_dir`

```toml
[sync.custom.emdb]
dest = "data/emdb"  # Saved to /data/pdb/data/emdb
```

#### `description`

**Type**: String
**Default**: None
**Description**: Configuration description (shown in `--list`)

```toml
[sync.custom.emdb]
description = "EMDB (Electron Microscopy Data Bank)"
```

#### `preset`

**Type**: String
**Default**: None
**Choices**: `safe`, `fast`, `minimal`, `conservative`
**Description**: rsync flag preset (see [Preset Reference](#preset-reference))

```toml
[sync.custom.structures]
url = "rsync.wwpdb.org::ftp_data/structures/"
dest = "data/structures"
preset = "fast"
```

---

## sync.custom.NAME.options Section

Configure individual rsync options. Takes priority over presets.

### Boolean Flags

All **Optional**. If not explicitly specified, uses preset or default values.

#### `delete`

**Type**: Boolean
**Default**: `false`
**Description**: Delete files not present on remote

```toml
[sync.custom.structures.options]
delete = true
```

#### `compress`

**Type**: Boolean
**Default**: `false`
**Description**: Compress data during transfer

```toml
[sync.custom.structures.options]
compress = true
```

#### `checksum`

**Type**: Boolean
**Default**: `false`
**Description**: Compare files by checksum (not timestamp)

```toml
[sync.custom.structures.options]
checksum = true
```

#### `size_only`

**Type**: Boolean
**Default**: `false`
**Description**: Compare files by size only, ignoring timestamps

```toml
[sync.custom.structures.options]
size_only = true
```

#### `ignore_times`

**Type**: Boolean
**Default**: `false`
**Description**: Always transfer files, ignoring timestamps

```toml
[sync.custom.structures.options]
ignore_times = true
```

#### `modify_window`

**Type**: Integer (seconds)
**Default**: None
**Description**: Timestamp tolerance in seconds for comparison

```toml
[sync.custom.structures.options]
modify_window = 2  # Allow 2-second difference in timestamps
```

#### `partial`

**Type**: Boolean
**Default**: `false`
**Description**: Keep partially transferred files (resumable)

```toml
[sync.custom.structures.options]
partial = true
```

#### `backup`

**Type**: Boolean
**Default**: `false`
**Description**: Create backups before overwriting

```toml
[sync.custom.structures.options]
backup = true
backup_dir = ".backup"
```

#### `verbose`

**Type**: Boolean
**Default**: `false`
**Description**: Verbose rsync output

```toml
[sync.custom.structures.options]
verbose = true
```

#### `quiet`

**Type**: Boolean
**Default**: `false`
**Description**: Quiet rsync output (mutually exclusive with `verbose`)

```toml
[sync.custom.structures.options]
quiet = true
```

#### `itemize_changes`

**Type**: Boolean
**Default**: `false`
**Description**: Itemize changes in output

```toml
[sync.custom.structures.options]
itemize_changes = true
```

### String / Integer Options

#### `partial_dir`

**Type**: String
**Default**: None
**Description**: Directory for partial files (requires `partial = true`)

```toml
[sync.custom.structures.options]
partial = true
partial_dir = ".rsync-partial"
```

#### `backup_dir`

**Type**: String
**Default**: None
**Description**: Backup directory (requires `backup = true`)

```toml
[sync.custom.structures.options]
backup = true
backup_dir = ".backup"
```

#### `max_size`

**Type**: String
**Default**: None
**Description**: Maximum file size to transfer

Format: `5GB`, `500MB`, `1024K`

```toml
[sync.custom.structures.options]
max_size = "5GB"
```

#### `min_size`

**Type**: String
**Default**: None
**Description**: Minimum file size to transfer

```toml
[sync.custom.structures.options]
min_size = "1K"
```

#### `timeout`

**Type**: Integer (seconds)
**Default**: None
**Description**: I/O timeout

```toml
[sync.custom.structures.options]
timeout = 300
```

#### `contimeout`

**Type**: Integer (seconds)
**Default**: None
**Description**: Connection timeout

```toml
[sync.custom.structures.options]
contimeout = 30
```

#### `chmod`

**Type**: String
**Default**: None
**Description**: Permission change flags

```toml
[sync.custom.structures.options]
chmod = "644"
```

### Array Options

#### `exclude`

**Type**: Array of String
**Default**: `[]`
**Description**: Exclusion patterns (rsync glob format)

```toml
[sync.custom.structures.options]
exclude = ["obsolete/", "*.tmp", "test/*"]
```

#### `include`

**Type**: Array of String
**Default**: `[]`
**Description**: Inclusion patterns

```toml
[sync.custom.structures.options]
include = ["*.cif.gz"]
exclude = ["*"]  # Exclude everything except includes
```

#### `exclude_from`

**Type**: String
**Default**: None
**Description**: File path containing exclusion patterns

```toml
[sync.custom.structures.options]
exclude_from = "/path/to/exclude.txt"
```

#### `include_from`

**Type**: String
**Default**: None
**Description**: File path containing inclusion patterns

```toml
[sync.custom.structures.options]
include_from = "/path/to/include.txt"
```

---

## mirror_selection Section

Configure automatic mirror selection.

### `auto_select`

**Type**: Boolean
**Default**: `false`
**Description**: Enable automatic mirror selection based on latency

```toml
[mirror_selection]
auto_select = true
```

### `preferred_region`

**Type**: String
**Default**: None
**Choices**: `us`, `jp`, `europe`
**Description**: Preferred region (prioritized within 2x latency tolerance)

```toml
[mirror_selection]
auto_select = true
preferred_region = "jp"
```

### `latency_cache_ttl`

**Type**: Integer (seconds)
**Default**: `3600` (1 hour)
**Description**: Latency cache TTL

```toml
[mirror_selection]
latency_cache_ttl = 7200  # 2 hours
```

---

## Preset Reference

### `safe` (Safety First)

For first-time sync or cautious users.

| Option | Value |
|--------|-------|
| `delete` | ❌ `false` |
| `compress` | ✅ `true` |
| `checksum` | ✅ `true` |
| `partial` | ✅ `true` |
| `backup` | ❌ `false` |
| `verbose` | ✅ `true` |
| `quiet` | ❌ `false` |

**Use case**: Prevent accidental deletion, ensure reliable sync

```toml
[sync.custom.structures]
url = "rsync.wwpdb.org::ftp_data/structures/"
dest = "data/structures"
preset = "safe"
```

### `fast` (Speed Priority)

For regular updates prioritizing speed.

| Option | Value |
|--------|-------|
| `delete` | ✅ `true` |
| `compress` | ✅ `true` |
| `checksum` | ❌ `false` |
| `partial` | ✅ `true` |
| `backup` | ❌ `false` |
| `verbose` | ❌ `false` |
| `quiet` | ✅ `true` |

**Use case**: Daily scheduled sync, maintain complete mirror

```toml
[sync.custom.structures]
preset = "fast"
```

### `minimal` (Minimal Settings)

Minimal configuration for full control.

| Option | Value |
|--------|-------|
| `delete` | ❌ `false` |
| `compress` | ❌ `false` |
| `checksum` | ❌ `false` |
| `partial` | ❌ `false` |
| `backup` | ❌ `false` |
| `verbose` | ❌ `false` |
| `quiet` | ❌ `false` |

**Use case**: Fine-grained control with custom options

```toml
[sync.custom.structures]
preset = "minimal"

[sync.custom.structures.options]
# Add only needed options
delete = true
timeout = 600
```

### `conservative` (Maximum Safety)

Maximum safety for production environments.

| Option | Value |
|--------|-------|
| `delete` | ❌ `false` |
| `compress` | ✅ `true` |
| `checksum` | ✅ `true` |
| `partial` | ✅ `true` |
| `backup` | ✅ `true` |
| `verbose` | ✅ `true` |
| `quiet` | ❌ `false` |

**Use case**: Production servers, avoid data loss

```toml
[sync.custom.structures]
preset = "conservative"

[sync.custom.structures.options]
backup_dir = ".backup"
```

---

## Priority Rules

When multiple configuration methods are combined:

**options > preset > legacy > defaults**

### Example: Delete Flag Resolution

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

**Result**: `delete = false` (from options)

### Priority Details

1. **CLI arguments** (highest priority)
   ```bash
   pdb-sync sync structures --delete
   ```

2. **options section**
   ```toml
   [sync.custom.structures.options]
   delete = true
   ```

3. **preset**
   ```toml
   [sync.custom.structures]
   preset = "fast"  # delete=true
   ```

4. **legacy fields** (backward compatibility)
   ```toml
   [sync.custom.structures]
   rsync_delete = true
   ```

5. **defaults**
   - `delete = false`
   - `compress = false`
   - etc.

---

## Configuration Examples

### 1. Simple Configuration (Preset Only)

```toml
[paths]
pdb_dir = "/data/pdb"

[sync.custom.structures]
url = "rsync.wwpdb.org::ftp_data/structures/divided/mmCIF/"
dest = "data/structures"
preset = "fast"
```

### 2. Multiple Data Sources + Preset Overrides

```toml
[paths]
pdb_dir = "/data/pdb"

# Structure data (speed priority)
[sync.custom.structures]
url = "rsync.wwpdb.org::ftp_data/structures/divided/mmCIF/"
dest = "data/structures/mmCIF"
description = "PDB structures (mmCIF format)"
preset = "fast"

[sync.custom.structures.options]
max_size = "10GB"
exclude = ["obsolete/"]

# EMDB (safety + size limit)
[sync.custom.emdb]
url = "data.pdbj.org::rsync/pub/emdb/"
dest = "data/emdb"
description = "Electron Microscopy Data Bank"
preset = "safe"

[sync.custom.emdb.options]
max_size = "5GB"
timeout = 600

# SIFTS (custom settings)
[sync.custom.sifts]
url = "ftp.pdbj.org::pub/pdbj/data/sifts/"
dest = "pdbj/sifts"
description = "SIFTS data from PDBj"

[sync.custom.sifts.options]
delete = true
compress = true
checksum = true
exclude = ["*.tmp", "test/*"]
```

### 3. Production Environment (Conservative)

```toml
[paths]
pdb_dir = "/mnt/storage/pdb"

[sync.defaults]
delete = false

[mirror_selection]
auto_select = true
preferred_region = "us"
latency_cache_ttl = 7200

[sync.custom.structures]
url = "rsync.wwpdb.org::ftp_data/structures/divided/mmCIF/"
dest = "structures/mmCIF"
preset = "conservative"

[sync.custom.structures.options]
backup_dir = "/backup/pdb/structures"
timeout = 1800
partial = true
partial_dir = ".rsync-partial"
itemize_changes = true
```

### 4. Development Environment (Minimal + Verbose)

```toml
[paths]
pdb_dir = "/home/user/dev/pdb-data"

[sync.custom.test-structures]
url = "rsync.wwpdb.org::ftp_data/structures/divided/mmCIF/"
dest = "structures"
preset = "minimal"

[sync.custom.test-structures.options]
max_size = "100MB"  # Small files only for testing
verbose = true
itemize_changes = true
exclude = ["obsolete/"]
```

### 5. Parallel Execution Optimized

```toml
[paths]
pdb_dir = "/data/pdb"

# Small files (recommend parallel 10)
[sync.custom.structures]
url = "rsync.wwpdb.org::ftp_data/structures/divided/mmCIF/"
dest = "structures"
preset = "fast"

[sync.custom.structures.options]
timeout = 300

# Large files (recommend parallel 2-4)
[sync.custom.emdb]
url = "data.pdbj.org::rsync/pub/emdb/"
dest = "emdb"
preset = "fast"

[sync.custom.emdb.options]
timeout = 3600
bwlimit = 5000  # Individual bandwidth limit
```

Run:
```bash
pdb-sync sync --all --parallel 10
```

---

## Environment Variables

Config file values can be overridden by environment variables.

### `PDB_DIR`

**Description**: PDB data directory
**Priority**: CLI args > env vars > config.toml

```bash
export PDB_DIR=/mnt/data/pdb
pdb-sync sync structures
```

### `PDB_SYNC_CONFIG`

**Description**: Config file path
**Default**: `~/.config/pdb-sync/config.toml`

```bash
export PDB_SYNC_CONFIG=/etc/pdb-sync/config.toml
pdb-sync sync
```

### Priority Order

```
CLI args > Environment variables > config.toml > Defaults
```

Example:
```bash
# Overrides pdb_dir = "/data/pdb" in config.toml
export PDB_DIR=/tmp/pdb
pdb-sync sync structures

# CLI args override both
pdb-sync sync structures --dest /override/path
```

---

## Troubleshooting

### Validate Configuration

```bash
pdb-sync config validate
```

### Check Configuration

```bash
# List custom configs
pdb-sync sync --list

# List presets
pdb-sync config presets
```

### Common Errors

#### Error: "Config name cannot contain spaces"

**Cause**: HashMap key (config name) contains spaces

**Before**:
```toml
[sync.custom."my structures"]  # NG
```

**After**:
```toml
[sync.custom.my-structures]  # OK
```

#### Error: "partial_dir is set but partial is false"

**Cause**: `partial_dir` specified without `partial = true`

**Fix**:
```toml
[sync.custom.structures.options]
partial = true
partial_dir = ".rsync-partial"
```

#### Error: "verbose and quiet are both true"

**Cause**: Mutually exclusive options both enabled

**Fix**:
```toml
[sync.custom.structures.options]
verbose = true
# quiet = true  # Remove
```

---

## Related Commands

```bash
# Validate configuration
pdb-sync config validate

# List presets
pdb-sync config presets

# List custom configs
pdb-sync sync --list

# Dry run (preview without executing)
pdb-sync sync structures --dry-run

# Plan mode (preview changes)
pdb-sync sync structures --plan
```

---

**Last Updated**: 2026-01-23
**Version**: pdb-sync v0.1.0
