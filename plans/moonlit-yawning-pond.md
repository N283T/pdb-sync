# Phase 10: Convert Command Implementation Plan

## Overview

Add a `convert` command for format conversion and compression management of PDB files.

**Branch:** `feature/v3-phase10`
**Plan file:** `plans/v3-phase10-convert.md`

## Usage Examples

```bash
# Decompress files
pdb-cli convert 4hhb.cif.gz --decompress
pdb-cli convert *.cif.gz --decompress --dest ./uncompressed/

# Compress files
pdb-cli convert 4hhb.cif --compress
pdb-cli convert *.cif --compress --dest ./compressed/

# Format conversion (requires gemmi)
pdb-cli convert 4hhb.cif --to pdb
pdb-cli convert 4hhb.pdb --to cif

# Batch from stdin
pdb-cli list -o ids | pdb-cli convert --stdin --to pdb

# In-place conversion
pdb-cli convert 4hhb.cif.gz --decompress --in-place
```

## Files to Create

| File | Purpose |
|------|---------|
| `src/convert/mod.rs` | Module definition, `Converter` struct, `ConvertTask`, `ConvertResult` |
| `src/convert/compress.rs` | `is_gzipped()`, `decompress_file()`, `compress_file()` |
| `src/convert/format.rs` | `check_gemmi_available()`, `convert_with_gemmi()` |
| `src/cli/commands/convert.rs` | `run_convert()` command handler |

## Files to Modify

| File | Changes |
|------|---------|
| `src/cli/args.rs` | Add `ConvertArgs` struct, `Commands::Convert` variant |
| `src/cli/commands/mod.rs` | Add `pub mod convert;` and `pub use convert::run_convert;` |
| `src/main.rs` | Add `Commands::Convert(args) => run_convert(args, ctx).await?;` |
| `src/error.rs` | Add `Conversion(String)`, `ToolNotFound(String)` variants |

## Implementation Steps (TDD Order)

### Step 1: Add Error Variants

**File:** `src/error.rs`

Add to `PdbCliError` enum:
```rust
#[error("Conversion error: {0}")]
Conversion(String),

#[error("External tool not found: {0}")]
ToolNotFound(String),
```

### Step 2: Create Compression Module

**File:** `src/convert/compress.rs`

```rust
use crate::error::Result;
use async_compression::tokio::bufread::GzipDecoder;
use async_compression::tokio::write::GzipEncoder;
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};

/// Check if file starts with gzip magic bytes (0x1f 0x8b)
pub async fn is_gzipped(path: &Path) -> std::io::Result<bool> {
    let mut file = File::open(path).await?;
    let mut magic = [0u8; 2];
    if file.read_exact(&mut magic).await.is_ok() {
        Ok(magic == [0x1f, 0x8b])
    } else {
        Ok(false)
    }
}

/// Decompress a gzip file
pub async fn decompress_file(src: &Path, dest: &Path) -> Result<()> {
    let file = File::open(src).await?;
    let reader = BufReader::new(file);
    let mut decoder = GzipDecoder::new(reader);
    let mut output = File::create(dest).await?;
    tokio::io::copy(&mut decoder, &mut output).await?;
    output.flush().await?;
    Ok(())
}

/// Compress a file to gzip format
pub async fn compress_file(src: &Path, dest: &Path) -> Result<()> {
    let file = File::open(src).await?;
    let reader = BufReader::new(file);
    let mut encoder = GzipEncoder::new(reader);
    let mut output = File::create(dest).await?;
    tokio::io::copy(&mut encoder, &mut output).await?;
    output.flush().await?;
    Ok(())
}
```

### Step 3: Create Format Conversion Module

**File:** `src/convert/format.rs`

```rust
use crate::error::{PdbCliError, Result};
use crate::files::FileFormat;
use std::path::Path;
use tokio::process::Command;

/// Check if gemmi CLI is available
pub async fn check_gemmi_available() -> bool {
    Command::new("gemmi")
        .arg("--version")
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Convert file format using gemmi
pub async fn convert_with_gemmi(
    src: &Path,
    dest: &Path,
    to_format: FileFormat,
) -> Result<()> {
    if !check_gemmi_available().await {
        return Err(PdbCliError::ToolNotFound(
            "gemmi not found. Install with: pip install gemmi".into()
        ));
    }

    let mut cmd = Command::new("gemmi");
    cmd.arg("convert").arg(src).arg(dest);

    let output = cmd.output().await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PdbCliError::Conversion(format!("gemmi failed: {}", stderr)));
    }
    Ok(())
}
```

### Step 4: Create Convert Orchestration Module

**File:** `src/convert/mod.rs`

Key types:
- `ConvertTask`: Source path, target format, options
- `ConvertResult`: Success/Failed/Skipped with details
- `Converter`: Semaphore-controlled parallel execution

### Step 5: Add CLI Arguments

**File:** `src/cli/args.rs`

```rust
#[derive(Parser)]
pub struct ConvertArgs {
    /// Files to convert (paths or PDB IDs)
    pub files: Vec<String>,

    /// Decompress .gz files
    #[arg(long, conflicts_with = "compress")]
    pub decompress: bool,

    /// Compress files to .gz format
    #[arg(long, conflicts_with = "decompress")]
    pub compress: bool,

    /// Target format (requires gemmi for format conversion)
    #[arg(long, value_enum)]
    pub to: Option<FileFormat>,

    /// Source format filter for batch mode
    #[arg(long, value_enum)]
    pub from: Option<FileFormat>,

    /// Destination directory
    #[arg(short, long)]
    pub dest: Option<PathBuf>,

    /// Replace original files
    #[arg(long)]
    pub in_place: bool,

    /// Read paths/IDs from stdin
    #[arg(long)]
    pub stdin: bool,

    /// Number of parallel conversions
    #[arg(short, long, default_value = "4")]
    pub parallel: u8,
}
```

Add to `Commands` enum:
```rust
/// Convert file formats and manage compression
Convert(ConvertArgs),
```

### Step 6: Create Command Handler

**File:** `src/cli/commands/convert.rs`

```rust
pub async fn run_convert(args: ConvertArgs, ctx: AppContext) -> Result<()> {
    // 1. Collect input files (args, stdin, batch scan)
    // 2. Build ConvertTask for each file
    // 3. Execute with Converter (parallel semaphore)
    // 4. Report results summary
}
```

### Step 7: Wire Up Main

**File:** `src/main.rs`

Add match arm:
```rust
Commands::Convert(args) => {
    cli::commands::run_convert(args, ctx).await?;
}
```

## Test Cases

### Unit Tests

| Module | Test | Description |
|--------|------|-------------|
| `compress.rs` | `test_is_gzipped_true` | Detect gzip magic bytes |
| `compress.rs` | `test_is_gzipped_false` | Detect non-gzipped files |
| `compress.rs` | `test_decompress_file` | Decompress .gz to plain |
| `compress.rs` | `test_compress_file` | Compress plain to .gz |
| `format.rs` | `test_gemmi_not_found` | Graceful error when gemmi missing |
| `convert.rs` | `test_detect_format` | Detect format from filename |

### Integration Tests

| Test | Description |
|------|-------------|
| `test_decompress_cifgz` | End-to-end decompress .cif.gz |
| `test_compress_cif` | End-to-end compress .cif |
| `test_in_place_decompress` | In-place replaces original |
| `test_parallel_conversion` | Multiple files in parallel |

## Verification Plan

1. **Unit Tests**
   ```bash
   cargo test convert
   ```

2. **Manual Testing**
   ```bash
   # Create test file
   echo "test content" > /tmp/test.txt
   gzip -k /tmp/test.txt

   # Test decompress
   cargo run -- convert /tmp/test.txt.gz --decompress --dest /tmp/out/

   # Test compress
   cargo run -- convert /tmp/test.txt --compress --dest /tmp/out/

   # Test in-place
   cargo run -- convert /tmp/test.txt.gz --decompress --in-place
   ```

3. **Quality Checks**
   ```bash
   cargo fmt --check
   cargo clippy -- -D warnings
   cargo test
   ```

## Dependencies

Already available in `Cargo.toml`:
- `async-compression = { version = "0.4", features = ["tokio", "gzip"] }`
- `tokio` with `fs`, `process`, `sync` features
- `futures-util` for `join_all`

Optional external tool:
- `gemmi` CLI for format conversion (not required for compression)

## Notes

- Compression/decompression is built-in using `async-compression`
- Format conversion (PDB <-> mmCIF) requires external `gemmi` tool
- Follows existing patterns from `download` command for parallel execution
- Error handling follows existing `PdbCliError` patterns
