//! Unified ID collection utility for batch processing.
//!
//! This module provides the `IdSource` struct for gathering PDB IDs from multiple sources:
//! - Command-line arguments
//! - List files
//! - Standard input (for piping)

use crate::error::Result;
use std::collections::HashSet;
use std::io::{self, BufRead};
use std::path::Path;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};

/// A collection of PDB IDs gathered from multiple sources.
#[derive(Debug, Default)]
pub struct IdSource {
    /// The collected IDs (deduplicated).
    pub ids: Vec<String>,
}

impl IdSource {
    /// Collect IDs from multiple sources: command-line args, a list file, and/or stdin.
    ///
    /// # Arguments
    ///
    /// * `args_ids` - IDs provided as command-line arguments
    /// * `list_file` - Optional path to a file containing IDs (one per line)
    /// * `use_stdin` - Whether to read IDs from stdin
    ///
    /// # Returns
    ///
    /// An `IdSource` containing deduplicated IDs from all sources.
    ///
    /// # Notes
    ///
    /// - Empty lines and lines starting with `#` are skipped (comments)
    /// - IDs are deduplicated while preserving order (first occurrence wins)
    /// - When using stdin, the function reads synchronously since tokio's stdin
    ///   has limitations with piped input detection
    pub async fn collect(
        args_ids: Vec<String>,
        list_file: Option<&Path>,
        use_stdin: bool,
    ) -> Result<Self> {
        let mut seen = HashSet::new();
        let mut ids = Vec::new();

        // Helper to add an ID if not seen
        let mut add_id = |id: String| {
            let trimmed = id.trim().to_string();
            if !trimmed.is_empty() && !trimmed.starts_with('#') && seen.insert(trimmed.clone()) {
                ids.push(trimmed);
            }
        };

        // Add IDs from command-line args
        for id in args_ids {
            add_id(id);
        }

        // Add IDs from list file
        if let Some(path) = list_file {
            let file_ids = read_id_list_async(path).await?;
            for id in file_ids {
                add_id(id);
            }
        }

        // Add IDs from stdin
        if use_stdin {
            let stdin_ids = read_stdin_ids()?;
            for id in stdin_ids {
                add_id(id);
            }
        }

        Ok(Self { ids })
    }

    /// Check if the source contains any IDs.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }
}

/// Read IDs from a file asynchronously (one per line).
async fn read_id_list_async(path: &Path) -> Result<Vec<String>> {
    let file = fs::File::open(path).await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut ids = Vec::new();

    while let Some(line) = lines.next_line().await? {
        let trimmed = line.trim();
        // Skip empty lines and comments
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            ids.push(trimmed.to_string());
        }
    }

    Ok(ids)
}

/// Read IDs from stdin synchronously.
///
/// This must be synchronous because tokio's stdin has limitations when detecting
/// whether input is piped vs interactive.
fn read_stdin_ids() -> Result<Vec<String>> {
    let stdin = io::stdin();
    let reader = stdin.lock();
    let mut ids = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        // Skip empty lines and comments
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            ids.push(trimmed.to_string());
        }
    }

    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_collect_from_args_only() {
        let ids = IdSource::collect(vec!["1abc".to_string(), "2xyz".to_string()], None, false)
            .await
            .unwrap();

        assert_eq!(ids.ids, vec!["1abc", "2xyz"]);
    }

    #[tokio::test]
    async fn test_collect_from_file_only() {
        let temp_dir = tempfile::tempdir().unwrap();
        let list_file = temp_dir.path().join("ids.txt");

        let mut file = fs::File::create(&list_file).await.unwrap();
        file.write_all(b"1abc\n2xyz\n# comment\n\n3def\n")
            .await
            .unwrap();
        file.flush().await.unwrap();

        let ids = IdSource::collect(vec![], Some(&list_file), false)
            .await
            .unwrap();

        assert_eq!(ids.ids, vec!["1abc", "2xyz", "3def"]);
    }

    #[tokio::test]
    async fn test_collect_deduplication() {
        let temp_dir = tempfile::tempdir().unwrap();
        let list_file = temp_dir.path().join("ids.txt");

        let mut file = fs::File::create(&list_file).await.unwrap();
        file.write_all(b"1abc\n2xyz\n").await.unwrap();
        file.flush().await.unwrap();

        // 1abc appears in both args and file
        let ids = IdSource::collect(
            vec!["1abc".to_string(), "4hhb".to_string()],
            Some(&list_file),
            false,
        )
        .await
        .unwrap();

        // Should preserve order: 1abc, 4hhb from args, then 2xyz from file (1abc skipped)
        assert_eq!(ids.ids, vec!["1abc", "4hhb", "2xyz"]);
    }

    #[tokio::test]
    async fn test_collect_empty_lines_and_comments() {
        let temp_dir = tempfile::tempdir().unwrap();
        let list_file = temp_dir.path().join("ids.txt");

        let mut file = fs::File::create(&list_file).await.unwrap();
        file.write_all(b"# Header comment\n\n1abc\n  # inline comment\n  \n2xyz\n")
            .await
            .unwrap();
        file.flush().await.unwrap();

        let ids = IdSource::collect(vec![], Some(&list_file), false)
            .await
            .unwrap();

        assert_eq!(ids.ids, vec!["1abc", "2xyz"]);
    }

    #[tokio::test]
    async fn test_is_empty() {
        let ids = IdSource::collect(vec![], None, false).await.unwrap();
        assert!(ids.is_empty());
        assert_eq!(ids.ids.len(), 0);

        let ids = IdSource::collect(vec!["1abc".to_string()], None, false)
            .await
            .unwrap();
        assert!(!ids.is_empty());
        assert_eq!(ids.ids.len(), 1);
    }

    #[test]
    fn test_read_id_list_async_sync() {
        // Test the async read function via tokio runtime
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let temp_dir = tempfile::tempdir().unwrap();
            let list_file = temp_dir.path().join("ids.txt");

            let mut file = fs::File::create(&list_file).await.unwrap();
            file.write_all(b"1abc\n2xyz\n").await.unwrap();
            file.flush().await.unwrap();

            let ids = read_id_list_async(&list_file).await.unwrap();
            assert_eq!(ids, vec!["1abc", "2xyz"]);
        });
    }
}
