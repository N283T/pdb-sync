//! wwPDB standard data sync handler.

use std::path::Path;

use crate::cli::args::{ShortcutSyncArgs, SyncArgs, SyncFormat, WwpdbSyncArgs};
use crate::context::AppContext;
use crate::data_types::{DataType, Layout};
use crate::error::Result;
use crate::mirrors::MirrorId;
use crate::sync::{RsyncOptions, RsyncRunner};

use super::common::print_summary;

/// Run wwPDB sync with explicit subcommand arguments.
pub async fn run(args: WwpdbSyncArgs, parent_args: &SyncArgs, ctx: AppContext) -> Result<()> {
    let dest = parent_args
        .dest
        .clone()
        .unwrap_or_else(|| ctx.pdb_dir.clone());
    let mirror = parent_args.mirror.unwrap_or(ctx.mirror);

    // Default to Structures if no data types specified
    let data_types = if args.data_types.is_empty() {
        vec![DataType::Structures]
    } else {
        args.data_types.clone()
    };

    run_wwpdb_sync(
        mirror,
        data_types,
        args.format,
        args.layout,
        parent_args.delete,
        parent_args.bwlimit,
        parent_args.dry_run,
        args.filters,
        parent_args.progress,
        &dest,
    )
    .await
}

/// Run structures shortcut command.
pub async fn run_structures(
    args: ShortcutSyncArgs,
    parent_args: &SyncArgs,
    ctx: AppContext,
) -> Result<()> {
    let dest = parent_args
        .dest
        .clone()
        .unwrap_or_else(|| ctx.pdb_dir.clone());
    let mirror = parent_args.mirror.unwrap_or(ctx.mirror);

    run_wwpdb_sync(
        mirror,
        vec![DataType::Structures],
        args.format,
        args.layout,
        parent_args.delete,
        parent_args.bwlimit,
        parent_args.dry_run,
        args.filters,
        parent_args.progress,
        &dest,
    )
    .await
}

/// Run assemblies shortcut command.
pub async fn run_assemblies(
    args: ShortcutSyncArgs,
    parent_args: &SyncArgs,
    ctx: AppContext,
) -> Result<()> {
    let dest = parent_args
        .dest
        .clone()
        .unwrap_or_else(|| ctx.pdb_dir.clone());
    let mirror = parent_args.mirror.unwrap_or(ctx.mirror);

    run_wwpdb_sync(
        mirror,
        vec![DataType::Assemblies],
        args.format,
        args.layout,
        parent_args.delete,
        parent_args.bwlimit,
        parent_args.dry_run,
        args.filters,
        parent_args.progress,
        &dest,
    )
    .await
}

/// Run wwPDB sync in legacy mode (backward compatibility when no subcommand is specified).
pub async fn run_legacy(args: SyncArgs, ctx: AppContext) -> Result<()> {
    let dest = args.dest.clone().unwrap_or_else(|| ctx.pdb_dir.clone());
    let mirror = args.mirror.unwrap_or(ctx.mirror);

    // Default to Structures if no data types specified
    let data_types = if args.data_types.is_empty() {
        vec![DataType::Structures]
    } else {
        args.data_types.clone()
    };

    run_wwpdb_sync(
        mirror,
        data_types,
        args.format,
        args.layout,
        args.delete,
        args.bwlimit,
        args.dry_run,
        args.filters,
        args.progress,
        &dest,
    )
    .await
}

/// Internal function to run wwPDB sync with all parameters.
#[allow(clippy::too_many_arguments)]
async fn run_wwpdb_sync(
    mirror: MirrorId,
    data_types: Vec<DataType>,
    format: SyncFormat,
    layout: Layout,
    delete: bool,
    bwlimit: Option<u32>,
    dry_run: bool,
    filters: Vec<String>,
    show_progress: bool,
    dest: &Path,
) -> Result<()> {
    let options = RsyncOptions {
        mirror,
        data_types: data_types.clone(),
        formats: format.to_file_formats(),
        layout,
        delete,
        bwlimit,
        dry_run,
        filters,
        show_progress,
    };

    let runner = RsyncRunner::new(options);

    // Print sync configuration
    println!("Syncing from {} mirror...", mirror);
    println!(
        "Data types: {}",
        data_types
            .iter()
            .map(|dt| dt.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("Layout: {}", layout);
    println!("Destination: {}", dest.display());

    if dry_run {
        println!("\nDry run - would execute:");
        for arg in runner.build_command_string(dest) {
            print!("{} ", arg);
        }
        println!();
    }

    let results = runner.run(dest).await?;

    // Print summary
    print_summary(&results);

    Ok(())
}
