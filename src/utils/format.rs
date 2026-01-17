//! Formatting utilities for human-readable output.

use thousands::Separable;

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

/// Format a number with thousands separators (e.g., 12345 -> "12,345").
pub fn format_with_commas(n: u64) -> String {
    n.separate_with_commas()
}

/// Format a number with thousands separators (usize version).
pub fn format_count(n: usize) -> String {
    (n as u64).separate_with_commas()
}

/// Escape a string for CSV output following RFC 4180.
///
/// - If the string contains commas, quotes, or newlines, wrap it in quotes
/// - Escape any existing quotes by doubling them
pub fn escape_csv_field(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
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

    #[test]
    fn test_format_with_commas() {
        assert_eq!(format_with_commas(0), "0");
        assert_eq!(format_with_commas(123), "123");
        assert_eq!(format_with_commas(1234), "1,234");
        assert_eq!(format_with_commas(12345), "12,345");
        assert_eq!(format_with_commas(1234567), "1,234,567");
    }

    #[test]
    fn test_format_count() {
        assert_eq!(format_count(0), "0");
        assert_eq!(format_count(12345), "12,345");
    }

    #[test]
    fn test_escape_csv_field_no_special() {
        assert_eq!(escape_csv_field("hello"), "hello");
        assert_eq!(escape_csv_field("1abc"), "1abc");
    }

    #[test]
    fn test_escape_csv_field_with_comma() {
        assert_eq!(escape_csv_field("hello,world"), "\"hello,world\"");
    }

    #[test]
    fn test_escape_csv_field_with_quote() {
        assert_eq!(escape_csv_field("he said \"hi\""), "\"he said \"\"hi\"\"\"");
    }

    #[test]
    fn test_escape_csv_field_with_newline() {
        assert_eq!(escape_csv_field("line1\nline2"), "\"line1\nline2\"");
    }
}
