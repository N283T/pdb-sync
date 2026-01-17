# Phase 12: Tree Command

## Parallel Development Note
This phase is being developed in parallel with other v3 phases.
- Create PR and request review
- Wait for review approval before merging
- Rebase on master if other phases are merged first

## Goal
Add `tree` command to visualize the local PDB directory structure.

## Usage Examples

```bash
# Show full directory tree
pdb-cli tree

# Output:
# /data/pdb
# ├── mmCIF/
# │   ├── aa/ (156 files, 234 MB)
# │   ├── ab/ (203 files, 312 MB)
# │   ├── ac/ (98 files, 145 MB)
# │   └── ... (1296 more directories)
# ├── pdb/
# │   ├── aa/ (156 files, 89 MB)
# │   └── ...
# ├── assemblies/
# │   └── ...
# └── structure_factors/
#     └── ...
#
# Total: 45,678 files, 125.4 GB

# Limit depth
pdb-cli tree --depth 1
# Output:
# /data/pdb
# ├── mmCIF/ (12,345 files, 45.6 GB)
# ├── pdb/ (8,901 files, 23.4 GB)
# ├── assemblies/ (5,678 files, 34.5 GB)
# └── structure_factors/ (2,345 files, 12.3 GB)

pdb-cli tree --depth 2

# Show specific data type only
pdb-cli tree --type structures
pdb-cli tree --type assemblies

# Show specific format only
pdb-cli tree --format mmcif
pdb-cli tree --format pdb

# Show sizes
pdb-cli tree --size
pdb-cli tree -s

# Show file counts
pdb-cli tree --count
pdb-cli tree -c

# Show both (default)
pdb-cli tree --size --count

# No summary, just structure
pdb-cli tree --no-summary

# JSON output (for scripting)
pdb-cli tree -o json

# Show only non-empty directories
pdb-cli tree --non-empty

# Show hash directories with most files
pdb-cli tree --top 10
# Output:
# Top 10 directories by file count:
# 1. mmCIF/ab/ - 523 files (1.2 GB)
# 2. mmCIF/ac/ - 498 files (1.1 GB)
# ...
```

## Implementation Tasks

### 1. Create tree module

```rust
// src/tree/mod.rs
pub struct TreeOptions {
    pub depth: Option<usize>,
    pub data_type: Option<DataType>,
    pub format: Option<FileFormat>,
    pub show_size: bool,
    pub show_count: bool,
    pub show_summary: bool,
    pub non_empty_only: bool,
    pub top_n: Option<usize>,
}

pub struct DirNode {
    pub name: String,
    pub path: PathBuf,
    pub file_count: u64,
    pub total_size: u64,
    pub children: Vec<DirNode>,
    pub is_leaf: bool,  // Has files, no subdirs with PDB files
}

pub fn build_tree(pdb_dir: &Path, options: &TreeOptions) -> Result<DirNode> {
    // Walk directory and build tree structure
}
```

### 2. Tree building logic

```rust
// src/tree/build.rs
pub fn build_tree(pdb_dir: &Path, options: &TreeOptions) -> Result<DirNode> {
    let mut root = DirNode {
        name: pdb_dir.file_name().unwrap().to_string_lossy().to_string(),
        path: pdb_dir.to_path_buf(),
        file_count: 0,
        total_size: 0,
        children: Vec::new(),
        is_leaf: false,
    };

    build_tree_recursive(&mut root, 0, options)?;
    Ok(root)
}

fn build_tree_recursive(
    node: &mut DirNode,
    current_depth: usize,
    options: &TreeOptions,
) -> Result<()> {
    if let Some(max_depth) = options.depth {
        if current_depth >= max_depth {
            // Summarize remaining contents
            let (count, size) = count_files_recursive(&node.path)?;
            node.file_count = count;
            node.total_size = size;
            return Ok(());
        }
    }

    for entry in fs::read_dir(&node.path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let mut child = DirNode {
                name: path.file_name().unwrap().to_string_lossy().to_string(),
                path: path.clone(),
                file_count: 0,
                total_size: 0,
                children: Vec::new(),
                is_leaf: false,
            };

            build_tree_recursive(&mut child, current_depth + 1, options)?;

            if !options.non_empty_only || child.file_count > 0 {
                node.children.push(child);
            }
        } else if is_pdb_file(&path, options) {
            node.file_count += 1;
            node.total_size += entry.metadata()?.len();
        }
    }

    // Aggregate counts from children
    for child in &node.children {
        node.file_count += child.file_count;
        node.total_size += child.total_size;
    }

    node.is_leaf = node.children.is_empty() && node.file_count > 0;

    Ok(())
}
```

### 3. Tree rendering

```rust
// src/tree/render.rs
pub fn render_tree(node: &DirNode, options: &TreeOptions) -> String {
    let mut output = String::new();
    render_node(&mut output, node, "", true, options);

    if options.show_summary {
        output.push_str(&format!(
            "\nTotal: {} files, {}\n",
            node.file_count.separate_with_commas(),
            human_bytes(node.total_size)
        ));
    }

    output
}

fn render_node(
    output: &mut String,
    node: &DirNode,
    prefix: &str,
    is_last: bool,
    options: &TreeOptions,
) {
    let connector = if is_last { "└── " } else { "├── " };
    let stats = format_node_stats(node, options);

    output.push_str(&format!("{}{}{}/{}\n", prefix, connector, node.name, stats));

    let child_prefix = if is_last {
        format!("{}    ", prefix)
    } else {
        format!("{}│   ", prefix)
    };

    let children_count = node.children.len();
    for (i, child) in node.children.iter().enumerate() {
        render_node(output, child, &child_prefix, i == children_count - 1, options);
    }
}

fn format_node_stats(node: &DirNode, options: &TreeOptions) -> String {
    let mut parts = Vec::new();

    if options.show_count && node.file_count > 0 {
        parts.push(format!("{} files", node.file_count));
    }

    if options.show_size && node.total_size > 0 {
        parts.push(human_bytes(node.total_size));
    }

    if parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", parts.join(", "))
    }
}
```

### 4. Top N directories

```rust
// src/tree/top.rs
pub fn get_top_directories(
    root: &DirNode,
    n: usize,
    by: SortBy,
) -> Vec<&DirNode> {
    let mut leaves = Vec::new();
    collect_leaves(root, &mut leaves);

    leaves.sort_by(|a, b| match by {
        SortBy::Count => b.file_count.cmp(&a.file_count),
        SortBy::Size => b.total_size.cmp(&a.total_size),
    });

    leaves.into_iter().take(n).collect()
}

fn collect_leaves<'a>(node: &'a DirNode, leaves: &mut Vec<&'a DirNode>) {
    if node.is_leaf {
        leaves.push(node);
    }
    for child in &node.children {
        collect_leaves(child, leaves);
    }
}
```

### 5. Add CLI args

```rust
// src/cli/args.rs
#[derive(Args)]
pub struct TreeArgs {
    /// Maximum depth to display
    #[arg(short, long)]
    pub depth: Option<usize>,

    /// Filter by data type
    #[arg(short, long)]
    pub r#type: Option<DataType>,

    /// Filter by format
    #[arg(short, long)]
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
    #[arg(short, long, default_value = "text")]
    pub output: OutputFormat,
}
```

### 6. Implement command handler

```rust
// src/cli/commands/tree.rs
pub async fn run_tree(args: TreeArgs, ctx: AppContext) -> Result<()> {
    let options = TreeOptions {
        depth: args.depth,
        data_type: args.r#type,
        format: args.format,
        show_size: args.size || (!args.size && !args.count),  // Default both
        show_count: args.count || (!args.size && !args.count),
        show_summary: !args.no_summary,
        non_empty_only: args.non_empty,
        top_n: args.top,
    };

    let tree = build_tree(&ctx.pdb_dir, &options)?;

    if let Some(n) = args.top {
        // Show top N mode
        let top_dirs = get_top_directories(&tree, n, SortBy::Count);
        print_top_directories(&top_dirs);
    } else {
        // Normal tree mode
        match args.output {
            OutputFormat::Text => println!("{}", render_tree(&tree, &options)),
            OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&tree)?),
        }
    }

    Ok(())
}
```

## Files to Create/Modify

- `src/tree/mod.rs` - New: Tree module
- `src/tree/build.rs` - New: Tree building logic
- `src/tree/render.rs` - New: Tree rendering
- `src/tree/top.rs` - New: Top N analysis
- `src/lib.rs` - Export tree module
- `src/cli/args.rs` - Add TreeArgs
- `src/cli/commands/tree.rs` - New: Tree command handler
- `src/cli/commands/mod.rs` - Export tree
- `src/main.rs` - Add tree command

## Dependencies

```toml
# Already have thousands crate from stats phase
```

## Testing

- Test tree building with mock directory
- Test depth limiting
- Test filtering by type/format
- Test rendering output
- Test top N mode
- Test JSON output
- Test empty directory handling
