# Phase 12: Tree Command Implementation Plan

## Summary

Implement `pdb-cli tree` command to visualize the local PDB directory structure with file counts, sizes, and filtering options.

## Files to Create

### 1. `src/tree/mod.rs` - Tree module with core types

```rust
pub mod build;
pub mod render;

pub use build::{build_tree, TreeOptions};
pub use render::render_tree;

/// Directory node in the tree
#[derive(Debug, Clone, serde::Serialize)]
pub struct DirNode {
    pub name: String,
    pub path: std::path::PathBuf,
    pub file_count: u64,
    pub total_size: u64,
    pub children: Vec<DirNode>,
    pub is_leaf: bool,
}
```

### 2. `src/tree/build.rs` - Tree building logic

- `build_tree(pdb_dir: &Path, options: &TreeOptions) -> Result<DirNode>`
- `build_tree_recursive(node: &mut DirNode, current_depth: usize, options: &TreeOptions) -> Result<()>`
- `count_files_recursive(path: &Path, options: &TreeOptions) -> Result<(u64, u64)>` - for summarizing at depth limit
- `is_pdb_file(path: &Path, options: &TreeOptions) -> bool` - filter by format

### 3. `src/tree/render.rs` - Tree rendering

- `render_tree(node: &DirNode, options: &TreeOptions) -> String` - text output with ASCII art
- `render_node(output: &mut String, node: &DirNode, prefix: &str, is_last: bool, options: &TreeOptions)`
- `format_node_stats(node: &DirNode, options: &TreeOptions) -> String`
- `render_top_directories(nodes: &[&DirNode], by: SortBy) -> String` - for --top N mode
- `human_bytes(bytes: u64) -> String` - reuse pattern from list.rs

### 4. `src/cli/commands/tree.rs` - Command handler

```rust
pub async fn run_tree(args: TreeArgs, ctx: AppContext) -> Result<()>
```

## Files to Modify

### 1. `src/cli/args.rs`

Add to Commands enum:
```rust
/// Show directory tree of local PDB mirror
Tree(TreeArgs),
```

Add TreeArgs struct:
```rust
#[derive(Parser)]
pub struct TreeArgs {
    /// Maximum depth to display (0 = root only)
    #[arg(short, long)]
    pub depth: Option<usize>,

    /// Filter by file format
    #[arg(short, long, value_enum)]
    pub format: Option<FileFormat>,

    /// Show file sizes
    #[arg(short, long)]
    pub size: bool,

    /// Show file counts
    #[arg(short, long)]
    pub count: bool,

    /// Hide summary line
    #[arg(long)]
    pub no_summary: bool,

    /// Show only non-empty directories
    #[arg(long)]
    pub non_empty: bool,

    /// Show top N directories by file count
    #[arg(long)]
    pub top: Option<usize>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "text")]
    pub output: OutputFormat,
}
```

### 2. `src/cli/commands/mod.rs`

Add:
```rust
pub mod tree;
pub use tree::run_tree;
```

### 3. `src/main.rs`

Add import and match arm:
```rust
mod tree;  // Add to modules

// In match cli.command:
Commands::Tree(args) => {
    cli::commands::run_tree(args, ctx).await?;
}
```

## Implementation Details

### Directory Structure Handled

The PDB mirror uses this structure:
```
pdb_dir/
├── mmCIF/
│   ├── aa/
│   │   ├── 1aaa.cif.gz
│   │   └── ...
│   ├── ab/
│   └── ...
├── pdb/
│   ├── aa/
│   │   ├── pdb1aaa.ent.gz
│   │   └── ...
│   └── ...
└── bcif/
    └── ...
```

### Tree Rendering Format

```
/data/pdb
├── mmCIF/ (12,345 files, 45.6 GB)
│   ├── aa/ (156 files, 234 MB)
│   ├── ab/ (203 files, 312 MB)
│   └── ... (1296 more directories)
├── pdb/ (8,901 files, 23.4 GB)
└── bcif/ (5,678 files, 12.3 GB)

Total: 26,924 files, 81.3 GB
```

### Key Behaviors

1. **Default display**: Both --size and --count enabled when neither specified
2. **Depth limiting**: When depth reached, summarize remaining contents inline
3. **Format filter**: Only count files matching the specified format
4. **Top N mode**: Collect leaf directories, sort by count/size, display top N
5. **JSON output**: Serialize DirNode directly (already has Serialize)

### File Recognition

Files are recognized by extension patterns:
- mmCIF: `*.cif.gz` in `mmCIF/*/`
- PDB: `pdb*.ent.gz` or `*.ent.gz` in `pdb/*/`
- BCIF: `*.bcif.gz` in `bcif/*/`

## Testing Strategy

Tests will be inline in implementation files using `#[cfg(test)]`:

1. **build.rs tests**:
   - Test with tempdir containing mock structure
   - Test depth limiting
   - Test format filtering
   - Test empty directory handling

2. **render.rs tests**:
   - Test ASCII tree rendering
   - Test stats formatting
   - Test summary output
   - Test top N rendering

3. **tree.rs tests**:
   - Integration test with full command flow

## Implementation Order (TDD)

1. **Red**: Write tests for DirNode, TreeOptions, and build_tree
2. **Green**: Implement tree/mod.rs and tree/build.rs
3. **Red**: Write tests for render_tree
4. **Green**: Implement tree/render.rs
5. **Red**: Write tests for CLI args parsing
6. **Green**: Add TreeArgs to args.rs
7. Implement tree.rs command handler
8. Wire up in main.rs and commands/mod.rs
9. Run `cargo fmt`, `cargo clippy`, `cargo test`

## Verification

After implementation:
```bash
# Format and lint
cargo fmt --check
cargo clippy -- -D warnings

# Run tests
cargo test

# Manual testing (requires a PDB mirror directory)
cargo run -- tree
cargo run -- tree --depth 1
cargo run -- tree --format cif-gz
cargo run -- tree --top 10
cargo run -- tree -o json
```
