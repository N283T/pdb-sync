//! Shared utilities for sync command handlers.

use crate::sync::SyncResult;

/// Print a summary of sync results.
pub fn print_summary(results: &[SyncResult]) {
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

/// Print a simple summary for mirror-specific syncs (without file format info).
pub fn print_mirror_summary(data_type: &str, success: bool, files: u64, bytes: u64) {
    let status = if success { "OK" } else { "FAILED" };
    if files > 0 || bytes > 0 {
        println!(
            "  {}: {} - {} files, {}",
            data_type,
            status,
            files,
            human_bytes(bytes)
        );
    } else {
        println!("  {}: {}", data_type, status);
    }
}

/// Convert bytes to human-readable format.
pub fn human_bytes(bytes: u64) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_human_bytes_bytes() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(500), "500 B");
        assert_eq!(human_bytes(1023), "1023 B");
    }

    #[test]
    fn test_human_bytes_kilobytes() {
        assert_eq!(human_bytes(1024), "1.00 KB");
        assert_eq!(human_bytes(1536), "1.50 KB");
        assert_eq!(human_bytes(1024 * 1023), "1023.00 KB");
    }

    #[test]
    fn test_human_bytes_megabytes() {
        assert_eq!(human_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(human_bytes(1024 * 1024 * 100), "100.00 MB");
    }

    #[test]
    fn test_human_bytes_gigabytes() {
        assert_eq!(human_bytes(1024 * 1024 * 1024), "1.00 GB");
        assert_eq!(human_bytes(1024 * 1024 * 1024 * 10), "10.00 GB");
    }
}
