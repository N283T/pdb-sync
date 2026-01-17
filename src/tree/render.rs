//! Tree rendering logic for ASCII tree output.

use super::{DirNode, SortBy};

/// Options for rendering the tree
#[derive(Debug, Clone, Default)]
pub struct RenderOptions {
    /// Show file sizes
    pub show_size: bool,
    /// Show file counts
    pub show_count: bool,
    /// Hide summary line
    pub no_summary: bool,
}

/// Render the tree to a string
pub fn render_tree(node: &DirNode, options: &RenderOptions) -> String {
    let mut output = String::new();

    // Show both size and count by default if neither is specified
    let effective_options = if !options.show_size && !options.show_count {
        RenderOptions {
            show_size: true,
            show_count: true,
            ..*options
        }
    } else {
        options.clone()
    };

    // Render the root path
    output.push_str(&node.path.display().to_string());
    output.push('\n');

    // Render children
    let children_count = node.children.len();
    for (i, child) in node.children.iter().enumerate() {
        let is_last = i == children_count - 1;
        render_node(&mut output, child, "", is_last, &effective_options);
    }

    // Add summary unless disabled
    if !options.no_summary {
        output.push('\n');
        let summary = format_summary(node, &effective_options);
        output.push_str(&summary);
        output.push('\n');
    }

    output
}

/// Render a single node recursively
fn render_node(
    output: &mut String,
    node: &DirNode,
    prefix: &str,
    is_last: bool,
    options: &RenderOptions,
) {
    // Build the connector
    let connector = if is_last { "└── " } else { "├── " };

    // Format node name with stats
    let stats = format_node_stats(node, options);
    let name_with_stats = if stats.is_empty() {
        format!("{}/", node.name)
    } else {
        format!("{}/ ({})", node.name, stats)
    };

    output.push_str(prefix);
    output.push_str(connector);
    output.push_str(&name_with_stats);
    output.push('\n');

    // Render children
    let new_prefix = format!("{}{}   ", prefix, if is_last { " " } else { "│" });
    let children_count = node.children.len();

    for (i, child) in node.children.iter().enumerate() {
        let child_is_last = i == children_count - 1;
        render_node(output, child, &new_prefix, child_is_last, options);
    }
}

/// Format node statistics string
fn format_node_stats(node: &DirNode, options: &RenderOptions) -> String {
    let mut parts = Vec::new();

    if options.show_count && node.file_count > 0 {
        let files_str = if node.file_count == 1 {
            "1 file".to_string()
        } else {
            format!("{} files", format_number(node.file_count))
        };
        parts.push(files_str);
    }

    if options.show_size && node.total_size > 0 {
        parts.push(human_bytes(node.total_size));
    }

    parts.join(", ")
}

/// Format the summary line
fn format_summary(node: &DirNode, options: &RenderOptions) -> String {
    let mut parts = Vec::new();

    if options.show_count {
        let files_str = if node.file_count == 1 {
            "1 file".to_string()
        } else {
            format!("{} files", format_number(node.file_count))
        };
        parts.push(files_str);
    }

    if options.show_size {
        parts.push(human_bytes(node.total_size));
    }

    if parts.is_empty() {
        String::new()
    } else {
        format!("Total: {}", parts.join(", "))
    }
}

/// Render top N directories sorted by count or size
pub fn render_top_directories(root: &DirNode, top_n: usize, sort_by: SortBy) -> String {
    let mut output = String::new();

    // Collect all leaf directories
    let mut leaves: Vec<&DirNode> = Vec::new();
    collect_leaf_directories(root, &mut leaves);

    // Sort by the specified field (descending)
    match sort_by {
        SortBy::Count => leaves.sort_by(|a, b| b.file_count.cmp(&a.file_count)),
        SortBy::Size => leaves.sort_by(|a, b| b.total_size.cmp(&a.total_size)),
    }

    // Take top N
    let top_dirs: Vec<&DirNode> = leaves.into_iter().take(top_n).collect();

    // Header
    let sort_label = match sort_by {
        SortBy::Count => "file count",
        SortBy::Size => "size",
    };
    output.push_str(&format!("Top {} directories by {}:\n\n", top_n, sort_label));

    // Table header
    output.push_str(&format!(
        "{:<50} {:>12} {:>12}\n",
        "Directory", "Files", "Size"
    ));
    output.push_str(&format!("{:-<50} {:->12} {:->12}\n", "", "", ""));

    // Rows
    for node in &top_dirs {
        let path_str = node.path.display().to_string();
        let truncated_path = if path_str.len() > 50 {
            format!("...{}", &path_str[path_str.len() - 47..])
        } else {
            path_str
        };
        output.push_str(&format!(
            "{:<50} {:>12} {:>12}\n",
            truncated_path,
            format_number(node.file_count),
            human_bytes(node.total_size)
        ));
    }

    output
}

/// Collect all leaf directories (directories containing files but no subdirectories)
fn collect_leaf_directories<'a>(node: &'a DirNode, leaves: &mut Vec<&'a DirNode>) {
    if node.is_leaf || (node.children.is_empty() && node.file_count > 0) {
        leaves.push(node);
    } else {
        for child in &node.children {
            collect_leaf_directories(child, leaves);
        }
    }
}

/// Convert bytes to human-readable format
fn human_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format a number with thousand separators
fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(*c);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_tree() -> DirNode {
        DirNode {
            name: "pdb_mirror".to_string(),
            path: PathBuf::from("/data/pdb"),
            file_count: 5,
            total_size: 1234567890,
            is_leaf: false,
            children: vec![
                DirNode {
                    name: "mmCIF".to_string(),
                    path: PathBuf::from("/data/pdb/mmCIF"),
                    file_count: 3,
                    total_size: 987654321,
                    is_leaf: false,
                    children: vec![
                        DirNode {
                            name: "aa".to_string(),
                            path: PathBuf::from("/data/pdb/mmCIF/aa"),
                            file_count: 2,
                            total_size: 500000000,
                            is_leaf: true,
                            children: vec![],
                        },
                        DirNode {
                            name: "ab".to_string(),
                            path: PathBuf::from("/data/pdb/mmCIF/ab"),
                            file_count: 1,
                            total_size: 487654321,
                            is_leaf: true,
                            children: vec![],
                        },
                    ],
                },
                DirNode {
                    name: "pdb".to_string(),
                    path: PathBuf::from("/data/pdb/pdb"),
                    file_count: 2,
                    total_size: 246913569,
                    is_leaf: true,
                    children: vec![],
                },
            ],
        }
    }

    #[test]
    fn test_render_tree_basic() {
        let tree = create_test_tree();
        let options = RenderOptions::default();
        let output = render_tree(&tree, &options);

        assert!(output.contains("/data/pdb"));
        assert!(output.contains("mmCIF/"));
        assert!(output.contains("pdb/"));
        assert!(output.contains("Total:"));
    }

    #[test]
    fn test_render_tree_with_count_only() {
        let tree = create_test_tree();
        let options = RenderOptions {
            show_count: true,
            show_size: false,
            ..Default::default()
        };
        let output = render_tree(&tree, &options);

        assert!(output.contains("5 files"));
        assert!(!output.contains("GB"));
    }

    #[test]
    fn test_render_tree_with_size_only() {
        let tree = create_test_tree();
        let options = RenderOptions {
            show_count: false,
            show_size: true,
            ..Default::default()
        };
        let output = render_tree(&tree, &options);

        assert!(output.contains("GB"));
    }

    #[test]
    fn test_render_tree_no_summary() {
        let tree = create_test_tree();
        let options = RenderOptions {
            no_summary: true,
            ..Default::default()
        };
        let output = render_tree(&tree, &options);

        assert!(!output.contains("Total:"));
    }

    #[test]
    fn test_format_node_stats() {
        let node = DirNode {
            name: "test".to_string(),
            path: PathBuf::from("/test"),
            file_count: 1234,
            total_size: 1073741824, // 1 GB
            is_leaf: true,
            children: vec![],
        };

        let options = RenderOptions {
            show_count: true,
            show_size: true,
            ..Default::default()
        };

        let stats = format_node_stats(&node, &options);
        assert!(stats.contains("1,234 files"));
        assert!(stats.contains("1.00 GB"));
    }

    #[test]
    fn test_human_bytes() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(1024), "1.00 KB");
        assert_eq!(human_bytes(1536), "1.50 KB");
        assert_eq!(human_bytes(1048576), "1.00 MB");
        assert_eq!(human_bytes(1073741824), "1.00 GB");
        assert_eq!(human_bytes(1099511627776), "1.00 TB");
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(123), "123");
        assert_eq!(format_number(1234), "1,234");
        assert_eq!(format_number(12345), "12,345");
        assert_eq!(format_number(123456), "123,456");
        assert_eq!(format_number(1234567), "1,234,567");
    }

    #[test]
    fn test_render_top_directories() {
        let tree = create_test_tree();
        let output = render_top_directories(&tree, 10, SortBy::Count);

        assert!(output.contains("Top 10 directories"));
        assert!(output.contains("file count"));
        assert!(output.contains("aa"));
    }

    #[test]
    fn test_collect_leaf_directories() {
        let tree = create_test_tree();
        let mut leaves = Vec::new();
        collect_leaf_directories(&tree, &mut leaves);

        // Should have 3 leaf directories: mmCIF/aa, mmCIF/ab, pdb
        assert_eq!(leaves.len(), 3);
    }

    #[test]
    fn test_single_file_grammar() {
        let node = DirNode {
            name: "single".to_string(),
            path: PathBuf::from("/test"),
            file_count: 1,
            total_size: 1024,
            is_leaf: true,
            children: vec![],
        };

        let options = RenderOptions {
            show_count: true,
            show_size: false,
            ..Default::default()
        };

        let stats = format_node_stats(&node, &options);
        assert_eq!(stats, "1 file");
    }
}
