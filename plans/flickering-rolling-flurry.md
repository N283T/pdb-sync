# Phase 6: Validate Command Implementation Plan

## Overview

Add `validate` command to verify local PDB files against checksums from mirrors, with corruption detection and auto-repair.

## Files to Create

| File | Purpose |
|------|---------|
| `src/validation/mod.rs` | Module exports |
| `src/validation/checksum.rs` | ChecksumVerifier, calculate_md5(), VerifyResult |
| `src/cli/commands/validate.rs` | run_validate() command implementation |

## Files to Modify

| File | Changes |
|------|---------|
| `Cargo.toml` | Add `md-5 = "0.10"` dependency |
| `src/error.rs` | Add `ChecksumMismatch`, `ChecksumFetch` variants |
| `src/cli/args.rs` | Add `ValidateArgs`, `Commands::Validate` |
| `src/cli/commands/mod.rs` | Export validate module |
| `src/main.rs` | Add `mod validation`, dispatch to run_validate |

## Implementation Steps

### Step 1: Add md-5 Dependency
```toml
# Cargo.toml
md-5 = "0.10"
```
Verify: `cargo check`

### Step 2: Add Error Variants
```rust
// src/error.rs
#[error("Checksum mismatch for {0}: expected {1}, got {2}")]
ChecksumMismatch(String, String, String),

#[error("Checksum fetch failed: {0}")]
ChecksumFetch(String),
```

### Step 3: Create validation Module

**src/validation/checksum.rs:**
- `VerifyResult` enum: `Valid`, `Invalid { expected, actual }`, `Missing`, `NoChecksum`
- `ChecksumVerifier` struct with reqwest client and HashMap cache
- `fetch_checksums()` - GET `{mirror}/CHECKSUMS`
- `parse_checksums()` - Parse both formats:
  - Format 1: `MD5 (filename) = hash`
  - Format 2: `hash  filename`
- `calculate_md5(path)` - Async MD5 with 8KB buffer

### Step 4: Add ValidateArgs
```rust
// src/cli/args.rs
#[derive(Parser)]
pub struct ValidateArgs {
    pub pdb_ids: Vec<String>,           // Empty = all local files
    #[arg(short = 't', long = "type", value_enum)]
    pub data_type: Option<DataType>,
    #[arg(short, long, value_enum)]
    pub format: Option<FileFormat>,
    #[arg(long)]
    pub fix: bool,                       // Re-download corrupted
    #[arg(short = 'P', long)]
    pub progress: bool,
    #[arg(long)]
    pub errors_only: bool,
    #[arg(short, long, value_enum)]
    pub mirror: Option<MirrorId>,
}
```

### Step 5: Implement validate Command

**src/cli/commands/validate.rs:**
- `run_validate(args, ctx)` - Main entry point
- `scan_local_files()` - Discover all PDB files in mirror directory
- Use `indicatif::ProgressBar` for progress display
- Reuse `HttpsDownloader` for `--fix` option

### Step 6: Wire up Command Dispatch
```rust
// src/main.rs
mod validation;

// In match:
Commands::Validate(args) => {
    cli::commands::run_validate(args, ctx).await?;
}
```

### Step 7: Add Unit Tests

```rust
// src/validation/checksum.rs tests
#[tokio::test] test_calculate_md5()
#[test] test_parse_checksum_format1()
#[test] test_parse_checksum_format2()
```

## Usage Examples

```bash
# Validate all local files
pdb-cli validate --progress

# Validate specific IDs
pdb-cli validate 1abc 2xyz 3def

# Show only errors
pdb-cli validate --errors-only

# Fix corrupted files
pdb-cli validate --fix

# Specific format
pdb-cli validate --format cif-gz
```

## Verification

```bash
# After each step
cargo check

# Final verification
cargo build
cargo test
cargo clippy
cargo run -- validate --help
```

## Edge Cases

| Scenario | Handling |
|----------|----------|
| File missing | `VerifyResult::Missing` |
| CHECKSUMS 404 | Treat as `NoChecksum`, continue |
| Network error | Log warning, treat as `NoChecksum` |
| Invalid PDB ID | Skip with warning |
| --fix network error | Per-file error, continue others |
