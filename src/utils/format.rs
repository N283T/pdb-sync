//! Formatting utilities for human-readable output.

/// Format bytes as human-readable size (e.g., "1.5 GB", "234 KB", "100 B").
pub fn human_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
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
        assert_eq!(human_bytes(100), "100 B");
        assert_eq!(human_bytes(1023), "1023 B");
    }

    #[test]
    fn test_human_bytes_kilobytes() {
        assert_eq!(human_bytes(1024), "1.0 KB");
        assert_eq!(human_bytes(1536), "1.5 KB");
        assert_eq!(human_bytes(10 * 1024), "10.0 KB");
    }

    #[test]
    fn test_human_bytes_megabytes() {
        assert_eq!(human_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(human_bytes(50 * 1024 * 1024), "50.0 MB");
    }

    #[test]
    fn test_human_bytes_gigabytes() {
        assert_eq!(human_bytes(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(
            human_bytes(45 * 1024 * 1024 * 1024 + 600 * 1024 * 1024),
            "45.6 GB"
        );
    }

    #[test]
    fn test_human_bytes_terabytes() {
        assert_eq!(human_bytes(1024 * 1024 * 1024 * 1024), "1.0 TB");
    }
}
