//! List command - displays local PDB files with filtering and statistics.

use crate::cli::args::{ListArgs, OutputFormat, SortField};
use crate::context::AppContext;
use crate::error::{PdbCliError, Result};
use crate::files::FileFormat;
use chrono::{DateTime, Local};
use glob::Pattern;
use serde::Serialize;
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Represents a local PDB file with metadata
#[derive(Debug, Clone, Serialize)]
pub struct LocalFile {
    pub pdb_id: String,
    pub path: PathBuf,
    pub size: u64,
    pub modified: Option<DateTime<Local>>,
    pub format: String,
}

/// Statistics about local files
#[derive(Debug, Default, Serialize)]
struct Statistics {
    total_files: usize,
    total_size: u64,
    by_format: std::collections::BTreeMap<String, FormatStats>,
}

#[derive(Debug, Default, Clone, Serialize)]
struct FormatStats {
    count: usize,
    size: u64,
}

/// Main entry point for the list command
pub async fn run_list(args: ListArgs, ctx: AppContext) -> Result<()> {
    let mirror_dir = &ctx.pdb_dir;

    if !mirror_dir.exists() {
        return Err(PdbCliError::Path(format!(
            "Mirror directory does not exist: {}",
            mirror_dir.display()
        )));
    }

    // Compile the pattern if provided
    let pattern = args
        .pattern
        .as_ref()
        .map(|p| {
            // Convert simple glob to Pattern
            let pattern_str = if p.contains('*') || p.contains('?') {
                p.to_lowercase()
            } else {
                format!("{}*", p.to_lowercase())
            };
            Pattern::new(&pattern_str)
        })
        .transpose()
        .map_err(|e| PdbCliError::InvalidInput(format!("Invalid pattern: {}", e)))?;

    // Scan for files
    let mut files = scan_local_files(mirror_dir, pattern.as_ref(), args.format).await?;

    // Sort files
    sort_files(&mut files, args.sort, args.reverse);

    // Output
    if args.stats {
        let stats = compute_statistics(&files);
        match args.output {
            OutputFormat::Text => print_statistics_text(&stats),
            OutputFormat::Json => print_statistics_json(&stats)?,
            OutputFormat::Csv => print_statistics_csv(&stats),
        }
    } else {
        match args.output {
            OutputFormat::Text => print_text(&files, args.size, args.time),
            OutputFormat::Json => print_json(&files)?,
            OutputFormat::Csv => print_csv(&files, args.size, args.time),
        }
    }

    Ok(())
}

/// Scan local files in the mirror directory
async fn scan_local_files(
    mirror_dir: &Path,
    pattern: Option<&Pattern>,
    format_filter: Option<FileFormat>,
) -> Result<Vec<LocalFile>> {
    let mut files = Vec::new();

    // Scan each format directory
    let format_dirs = [
        ("mmCIF", "cif.gz", "mmcif"),
        ("pdb", "ent.gz", "pdb"),
        ("bcif", "bcif.gz", "bcif"),
    ];

    for (dir_name, extension, format_name) in format_dirs {
        // Apply format filter
        if let Some(filter) = format_filter {
            let matches = match filter {
                FileFormat::Mmcif | FileFormat::CifGz => dir_name == "mmCIF",
                FileFormat::Pdb | FileFormat::PdbGz => dir_name == "pdb",
                FileFormat::Bcif | FileFormat::BcifGz => dir_name == "bcif",
            };
            if !matches {
                continue;
            }
        }

        let format_dir = mirror_dir.join(dir_name);
        if !format_dir.exists() {
            continue;
        }

        // Scan hash directories (divided layout)
        let mut hash_entries = fs::read_dir(&format_dir).await?;
        while let Some(hash_entry) = hash_entries.next_entry().await? {
            let hash_path = hash_entry.path();
            if !hash_path.is_dir() {
                continue;
            }

            // Scan files in the hash directory
            let mut file_entries = fs::read_dir(&hash_path).await?;
            while let Some(file_entry) = file_entries.next_entry().await? {
                let file_path = file_entry.path();
                let file_name = match file_path.file_name().and_then(|n| n.to_str()) {
                    Some(n) => n,
                    None => continue,
                };

                // Check extension
                if !file_name.ends_with(extension) {
                    continue;
                }

                // Extract PDB ID
                let pdb_id = match extract_pdb_id(file_name, format_name) {
                    Some(id) => id,
                    None => continue,
                };

                // Apply pattern filter
                if let Some(pat) = pattern {
                    if !pat.matches(&pdb_id.to_lowercase()) {
                        continue;
                    }
                }

                // Get metadata
                let metadata = file_entry.metadata().await?;
                let modified = metadata.modified().ok().map(|t| {
                    let datetime: DateTime<Local> = t.into();
                    datetime
                });

                files.push(LocalFile {
                    pdb_id,
                    path: file_path,
                    size: metadata.len(),
                    modified,
                    format: format_name.to_string(),
                });
            }
        }
    }

    Ok(files)
}

/// Extract PDB ID from filename based on format
fn extract_pdb_id(filename: &str, format: &str) -> Option<String> {
    match format {
        "mmcif" => {
            // Format: {pdb_id}.cif.gz
            filename.strip_suffix(".cif.gz").map(|s| s.to_string())
        }
        "pdb" => {
            // Format: pdb{pdb_id}.ent.gz
            filename
                .strip_prefix("pdb")
                .and_then(|s| s.strip_suffix(".ent.gz"))
                .map(|s| s.to_string())
        }
        "bcif" => {
            // Format: {pdb_id}.bcif.gz
            filename.strip_suffix(".bcif.gz").map(|s| s.to_string())
        }
        _ => None,
    }
}

/// Sort files by the specified field
fn sort_files(files: &mut [LocalFile], sort_field: SortField, reverse: bool) {
    files.sort_by(|a, b| {
        let cmp = match sort_field {
            SortField::Name => a.pdb_id.cmp(&b.pdb_id),
            SortField::Size => a.size.cmp(&b.size),
            SortField::Time => match (&a.modified, &b.modified) {
                (Some(a_time), Some(b_time)) => a_time.cmp(b_time),
                (Some(_), None) => Ordering::Greater,
                (None, Some(_)) => Ordering::Less,
                (None, None) => Ordering::Equal,
            },
        };
        if reverse {
            cmp.reverse()
        } else {
            cmp
        }
    });
}

/// Compute statistics from the file list
fn compute_statistics(files: &[LocalFile]) -> Statistics {
    let mut stats = Statistics {
        total_files: files.len(),
        total_size: 0,
        by_format: std::collections::BTreeMap::new(),
    };

    for file in files {
        stats.total_size += file.size;
        let entry = stats.by_format.entry(file.format.clone()).or_default();
        entry.count += 1;
        entry.size += file.size;
    }

    stats
}

/// Print files in text format
fn print_text(files: &[LocalFile], show_size: bool, show_time: bool) {
    for file in files {
        let mut parts = vec![file.pdb_id.clone()];

        if show_size {
            parts.push(human_bytes(file.size));
        }

        if show_time {
            if let Some(modified) = &file.modified {
                parts.push(modified.format("%Y-%m-%d %H:%M").to_string());
            } else {
                parts.push("-".to_string());
            }
        }

        parts.push(file.format.clone());

        println!("{}", parts.join("\t"));
    }

    println!("\nTotal: {} files", files.len());
}

/// Print files in JSON format
fn print_json(files: &[LocalFile]) -> Result<()> {
    let json = serde_json::to_string_pretty(files)
        .map_err(|e| PdbCliError::InvalidInput(format!("JSON serialization failed: {}", e)))?;
    println!("{}", json);
    Ok(())
}

/// Print files in CSV format
fn print_csv(files: &[LocalFile], show_size: bool, show_time: bool) {
    // Print header
    let mut headers = vec!["pdb_id"];
    if show_size {
        headers.push("size");
    }
    if show_time {
        headers.push("modified");
    }
    headers.push("format");
    headers.push("path");
    println!("{}", headers.join(","));

    // Print rows
    for file in files {
        let mut parts = vec![escape_csv_field(&file.pdb_id)];

        if show_size {
            parts.push(file.size.to_string());
        }

        if show_time {
            if let Some(modified) = &file.modified {
                parts.push(escape_csv_field(&modified.to_rfc3339()));
            } else {
                parts.push(String::new());
            }
        }

        parts.push(escape_csv_field(&file.format));
        parts.push(escape_csv_field(&file.path.display().to_string()));

        println!("{}", parts.join(","));
    }
}

/// Escape a CSV field to prevent injection and handle special characters
fn escape_csv_field(s: &str) -> String {
    // Check if escaping is needed
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        // Escape quotes by doubling them and wrap in quotes
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Print statistics in text format
fn print_statistics_text(stats: &Statistics) {
    println!("Local PDB Mirror Statistics");
    println!("===========================");
    println!("Total files: {}", stats.total_files);
    println!("Total size:  {}", human_bytes(stats.total_size));
    println!();
    println!("By format:");
    for (format, format_stats) in &stats.by_format {
        println!(
            "  {}: {} files ({})",
            format,
            format_stats.count,
            human_bytes(format_stats.size)
        );
    }
}

/// Print statistics in JSON format
fn print_statistics_json(stats: &Statistics) -> Result<()> {
    let json = serde_json::to_string_pretty(stats)
        .map_err(|e| PdbCliError::InvalidInput(format!("JSON serialization failed: {}", e)))?;
    println!("{}", json);
    Ok(())
}

/// Print statistics in CSV format
fn print_statistics_csv(stats: &Statistics) {
    println!("metric,value");
    println!("total_files,{}", stats.total_files);
    println!("total_size,{}", stats.total_size);
    for (format, format_stats) in &stats.by_format {
        println!("{}_count,{}", format, format_stats.count);
        println!("{}_size,{}", format, format_stats.size);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_pdb_id_cifgz() {
        assert_eq!(
            extract_pdb_id("1abc.cif.gz", "mmcif"),
            Some("1abc".to_string())
        );
        assert_eq!(
            extract_pdb_id("2xyz.cif.gz", "mmcif"),
            Some("2xyz".to_string())
        );
        assert_eq!(extract_pdb_id("invalid.txt", "mmcif"), None);
    }

    #[test]
    fn test_extract_pdb_id_pdbgz() {
        assert_eq!(
            extract_pdb_id("pdb1abc.ent.gz", "pdb"),
            Some("1abc".to_string())
        );
        assert_eq!(
            extract_pdb_id("pdb2xyz.ent.gz", "pdb"),
            Some("2xyz".to_string())
        );
        assert_eq!(extract_pdb_id("1abc.ent.gz", "pdb"), None);
    }

    #[test]
    fn test_extract_pdb_id_bcifgz() {
        assert_eq!(
            extract_pdb_id("1abc.bcif.gz", "bcif"),
            Some("1abc".to_string())
        );
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
    fn test_sort_files_by_name() {
        let mut files = vec![
            LocalFile {
                pdb_id: "2abc".to_string(),
                path: PathBuf::from("/tmp/2abc.cif.gz"),
                size: 100,
                modified: None,
                format: "mmcif".to_string(),
            },
            LocalFile {
                pdb_id: "1abc".to_string(),
                path: PathBuf::from("/tmp/1abc.cif.gz"),
                size: 200,
                modified: None,
                format: "mmcif".to_string(),
            },
        ];

        sort_files(&mut files, SortField::Name, false);
        assert_eq!(files[0].pdb_id, "1abc");
        assert_eq!(files[1].pdb_id, "2abc");

        sort_files(&mut files, SortField::Name, true);
        assert_eq!(files[0].pdb_id, "2abc");
        assert_eq!(files[1].pdb_id, "1abc");
    }

    #[test]
    fn test_sort_files_by_size() {
        let mut files = vec![
            LocalFile {
                pdb_id: "1abc".to_string(),
                path: PathBuf::from("/tmp/1abc.cif.gz"),
                size: 200,
                modified: None,
                format: "mmcif".to_string(),
            },
            LocalFile {
                pdb_id: "2abc".to_string(),
                path: PathBuf::from("/tmp/2abc.cif.gz"),
                size: 100,
                modified: None,
                format: "mmcif".to_string(),
            },
        ];

        sort_files(&mut files, SortField::Size, false);
        assert_eq!(files[0].size, 100);
        assert_eq!(files[1].size, 200);

        sort_files(&mut files, SortField::Size, true);
        assert_eq!(files[0].size, 200);
        assert_eq!(files[1].size, 100);
    }

    #[test]
    fn test_compute_statistics() {
        let files = vec![
            LocalFile {
                pdb_id: "1abc".to_string(),
                path: PathBuf::from("/tmp/1abc.cif.gz"),
                size: 100,
                modified: None,
                format: "mmcif".to_string(),
            },
            LocalFile {
                pdb_id: "2abc".to_string(),
                path: PathBuf::from("/tmp/2abc.cif.gz"),
                size: 200,
                modified: None,
                format: "mmcif".to_string(),
            },
            LocalFile {
                pdb_id: "3abc".to_string(),
                path: PathBuf::from("/tmp/3abc.ent.gz"),
                size: 150,
                modified: None,
                format: "pdb".to_string(),
            },
        ];

        let stats = compute_statistics(&files);
        assert_eq!(stats.total_files, 3);
        assert_eq!(stats.total_size, 450);
        assert_eq!(stats.by_format.get("mmcif").unwrap().count, 2);
        assert_eq!(stats.by_format.get("mmcif").unwrap().size, 300);
        assert_eq!(stats.by_format.get("pdb").unwrap().count, 1);
        assert_eq!(stats.by_format.get("pdb").unwrap().size, 150);
    }

    #[test]
    fn test_escape_csv_field() {
        assert_eq!(escape_csv_field("simple"), "simple");
        assert_eq!(escape_csv_field("with,comma"), "\"with,comma\"");
        assert_eq!(escape_csv_field("with\"quote"), "\"with\"\"quote\"");
        assert_eq!(escape_csv_field("with\nnewline"), "\"with\nnewline\"");
        assert_eq!(
            escape_csv_field("/path/to,file\"name"),
            "\"/path/to,file\"\"name\""
        );
    }
}
