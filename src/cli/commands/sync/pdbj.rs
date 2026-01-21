//! PDBj-specific data sync handler.

use std::path::Path;

use tokio::process::Command;

use crate::cli::args::{PdbjSyncArgs, SyncArgs};
use crate::context::AppContext;
use crate::data_types::PdbjDataType;
use crate::error::{PdbSyncError, Result};
use crate::mirrors::{Mirror, MirrorId};

use super::common::{human_bytes, parse_rsync_output, print_mirror_summary, validate_subpath};

/// Run PDBj-specific data sync.
pub async fn run(args: PdbjSyncArgs, parent_args: &SyncArgs, ctx: AppContext) -> Result<()> {
    let dest = parent_args
        .dest
        .clone()
        .unwrap_or_else(|| ctx.pdb_dir.clone());

    // Validate subpath if provided
    if let Some(ref subpath) = args.subpath {
        validate_subpath(subpath).map_err(|e| PdbSyncError::Config {
            message: e.to_string(),
            key: Some("subpath".to_string()),
            source: None,
        })?;
    }

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
        .ok_or_else(|| PdbSyncError::Config {
            message: format!("PDBj URL not available for {}", data_type),
            key: Some("pdbj_url".to_string()),
            source: None,
        })?;

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

    // Create destination directory if needed (using tokio for async)
    tokio::fs::create_dir_all(&dest_path).await?;

    // Execute rsync (using tokio::process::Command for async)
    let output = cmd.output().await?;

    let success = output.status.success();

    // Parse output for file count and bytes
    let (files, bytes) = parse_rsync_output(&output.stdout);

    Ok((success, files, bytes))
}
