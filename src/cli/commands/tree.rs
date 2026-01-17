//! Tree command - displays the local PDB directory structure.

use crate::cli::args::{OutputFormat, TreeArgs};
use crate::context::AppContext;
use crate::error::{PdbCliError, Result};
use crate::tree::render::{render_top_directories, RenderOptions};
use crate::tree::{build_tree, render_tree, DirNode, TreeOptions};

/// Main entry point for the tree command
pub async fn run_tree(args: TreeArgs, ctx: AppContext) -> Result<()> {
    let pdb_dir = &ctx.pdb_dir;

    if !pdb_dir.exists() {
        return Err(PdbCliError::Path(format!(
            "PDB directory does not exist: {}",
            pdb_dir.display()
        )));
    }

    // Build tree options
    let build_options = TreeOptions {
        max_depth: args.depth,
        format_filter: args.format,
        non_empty: args.non_empty,
    };

    // Build the tree
    let tree = build_tree(pdb_dir, &build_options).await?;

    // Render options
    let render_options = RenderOptions {
        show_size: args.size,
        show_count: args.count,
        no_summary: args.no_summary,
    };

    // Output based on format and mode
    match args.output {
        OutputFormat::Text => {
            if let Some(top_n) = args.top {
                // Top N mode
                let output = render_top_directories(&tree, top_n, args.sort_by);
                print!("{}", output);
            } else {
                // Normal tree mode
                let output = render_tree(&tree, &render_options);
                print!("{}", output);
            }
        }
        OutputFormat::Json => {
            print_json(&tree)?;
        }
        OutputFormat::Csv => {
            print_csv(&tree);
        }
        OutputFormat::Ids => {
            // Ids format doesn't apply to tree command, fall back to text
            if let Some(top_n) = args.top {
                let output = render_top_directories(&tree, top_n, args.sort_by);
                print!("{}", output);
            } else {
                let output = render_tree(&tree, &render_options);
                print!("{}", output);
            }
        }
    }

    Ok(())
}

/// Print the tree as JSON
fn print_json(tree: &DirNode) -> Result<()> {
    let json = serde_json::to_string_pretty(tree)
        .map_err(|e| PdbCliError::InvalidInput(format!("JSON serialization failed: {}", e)))?;
    println!("{}", json);
    Ok(())
}

/// Print the tree as CSV
fn print_csv(tree: &DirNode) {
    // Print header
    println!("path,file_count,total_size,is_leaf");

    // Print all nodes recursively
    print_csv_node(tree);
}

/// Print a single node and its children as CSV
fn print_csv_node(node: &DirNode) {
    println!(
        "{},{},{},{}",
        escape_csv_field(&node.path.display().to_string()),
        node.file_count,
        node.total_size,
        node.is_leaf
    );

    for child in &node.children {
        print_csv_node(child);
    }
}

/// Escape a CSV field to prevent injection and handle special characters
fn escape_csv_field(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_csv_field() {
        assert_eq!(escape_csv_field("simple"), "simple");
        assert_eq!(escape_csv_field("with,comma"), "\"with,comma\"");
        assert_eq!(escape_csv_field("with\"quote"), "\"with\"\"quote\"");
        assert_eq!(escape_csv_field("with\nnewline"), "\"with\nnewline\"");
    }
}
