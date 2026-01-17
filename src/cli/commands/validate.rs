//! Validate command implementation.

use crate::cli::args::ValidateArgs;
use crate::context::AppContext;
use crate::download::{DownloadOptions, HttpsDownloader};
use crate::error::{PdbCliError, Result};
use crate::files::{paths::build_relative_path, FileFormat, PdbId};
use crate::validation::{ChecksumVerifier, VerifyResult};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Statistics for validation run.
#[derive(Debug, Default)]
struct ValidationStats {
    valid: usize,
    invalid: usize,
    missing: usize,
    no_checksum: usize,
    fixed: usize,
    fix_failed: usize,
}

impl ValidationStats {
    fn total(&self) -> usize {
        self.valid + self.invalid + self.missing + self.no_checksum
    }
}

pub async fn run_validate(args: ValidateArgs, ctx: AppContext) -> Result<()> {
    let mirror = args.mirror.unwrap_or(ctx.mirror);
    let format = args.format.unwrap_or(FileFormat::CifGz);

    // Collect files to validate
    let files_to_validate = if args.pdb_ids.is_empty() {
        // Scan local directory for all PDB files
        scan_local_files(&ctx.pdb_dir, format).await?
    } else {
        // Validate specific PDB IDs
        let mut files = Vec::new();
        for id_str in &args.pdb_ids {
            match PdbId::new(id_str) {
                Ok(pdb_id) => {
                    let path = ctx.pdb_dir.join(build_relative_path(&pdb_id, format));
                    files.push((pdb_id, path));
                }
                Err(e) => {
                    eprintln!("Warning: Invalid PDB ID '{}': {}", id_str, e);
                }
            }
        }
        files
    };

    if files_to_validate.is_empty() {
        println!("No files to validate.");
        return Ok(());
    }

    println!(
        "Validating {} file(s) against {} checksums...\n",
        files_to_validate.len(),
        mirror
    );

    // Set up progress bar
    let pb = if args.progress {
        let pb = ProgressBar::new(files_to_validate.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );
        Some(pb)
    } else {
        None
    };

    // Create verifier and downloader (for --fix)
    let mut verifier = ChecksumVerifier::new();
    let downloader = if args.fix {
        Some(HttpsDownloader::new(DownloadOptions {
            mirror,
            decompress: false,
            overwrite: true,
            ..Default::default()
        }))
    } else {
        None
    };

    let mut stats = ValidationStats::default();

    for (pdb_id, local_path) in &files_to_validate {
        // Build the subpath for checksum lookup
        // For structures/divided/mmCIF/ab/1abc.cif.gz, subpath is "structures/divided/mmCIF/ab"
        let subpath = build_checksum_subpath(format, pdb_id);
        let filename = local_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        let result = verifier
            .verify(local_path, mirror, &subpath, &filename)
            .await?;

        match &result {
            VerifyResult::Valid => {
                stats.valid += 1;
                if !args.errors_only {
                    if let Some(ref pb) = pb {
                        pb.println(format!("✓ {}", pdb_id));
                    } else {
                        println!("✓ {}", pdb_id);
                    }
                }
            }
            VerifyResult::Invalid { expected, actual } => {
                stats.invalid += 1;

                if let Some(ref pb) = pb {
                    pb.println(format!(
                        "✗ {} (expected: {}, got: {})",
                        pdb_id, expected, actual
                    ));
                } else {
                    eprintln!("✗ {} (expected: {}, got: {})", pdb_id, expected, actual);
                }

                // Attempt to fix if requested
                if let Some(ref dl) = downloader {
                    match fix_file(dl, pdb_id, format, &ctx.pdb_dir).await {
                        Ok(()) => {
                            stats.fixed += 1;
                            if let Some(ref pb) = pb {
                                pb.println(format!("  → Fixed: {}", pdb_id));
                            } else {
                                println!("  → Fixed: {}", pdb_id);
                            }
                        }
                        Err(e) => {
                            stats.fix_failed += 1;
                            if let Some(ref pb) = pb {
                                pb.println(format!("  → Fix failed: {} ({})", pdb_id, e));
                            } else {
                                eprintln!("  → Fix failed: {} ({})", pdb_id, e);
                            }
                        }
                    }
                }
            }
            VerifyResult::Missing => {
                stats.missing += 1;
                if let Some(ref pb) = pb {
                    pb.println(format!("? {} (file missing)", pdb_id));
                } else {
                    eprintln!("? {} (file missing)", pdb_id);
                }
            }
            VerifyResult::NoChecksum => {
                stats.no_checksum += 1;
                if !args.errors_only {
                    if let Some(ref pb) = pb {
                        pb.println(format!("- {} (no checksum available)", pdb_id));
                    } else {
                        println!("- {} (no checksum available)", pdb_id);
                    }
                }
            }
        }

        if let Some(ref pb) = pb {
            pb.inc(1);
        }
    }

    if let Some(pb) = pb {
        pb.finish_and_clear();
    }

    // Print summary
    println!("\n--- Validation Summary ---");
    println!("Total files:    {}", stats.total());
    println!("Valid:          {}", stats.valid);
    println!("Invalid:        {}", stats.invalid);
    println!("Missing:        {}", stats.missing);
    println!("No checksum:    {}", stats.no_checksum);

    if args.fix {
        println!("Fixed:          {}", stats.fixed);
        if stats.fix_failed > 0 {
            println!("Fix failed:     {}", stats.fix_failed);
        }
    }

    if stats.invalid > 0 && !args.fix {
        println!("\nTip: Use --fix to re-download corrupted files.");
    }

    Ok(())
}

/// Scan local mirror directory for PDB files of the given format.
async fn scan_local_files(mirror_dir: &Path, format: FileFormat) -> Result<Vec<(PdbId, PathBuf)>> {
    let mut files = Vec::new();

    // Structure: mirror_dir/mmCIF/XX/XXXX.cif.gz (or pdb/XX/pdbXXXX.ent.gz)
    let subdir = format.subdir();
    let format_dir = mirror_dir.join(subdir);

    if !format_dir.exists() {
        return Err(PdbCliError::Path(format!(
            "Format directory not found: {}",
            format_dir.display()
        )));
    }

    // Iterate over hash directories (e.g., aa, ab, ac, ...)
    let mut hash_dirs = fs::read_dir(&format_dir).await?;
    while let Some(entry) = hash_dirs.next_entry().await? {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        // Iterate over files in hash directory
        let mut entries = fs::read_dir(&path).await?;
        while let Some(file_entry) = entries.next_entry().await? {
            let file_path = file_entry.path();
            if !file_path.is_file() {
                continue;
            }

            // Try to extract PDB ID from filename
            if let Some(pdb_id) = extract_pdb_id_from_filename(&file_path, format) {
                files.push((pdb_id, file_path));
            }
        }
    }

    files.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));
    Ok(files)
}

/// Extract PDB ID from a filename based on format.
fn extract_pdb_id_from_filename(path: &Path, format: FileFormat) -> Option<PdbId> {
    let filename = path.file_name()?.to_string_lossy();

    let id_str = match format {
        // mmCIF: 1abc.cif or 1abc.cif.gz
        FileFormat::Mmcif | FileFormat::CifGz => filename
            .strip_suffix(".cif.gz")
            .or_else(|| filename.strip_suffix(".cif"))?,
        // PDB: pdb1abc.ent or pdb1abc.ent.gz
        FileFormat::Pdb | FileFormat::PdbGz => {
            let without_ext = filename
                .strip_suffix(".ent.gz")
                .or_else(|| filename.strip_suffix(".ent"))?;
            without_ext.strip_prefix("pdb")?
        }
        // BinaryCIF: 1abc.bcif or 1abc.bcif.gz
        FileFormat::Bcif | FileFormat::BcifGz => filename
            .strip_suffix(".bcif.gz")
            .or_else(|| filename.strip_suffix(".bcif"))?,
    };

    PdbId::new(id_str).ok()
}

/// Build the checksum subpath for a given format and PDB ID.
fn build_checksum_subpath(format: FileFormat, pdb_id: &PdbId) -> String {
    let middle = pdb_id.middle_chars();
    match format {
        FileFormat::Pdb | FileFormat::PdbGz => {
            format!("structures/divided/pdb/{}", middle)
        }
        FileFormat::Mmcif | FileFormat::CifGz => {
            format!("structures/divided/mmCIF/{}", middle)
        }
        FileFormat::Bcif | FileFormat::BcifGz => {
            format!("structures/divided/bcif/{}", middle)
        }
    }
}

/// Attempt to fix a corrupted file by re-downloading it.
async fn fix_file(
    downloader: &HttpsDownloader,
    pdb_id: &PdbId,
    format: FileFormat,
    dest_dir: &Path,
) -> Result<()> {
    // Build the destination directory for this format/hash
    let relative_path = build_relative_path(pdb_id, format);
    let full_path = dest_dir.join(&relative_path);

    // Delete the corrupted file
    if full_path.exists() {
        fs::remove_file(&full_path).await?;
    }

    // Download fresh copy
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).await?;
    }

    // Download to parent directory (downloader builds its own filename)
    let dest = full_path.parent().unwrap_or(dest_dir);
    downloader.download(pdb_id, format, dest).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_pdb_id_mmcif() {
        let path = PathBuf::from("/data/mmCIF/ab/1abc.cif.gz");
        let id = extract_pdb_id_from_filename(&path, FileFormat::CifGz);
        assert_eq!(id.map(|i| i.to_string()), Some("1abc".to_string()));
    }

    #[test]
    fn test_extract_pdb_id_pdb() {
        let path = PathBuf::from("/data/pdb/ab/pdb1abc.ent.gz");
        let id = extract_pdb_id_from_filename(&path, FileFormat::PdbGz);
        assert_eq!(id.map(|i| i.to_string()), Some("1abc".to_string()));
    }

    #[test]
    fn test_extract_pdb_id_bcif() {
        let path = PathBuf::from("/data/bcif/ab/1abc.bcif.gz");
        let id = extract_pdb_id_from_filename(&path, FileFormat::BcifGz);
        assert_eq!(id.map(|i| i.to_string()), Some("1abc".to_string()));
    }

    #[test]
    fn test_build_checksum_subpath() {
        let pdb_id = PdbId::new("1abc").unwrap();
        assert_eq!(
            build_checksum_subpath(FileFormat::CifGz, &pdb_id),
            "structures/divided/mmCIF/ab"
        );
        assert_eq!(
            build_checksum_subpath(FileFormat::PdbGz, &pdb_id),
            "structures/divided/pdb/ab"
        );
    }
}
