//! Stats command implementation.

use crate::api::RcsbClient;
use crate::cli::{OutputFormat, StatsArgs};
use crate::context::AppContext;
use crate::error::Result;
use crate::history::HistoryTracker;
use crate::stats::{Comparison, LocalStats, LocalStatsCollector, RemoteStatsProvider};
use crate::utils::{escape_csv_field, format_count, format_with_commas, human_bytes};

/// Run the stats command.
pub async fn run_stats(args: StatsArgs, ctx: AppContext) -> Result<()> {
    // Collect local statistics
    let collector = LocalStatsCollector::new(&ctx.pdb_dir);
    let mut stats = collector
        .collect(args.detailed, args.format, args.data_type)
        .await?;

    // Load history for timestamps
    if let Ok(history) = HistoryTracker::load() {
        stats.last_sync = history.last_sync();
        stats.last_download = history.last_download();
    }

    // Optionally compare with remote
    let comparison = if args.compare_remote {
        let client = RcsbClient::new();
        match RemoteStatsProvider::fetch_stats(&client).await {
            Ok(remote) => Some(RemoteStatsProvider::compare(stats.unique_pdb_ids, &remote)),
            Err(e) => {
                eprintln!("Warning: Failed to fetch remote stats: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Output based on format
    match args.output {
        OutputFormat::Text => print_text(&stats, comparison.as_ref(), args.detailed),
        OutputFormat::Json => print_json(&stats, comparison.as_ref())?,
        OutputFormat::Csv => print_csv(&stats, comparison.as_ref()),
    }

    Ok(())
}

fn print_text(stats: &LocalStats, comparison: Option<&Comparison>, detailed: bool) {
    println!("Local PDB Collection Statistics");
    println!("===============================");
    println!();
    println!("Total entries: {}", format_count(stats.total_entries));
    println!("Unique PDB IDs: {}", format_count(stats.unique_pdb_ids));
    println!("Total size: {}", human_bytes(stats.total_size));
    println!();

    // By format
    if !stats.by_format.is_empty() {
        println!("By format:");
        for (format, format_stats) in &stats.by_format {
            println!(
                "  {}: {} files ({})",
                format,
                format_count(format_stats.count),
                human_bytes(format_stats.size)
            );
        }
        println!();
    }

    // By data type
    if !stats.by_data_type.is_empty() {
        println!("By data type:");
        for (data_type, type_stats) in &stats.by_data_type {
            println!(
                "  {}: {} files ({} unique IDs)",
                data_type,
                format_count(type_stats.count),
                format_count(type_stats.unique_pdb_ids)
            );
        }
        println!();
    }

    // Timestamps
    if let Some(ts) = stats.last_sync {
        println!("Last sync: {}", ts.format("%Y-%m-%d %H:%M:%S UTC"));
    }
    if let Some(ts) = stats.last_download {
        println!("Last download: {}", ts.format("%Y-%m-%d %H:%M:%S UTC"));
    }

    // Comparison with remote
    if let Some(cmp) = comparison {
        println!();
        println!(
            "Coverage: {:.1}% of PDB archive ({} / {})",
            cmp.coverage_percent,
            format_with_commas(cmp.local_count as u64),
            format_with_commas(cmp.remote_count as u64)
        );
    }

    // Detailed statistics
    if detailed {
        if let Some(ref detail) = stats.detailed {
            println!();
            println!("Size Distribution:");
            println!(
                "  < 1 KB:        {} files",
                format_count(detail.size_distribution.under_1kb)
            );
            println!(
                "  1-10 KB:       {} files",
                format_count(detail.size_distribution.kb_1_10)
            );
            println!(
                "  10-100 KB:     {} files",
                format_count(detail.size_distribution.kb_10_100)
            );
            println!(
                "  100 KB - 1 MB: {} files",
                format_count(detail.size_distribution.kb_100_1mb)
            );
            println!(
                "  1-10 MB:       {} files",
                format_count(detail.size_distribution.mb_1_10)
            );
            println!(
                "  > 10 MB:       {} files",
                format_count(detail.size_distribution.over_10mb)
            );

            println!();
            if let Some(ref smallest) = detail.smallest_file {
                println!(
                    "Smallest: {} ({})",
                    human_bytes(smallest.size),
                    smallest.pdb_id
                );
            }
            if let Some(ref largest) = detail.largest_file {
                println!(
                    "Largest:  {} ({})",
                    human_bytes(largest.size),
                    largest.pdb_id
                );
            }
            println!("Average:  {}", human_bytes(detail.avg_size as u64));

            println!();
            if let Some(ref oldest) = detail.oldest_file {
                println!(
                    "Oldest: {} ({})",
                    oldest.pdb_id,
                    oldest.modified.format("%Y-%m-%d %H:%M")
                );
            }
            if let Some(ref newest) = detail.newest_file {
                println!(
                    "Newest: {} ({})",
                    newest.pdb_id,
                    newest.modified.format("%Y-%m-%d %H:%M")
                );
            }
        }
    }
}

fn print_json(stats: &LocalStats, comparison: Option<&Comparison>) -> Result<()> {
    let mut output = serde_json::json!({
        "total_entries": stats.total_entries,
        "unique_pdb_ids": stats.unique_pdb_ids,
        "total_size": stats.total_size,
        "total_size_human": human_bytes(stats.total_size),
        "by_format": stats.by_format,
        "by_data_type": stats.by_data_type,
        "last_sync": stats.last_sync.map(|t| t.to_rfc3339()),
        "last_download": stats.last_download.map(|t| t.to_rfc3339()),
    });

    if let Some(ref detail) = stats.detailed {
        if let Some(obj) = output.as_object_mut() {
            obj.insert("detailed".to_string(), serde_json::to_value(detail)?);
        }
    }

    if let Some(cmp) = comparison {
        if let Some(obj) = output.as_object_mut() {
            obj.insert(
                "comparison".to_string(),
                serde_json::json!({
                    "local_count": cmp.local_count,
                    "remote_count": cmp.remote_count,
                    "coverage_percent": cmp.coverage_percent,
                }),
            );
        }
    }

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn print_csv(stats: &LocalStats, comparison: Option<&Comparison>) {
    // Header
    println!("metric,value,unit");

    // Basic stats
    println!("total_entries,{},count", stats.total_entries);
    println!("unique_pdb_ids,{},count", stats.unique_pdb_ids);
    println!("total_size,{},bytes", stats.total_size);

    // By format
    for (format, format_stats) in &stats.by_format {
        println!(
            "{}_count,{},count",
            escape_csv_field(format),
            format_stats.count
        );
        println!(
            "{}_size,{},bytes",
            escape_csv_field(format),
            format_stats.size
        );
    }

    // By data type
    for (data_type, type_stats) in &stats.by_data_type {
        println!(
            "{}_count,{},count",
            escape_csv_field(data_type),
            type_stats.count
        );
        println!(
            "{}_unique_ids,{},count",
            escape_csv_field(data_type),
            type_stats.unique_pdb_ids
        );
    }

    // Comparison
    if let Some(cmp) = comparison {
        println!("remote_count,{},count", cmp.remote_count);
        println!("coverage_percent,{:.2},percent", cmp.coverage_percent);
    }

    // Timestamps
    if let Some(ts) = stats.last_sync {
        println!("last_sync,{},timestamp", ts.to_rfc3339());
    }
    if let Some(ts) = stats.last_download {
        println!("last_download,{},timestamp", ts.to_rfc3339());
    }
}
