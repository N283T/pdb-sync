//! Sync command handler.

use crate::cli::args::SyncArgs;
use crate::context::AppContext;
use crate::data_types::DataType;
use crate::error::Result;
use crate::sync::{RsyncOptions, RsyncRunner, SyncResult};

pub async fn run_sync(args: SyncArgs, ctx: AppContext) -> Result<()> {
    let dest = args.dest.unwrap_or_else(|| ctx.pdb_dir.clone());
    let mirror = args.mirror.unwrap_or(ctx.mirror);

    // Default to Structures if no data types specified
    let data_types = if args.data_types.is_empty() {
        vec![DataType::Structures]
    } else {
        args.data_types.clone()
    };

    let options = RsyncOptions {
        mirror,
        data_types: data_types.clone(),
        formats: args.format.to_file_formats(),
        layout: args.layout,
        delete: args.delete,
        bwlimit: args.bwlimit,
        dry_run: args.dry_run,
        filters: args.filters.clone(),
        show_progress: args.progress,
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
    println!("Layout: {}", args.layout);
    println!("Destination: {}", dest.display());

    if args.dry_run {
        println!("\nDry run - would execute:");
        for arg in runner.build_command_string(&dest) {
            print!("{} ", arg);
        }
        println!();
    }

    let results = runner.run(&dest).await?;

    // Print summary
    print_summary(&results);

    Ok(())
}

fn print_summary(results: &[SyncResult]) {
    println!("\n--- Sync Summary ---");
    for result in results {
        let format_str = result
            .format
            .map(|f| format!(" ({}/{})", result.layout, f.subdir()))
            .unwrap_or_else(|| format!(" ({})", result.layout));
        let status = if result.success { "OK" } else { "FAILED" };

        if result.files_count > 0 || result.bytes_transferred > 0 {
            println!(
                "  {}{}: {} - {} files, {}",
                result.data_type,
                format_str,
                status,
                result.files_count,
                human_bytes(result.bytes_transferred)
            );
        } else {
            println!("  {}{}: {}", result.data_type, format_str, status);
        }
    }

    let total_files: u64 = results.iter().map(|r| r.files_count).sum();
    let total_bytes: u64 = results.iter().map(|r| r.bytes_transferred).sum();
    let all_success = results.iter().all(|r| r.success);

    if total_files > 0 || total_bytes > 0 {
        println!("---");
        println!(
            "Total: {} files, {} - {}",
            total_files,
            human_bytes(total_bytes),
            if all_success { "All OK" } else { "Some failed" }
        );
    }
}

fn human_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
