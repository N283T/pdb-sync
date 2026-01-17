# Phase 2: Command Aliases

## Parallel Development Note
This phase is being developed in parallel with other v3 phases.
- Create PR and request review
- Wait for review approval before merging
- Rebase on master if other phases are merged first

## Goal
Add short aliases for commonly used commands and options.

## Proposed Aliases

### Command Aliases
| Full Command | Alias | Description |
|--------------|-------|-------------|
| `download` | `dl` | Download files |
| `validate` | `val` | Validate files |
| `config` | `cfg` | Configuration |

### DataType Aliases (for --type)
| Full Name | Aliases |
|-----------|---------|
| `structures` | `st`, `struct` |
| `assemblies` | `asm`, `assembly` |
| `structure-factors` | `sf`, `xray` |
| `nmr-chemical-shifts` | `nmr-cs`, `cs` |
| `nmr-restraints` | `nmr-r` |

### Format Aliases
| Full Name | Aliases |
|-----------|---------|
| `mmcif` | `cif` |
| `pdb` | (no alias needed) |
| `bcif` | (no alias needed) |

### Mirror Aliases (already exist, verify)
| Full Name | Aliases |
|-----------|---------|
| `rcsb` | `us` |
| `pdbj` | `jp` |
| `pdbe` | `uk`, `eu` |
| `wwpdb` | `global` |

## Usage Examples

```bash
# Before
pdb-cli download 4hhb --type structures --format mmcif

# After (with aliases)
pdb-cli dl 4hhb -t st -f cif

# Validate
pdb-cli val --fix -P

# Config
pdb-cli cfg show
```

## Implementation Tasks

### 1. Add command aliases in clap

```rust
// src/cli/args.rs
#[derive(Subcommand)]
pub enum Commands {
    #[command(visible_alias = "dl")]
    Download(DownloadArgs),

    #[command(visible_alias = "val")]
    Validate(ValidateArgs),

    #[command(visible_alias = "cfg")]
    Config(ConfigArgs),
}
```

### 2. Add value aliases for enums

```rust
// src/data_types.rs
#[derive(clap::ValueEnum)]
pub enum DataType {
    #[value(alias = "st", alias = "struct")]
    Structures,

    #[value(alias = "asm", alias = "assembly")]
    Assemblies,

    #[value(alias = "sf", alias = "xray")]
    StructureFactors,
    // ...
}
```

### 3. Add format aliases

```rust
// src/files/mod.rs
#[derive(clap::ValueEnum)]
pub enum FileFormat {
    #[value(alias = "cif")]
    Mmcif,
    // ...
}
```

### 4. Update help text
- Show aliases in help output
- Document aliases in README

## Files to Modify

- `src/cli/args.rs` - Add command aliases
- `src/data_types.rs` - Add DataType value aliases
- `src/files/mod.rs` - Add FileFormat aliases
- `src/mirrors/registry.rs` - Verify/add mirror aliases
- `README.md` - Document aliases

## Testing

- Test all command aliases work
- Test all value aliases work
- Verify help text shows aliases
- Test tab completion (if applicable)
