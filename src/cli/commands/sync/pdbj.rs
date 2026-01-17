//! PDBj-specific data sync handler.

use std::path::Path;
use std::process::Command;

use crate::cli::args::{PdbjSyncArgs, SyncArgs};
use crate::context::AppContext;
use crate::data_types::PdbjDataType;
use crate::error::{PdbCliError, Result};
use crate::mirrors::{Mirror, MirrorId};

use super::common::{human_bytes, print_mirror_summary};

/// Run PDBj-specific data sync.
pub async fn run(args: PdbjSyncArgs, parent_args: &SyncArgs, ctx: AppContext) -> Result<()> {
    let dest = parent_args
        .dest
        .clone()
        .unwrap_or_else(|| ctx.pdb_dir.clone());

    // PDBj-specific data can only be synced from PDBj mirror
    let mirror = Mirror::get(MirrorId::Pdbj);

    println!("Syncing PDBj-specific data from {}...", mirror.name);
    println!(
        "Data types: {}",
        args.data_types
            .iter()
            .map(|dt| dt.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("Destination: {}", dest.display());

    if parent_args.dry_run {
        println!("\nDry run mode enabled");
    }

    println!("\n--- Sync Summary ---");
    let mut all_success = true;
    let mut total_files: u64 = 0;
    let mut total_bytes: u64 = 0;

    for data_type in &args.data_types {
        let (success, files, bytes) = sync_pdbj_data_type(
            *data_type,
            &dest,
            args.subpath.as_deref(),
            parent_args.delete,
            parent_args.bwlimit,
            parent_args.dry_run,
            parent_args.progress,
        )
        .await?;

        print_mirror_summary(&data_type.to_string(), success, files, bytes);

        all_success = all_success && success;
        total_files += files;
        total_bytes += bytes;
    }

    if total_files > 0 || total_bytes > 0 {
        println!("---");
        println!(
            "Total: {} files, {} - {}",
            total_files,
            human_bytes(total_bytes),
            if all_success { "All OK" } else { "Some failed" }
        );
    }

    Ok(())
}

/// Sync a single PDBj data type.
#[allow(clippy::too_many_arguments)]
async fn sync_pdbj_data_type(
    data_type: PdbjDataType,
    dest: &Path,
    subpath: Option<&str>,
    delete: bool,
    bwlimit: Option<u32>,
    dry_run: bool,
    show_progress: bool,
) -> Result<(bool, u64, u64)> {
    let mirror = Mirror::get(MirrorId::Pdbj);

    // Get the rsync URL for this PDBj data type
    let base_url = mirror
        .pdbj_rsync_url(data_type)
        .ok_or_else(|| PdbCliError::Config("PDBj URL not available".into()))?;

    // Append subpath if provided
    let source_url = if let Some(subpath) = subpath {
        format!("{}{}", base_url, subpath.trim_start_matches('/'))
    } else {
        base_url
    };

    // Build destination path (create subdirectory based on data type)
    let dest_path = dest.join(data_type.rsync_module());

    // Build rsync command
    let mut cmd = Command::new("rsync");
    cmd.arg("-av");

    if delete {
        cmd.arg("--delete");
    }

    if let Some(limit) = bwlimit {
        cmd.arg(format!("--bwlimit={}", limit));
    }

    if dry_run {
        cmd.arg("--dry-run");
    }

    if show_progress {
        cmd.arg("--progress");
    }

    cmd.arg(&source_url);
    cmd.arg(&dest_path);

    if dry_run {
        println!(
            "Would execute: rsync -av{}{}{} {} {}",
            if delete { " --delete" } else { "" },
            bwlimit
                .map(|l| format!(" --bwlimit={}", l))
                .unwrap_or_default(),
            if show_progress { " --progress" } else { "" },
            source_url,
            dest_path.display()
        );
        return Ok((true, 0, 0));
    }

    // Create destination directory if needed
    std::fs::create_dir_all(&dest_path)?;

    // Execute rsync
    let output = cmd.output()?;

    let success = output.status.success();

    // Parse output for file count and bytes (simplified)
    let (files, bytes) = parse_rsync_output(&output.stdout);

    Ok((success, files, bytes))
}

/// Parse rsync output to extract file count and bytes transferred.
/// This is a simplified parser that looks for common rsync output patterns.
fn parse_rsync_output(output: &[u8]) -> (u64, u64) {
    let output_str = String::from_utf8_lossy(output);

    // Look for "sent X bytes  received Y bytes" pattern
    let bytes = output_str
        .lines()
        .find(|line| line.contains("sent") && line.contains("bytes"))
        .map(|line| {
            // Parse "sent X bytes  received Y bytes" and sum
            let parts: Vec<&str> = line.split_whitespace().collect();
            let sent = parts
                .get(1)
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            let received = parts
                .get(4)
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            sent + received
        })
        .unwrap_or(0);

    // Count files transferred (lines that don't start with certain patterns)
    let files = output_str
        .lines()
        .filter(|line| {
            !line.is_empty()
                && !line.starts_with("sending")
                && !line.starts_with("receiving")
                && !line.starts_with("sent")
                && !line.starts_with("total")
                && !line.contains("speedup")
                && !line.contains("bytes/sec")
        })
        .count() as u64;

    (files, bytes)
}
