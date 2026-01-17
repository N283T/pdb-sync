# Phase 6: aria2c Support

## Parallel Development Note
This phase is being developed in parallel with other v3 phases.
- Create PR and request review
- Wait for review approval before merging
- Rebase on master if other phases are merged first

## Goal
Add aria2c as an optional download engine for faster parallel downloads.

## Why aria2c?
- Multi-connection downloads (splits file into segments)
- Built-in retry and resume
- Lower overhead than multiple HTTP clients
- Supports metalinks for mirror selection
- Popular in scientific computing environments

## Usage Examples

```bash
# Use aria2c for downloads (if available)
pdb-cli download 4hhb 1abc --engine aria2c
pdb-cli dl 4hhb --engine aria2c

# Configure aria2c options
pdb-cli download -l ids.txt --engine aria2c --connections 16
pdb-cli download -l ids.txt --engine aria2c --split 4

# Set default engine in config
pdb-cli config set download.engine aria2c

# Fall back to built-in if aria2c not available
pdb-cli download 4hhb --engine aria2c
# Warning: aria2c not found, using built-in downloader

# Generate aria2c input file (for manual use)
pdb-cli download -l ids.txt --export-aria2c > downloads.txt
# Then: aria2c -i downloads.txt
```

## Implementation Tasks

### 1. Create download engine abstraction

```rust
// src/download/engine.rs
pub trait DownloadEngine {
    async fn download(&self, tasks: Vec<DownloadTask>) -> Result<Vec<DownloadResult>>;
    fn name(&self) -> &str;
}

pub struct DownloadTask {
    pub url: String,
    pub dest: PathBuf,
    pub checksum: Option<String>,
}

pub struct DownloadResult {
    pub task: DownloadTask,
    pub success: bool,
    pub error: Option<String>,
    pub bytes: u64,
    pub duration: Duration,
}
```

### 2. Implement built-in engine (refactor existing)

```rust
// src/download/builtin.rs
pub struct BuiltinEngine {
    client: reqwest::Client,
    parallel: usize,
    retry_count: u32,
}

impl DownloadEngine for BuiltinEngine {
    async fn download(&self, tasks: Vec<DownloadTask>) -> Result<Vec<DownloadResult>> {
        // Existing parallel download logic
    }
}
```

### 3. Implement aria2c engine

```rust
// src/download/aria2c.rs
pub struct Aria2cEngine {
    aria2c_path: PathBuf,
    connections: u32,     // -x, max connections per server
    split: u32,           // -s, split file into N parts
    max_concurrent: u32,  // -j, max concurrent downloads
}

impl Aria2cEngine {
    pub fn is_available() -> bool {
        which::which("aria2c").is_ok()
    }

    pub fn new() -> Result<Self> {
        if !Self::is_available() {
            return Err(PdbCliError::Aria2cNotFound);
        }
        // ...
    }
}

impl DownloadEngine for Aria2cEngine {
    async fn download(&self, tasks: Vec<DownloadTask>) -> Result<Vec<DownloadResult>> {
        // 1. Generate input file
        // 2. Run aria2c with input file
        // 3. Parse output and return results
    }
}
```

### 4. aria2c input file format

```
# downloads.txt
https://files.rcsb.org/download/4hhb.cif.gz
  dir=/path/to/dest
  out=4hhb.cif.gz
  checksum=md5=abc123...
https://files.rcsb.org/download/1abc.cif.gz
  dir=/path/to/dest
  out=1abc.cif.gz
```

### 5. Add CLI args

```rust
// src/cli/args.rs
#[derive(Args)]
pub struct DownloadArgs {
    // ... existing args ...

    /// Download engine to use
    #[arg(long, default_value = "builtin")]
    pub engine: DownloadEngineType,

    /// Connections per server (aria2c only)
    #[arg(long, default_value = "4")]
    pub connections: u32,

    /// Split each file into N parts (aria2c only)
    #[arg(long, default_value = "1")]
    pub split: u32,

    /// Export aria2c input file instead of downloading
    #[arg(long)]
    pub export_aria2c: bool,
}

#[derive(ValueEnum, Clone)]
pub enum DownloadEngineType {
    Builtin,
    Aria2c,
}
```

### 6. Add config option

```toml
# ~/.config/pdb-cli/config.toml
[download]
engine = "builtin"  # or "aria2c"
aria2c_connections = 4
aria2c_split = 1
```

### 7. Engine selection logic

```rust
// src/download/mod.rs
pub fn get_engine(engine_type: DownloadEngineType, config: &Config) -> Box<dyn DownloadEngine> {
    match engine_type {
        DownloadEngineType::Aria2c => {
            if Aria2cEngine::is_available() {
                Box::new(Aria2cEngine::new(config))
            } else {
                eprintln!("Warning: aria2c not found, using built-in downloader");
                Box::new(BuiltinEngine::new(config))
            }
        }
        DownloadEngineType::Builtin => Box::new(BuiltinEngine::new(config)),
    }
}
```

## Files to Create/Modify

- `src/download/engine.rs` - New: DownloadEngine trait
- `src/download/builtin.rs` - New: Refactored built-in engine
- `src/download/aria2c.rs` - New: aria2c engine
- `src/download/mod.rs` - Export engines, add get_engine()
- `src/cli/args.rs` - Add engine options
- `src/cli/commands/download.rs` - Use engine abstraction
- `src/config/schema.rs` - Add download.engine config

## Dependencies

```toml
# Cargo.toml
[dependencies]
which = "6.0"  # For finding aria2c in PATH
```

## Testing

- Test DownloadEngine trait implementations
- Test aria2c input file generation
- Test fallback when aria2c not available
- Integration test with aria2c (if available)
- Test config-based engine selection
