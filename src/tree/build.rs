//! Tree building logic for directory structure traversal.

use super::DirNode;
use crate::error::Result;
use crate::files::FileFormat;
use std::path::Path;
use tokio::fs;

/// Options for building the tree
#[derive(Debug, Clone, Default)]
pub struct TreeOptions {
    /// Maximum depth to traverse (None = unlimited)
    pub max_depth: Option<usize>,
    /// Filter by file format
    pub format_filter: Option<FileFormat>,
    /// Only include non-empty directories
    pub non_empty: bool,
}

/// Build a directory tree from the PDB mirror directory
pub async fn build_tree(pdb_dir: &Path, options: &TreeOptions) -> Result<DirNode> {
    let name = pdb_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| pdb_dir.to_string_lossy().to_string());

    let mut root = DirNode::new(name, pdb_dir.to_path_buf());
    build_tree_recursive(&mut root, 0, options).await?;
    Ok(root)
}

/// Recursively build the tree
async fn build_tree_recursive(
    node: &mut DirNode,
    current_depth: usize,
    options: &TreeOptions,
) -> Result<()> {
    // Check depth limit
    if let Some(max_depth) = options.max_depth {
        if current_depth >= max_depth {
            // Summarize remaining contents
            let (count, size) = count_files_recursive(&node.path, options).await?;
            node.file_count = count;
            node.total_size = size;
            node.is_leaf = true;
            return Ok(());
        }
    }

    // Read directory entries
    let mut entries = match fs::read_dir(&node.path).await {
        Ok(entries) => entries,
        Err(e) => {
            tracing::debug!(
                "Skipping inaccessible directory {}: {}",
                node.path.display(),
                e
            );
            return Ok(());
        }
    };

    let mut children = Vec::new();
    let mut file_count = 0u64;
    let mut total_size = 0u64;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let metadata = entry.metadata().await?;

        if metadata.is_dir() {
            // Apply format filter at top level (mmCIF, pdb, bcif directories)
            if current_depth == 0 {
                if let Some(format) = options.format_filter {
                    let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    let target_subdir = format.subdir();
                    if dir_name != target_subdir {
                        continue;
                    }
                }
            }

            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let mut child = DirNode::new(name, path);
            Box::pin(build_tree_recursive(&mut child, current_depth + 1, options)).await?;

            // Skip empty directories if non_empty option is set
            if options.non_empty && child.file_count == 0 {
                continue;
            }

            file_count += child.file_count;
            total_size += child.total_size;
            children.push(child);
        } else if metadata.is_file() {
            // Check if file matches format filter
            if is_pdb_file(&path, options) {
                file_count += 1;
                total_size += metadata.len();
            }
        }
    }

    // Sort children by name
    children.sort_by(|a, b| a.name.cmp(&b.name));

    node.children = children;
    node.file_count = file_count;
    node.total_size = total_size;
    node.is_leaf = node.children.is_empty() && file_count > 0;

    Ok(())
}

/// Count files recursively for summarizing at depth limit
async fn count_files_recursive(path: &Path, options: &TreeOptions) -> Result<(u64, u64)> {
    let mut total_count = 0u64;
    let mut total_size = 0u64;

    let mut stack = vec![path.to_path_buf()];

    while let Some(current) = stack.pop() {
        let mut entries = match fs::read_dir(&current).await {
            Ok(entries) => entries,
            Err(e) => {
                tracing::debug!(
                    "Skipping inaccessible directory {}: {}",
                    current.display(),
                    e
                );
                continue;
            }
        };

        while let Some(entry) = entries.next_entry().await? {
            let entry_path = entry.path();
            let metadata = entry.metadata().await?;

            if metadata.is_dir() {
                stack.push(entry_path);
            } else if metadata.is_file() && is_pdb_file(&entry_path, options) {
                total_count += 1;
                total_size += metadata.len();
            }
        }
    }

    Ok((total_count, total_size))
}

/// Check if a file is a PDB file matching the format filter
fn is_pdb_file(path: &Path, options: &TreeOptions) -> bool {
    let file_name = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return false,
    };

    // Check extension patterns
    let is_cif = file_name.ends_with(".cif.gz");
    let is_pdb = file_name.ends_with(".ent.gz");
    let is_bcif = file_name.ends_with(".bcif.gz");

    if !is_cif && !is_pdb && !is_bcif {
        return false;
    }

    // Apply format filter if specified
    if let Some(format) = options.format_filter {
        match format {
            FileFormat::Mmcif | FileFormat::CifGz => is_cif,
            FileFormat::Pdb | FileFormat::PdbGz => is_pdb,
            FileFormat::Bcif | FileFormat::BcifGz => is_bcif,
        }
    } else {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_structure(temp_dir: &TempDir) -> std::io::Result<()> {
        use std::fs;

        // Create mmCIF structure
        let mmcif_aa = temp_dir.path().join("mmCIF/aa");
        fs::create_dir_all(&mmcif_aa)?;
        fs::write(mmcif_aa.join("1aaa.cif.gz"), "test data 1")?;
        fs::write(mmcif_aa.join("1aab.cif.gz"), "test data 12")?;

        let mmcif_ab = temp_dir.path().join("mmCIF/ab");
        fs::create_dir_all(&mmcif_ab)?;
        fs::write(mmcif_ab.join("1abc.cif.gz"), "test data 123")?;

        // Create pdb structure
        let pdb_aa = temp_dir.path().join("pdb/aa");
        fs::create_dir_all(&pdb_aa)?;
        fs::write(pdb_aa.join("pdb1aaa.ent.gz"), "pdb data 1234")?;

        // Create bcif structure
        let bcif_aa = temp_dir.path().join("bcif/aa");
        fs::create_dir_all(&bcif_aa)?;
        fs::write(bcif_aa.join("1aaa.bcif.gz"), "bcif data 12345")?;

        // Create empty directory
        fs::create_dir_all(temp_dir.path().join("mmCIF/zz"))?;

        Ok(())
    }

    #[tokio::test]
    async fn test_build_tree_basic() {
        let temp_dir = TempDir::new().unwrap();
        create_test_structure(&temp_dir).unwrap();

        let options = TreeOptions::default();
        let tree = build_tree(temp_dir.path(), &options).await.unwrap();

        // Should have 3 format directories
        assert_eq!(tree.children.len(), 3);
        assert_eq!(tree.file_count, 5); // Total files across all formats
    }

    #[tokio::test]
    async fn test_build_tree_with_format_filter() {
        let temp_dir = TempDir::new().unwrap();
        create_test_structure(&temp_dir).unwrap();

        let options = TreeOptions {
            format_filter: Some(FileFormat::CifGz),
            ..Default::default()
        };
        let tree = build_tree(temp_dir.path(), &options).await.unwrap();

        // Should only have mmCIF directory
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].name, "mmCIF");
        assert_eq!(tree.file_count, 3); // Only cif.gz files
    }

    #[tokio::test]
    async fn test_build_tree_with_depth_limit() {
        let temp_dir = TempDir::new().unwrap();
        create_test_structure(&temp_dir).unwrap();

        let options = TreeOptions {
            max_depth: Some(1),
            ..Default::default()
        };
        let tree = build_tree(temp_dir.path(), &options).await.unwrap();

        // Children should exist but be leaves (not have grandchildren)
        for child in &tree.children {
            assert!(child.is_leaf || child.children.is_empty());
        }
    }

    #[tokio::test]
    async fn test_build_tree_non_empty_filter() {
        let temp_dir = TempDir::new().unwrap();
        create_test_structure(&temp_dir).unwrap();

        let options = TreeOptions {
            non_empty: true,
            ..Default::default()
        };
        let tree = build_tree(temp_dir.path(), &options).await.unwrap();

        // Find mmCIF directory
        let mmcif = tree.children.iter().find(|c| c.name == "mmCIF").unwrap();

        // Should not contain the empty "zz" directory
        assert!(!mmcif.children.iter().any(|c| c.name == "zz"));
    }

    #[test]
    fn test_is_pdb_file_cif() {
        let options = TreeOptions::default();
        assert!(is_pdb_file(Path::new("/tmp/1abc.cif.gz"), &options));
        assert!(!is_pdb_file(Path::new("/tmp/1abc.cif"), &options));
        assert!(!is_pdb_file(Path::new("/tmp/1abc.txt"), &options));
    }

    #[test]
    fn test_is_pdb_file_pdb() {
        let options = TreeOptions::default();
        assert!(is_pdb_file(Path::new("/tmp/pdb1abc.ent.gz"), &options));
        assert!(!is_pdb_file(Path::new("/tmp/pdb1abc.ent"), &options));
    }

    #[test]
    fn test_is_pdb_file_bcif() {
        let options = TreeOptions::default();
        assert!(is_pdb_file(Path::new("/tmp/1abc.bcif.gz"), &options));
        assert!(!is_pdb_file(Path::new("/tmp/1abc.bcif"), &options));
    }

    #[test]
    fn test_is_pdb_file_with_format_filter() {
        let options = TreeOptions {
            format_filter: Some(FileFormat::CifGz),
            ..Default::default()
        };
        assert!(is_pdb_file(Path::new("/tmp/1abc.cif.gz"), &options));
        assert!(!is_pdb_file(Path::new("/tmp/pdb1abc.ent.gz"), &options));
        assert!(!is_pdb_file(Path::new("/tmp/1abc.bcif.gz"), &options));
    }
}
