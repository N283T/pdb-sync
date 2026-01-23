# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

```bash
# Build the project
cargo build

# Run with debug output
RUST_LOG=debug cargo run -- [args]

# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Format code
cargo fmt

# Check formatting without modifying
cargo fmt --all -- --check

# Run linter (treats warnings as errors)
cargo clippy -- -D warnings

# Build for release
cargo build --release

# Install from local source
cargo install --path .

# Run a specific test
cargo test test_name
```

## Architecture Overview

This is a Rust CLI tool (`pdb-sync`) for managing Protein Data Bank (PDB) files. The architecture is modular, async-first (Tokio), and follows a clear separation of concerns.

### Core Abstraction: AppContext

The `AppContext` (`src/context.rs`) is the central dependency injection container that holds:
- `config`: Merged configuration from file, ENV, and defaults
- `pdb_dir`: Resolved PDB directory path
- `mirror`: Selected mirror (with auto-selection support)

All commands receive `AppContext` for accessing configuration and environment state.

### Command Structure

Commands are organized in `src/cli/commands/` following this pattern:

```rust
// Each command module exports a `run_<command>` function
pub async fn run_download(args: DownloadArgs, ctx: AppContext) -> Result<()> {
    // Command implementation
}
```

The CLI argument parsing (`src/cli/args/`) uses `clap` derive macros with shared argument groups like `PdbDirArgs`, `MirrorArgs`, `FormatArgs`.

### Key Modules

- **`src/config/`**: Configuration loader with TOML serialization. Multi-source priority: CLI args > ENV vars > config file > defaults
- **`src/data_types.rs`**: Enum-based data type definitions (structures, assemblies, structure-factors, etc.)
- **`src/mirrors/`**: Mirror registry with latency-based auto-selection
- **`src/files/`**: PDB ID validation, path resolution for both "divided" (hash-based) and "all" (flat) layouts
- **`src/download/`**: Pluggable download engine (HTTPS, aria2c) with parallel task execution
- **`src/sync/`**: rsync wrapper with progress tracking
- **`src/validation/`**: Checksum verification against remote checksum files
- **`src/api/`**: RCSB PDB API client for metadata queries
- **`src/watch/`**: Polling-based new entry monitoring
- **`src/jobs/`**: Background job management with status tracking and logging

### Error Handling

Uses `thiserror` for a comprehensive `PdbSyncError` enum in `src/error.rs`. Key features:
- Context-rich error variants (Network, Download, Config, etc.)
- `is_retriable()` method for retry logic
- Helper methods `pdb_id()` and `url()` for error context
- Type alias: `pub type Result<T> = std::result::Result<T, PdbSyncError>`

### Configuration File

Location: `~/.config/pdb-sync/config.toml`

Key sections:
- `[paths]`: `pdb_dir`
- `[sync]`: `defaults`, `custom` (rsync configurations with presets)
- `[download]`: `default_format`, `auto_decompress`, `parallel`, `retry_count`

### PDB ID Support

Both classic (4-char: `1abc`, `4hhb`) and extended (12-char: `pdb_00001abc`) formats are supported via validation in `src/files/`.

### Alias System

Commands and options support shortcuts:
- Commands: `dl` (download), `val` (validate), `cfg` (config)
- Data types: `st`/`struct` (structures), `asm`/`assembly` (assemblies), `sf`/`xray` (structure-factors)
- Formats: `cif` (mmcif)
- Mirrors: `us` (rcsb), `jp` (pdbj), `uk`/`eu` (pdbe), `global` (wwpdb)

### Background Jobs

Long-running commands (sync, download, watch) support `--bg` flag to run in background. Jobs are tracked in `~/.local/share/pdb-sync/jobs/` with status logs.

### CI Pipeline

The project uses GitHub Actions (`.github/workflows/ci.yml`) with:
- Standard `cargo build` and `cargo test`
- `cargo clippy -- -D warnings` (warnings as errors)
- `cargo fmt --all -- --check` (format verification)
