//! aria2c download engine for parallel file downloads.
//!
//! This module provides an alternative download engine using the external
//! aria2c tool, which can significantly speed up downloads through
//! parallel connections and file splitting.

use crate::download::task::{DownloadResult, DownloadTask};
use crate::download::DownloadOptions;
use crate::download::HttpsDownloader;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// Check if aria2c is available in PATH.
#[allow(dead_code)]
pub fn is_available() -> bool {
    which::which("aria2c").is_ok()
}

/// Get the path to aria2c if available.
pub fn get_aria2c_path() -> Option<PathBuf> {
    which::which("aria2c").ok()
}

/// aria2c-based downloader for parallel file downloads.
pub struct Aria2cDownloader {
    aria2c_path: PathBuf,
    connections: u32,
    split: u32,
    max_concurrent: u32,
    options: DownloadOptions,
}

impl Aria2cDownloader {
    /// Create a new aria2c downloader with the given options.
    ///
    /// Returns `None` if aria2c is not available in PATH.
    pub fn new(options: DownloadOptions, connections: u32, split: u32) -> Option<Self> {
        let aria2c_path = get_aria2c_path()?;
        Some(Self {
            aria2c_path,
            connections,
            split,
            max_concurrent: options.parallel as u32,
            options,
        })
    }

    /// Download multiple tasks using aria2c.
    ///
    /// Uses the built-in HttpsDownloader to generate URLs and destination paths,
    /// then delegates the actual downloading to aria2c.
    pub async fn download_many(
        &self,
        tasks: Vec<DownloadTask>,
        dest: &Path,
    ) -> Vec<DownloadResult> {
        if tasks.is_empty() {
            return Vec::new();
        }

        // Create a temporary HttpsDownloader just for URL/path building
        let url_builder = HttpsDownloader::new(DownloadOptions {
            mirror: self.options.mirror,
            decompress: false, // aria2c downloads raw files
            overwrite: self.options.overwrite,
            parallel: 1,
            retry_count: self.options.retry_count,
            retry_delay: self.options.retry_delay,
        });

        // Generate input file content
        let input_content = self.generate_input_file(&tasks, dest, &url_builder);

        // Create temporary input file
        let temp_dir = std::env::temp_dir();
        let input_file = temp_dir.join(format!("aria2c-{}.txt", std::process::id()));

        if let Err(e) = tokio::fs::write(&input_file, &input_content).await {
            eprintln!("Failed to write aria2c input file: {}", e);
            return tasks
                .into_iter()
                .map(|t| {
                    DownloadResult::failed(
                        t.pdb_id.clone(),
                        t.data_type,
                        format!("Failed to create input file: {}", e),
                    )
                })
                .collect();
        }

        // Build aria2c command
        let mut cmd = Command::new(&self.aria2c_path);
        cmd.arg("-i")
            .arg(&input_file)
            .arg("-x")
            .arg(self.connections.to_string())
            .arg("-s")
            .arg(self.split.to_string())
            .arg("-j")
            .arg(self.max_concurrent.to_string())
            .arg("--auto-file-renaming=false")
            .arg("--console-log-level=warn")
            .arg("--summary-interval=0");

        if self.options.overwrite {
            cmd.arg("--allow-overwrite=true");
        }

        // Run aria2c
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        println!(
            "Running aria2c with {} connections, {} splits, {} concurrent downloads...",
            self.connections, self.split, self.max_concurrent
        );

        let result = match cmd.spawn() {
            Ok(mut child) => {
                // Stream stderr for progress
                if let Some(stderr) = child.stderr.take() {
                    let reader = BufReader::new(stderr);
                    let mut lines = reader.lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        eprintln!("{}", line);
                    }
                }

                child.wait().await
            }
            Err(e) => {
                eprintln!("Failed to run aria2c: {}", e);
                // Cleanup
                let _ = tokio::fs::remove_file(&input_file).await;
                return tasks
                    .into_iter()
                    .map(|t| {
                        DownloadResult::failed(
                            t.pdb_id.clone(),
                            t.data_type,
                            format!("Failed to execute aria2c: {}", e),
                        )
                    })
                    .collect();
            }
        };

        // Cleanup input file
        let _ = tokio::fs::remove_file(&input_file).await;

        // Check results
        match result {
            Ok(status) if status.success() => {
                // Verify which files were downloaded
                let mut results = Vec::new();
                for task in tasks {
                    let dest_path = url_builder.build_dest_path_for_task(dest, &task);
                    if dest_path.exists() {
                        println!("Downloaded: {}", dest_path.display());
                        results.push(DownloadResult::success(
                            task.pdb_id.clone(),
                            task.data_type,
                            dest_path,
                        ));
                    } else {
                        results.push(DownloadResult::failed(
                            task.pdb_id.clone(),
                            task.data_type,
                            "File not found after download".to_string(),
                        ));
                    }
                }
                results
            }
            Ok(status) => {
                eprintln!("aria2c exited with status: {}", status);
                // Still check which files were downloaded
                let mut results = Vec::new();
                for task in tasks {
                    let dest_path = url_builder.build_dest_path_for_task(dest, &task);
                    if dest_path.exists() {
                        results.push(DownloadResult::success(
                            task.pdb_id.clone(),
                            task.data_type,
                            dest_path,
                        ));
                    } else {
                        results.push(DownloadResult::failed(
                            task.pdb_id.clone(),
                            task.data_type,
                            format!("aria2c failed with status: {}", status),
                        ));
                    }
                }
                results
            }
            Err(e) => tasks
                .into_iter()
                .map(|t| {
                    DownloadResult::failed(
                        t.pdb_id.clone(),
                        t.data_type,
                        format!("Failed to wait for aria2c: {}", e),
                    )
                })
                .collect(),
        }
    }

    /// Generate aria2c input file content.
    ///
    /// Format:
    /// ```text
    /// https://example.com/file1.gz
    ///   dir=/path/to/dest
    ///   out=file1.gz
    /// https://example.com/file2.gz
    ///   dir=/path/to/dest
    ///   out=file2.gz
    /// ```
    pub fn generate_input_file(
        &self,
        tasks: &[DownloadTask],
        dest: &Path,
        url_builder: &HttpsDownloader,
    ) -> String {
        let mut content = String::new();
        let dest_str = dest.to_string_lossy();

        for task in tasks {
            let url = url_builder.build_url_for_task(task);
            let dest_path = url_builder.build_dest_path_for_task(dest, task);
            let filename = dest_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("{}.cif", task.pdb_id.as_str()));

            content.push_str(&url);
            content.push('\n');
            content.push_str(&format!("  dir={}\n", dest_str));
            content.push_str(&format!("  out={}\n", filename));
        }

        content
    }
}

/// Generate aria2c input file content for export (without downloading).
///
/// This is useful for users who want to manually run aria2c with custom options.
pub fn generate_export_input(
    tasks: &[DownloadTask],
    dest: &Path,
    options: &DownloadOptions,
) -> String {
    let url_builder = HttpsDownloader::new(DownloadOptions {
        mirror: options.mirror,
        decompress: false,
        overwrite: options.overwrite,
        parallel: 1,
        retry_count: options.retry_count,
        retry_delay: options.retry_delay,
    });

    let mut content = String::new();
    let dest_str = dest.to_string_lossy();

    for task in tasks {
        let url = url_builder.build_url_for_task(task);
        let dest_path = url_builder.build_dest_path_for_task(dest, task);
        let filename = dest_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("{}.cif", task.pdb_id.as_str()));

        content.push_str(&url);
        content.push('\n');
        content.push_str(&format!("  dir={}\n", dest_str));
        content.push_str(&format!("  out={}\n", filename));
    }

    content
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::{FileFormat, PdbId};
    use crate::mirrors::MirrorId;
    use std::time::Duration;

    fn test_options() -> DownloadOptions {
        DownloadOptions {
            mirror: MirrorId::Rcsb,
            decompress: false,
            overwrite: false,
            parallel: 4,
            retry_count: 3,
            retry_delay: Duration::from_secs(1),
        }
    }

    #[test]
    fn test_is_available_returns_bool() {
        // This just tests that the function runs without panicking
        let _ = is_available();
    }

    #[test]
    fn test_generate_input_file_single_task() {
        let options = test_options();
        let url_builder = HttpsDownloader::new(options.clone());
        let downloader = Aria2cDownloader {
            aria2c_path: PathBuf::from("/usr/bin/aria2c"),
            connections: 4,
            split: 1,
            max_concurrent: 4,
            options,
        };

        let pdb_id = PdbId::new("4hhb").unwrap();
        let task = DownloadTask::structure(pdb_id, FileFormat::Mmcif);
        let tasks = vec![task];

        let content =
            downloader.generate_input_file(&tasks, Path::new("/tmp/downloads"), &url_builder);

        assert!(content.contains("https://files.rcsb.org/download/4hhb.cif"));
        assert!(content.contains("dir=/tmp/downloads"));
        assert!(content.contains("out=4hhb.cif"));
    }

    #[test]
    fn test_generate_input_file_multiple_tasks() {
        let options = test_options();
        let url_builder = HttpsDownloader::new(options.clone());
        let downloader = Aria2cDownloader {
            aria2c_path: PathBuf::from("/usr/bin/aria2c"),
            connections: 4,
            split: 1,
            max_concurrent: 4,
            options,
        };

        let pdb_ids = vec![
            PdbId::new("4hhb").unwrap(),
            PdbId::new("1abc").unwrap(),
            PdbId::new("2xyz").unwrap(),
        ];

        let tasks: Vec<_> = pdb_ids
            .into_iter()
            .map(|id| DownloadTask::structure(id, FileFormat::Mmcif))
            .collect();

        let content = downloader.generate_input_file(&tasks, Path::new("/data/pdb"), &url_builder);

        // Verify all URLs are present
        assert!(content.contains("https://files.rcsb.org/download/4hhb.cif"));
        assert!(content.contains("https://files.rcsb.org/download/1abc.cif"));
        assert!(content.contains("https://files.rcsb.org/download/2xyz.cif"));

        // Verify output filenames
        assert!(content.contains("out=4hhb.cif"));
        assert!(content.contains("out=1abc.cif"));
        assert!(content.contains("out=2xyz.cif"));

        // All should go to same dir
        let dir_count = content.matches("dir=/data/pdb").count();
        assert_eq!(dir_count, 3);
    }

    #[test]
    fn test_generate_input_file_with_spaces_in_path() {
        let options = test_options();
        let url_builder = HttpsDownloader::new(options.clone());
        let downloader = Aria2cDownloader {
            aria2c_path: PathBuf::from("/usr/bin/aria2c"),
            connections: 4,
            split: 1,
            max_concurrent: 4,
            options,
        };

        let task = DownloadTask::structure(PdbId::new("4hhb").unwrap(), FileFormat::Mmcif);
        let tasks = vec![task];

        let content = downloader.generate_input_file(
            &tasks,
            Path::new("/home/user/My Documents/pdb files"),
            &url_builder,
        );

        // Path should be preserved (aria2c handles spaces in dir= option)
        assert!(content.contains("dir=/home/user/My Documents/pdb files"));
    }

    #[test]
    fn test_generate_input_file_assemblies() {
        let options = test_options();
        let url_builder = HttpsDownloader::new(options.clone());
        let downloader = Aria2cDownloader {
            aria2c_path: PathBuf::from("/usr/bin/aria2c"),
            connections: 4,
            split: 1,
            max_concurrent: 4,
            options,
        };

        let pdb_id = PdbId::new("4hhb").unwrap();
        let task = DownloadTask::assembly(pdb_id, 1);
        let tasks = vec![task];

        let content = downloader.generate_input_file(&tasks, Path::new("/tmp"), &url_builder);

        assert!(content.contains("https://files.rcsb.org/download/4hhb-assembly1.cif.gz"));
        assert!(content.contains("out=4hhb-assembly1.cif.gz"));
    }

    #[test]
    fn test_generate_input_file_structure_factors() {
        let options = test_options();
        let url_builder = HttpsDownloader::new(options.clone());
        let downloader = Aria2cDownloader {
            aria2c_path: PathBuf::from("/usr/bin/aria2c"),
            connections: 4,
            split: 1,
            max_concurrent: 4,
            options,
        };

        let pdb_id = PdbId::new("1abc").unwrap();
        let task = DownloadTask::structure_factors(pdb_id);
        let tasks = vec![task];

        let content = downloader.generate_input_file(&tasks, Path::new("/tmp"), &url_builder);

        assert!(content.contains("https://files.rcsb.org/download/r1abcsf.ent.gz"));
        assert!(content.contains("out=r1abcsf.ent.gz"));
    }

    #[test]
    fn test_generate_export_input() {
        let options = test_options();
        let pdb_id = PdbId::new("4hhb").unwrap();
        let task = DownloadTask::structure(pdb_id, FileFormat::Mmcif);
        let tasks = vec![task];

        let content = generate_export_input(&tasks, Path::new("/tmp/out"), &options);

        assert!(content.contains("https://files.rcsb.org/download/4hhb.cif"));
        assert!(content.contains("dir=/tmp/out"));
        assert!(content.contains("out=4hhb.cif"));
    }

    #[test]
    fn test_aria2c_downloader_new_with_valid_options() {
        // If aria2c is installed, this should return Some
        // If not, it returns None - both are valid outcomes
        let options = test_options();
        let result = Aria2cDownloader::new(options, 4, 1);

        if is_available() {
            assert!(result.is_some());
            let downloader = result.unwrap();
            assert_eq!(downloader.connections, 4);
            assert_eq!(downloader.split, 1);
        } else {
            assert!(result.is_none());
        }
    }

    #[test]
    fn test_different_mirrors() {
        let mirrors = [
            MirrorId::Rcsb,
            MirrorId::Wwpdb,
            MirrorId::Pdbe,
            MirrorId::Pdbj,
        ];

        for mirror in mirrors {
            let options = DownloadOptions {
                mirror,
                decompress: false,
                overwrite: false,
                parallel: 4,
                retry_count: 3,
                retry_delay: Duration::from_secs(1),
            };
            let url_builder = HttpsDownloader::new(options.clone());
            let downloader = Aria2cDownloader {
                aria2c_path: PathBuf::from("/usr/bin/aria2c"),
                connections: 4,
                split: 1,
                max_concurrent: 4,
                options,
            };

            let pdb_id = PdbId::new("4hhb").unwrap();
            let task = DownloadTask::structure(pdb_id, FileFormat::Mmcif);
            let tasks = vec![task];

            let content = downloader.generate_input_file(&tasks, Path::new("/tmp"), &url_builder);

            // Should produce valid URL for each mirror
            assert!(content.contains("4hhb"));
            assert!(content.contains("dir=/tmp"));
        }
    }
}
