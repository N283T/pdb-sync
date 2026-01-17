//! Tree command - visualizes the local PDB directory structure.

pub mod build;
pub mod render;

pub use build::{build_tree, TreeOptions};
pub use render::render_tree;

use serde::Serialize;
use std::path::PathBuf;

/// Directory node in the tree
#[derive(Debug, Clone, Serialize)]
pub struct DirNode {
    pub name: String,
    pub path: PathBuf,
    pub file_count: u64,
    pub total_size: u64,
    pub children: Vec<DirNode>,
    pub is_leaf: bool,
}

impl DirNode {
    /// Create a new directory node
    pub fn new(name: String, path: PathBuf) -> Self {
        Self {
            name,
            path,
            file_count: 0,
            total_size: 0,
            children: Vec::new(),
            is_leaf: false,
        }
    }
}

/// Field to sort top directories by
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum SortBy {
    /// Sort by file count
    #[default]
    Count,
    /// Sort by total size
    Size,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dir_node_new() {
        let node = DirNode::new("test".to_string(), PathBuf::from("/tmp/test"));
        assert_eq!(node.name, "test");
        assert_eq!(node.path, PathBuf::from("/tmp/test"));
        assert_eq!(node.file_count, 0);
        assert_eq!(node.total_size, 0);
        assert!(node.children.is_empty());
        assert!(!node.is_leaf);
    }
}
