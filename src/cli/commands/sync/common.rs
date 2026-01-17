//! Shared utilities for sync command handlers.

use crate::sync::SyncResult;

// Re-export human_bytes from utils for use by sync subcommands
pub use crate::utils::human_bytes;

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

/// Parse rsync output to extract file count and bytes transferred.
/// This is a simplified parser that looks for common rsync output patterns.
pub fn parse_rsync_output(output: &[u8]) -> (u64, u64) {
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

/// Validate a subpath to prevent path traversal attacks.
///
/// Returns an error if the subpath contains dangerous patterns like `..`.
pub fn validate_subpath(subpath: &str) -> Result<(), &'static str> {
    // Check for path traversal patterns
    if subpath.contains("..") {
        return Err("Subpath cannot contain '..' (path traversal)");
    }
    // Check for null bytes
    if subpath.contains('\0') {
        return Err("Subpath cannot contain null bytes");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rsync_output_with_stats() {
        let output = b"sending incremental file list\n\
            file1.txt\n\
            file2.txt\n\
            sent 1234 bytes  received 5678 bytes  100.00 bytes/sec\n\
            total size is 10000  speedup is 1.00\n";
        let (files, bytes) = parse_rsync_output(output);
        // 2 files (file1.txt, file2.txt)
        assert_eq!(files, 2);
        // 1234 + 5678 = 6912
        assert_eq!(bytes, 6912);
    }

    #[test]
    fn test_parse_rsync_output_empty() {
        let output = b"";
        let (files, bytes) = parse_rsync_output(output);
        assert_eq!(files, 0);
        assert_eq!(bytes, 0);
    }

    #[test]
    fn test_parse_rsync_output_no_files() {
        let output = b"sending incremental file list\n\
            sent 100 bytes  received 200 bytes  50.00 bytes/sec\n\
            total size is 0  speedup is 1.00\n";
        let (files, bytes) = parse_rsync_output(output);
        assert_eq!(files, 0);
        assert_eq!(bytes, 300);
    }

    #[test]
    fn test_validate_subpath_valid() {
        assert!(validate_subpath("foo/bar").is_ok());
        assert!(validate_subpath("data/2024").is_ok());
        assert!(validate_subpath("").is_ok());
    }

    #[test]
    fn test_validate_subpath_path_traversal() {
        assert!(validate_subpath("../etc/passwd").is_err());
        assert!(validate_subpath("foo/../bar").is_err());
        assert!(validate_subpath("foo/..").is_err());
    }

    #[test]
    fn test_validate_subpath_null_byte() {
        assert!(validate_subpath("foo\0bar").is_err());
    }
}
