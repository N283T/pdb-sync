# Phase 10: Convert Command

## Parallel Development Note
This phase is being developed in parallel with other v3 phases.
- Create PR and request review
- Wait for review approval before merging
- Rebase on master if other phases are merged first

## Goal
Add `convert` command for format conversion and compression management.

## Usage Examples

```bash
# Decompress files
pdb-cli convert 4hhb.cif.gz --decompress
pdb-cli convert *.cif.gz --decompress --dest ./uncompressed/

# Compress files
pdb-cli convert 4hhb.cif --compress
pdb-cli convert *.cif --compress --dest ./compressed/

# Format conversion (requires external tools)
pdb-cli convert 4hhb.cif --to pdb
pdb-cli convert 4hhb.pdb --to cif

# Batch conversion
pdb-cli convert --from cif-gz --to pdb --dest ./pdb_format/
pdb-cli list -o ids | pdb-cli convert --stdin --to pdb

# In-place conversion
pdb-cli convert 4hhb.cif.gz --decompress --in-place

# Convert and validate
pdb-cli convert 4hhb.cif.gz --decompress --validate
```

## Supported Conversions

### Compression (built-in)
| From | To | Method |
|------|-----|--------|
| .cif.gz | .cif | gzip decompress |
| .cif | .cif.gz | gzip compress |
| .pdb.gz / .ent.gz | .pdb / .ent | gzip decompress |
| .pdb / .ent | .pdb.gz / .ent.gz | gzip compress |

### Format Conversion (requires external tools)
| From | To | Tool |
|------|-----|------|
| mmCIF | PDB | gemmi (gemmi convert) |
| PDB | mmCIF | gemmi (gemmi convert) |
| mmCIF | PDBML | gemmi |
| mmCIF | bcif | gemmi |

## Implementation Tasks

### 1. Create convert module

```rust
// src/convert/mod.rs
pub enum ConversionType {
    Compress,
    Decompress,
    FormatConvert { from: FileFormat, to: FileFormat },
}

pub struct ConvertOptions {
    pub conversion: ConversionType,
    pub dest: Option<PathBuf>,
    pub in_place: bool,
    pub validate: bool,
    pub overwrite: bool,
}

pub async fn convert_file(
    source: &Path,
    options: &ConvertOptions,
) -> Result<PathBuf> {
    match &options.conversion {
        ConversionType::Compress => compress_file(source, options).await,
        ConversionType::Decompress => decompress_file(source, options).await,
        ConversionType::FormatConvert { from, to } => {
            format_convert(source, from, to, options).await
        }
    }
}
```

### 2. Implement compression/decompression

```rust
// src/convert/compress.rs
use async_compression::tokio::bufread::GzipDecoder;
use async_compression::tokio::write::GzipEncoder;

pub async fn decompress_file(source: &Path, options: &ConvertOptions) -> Result<PathBuf> {
    let dest = options.dest_path(source, false)?;

    let input = File::open(source).await?;
    let reader = BufReader::new(input);
    let mut decoder = GzipDecoder::new(reader);

    let mut output = File::create(&dest).await?;
    tokio::io::copy(&mut decoder, &mut output).await?;

    if options.in_place {
        fs::remove_file(source).await?;
    }

    Ok(dest)
}

pub async fn compress_file(source: &Path, options: &ConvertOptions) -> Result<PathBuf> {
    let dest = options.dest_path(source, true)?;

    let input = File::open(source).await?;
    let mut reader = BufReader::new(input);

    let output = File::create(&dest).await?;
    let mut encoder = GzipEncoder::new(output);

    tokio::io::copy(&mut reader, &mut encoder).await?;
    encoder.shutdown().await?;

    if options.in_place {
        fs::remove_file(source).await?;
    }

    Ok(dest)
}
```

### 3. Implement format conversion

```rust
// src/convert/format.rs
pub async fn format_convert(
    source: &Path,
    from: &FileFormat,
    to: &FileFormat,
    options: &ConvertOptions,
) -> Result<PathBuf> {
    // Check if gemmi is available
    if !is_gemmi_available() {
        return Err(PdbCliError::GemmiNotFound(
            "Format conversion requires gemmi. Install with: pip install gemmi".into()
        ));
    }

    let dest = options.dest_path_format(source, to)?;

    // Run gemmi convert
    let status = Command::new("gemmi")
        .arg("convert")
        .arg(source)
        .arg(&dest)
        .status()
        .await?;

    if !status.success() {
        return Err(PdbCliError::ConversionFailed(source.to_path_buf()));
    }

    Ok(dest)
}

fn is_gemmi_available() -> bool {
    which::which("gemmi").is_ok()
}
```

### 4. Add CLI args

```rust
// src/cli/args.rs
#[derive(Args)]
pub struct ConvertArgs {
    /// Files to convert (supports glob patterns)
    pub files: Vec<PathBuf>,

    /// Read file paths from stdin
    #[arg(long)]
    pub stdin: bool,

    /// Decompress files
    #[arg(long)]
    pub decompress: bool,

    /// Compress files
    #[arg(long)]
    pub compress: bool,

    /// Convert to format
    #[arg(long)]
    pub to: Option<FileFormat>,

    /// Source format (for batch conversion)
    #[arg(long)]
    pub from: Option<FileFormat>,

    /// Destination directory
    #[arg(short, long)]
    pub dest: Option<PathBuf>,

    /// Convert in place (replace original)
    #[arg(long)]
    pub in_place: bool,

    /// Validate output files
    #[arg(long)]
    pub validate: bool,

    /// Overwrite existing files
    #[arg(long)]
    pub overwrite: bool,

    /// Number of parallel conversions
    #[arg(short, long, default_value = "4")]
    pub parallel: usize,
}
```

### 5. Implement command handler

```rust
// src/cli/commands/convert.rs
pub async fn run_convert(args: ConvertArgs, ctx: AppContext) -> Result<()> {
    let files = collect_files(&args)?;
    let options = ConvertOptions::from(&args);

    // Validate args
    if args.decompress && args.compress {
        return Err(PdbCliError::InvalidArgs(
            "Cannot specify both --decompress and --compress".into()
        ));
    }

    // Process files in parallel
    let semaphore = Arc::new(Semaphore::new(args.parallel));
    let results = futures::future::join_all(
        files.iter().map(|file| {
            let sem = semaphore.clone();
            let opts = options.clone();
            async move {
                let _permit = sem.acquire().await;
                convert_file(file, &opts).await
            }
        })
    ).await;

    // Report results
    print_conversion_summary(&results);

    Ok(())
}
```

### 6. Batch conversion from local files

```rust
// Convert all local mmCIF files to PDB format
pub async fn batch_convert(
    pdb_dir: &Path,
    from: FileFormat,
    to: FileFormat,
    dest: &Path,
) -> Result<BatchConvertResult> {
    let files = find_files_by_format(pdb_dir, from)?;
    // ... convert each file
}
```

## Files to Create/Modify

- `src/convert/mod.rs` - New: Convert orchestration
- `src/convert/compress.rs` - New: Compression/decompression
- `src/convert/format.rs` - New: Format conversion
- `src/lib.rs` - Export convert module
- `src/cli/args.rs` - Add ConvertArgs
- `src/cli/commands/convert.rs` - New: Convert command handler
- `src/cli/commands/mod.rs` - Export convert
- `src/main.rs` - Add convert command

## Dependencies

Already have:
- `async-compression` - For gzip operations

Optional (for format conversion):
- gemmi CLI tool (external)

## Testing

- Test gzip compression/decompression
- Test in-place conversion
- Test batch conversion
- Test format conversion with gemmi (if available)
- Test error handling for missing tools
- Test parallel conversion
