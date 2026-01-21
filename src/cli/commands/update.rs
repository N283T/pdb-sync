//! Update command implementation.

use crate::cli::args::{OutputFormat, UpdateArgs};
use crate::context::AppContext;
use crate::error::{PdbSyncError, Result};
use crate::files::{paths::build_relative_path, FileFormat, PdbId};
use crate::update::{download_updates, UpdateChecker, UpdateResult, UpdateStatus};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Statistics for update run.
#[derive(Debug, Default)]
struct UpdateStats {
    up_to_date: usize,
    outdated: usize,
    missing: usize,
    unknown: usize,
    updated: usize,
    update_failed: usize,
}

impl UpdateStats {
    fn total(&self) -> usize {
        self.up_to_date + self.outdated + self.missing + self.unknown
    }

    fn from_results(results: &[UpdateResult]) -> Self {
        let mut stats = Self::default();
        for result in results {
            match &result.status {
                UpdateStatus::UpToDate => stats.up_to_date += 1,
                UpdateStatus::Outdated { .. } => stats.outdated += 1,
                UpdateStatus::Missing => stats.missing += 1,
                UpdateStatus::Unknown { .. } => stats.unknown += 1,
                UpdateStatus::Updated => stats.updated += 1,
                UpdateStatus::UpdateFailed { .. } => stats.update_failed += 1,
            }
        }
        stats
    }
}

pub async fn run_update(args: UpdateArgs, ctx: AppContext) -> Result<()> {
    let mirror = args.mirror.unwrap_or(ctx.mirror);
    let format = args.format.unwrap_or(FileFormat::CifGz);

    // 1. Collect files to check
    let files = if args.pdb_ids.is_empty() {
        scan_local_files(&ctx.pdb_dir, format).await?
    } else {
        // Build paths for specific PDB IDs
        let mut files = Vec::new();
        for id_str in &args.pdb_ids {
            match PdbId::new(id_str) {
                Ok(pdb_id) => {
                    let path = ctx.pdb_dir.join(build_relative_path(&pdb_id, format));
                    if path.exists() {
                        files.push((pdb_id, path));
                    } else {
                        eprintln!(
                            "Warning: Local file not found for '{}': {}",
                            id_str,
                            path.display()
                        );
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Invalid PDB ID '{}': {}", id_str, e);
                }
            }
        }
        files
    };

    if files.is_empty() {
        println!("No local files found. Use 'sync' or 'download' first.");
        return Ok(());
    }

    let total_files = files.len();

    // Handle JSON output differently
    if args.output == OutputFormat::Json {
        return run_update_json(args, files, mirror, format, &ctx.pdb_dir).await;
    }

    // 2. Check all files in parallel
    println!(
        "Checking {} file(s) against {} mirror...\n",
        total_files, mirror
    );

    let pb = if args.progress {
        let pb = ProgressBar::new(total_files as u64);
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

    let checker = UpdateChecker::new(args.parallel);
    let results = checker
        .check_many(files, mirror, format, args.verify, pb.as_ref())
        .await;

    if let Some(pb) = pb {
        pb.finish_and_clear();
    }

    // 3. Report results
    for result in &results {
        let status_str = match &result.status {
            UpdateStatus::UpToDate => {
                format!("✓ {} (up-to-date)", result.pdb_id)
            }
            UpdateStatus::Outdated {
                local_time,
                remote_time,
            } => {
                format!(
                    "↓ {} (outdated: local {} < remote {})",
                    result.pdb_id,
                    local_time.format("%Y-%m-%d %H:%M"),
                    remote_time.format("%Y-%m-%d %H:%M")
                )
            }
            UpdateStatus::Missing => {
                format!("? {} (not found on remote)", result.pdb_id)
            }
            UpdateStatus::Unknown { reason } => {
                format!("! {} (unknown: {})", result.pdb_id, reason)
            }
            UpdateStatus::Updated => {
                format!("✓ {} (updated)", result.pdb_id)
            }
            UpdateStatus::UpdateFailed { error } => {
                format!("✗ {} (update failed: {})", result.pdb_id, error)
            }
        };
        println!("{}", status_str);
    }

    // 4. Collect outdated files for update
    let outdated: Vec<_> = results
        .iter()
        .filter(|r| r.status.is_outdated() || (args.force && r.status.is_up_to_date()))
        .collect();

    let stats = UpdateStats::from_results(&results);

    // 5. Handle update mode
    if !args.check && !args.dry_run && !outdated.is_empty() {
        println!("\nDownloading {} update(s)...\n", outdated.len());

        let download_results =
            download_updates(&outdated, mirror, format, &ctx.pdb_dir, args.parallel).await?;
        let download_stats = UpdateStats::from_results(&download_results);

        // Print download results
        for result in &download_results {
            let status_str = match &result.status {
                UpdateStatus::Updated => {
                    format!("✓ {} updated", result.pdb_id)
                }
                UpdateStatus::UpdateFailed { error } => {
                    format!("✗ {} failed: {}", result.pdb_id, error)
                }
                _ => continue,
            };
            println!("{}", status_str);
        }

        // Print summary with download stats
        print_summary_with_downloads(&stats, &download_stats);
    } else if args.dry_run && !outdated.is_empty() {
        println!("\n--- Dry Run Summary ---");
        println!("Would update {} file(s):", outdated.len());
        for item in &outdated {
            println!("  - {}", item.pdb_id);
        }
        println!("\nRun without --dry-run to download updates.");
    } else {
        // Print summary
        print_summary(&stats, args.check);
    }

    Ok(())
}

async fn run_update_json(
    args: UpdateArgs,
    files: Vec<(PdbId, PathBuf)>,
    mirror: crate::mirrors::MirrorId,
    format: FileFormat,
    pdb_dir: &Path,
) -> Result<()> {
    let checker = UpdateChecker::new(args.parallel);
    // JSON mode doesn't need progress bar
    let results = checker
        .check_many(files, mirror, format, args.verify, None)
        .await;

    // Collect outdated files for update
    let outdated: Vec<_> = results
        .iter()
        .filter(|r| r.status.is_outdated() || (args.force && r.status.is_up_to_date()))
        .collect();

    let final_results = if !args.check && !args.dry_run && !outdated.is_empty() {
        // Download updates and merge results
        let download_results =
            download_updates(&outdated, mirror, format, pdb_dir, args.parallel).await?;

        // Create a map of download results by pdb_id
        let download_map: std::collections::HashMap<_, _> = download_results
            .into_iter()
            .map(|r| (r.pdb_id.clone(), r))
            .collect();

        // Merge: replace outdated status with download result
        results
            .into_iter()
            .map(|r| {
                if let Some(download_result) = download_map.get(&r.pdb_id) {
                    UpdateResult {
                        pdb_id: download_result.pdb_id.clone(),
                        status: download_result.status.clone(),
                        local_path: download_result.local_path.clone(),
                    }
                } else {
                    r
                }
            })
            .collect::<Vec<_>>()
    } else {
        results
    };

    // Output as JSON
    let json = serde_json::to_string_pretty(&final_results)?;
    println!("{}", json);

    Ok(())
}

fn print_summary(stats: &UpdateStats, check_only: bool) {
    println!("\n--- Update Check Summary ---");
    println!("Total files:    {}", stats.total());
    println!("Up-to-date:     {}", stats.up_to_date);
    println!("Outdated:       {}", stats.outdated);
    println!("Missing:        {}", stats.missing);
    println!("Unknown:        {}", stats.unknown);

    if check_only && stats.outdated > 0 {
        println!("\nTip: Run without --check to download updates.");
    }
}

fn print_summary_with_downloads(check_stats: &UpdateStats, download_stats: &UpdateStats) {
    println!("\n--- Update Summary ---");
    println!("Total checked:  {}", check_stats.total());
    println!("Up-to-date:     {}", check_stats.up_to_date);
    println!("Outdated:       {}", check_stats.outdated);
    println!("Missing:        {}", check_stats.missing);
    println!("Unknown:        {}", check_stats.unknown);
    println!("---");
    println!("Updated:        {}", download_stats.updated);
    if download_stats.update_failed > 0 {
        println!("Failed:         {}", download_stats.update_failed);
    }
}

/// Scan local mirror directory for PDB files of the given format.
///
/// Reuses the same logic as validate command.
async fn scan_local_files(mirror_dir: &Path, format: FileFormat) -> Result<Vec<(PdbId, PathBuf)>> {
    let mut files = Vec::new();

    // Structure: mirror_dir/mmCIF/XX/XXXX.cif.gz (or pdb/XX/pdbXXXX.ent.gz)
    let subdir = format.subdir();
    let format_dir = mirror_dir.join(subdir);

    if !format_dir.exists() {
        return Err(PdbSyncError::Path(format!(
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
        // mmCIF: 1abc.cif or 1abc.cif.gz or pdb_00001abc.cif.gz
        FileFormat::Mmcif | FileFormat::CifGz => filename
            .strip_suffix(".cif.gz")
            .or_else(|| filename.strip_suffix(".cif"))?,
        // PDB: pdb1abc.ent.gz (classic) or pdb_00001abc.ent.gz (extended)
        // Classic IDs have "pdb" prefix in filename, extended IDs don't
        FileFormat::Pdb | FileFormat::PdbGz => {
            let without_ext = filename
                .strip_suffix(".ent.gz")
                .or_else(|| filename.strip_suffix(".ent"))?;
            // Extended IDs start with "pdb_", classic filenames start with "pdb" (no underscore)
            if without_ext.starts_with("pdb_") {
                // Extended ID: pdb_00001abc -> keep as-is
                without_ext
            } else if let Some(id) = without_ext.strip_prefix("pdb") {
                // Classic ID: pdb1abc -> 1abc
                id
            } else {
                // Unknown format, try as-is
                without_ext
            }
        }
        // BinaryCIF: 1abc.bcif or 1abc.bcif.gz or pdb_00001abc.bcif.gz
        FileFormat::Bcif | FileFormat::BcifGz => filename
            .strip_suffix(".bcif.gz")
            .or_else(|| filename.strip_suffix(".bcif"))?,
    };

    PdbId::new(id_str).ok()
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
    fn test_extract_pdb_id_extended() {
        let path = PathBuf::from("/data/mmCIF/00/pdb_00001abc.cif.gz");
        let id = extract_pdb_id_from_filename(&path, FileFormat::CifGz);
        assert_eq!(id.map(|i| i.to_string()), Some("pdb_00001abc".to_string()));
    }

    #[test]
    fn test_extract_pdb_id_extended_pdb_format() {
        // Extended IDs in PDB format don't have extra "pdb" prefix
        let path = PathBuf::from("/data/pdb/00/pdb_00001abc.ent.gz");
        let id = extract_pdb_id_from_filename(&path, FileFormat::PdbGz);
        assert_eq!(id.map(|i| i.to_string()), Some("pdb_00001abc".to_string()));
    }

    #[test]
    fn test_update_stats_from_results() {
        let results = vec![
            UpdateResult {
                pdb_id: "1abc".to_string(),
                status: UpdateStatus::UpToDate,
                local_path: PathBuf::from("/tmp/1abc.cif.gz"),
            },
            UpdateResult {
                pdb_id: "2xyz".to_string(),
                status: UpdateStatus::Outdated {
                    local_time: chrono::Utc::now(),
                    remote_time: chrono::Utc::now(),
                },
                local_path: PathBuf::from("/tmp/2xyz.cif.gz"),
            },
        ];

        let stats = UpdateStats::from_results(&results);
        assert_eq!(stats.up_to_date, 1);
        assert_eq!(stats.outdated, 1);
        assert_eq!(stats.total(), 2);
    }
}
