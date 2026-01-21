//! File conversion module for PDB files.
//!
//! This module provides functionality for:
//! - Compressing and decompressing gzip files
//! - Converting between PDB and mmCIF formats (requires gemmi)

pub mod compress;
pub mod format;

pub use compress::{compress_file, decompress_file, is_gzipped};
pub use format::{
    build_output_filename, check_gemmi_available, convert_with_gemmi, detect_format_from_path,
};

use crate::error::{PdbSyncError, Result};
use crate::files::FileFormat;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Semaphore;

/// A single conversion task specification.
#[derive(Debug, Clone)]
pub struct ConvertTask {
    /// Source file path
    pub source: PathBuf,
    /// Destination file path
    pub dest: PathBuf,
    /// Operation to perform
    pub operation: ConvertOperation,
}

/// The type of conversion operation to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertOperation {
    /// Decompress a .gz file
    Decompress,
    /// Compress a file to .gz format
    Compress,
    /// Convert format using gemmi (e.g., PDB <-> mmCIF)
    ConvertFormat(FileFormat),
}

impl std::fmt::Display for ConvertOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConvertOperation::Decompress => write!(f, "decompress"),
            ConvertOperation::Compress => write!(f, "compress"),
            ConvertOperation::ConvertFormat(fmt) => write!(f, "convert to {}", fmt),
        }
    }
}

/// Result of a conversion operation.
#[derive(Debug)]
pub enum ConvertResult {
    /// Conversion completed successfully
    Success {
        source: PathBuf,
        dest: PathBuf,
        #[allow(dead_code)]
        operation: ConvertOperation,
    },
    /// Conversion failed with an error
    Failed {
        source: PathBuf,
        #[allow(dead_code)]
        operation: ConvertOperation,
        error: String,
    },
    /// Conversion was skipped
    Skipped { source: PathBuf, reason: String },
}

impl ConvertResult {
    /// Create a success result.
    pub fn success(source: PathBuf, dest: PathBuf, operation: ConvertOperation) -> Self {
        Self::Success {
            source,
            dest,
            operation,
        }
    }

    /// Create a failed result.
    pub fn failed(source: PathBuf, operation: ConvertOperation, error: impl Into<String>) -> Self {
        Self::Failed {
            source,
            operation,
            error: error.into(),
        }
    }

    /// Create a skipped result.
    pub fn skipped(source: PathBuf, reason: impl Into<String>) -> Self {
        Self::Skipped {
            source,
            reason: reason.into(),
        }
    }

    /// Check if this is a success.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Check if this is a failure.
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// Check if this was skipped.
    pub fn is_skipped(&self) -> bool {
        matches!(self, Self::Skipped { .. })
    }
}

/// Converter that handles parallel conversion operations.
pub struct Converter {
    semaphore: Arc<Semaphore>,
}

impl Converter {
    /// Create a new converter with the specified parallelism.
    pub fn new(parallel: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(parallel)),
        }
    }

    /// Convert multiple files in parallel.
    pub async fn convert_many(&self, tasks: Vec<ConvertTask>) -> Vec<ConvertResult> {
        let futures: Vec<_> = tasks
            .into_iter()
            .map(|task| self.convert_with_semaphore(task))
            .collect();

        futures_util::future::join_all(futures).await
    }

    /// Convert a single task with semaphore-controlled concurrency.
    async fn convert_with_semaphore(&self, task: ConvertTask) -> ConvertResult {
        let _permit = match self.semaphore.acquire().await {
            Ok(p) => p,
            Err(_) => {
                return ConvertResult::failed(
                    task.source.clone(),
                    task.operation,
                    "Internal error: semaphore closed",
                )
            }
        };
        self.convert_single(task).await
    }

    /// Execute a single conversion task.
    async fn convert_single(&self, task: ConvertTask) -> ConvertResult {
        // Verify source exists
        if !task.source.exists() {
            return ConvertResult::failed(
                task.source.clone(),
                task.operation,
                format!("Source file not found: {}", task.source.display()),
            );
        }

        // Create parent directory if needed
        if let Some(parent) = task.dest.parent() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                return ConvertResult::failed(
                    task.source.clone(),
                    task.operation,
                    format!("Failed to create directory: {}", e),
                );
            }
        }

        match task.operation {
            ConvertOperation::Decompress => self.execute_decompress(&task.source, &task.dest).await,
            ConvertOperation::Compress => self.execute_compress(&task.source, &task.dest).await,
            ConvertOperation::ConvertFormat(to_format) => {
                self.execute_format_convert(&task.source, &task.dest, to_format)
                    .await
            }
        }
    }

    async fn execute_decompress(&self, source: &Path, dest: &Path) -> ConvertResult {
        // Check if source is actually gzipped
        match is_gzipped(source).await {
            Ok(true) => {}
            Ok(false) => {
                return ConvertResult::skipped(
                    source.to_path_buf(),
                    "File is not gzipped".to_string(),
                );
            }
            Err(e) => {
                return ConvertResult::failed(
                    source.to_path_buf(),
                    ConvertOperation::Decompress,
                    format!("Failed to check file: {}", e),
                );
            }
        }

        match decompress_file(source, dest).await {
            Ok(()) => ConvertResult::success(
                source.to_path_buf(),
                dest.to_path_buf(),
                ConvertOperation::Decompress,
            ),
            Err(e) => ConvertResult::failed(
                source.to_path_buf(),
                ConvertOperation::Decompress,
                e.to_string(),
            ),
        }
    }

    async fn execute_compress(&self, source: &Path, dest: &Path) -> ConvertResult {
        // Check if source is already gzipped
        match is_gzipped(source).await {
            Ok(true) => {
                return ConvertResult::skipped(
                    source.to_path_buf(),
                    "File is already gzipped".to_string(),
                );
            }
            Ok(false) => {}
            Err(e) => {
                return ConvertResult::failed(
                    source.to_path_buf(),
                    ConvertOperation::Compress,
                    format!("Failed to check file: {}", e),
                );
            }
        }

        match compress_file(source, dest).await {
            Ok(()) => ConvertResult::success(
                source.to_path_buf(),
                dest.to_path_buf(),
                ConvertOperation::Compress,
            ),
            Err(e) => ConvertResult::failed(
                source.to_path_buf(),
                ConvertOperation::Compress,
                e.to_string(),
            ),
        }
    }

    async fn execute_format_convert(
        &self,
        source: &Path,
        dest: &Path,
        to_format: FileFormat,
    ) -> ConvertResult {
        // For format conversion, we need to handle compressed files
        // If source is gzipped, decompress first, convert, then optionally recompress

        let is_source_gzipped = is_gzipped(source).await.unwrap_or(false);
        let needs_compression = to_format.is_compressed();

        // Use RAII guard for temp file cleanup (cleaned up on drop, even on panic)
        let mut temp_files = TempFileGuard::new();

        // Create a temporary file for decompressed source if needed
        let temp_source = if is_source_gzipped {
            let parent = source.parent().unwrap_or(Path::new("."));
            let temp_path = match tempfile::Builder::new()
                .prefix("pdb_convert_src_")
                .tempfile_in(parent)
            {
                Ok(f) => f.into_temp_path(),
                Err(e) => {
                    return ConvertResult::failed(
                        source.to_path_buf(),
                        ConvertOperation::ConvertFormat(to_format),
                        format!("Failed to create temp file: {}", e),
                    );
                }
            };

            if let Err(e) = decompress_file(source, &temp_path).await {
                return ConvertResult::failed(
                    source.to_path_buf(),
                    ConvertOperation::ConvertFormat(to_format),
                    format!("Failed to decompress source: {}", e),
                );
            }
            Some(temp_path.to_path_buf())
        } else {
            None
        };

        // Register temp source for cleanup
        if let Some(ref path) = temp_source {
            temp_files.add(path.clone());
        }

        let source_path_buf = source.to_path_buf();
        let actual_source = temp_source.as_ref().unwrap_or(&source_path_buf);

        // Create temp destination if we need to compress afterward
        let (temp_dest, final_dest) = if needs_compression {
            let parent = dest.parent().unwrap_or(Path::new("."));
            let temp_path = match tempfile::Builder::new()
                .prefix("pdb_convert_dst_")
                .tempfile_in(parent)
            {
                Ok(f) => f.into_temp_path().to_path_buf(),
                Err(e) => {
                    return ConvertResult::failed(
                        source.to_path_buf(),
                        ConvertOperation::ConvertFormat(to_format),
                        format!("Failed to create temp file: {}", e),
                    );
                }
            };
            temp_files.add(temp_path.clone());
            (temp_path, dest.to_path_buf())
        } else {
            (dest.to_path_buf(), dest.to_path_buf())
        };

        // Perform format conversion
        if let Err(e) = convert_with_gemmi(actual_source, &temp_dest, to_format.base_format()).await
        {
            return ConvertResult::failed(
                source.to_path_buf(),
                ConvertOperation::ConvertFormat(to_format),
                e.to_string(),
            );
        }

        // Compress output if needed
        if needs_compression {
            if let Err(e) = compress_file(&temp_dest, &final_dest).await {
                return ConvertResult::failed(
                    source.to_path_buf(),
                    ConvertOperation::ConvertFormat(to_format),
                    format!("Failed to compress output: {}", e),
                );
            }
        }

        // Clear temp files from guard (don't delete them in success case where they're already gone)
        temp_files.clear();

        ConvertResult::success(
            source.to_path_buf(),
            dest.to_path_buf(),
            ConvertOperation::ConvertFormat(to_format),
        )
    }
}

/// RAII guard for cleaning up temporary files on drop.
/// Ensures temp files are deleted even if a panic occurs.
struct TempFileGuard {
    paths: Vec<PathBuf>,
}

impl TempFileGuard {
    fn new() -> Self {
        Self { paths: Vec::new() }
    }

    fn add(&mut self, path: PathBuf) {
        self.paths.push(path);
    }

    fn clear(&mut self) {
        self.paths.clear();
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        for path in &self.paths {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// Build the destination path for a conversion operation.
pub fn build_dest_path(
    source: &Path,
    dest_dir: Option<&Path>,
    operation: ConvertOperation,
    in_place: bool,
) -> Result<PathBuf> {
    let source_name = source
        .file_name()
        .ok_or_else(|| PdbSyncError::Path("Invalid source path".into()))?
        .to_str()
        .ok_or_else(|| PdbSyncError::Path("Invalid source filename".into()))?;

    let new_name = match operation {
        ConvertOperation::Decompress => {
            // Remove .gz extension
            if let Some(stripped) = source_name.strip_suffix(".gz") {
                stripped.to_string()
            } else {
                return Err(PdbSyncError::Path(format!(
                    "Cannot decompress: {} doesn't have .gz extension",
                    source_name
                )));
            }
        }
        ConvertOperation::Compress => {
            // Add .gz extension
            format!("{}.gz", source_name)
        }
        ConvertOperation::ConvertFormat(to_format) => build_output_filename(source, to_format)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| PdbSyncError::Path("Failed to build output filename".into()))?
            .to_string(),
    };

    let dest = if in_place {
        source.parent().unwrap_or(Path::new(".")).join(&new_name)
    } else if let Some(dir) = dest_dir {
        dir.join(&new_name)
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(&new_name)
    };

    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_converter_decompress() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("test.txt.gz");
        let dest = dir.path().join("test.txt");

        // Create a compressed file
        let content = b"Hello, world!";
        let temp_uncompressed = dir.path().join("temp.txt");
        {
            let mut file = tokio::fs::File::create(&temp_uncompressed).await.unwrap();
            file.write_all(content).await.unwrap();
            file.flush().await.unwrap();
        }
        compress_file(&temp_uncompressed, &source).await.unwrap();

        let converter = Converter::new(4);
        let task = ConvertTask {
            source: source.clone(),
            dest: dest.clone(),
            operation: ConvertOperation::Decompress,
        };

        let result = converter.convert_single(task).await;
        assert!(result.is_success());

        // Verify decompressed content
        let decompressed = tokio::fs::read(&dest).await.unwrap();
        assert_eq!(decompressed, content);
    }

    #[tokio::test]
    async fn test_converter_compress() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("test.txt");
        let dest = dir.path().join("test.txt.gz");

        // Create an uncompressed file
        let content = b"Hello, world!";
        {
            let mut file = tokio::fs::File::create(&source).await.unwrap();
            file.write_all(content).await.unwrap();
            file.flush().await.unwrap();
        }

        let converter = Converter::new(4);
        let task = ConvertTask {
            source: source.clone(),
            dest: dest.clone(),
            operation: ConvertOperation::Compress,
        };

        let result = converter.convert_single(task).await;
        assert!(result.is_success());

        // Verify the output is gzipped
        assert!(is_gzipped(&dest).await.unwrap());
    }

    #[tokio::test]
    async fn test_converter_skip_already_compressed() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("test.txt.gz");
        let dest = dir.path().join("test2.txt.gz");

        // Create a compressed file
        let temp = dir.path().join("temp.txt");
        {
            let mut file = tokio::fs::File::create(&temp).await.unwrap();
            file.write_all(b"test").await.unwrap();
            file.flush().await.unwrap();
        }
        compress_file(&temp, &source).await.unwrap();

        let converter = Converter::new(4);
        let task = ConvertTask {
            source: source.clone(),
            dest: dest.clone(),
            operation: ConvertOperation::Compress,
        };

        let result = converter.convert_single(task).await;
        assert!(result.is_skipped());
    }

    #[tokio::test]
    async fn test_converter_skip_not_gzipped() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("test.txt");
        let dest = dir.path().join("test_decompressed.txt");

        // Create a plain text file
        {
            let mut file = tokio::fs::File::create(&source).await.unwrap();
            file.write_all(b"test").await.unwrap();
            file.flush().await.unwrap();
        }

        let converter = Converter::new(4);
        let task = ConvertTask {
            source: source.clone(),
            dest: dest.clone(),
            operation: ConvertOperation::Decompress,
        };

        let result = converter.convert_single(task).await;
        assert!(result.is_skipped());
    }

    #[test]
    fn test_build_dest_path_decompress() {
        let source = Path::new("/tmp/test.cif.gz");
        let dest = build_dest_path(source, None, ConvertOperation::Decompress, true).unwrap();
        assert!(dest.ends_with("test.cif"));
    }

    #[test]
    fn test_build_dest_path_compress() {
        let source = Path::new("/tmp/test.cif");
        let dest = build_dest_path(source, None, ConvertOperation::Compress, true).unwrap();
        assert!(dest.ends_with("test.cif.gz"));
    }

    #[test]
    fn test_build_dest_path_with_dest_dir() {
        let source = Path::new("/tmp/test.cif.gz");
        let dest_dir = Path::new("/output");
        let dest =
            build_dest_path(source, Some(dest_dir), ConvertOperation::Decompress, false).unwrap();
        assert_eq!(dest, PathBuf::from("/output/test.cif"));
    }
}
