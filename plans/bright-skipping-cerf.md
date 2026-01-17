# Phase 6: aria2c Support - Implementation Plan

## Summary

Add aria2c as an optional download engine for faster parallel downloads. Uses composition over trait abstraction for minimal changes and backward compatibility.

## Architecture

```
                       DownloadOptions
                             |
                   +---------+---------+
                   |                   |
             HttpsDownloader     Aria2cDownloader
             (existing)          (new - reuses URL building)
                   |                   |
             reqwest streams     aria2c subprocess
```

## Files to Create

### 1. `src/download/engine.rs` (new)
- `EngineType` enum with `Builtin`, `Aria2c` variants (derives `ValueEnum`)
- `Aria2cOptions` struct for connections/split settings

### 2. `src/download/aria2c.rs` (new)
```rust
pub struct Aria2cDownloader {
    aria2c_path: PathBuf,
    connections: u32,     // -x flag
    split: u32,           // -s flag
    max_concurrent: u32,  // -j flag
    options: DownloadOptions,
}
```
Key methods:
- `is_available() -> bool` - detect aria2c in PATH
- `new(options, connections, split) -> Result<Self>`
- `download_many(tasks, dest, url_builder) -> Vec<DownloadResult>`
- `generate_input_file(tasks, dest, url_builder) -> String`

## Files to Modify

### 3. `src/download/https.rs`
- Make `build_url_for_task()` public
- Make `build_dest_path_for_task()` public

### 4. `src/download/mod.rs`
- Add `pub mod aria2c;`
- Add `pub mod engine;`
- Re-export `Aria2cDownloader`, `EngineType`, `Aria2cOptions`

### 5. `src/cli/args.rs`
Add to `DownloadArgs`:
```rust
#[arg(long, value_enum, default_value = "builtin")]
pub engine: EngineType,

#[arg(long, default_value = "4")]
pub connections: u32,

#[arg(long, default_value = "1")]
pub split: u32,

#[arg(long)]
pub export_aria2c: bool,
```

### 6. `src/cli/commands/download.rs`
Add engine selection in `run_download()`:
```rust
match args.engine {
    EngineType::Builtin => { /* existing logic */ }
    EngineType::Aria2c => {
        if args.export_aria2c {
            // Print input file to stdout
        } else if Aria2cDownloader::is_available() {
            // Use aria2c
        } else {
            eprintln!("Warning: aria2c not found, using built-in downloader");
            // Fall back to builtin
        }
    }
}
```

### 7. `src/config/schema.rs`
Add to `DownloadConfig`:
```rust
pub engine: String,            // "builtin" or "aria2c"
pub aria2c_connections: u32,   // default: 4
pub aria2c_split: u32,         // default: 1
```

### 8. `src/error.rs`
Add variants:
```rust
#[error("aria2c not found in PATH")]
Aria2cNotFound,

#[error("aria2c execution failed: {0}")]
Aria2cFailed(String),
```

### 9. `Cargo.toml`
Add:
```toml
which = "7.0"
```

## Implementation Order

1. Add `which` dependency to Cargo.toml
2. Add error variants to `src/error.rs`
3. Create `src/download/engine.rs` with EngineType
4. Update `src/download/mod.rs` exports
5. Add CLI arguments to `src/cli/args.rs`
6. Add config options to `src/config/schema.rs`
7. Make HttpsDownloader URL methods public
8. Create `src/download/aria2c.rs`:
   - `is_available()` function
   - `Aria2cDownloader` struct and impl
   - `generate_input_file()` method
   - `download_many()` method with subprocess
   - Unit tests
9. Integrate into `src/cli/commands/download.rs`
10. Integration testing

## aria2c Input File Format

```
https://files.rcsb.org/download/4hhb.cif.gz
  dir=/path/to/dest
  out=4hhb.cif.gz
https://files.rcsb.org/download/1abc.cif.gz
  dir=/path/to/dest
  out=1abc.cif.gz
```

## Edge Cases

1. **aria2c not found**: Warn and fall back to built-in
2. **Execution fails**: Map to `DownloadResult::Failed`
3. **File exists**: Pass `--allow-overwrite` based on `--overwrite` flag
4. **Special characters in paths**: Use absolute paths, escape if needed

## Testing

### Unit Tests (src/download/aria2c.rs)
- `test_is_available_returns_bool`
- `test_generate_input_file_single_task`
- `test_generate_input_file_multiple_tasks`
- `test_generate_input_file_with_spaces_in_path`
- `test_build_aria2c_args_default`
- `test_build_aria2c_args_with_options`

### Integration Tests
- `test_fallback_to_builtin_when_aria2c_unavailable`
- `test_export_aria2c_format`
- `test_download_with_aria2c_engine` (requires aria2c)

### Manual Verification
```bash
# Existing behavior unchanged
pdb-cli download 4hhb --engine builtin

# aria2c with custom options
pdb-cli download 4hhb --engine aria2c --connections 8 --split 4

# Fallback when aria2c unavailable
pdb-cli download 4hhb --engine aria2c  # should warn and use builtin

# Export input file
pdb-cli download -l ids.txt --export-aria2c > downloads.txt
aria2c -i downloads.txt  # verify format

# Config-based defaults
pdb-cli config set download.engine aria2c
pdb-cli download 4hhb  # should use aria2c
```

## Config Example

```toml
[download]
engine = "aria2c"
aria2c_connections = 8
aria2c_split = 4
parallel = 8
```
