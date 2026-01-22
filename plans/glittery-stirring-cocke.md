# Plan: config validate Command

## Overview
Implement `pdb-sync config validate` to validate configuration files for correctness.

## Requirements

From `plans/plan-config-validate.md`:
- Validate URLs and dest subpaths
- Validate rsync flag consistency (partial_dir requires partial, etc.)
- Optional `--fix` for safe automatic fixes
- Human + JSON output formats
- Follow TDD with 80%+ coverage

## Files to Create

### 1. `src/config/validator.rs`
New module for config validation logic.

**Key structs:**
```rust
pub enum ValidationSeverity {
    Error,   // Blocks operation
    Warning, // Advisory only
}

pub struct ValidationIssue {
    pub severity: ValidationSeverity,
    pub section: String,      // "paths.pdb_dir", "sync.custom[0].url", etc.
    pub message: String,
    pub suggestion: Option<String>,
}

pub struct ValidationResult {
    pub is_valid: bool,
    pub issues: Vec<ValidationIssue>,
}

pub struct ConfigValidator {
    pub config_path: Option<PathBuf>,
}
```

**Key methods:**
- `validate() -> ValidationResult`
- `validate_url(url: &str) -> Option<ValidationIssue>`
- `validate_dest_subpath(dest: &str) -> Option<ValidationIssue>`
- `validate_path_exists(path: &PathBuf, section: &str) -> Option<ValidationIssue>`
- `fix(&mut self, result: &ValidationResult) -> Result<Vec<String>>`

### 2. `src/cli/args/config.rs`
CLI argument definitions for config subcommand.

**Key structs:**
```rust
#[derive(Subcommand)]
pub enum ConfigCommand {
    Validate(ValidateArgs),
}

#[derive(Parser, Clone, Debug)]
pub struct ValidateArgs {
    /// Apply automatic fixes for safe corrections
    #[arg(long)]
    pub fix: bool,

    /// Output format
    #[arg(short, long, default_value = "text")]
    pub format: ValidateOutputFormat,
}

#[derive(ValueEnum, Clone, Copy, Debug, Default)]
pub enum ValidateOutputFormat {
    #[default]
    Text,
    Json,
}
```

### 3. `src/cli/commands/config/validate.rs`
Command handler for config validate.

**Key function:**
```rust
pub async fn run_validate(args: ValidateArgs, ctx: AppContext) -> Result<()>
```

**Text output format:**
```
Config validation: 2 errors, 3 warnings

Errors:
  [E001] sync.custom[0].url: Invalid rsync URL format
         URL should start with one of: rsync://, host::module
         Suggestion: Use format like "rsync://host/module" or "host::module"

  [E002] sync.custom[0].dest: Path traversal detected
         Destination cannot contain '..'

Warnings:
  [W001] paths.pdb_dir: Directory does not exist
         The configured directory will not be found
         Suggestion: Create the directory or update the path

  [W002] sync.custom[1].rsync_max_size: Invalid size format
         Size format should be: number + optional suffix (K, M, G, T, P)
         Suggestion: Use "1G" instead of "1gb"

  [W003] sync.custom[2].rsync_chmod: Potentially dangerous chmod
         chmod string contains shell metacharacters
```

**JSON output format:**
```json
{
  "valid": false,
  "errors": [
    {
      "code": "E001",
      "section": "sync.custom[0].url",
      "message": "Invalid rsync URL format",
      "suggestion": "Use format like \"rsync://host/module\" or \"host::module\""
    }
  ],
  "warnings": [
    {
      "code": "W001",
      "section": "paths.pdb_dir",
      "message": "Directory does not exist",
      "suggestion": "Create the directory or update the path"
    }
  ]
}
```

## Files to Modify

### 1. `src/cli/args/mod.rs`
Add config argument module:
```rust
pub mod config;
```

### 2. `src/cli/args/global.rs`
Add ConfigCommand to SyncCommand enum:
```rust
#[derive(Subcommand)]
pub enum Commands {
    /// Sync from a configured source (runs all if no name specified)
    Sync(SyncArgs),
    /// Configuration management
    Config(ConfigCommand),
}
```

### 3. `src/main.rs`
Add config command dispatch:
```rust
match cli.command {
    Commands::Sync(args) => { ... }
    Commands::Config(ConfigCommand::Validate(args)) => {
        cli::args::config::run_validate(args, ctx).await?;
    }
}
```

### 4. `src/config/mod.rs`
Add validator module:
```rust
pub mod validator;
```

## Implementation Steps

### Phase 1: Validation Module (TDD - RED)

1. Write tests in `src/config/validator.rs`:
   - URL validation tests (valid rsync URLs, invalid formats)
   - Subpath validation tests (valid paths, path traversal)
   - Path existence validation
   - Rsync flag validation (reuse `RsyncFlags::validate()`)
   - Fix functionality tests

2. Implement `ConfigValidator` struct:
   - URL validation: Check for `rsync://` or `host::module` format
   - Subpath validation: Reuse `validate_subpath()` from `sync/common.rs`
   - Path validation: Check directory existence
   - Flag validation: Call `RsyncFlags::validate()` via `to_rsync_flags()`

### Phase 2: CLI Arguments

1. Add `src/cli/args/config.rs` with `ValidateArgs`
2. Modify `src/cli/args/mod.rs` to export config module
3. Modify `src/cli/args/global.rs` to add Config subcommand

### Phase 3: Command Handler

1. Create `src/cli/commands/config/mod.rs`
2. Create `src/cli/commands/config/validate.rs`
3. Implement `run_validate()`:
   - Load config using `ConfigLoader::load()`
   - Run validation
   - Apply fixes if `--fix` flag is set
   - Format output (text or JSON)

### Phase 4: Integration

1. Update `src/main.rs` to dispatch config commands
2. Add error handling for config file not found (should not error, just use defaults)

## URL Validation Rules

**Format-only validation** (no network connectivity checks):
- Validate URL syntax, not reachability
- Acceptable for offline use

Rsync URL formats to accept:
- `host::module/path` (rsync daemon format)
- `rsync://host/module/path` (rsync:// URL format)
- `user@host::module/path` (with username)

Rsync URL formats to reject:
- Missing module path (e.g., `host::` only)
- Invalid characters
- Empty string

## Data Type Validation

**Validate against known DataType enum values:**
- Known values: `structures`, `assemblies`, `structure-factors`, `etc`
- Accept aliases (e.g., `st` -> `structures`, `asm` -> `assemblies`)
- Report error for unknown data types
- `--fix` will replace invalid values with closest valid alias

## Fix Behavior

The `--fix` flag will apply **safe** automatic fixes directly to the config file.

**Fixes to apply:**
1. Normalize paths (remove trailing slashes, expand `~`)
2. Normalize size strings (e.g., `100MB` -> `100M`)
3. Remove duplicate entries in arrays
4. Fix invalid data_type values (replace with closest valid alias)

**Safety measures:**
- Create backup of original config before modifying
- Show exactly what changes were made
- Only apply fixes that are unambiguous

**Fixes NOT to apply:**
- URL corrections (require user understanding)
- Path creation (requires filesystem changes)
- Flag dependency fixes (require user intent)

## Test Coverage Requirements

**Unit tests (80%+ coverage):**
- `validator.rs`: URL validation, subpath validation, path checks
- `config/validate.rs`: Output formatting

**Integration tests:**
- Test with valid config file
- Test with invalid URL
- Test with path traversal
- Test with rsync flag inconsistencies
- Test `--fix` functionality

**Test fixtures:**
Create `tests/fixtures/config/` directory with:
- `valid.toml` - Valid config
- `invalid-url.toml` - Invalid rsync URL
- `path-traversal.toml` - Dest with `..`
- `flag-inconsistency.toml` - partial_dir without partial

## Verification

After implementation, test manually:

```bash
# Validate current config
cargo run -- config validate

# Validate with JSON output
cargo run -- config validate --format json

# Validate and fix safe issues (creates backup)
cargo run -- config validate --fix

# Test with invalid config file
PDB_SYNC_CONFIG=/path/to/invalid.toml cargo run -- config validate

# Test offline (no network calls)
cargo run -- config validate
```

## Risks & Mitigations

1. **Overly strict URL validation** may reject valid rsync URLs
   - Mitigation: Start with basic format checks, warnings vs errors; format-only (no network)

2. **Path existence check may fail in CI environments**
   - Mitigation: Path existence is a warning, not an error

3. **Fix behavior may overwrite user intent**
   - Mitigation: Create backup, show exactly what changed, only apply safe fixes

4. **Data type validation may reject future custom data types**
   - Mitigation: Use warnings for unknown types to allow extensibility
