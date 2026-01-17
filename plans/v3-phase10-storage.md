# Phase 10: Storage Command

## Parallel Development Note
This phase is being developed in parallel with other v3 phases.
- Create PR and request review
- Wait for review approval before merging
- Rebase on master if other phases are merged first

## Goal
Add `storage` command for managing local PDB collection storage (compression/decompression).

## Usage Examples

```bash
# Show storage status
pdb-cli storage status
# Output:
# Local PDB Storage
# =================
# Total files: 12,345
# Compressed: 10,234 (38.2 GB)
# Uncompressed: 2,111 (85.4 GB)
# Potential savings: 47.2 GB (if all compressed)

# Compress all uncompressed files
pdb-cli storage compress
pdb-cli storage compress --progress

# Decompress all compressed files
pdb-cli storage decompress
pdb-cli storage decompress -P

# Compress/decompress specific data types
pdb-cli storage compress --type structures
pdb-cli storage decompress --type assemblies

# Compress/decompress specific formats
pdb-cli storage compress --format cif
pdb-cli storage decompress --format pdb

# Dry run
pdb-cli storage compress --dry-run
pdb-cli storage decompress -n

# Parallel processing
pdb-cli storage compress --parallel 8

# Clean up (remove both compressed and uncompressed duplicates)
pdb-cli storage dedupe
# Keeps compressed version by default
pdb-cli storage dedupe --keep uncompressed
```

## Subcommands

### `storage status`
Show current storage statistics:
- Total files count
- Compressed vs uncompressed breakdown
- Size comparison
- Potential savings

### `storage compress`
Compress uncompressed files (.cif → .cif.gz, .pdb → .ent.gz):
- Process entire collection or filter by type/format
- Parallel processing
- Progress display
- Remove original after successful compression

### `storage decompress`
Decompress compressed files (.cif.gz → .cif, .ent.gz → .pdb):
- Process entire collection or filter by type/format
- Parallel processing
- Progress display
- Remove original after successful decompression

### `storage dedupe`
Remove duplicate files (when both compressed and uncompressed exist):
- Keep compressed by default (saves space)
- Option to keep uncompressed

## Implementation Tasks

### 1. Create storage module

```rust
// src/storage/mod.rs
pub mod status;
pub mod compress;
pub mod decompress;
pub mod dedupe;

pub use status::get_storage_status;
pub use compress::compress_collection;
pub use decompress::decompress_collection;
pub use dedupe::dedupe_collection;
```

### 2. Implement status

```rust
// src/storage/status.rs
pub struct StorageStatus {
    pub total_files: u64,
    pub compressed_files: u64,
    pub compressed_size: u64,
    pub uncompressed_files: u64,
    pub uncompressed_size: u64,
    pub duplicates: u64,  // Files with both .gz and non-.gz versions
}

pub async fn get_storage_status(pdb_dir: &Path) -> Result<StorageStatus> {
    // Walk directory and collect statistics
}
```

### 3. Implement compress

```rust
// src/storage/compress.rs
use async_compression::tokio::write::GzipEncoder;

pub struct CompressOptions {
    pub data_type: Option<DataType>,
    pub format: Option<FileFormat>,
    pub parallel: usize,
    pub dry_run: bool,
    pub progress: bool,
}

pub async fn compress_collection(
    pdb_dir: &Path,
    options: CompressOptions,
) -> Result<CompressResult> {
    // 1. Find all uncompressed files
    // 2. Filter by data_type/format if specified
    // 3. Compress each file in parallel
    // 4. Remove original after successful compression
}

async fn compress_file(path: &Path) -> Result<()> {
    let dest = path.with_extension(format!("{}.gz", path.extension().unwrap_or_default().to_str().unwrap_or("")));

    let input = File::open(path).await?;
    let output = File::create(&dest).await?;
    let mut encoder = GzipEncoder::new(output);

    tokio::io::copy(&mut BufReader::new(input), &mut encoder).await?;
    encoder.shutdown().await?;

    // Remove original
    fs::remove_file(path).await?;

    Ok(())
}
```

### 4. Implement decompress

```rust
// src/storage/decompress.rs
use async_compression::tokio::bufread::GzipDecoder;

pub async fn decompress_collection(
    pdb_dir: &Path,
    options: DecompressOptions,
) -> Result<DecompressResult> {
    // Similar to compress but in reverse
}

async fn decompress_file(path: &Path) -> Result<()> {
    // Remove .gz extension for destination
    let dest = path.with_extension("");

    let input = File::open(path).await?;
    let mut decoder = GzipDecoder::new(BufReader::new(input));
    let mut output = File::create(&dest).await?;

    tokio::io::copy(&mut decoder, &mut output).await?;

    // Remove original
    fs::remove_file(path).await?;

    Ok(())
}
```

### 5. Implement dedupe

```rust
// src/storage/dedupe.rs
pub enum KeepVersion {
    Compressed,
    Uncompressed,
}

pub async fn dedupe_collection(
    pdb_dir: &Path,
    keep: KeepVersion,
) -> Result<DedupeResult> {
    // 1. Find files that have both .gz and non-.gz versions
    // 2. Remove the version not being kept
}
```

### 6. Add CLI args

```rust
// src/cli/args.rs
#[derive(Subcommand)]
pub enum StorageCommand {
    /// Show storage status
    Status,

    /// Compress uncompressed files
    Compress(StorageCompressArgs),

    /// Decompress compressed files
    Decompress(StorageDecompressArgs),

    /// Remove duplicate files
    Dedupe(StorageDedupeArgs),
}

#[derive(Args)]
pub struct StorageCompressArgs {
    /// Data type to compress
    #[arg(short, long)]
    pub r#type: Option<DataType>,

    /// Format to compress
    #[arg(short, long)]
    pub format: Option<FileFormat>,

    /// Parallel workers
    #[arg(short, long, default_value = "4")]
    pub parallel: usize,

    /// Dry run
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Show progress
    #[arg(short = 'P', long)]
    pub progress: bool,
}

#[derive(Args)]
pub struct StorageDecompressArgs {
    // Same as compress
}

#[derive(Args)]
pub struct StorageDedupeArgs {
    /// Which version to keep
    #[arg(long, default_value = "compressed")]
    pub keep: KeepVersion,

    /// Dry run
    #[arg(short = 'n', long)]
    pub dry_run: bool,
}
```

### 7. Implement command handler

```rust
// src/cli/commands/storage.rs
pub async fn run_storage(cmd: StorageCommand, ctx: AppContext) -> Result<()> {
    match cmd {
        StorageCommand::Status => run_status(&ctx).await,
        StorageCommand::Compress(args) => run_compress(args, &ctx).await,
        StorageCommand::Decompress(args) => run_decompress(args, &ctx).await,
        StorageCommand::Dedupe(args) => run_dedupe(args, &ctx).await,
    }
}
```

## Files to Create/Modify

- `src/storage/mod.rs` - New: Storage module
- `src/storage/status.rs` - New: Status collection
- `src/storage/compress.rs` - New: Compression logic
- `src/storage/decompress.rs` - New: Decompression logic
- `src/storage/dedupe.rs` - New: Deduplication logic
- `src/lib.rs` - Export storage module
- `src/cli/args.rs` - Add StorageCommand
- `src/cli/commands/storage.rs` - New: Storage command handler
- `src/cli/commands/mod.rs` - Export storage
- `src/main.rs` - Add storage command

## Dependencies

Already have:
- `async-compression` - For gzip operations

## Testing

- Test storage status calculation
- Test compression of various file types
- Test decompression
- Test parallel processing
- Test deduplication
- Test dry-run mode
- Test filtering by type/format
