//! Dry-run planning and stats parsing for sync operations.

use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Parsed rsync statistics from --stats output.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RsyncStats {
    pub num_files: u64,
    pub reg_files: u64,
    pub dirs: u64,
    pub symlinks: u64,
    pub total_size: u64,
    pub created: u64,
    pub deleted: u64,
    pub transferred: u64,
    pub literal_data: u64,
    pub matched_data: u64,
    pub file_list_size: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

impl RsyncStats {
    /// Check if there are any changes (created, deleted, or transferred files).
    pub fn has_changes(&self) -> bool {
        self.created > 0 || self.deleted > 0 || self.transferred > 0
    }

    /// Format byte count to human-readable size (K/M/G/T/P).
    pub fn format_size(&self) -> String {
        format_size(self.total_size)
    }

    /// Parse rsync --stats output from stderr.
    pub fn parse(stderr: &str) -> Result<Self> {
        parse_rsync_stats(stderr)
    }
}

/// Format a byte count to human-readable size.
fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["", "K", "M", "G", "T", "P"];

    if bytes == 0 {
        return "0".to_string();
    }

    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{}", bytes)
    } else {
        format!("{:.1}{}", size, UNITS[unit_index])
    }
}

/// Parse rsync --stats output.
fn parse_rsync_stats(stderr: &str) -> Result<RsyncStats> {
    let mut stats = RsyncStats::default();

    for line in stderr.lines() {
        let line = line.trim();

        // Parse "Number of files: 1,234 (reg: 1,100, dir: 100, link: 34)"
        if let Some(rest) = line.strip_prefix("Number of files:") {
            parse_num_files(rest, &mut stats);
            continue;
        }

        // Parse "Total file size: 1.5G bytes"
        if let Some(rest) = line.strip_prefix("Total file size:") {
            if let Some(size_str) = rest.split("bytes").next() {
                stats.total_size = parse_human_size(size_str.trim()).unwrap_or(0);
            }
            continue;
        }

        // Parse "Total transferred file size: 500M bytes"
        if let Some(rest) = line.strip_prefix("Total transferred file size:") {
            if let Some(size_str) = rest.split("bytes").next() {
                stats.literal_data = parse_human_size(size_str.trim()).unwrap_or(0);
            }
            continue;
        }

        // Parse "created: 100"
        if line.contains("created:") {
            if let Some(rest) = line.split("created:").nth(1) {
                if let Some(num_str) = rest.split_whitespace().next() {
                    stats.created = parse_number(num_str).unwrap_or(0);
                }
            }
            continue;
        }

        // Parse "deleted: 50"
        if line.contains("deleted:") {
            if let Some(rest) = line.split("deleted:").nth(1) {
                if let Some(num_str) = rest.split_whitespace().next() {
                    stats.deleted = parse_number(num_str).unwrap_or(0);
                }
            }
            continue;
        }

        // Parse "transferred: 200"
        if line.contains("transferred:") {
            if let Some(rest) = line.split("transferred:").nth(1) {
                if let Some(num_str) = rest.split_whitespace().next() {
                    stats.transferred = parse_number(num_str).unwrap_or(0);
                }
            }
            continue;
        }

        // Parse "Literal data: 100M bytes"
        if let Some(rest) = line.strip_prefix("Literal data:") {
            if let Some(size_str) = rest.split("bytes").next() {
                stats.literal_data = parse_human_size(size_str.trim()).unwrap_or(0);
            }
            continue;
        }

        // Parse "Matched data: 400M bytes"
        if let Some(rest) = line.strip_prefix("Matched data:") {
            if let Some(size_str) = rest.split("bytes").next() {
                stats.matched_data = parse_human_size(size_str.trim()).unwrap_or(0);
            }
            continue;
        }

        // Parse "File list size: 1.5M bytes"
        if let Some(rest) = line.strip_prefix("File list size:") {
            if let Some(size_str) = rest.split("bytes").next() {
                stats.file_list_size = parse_human_size(size_str.trim()).unwrap_or(0);
            }
            continue;
        }

        // Parse "Total bytes sent: 1.2M"
        if let Some(rest) = line.strip_prefix("Total bytes sent:") {
            stats.bytes_sent = parse_human_size(rest.trim()).unwrap_or(0);
            continue;
        }

        // Parse "Total bytes received: 3.4M"
        if let Some(rest) = line.strip_prefix("Total bytes received:") {
            stats.bytes_received = parse_human_size(rest.trim()).unwrap_or(0);
            continue;
        }
    }

    Ok(stats)
}

/// Parse the "Number of files: X (reg: Y, dir: Z, link: W)" line.
fn parse_num_files(line: &str, stats: &mut RsyncStats) {
    // Extract total files before the paren
    if let Some(paren_pos) = line.find('(') {
        let total_str = line[..paren_pos].trim();
        stats.num_files = parse_number(total_str).unwrap_or(0);
    } else {
        stats.num_files = parse_number(line.trim()).unwrap_or(0);
    }

    // Parse components inside parentheses
    // Split by ", " (comma followed by space) to avoid splitting numbers with commas
    if let Some(start) = line.find('(') {
        if let Some(end) = line.find(')') {
            let components = &line[start + 1..end];
            for part in components.split(", ") {
                let part = part.trim();
                if let Some(value_str) = part.split(':').nth(1) {
                    let value = parse_number(value_str.trim()).unwrap_or(0);
                    if part.starts_with("reg:") {
                        stats.reg_files = value;
                    } else if part.starts_with("dir:") {
                        stats.dirs = value;
                    } else if part.starts_with("link:") {
                        stats.symlinks = value;
                    }
                }
            }
        }
    }
}

/// Parse a number with optional commas (e.g., "1,234" -> 1234).
fn parse_number(s: &str) -> Option<u64> {
    s.replace(',', "").parse().ok()
}

/// Parse a human-readable size (e.g., "1.5G", "500M", "1024K").
fn parse_human_size(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return Some(0);
    }

    let last_char = s.chars().last()?.to_ascii_uppercase();

    if last_char.is_ascii_alphabetic() {
        let multiplier: u64 = match last_char {
            'K' => 1024,
            'M' => 1_048_576,
            'G' => 1_073_741_824,
            'T' => 1_099_511_627_776,
            'P' => 1_125_899_906_842_624,
            _ => return None,
        };

        let num_str = &s[..s.len() - 1];
        let num: f64 = num_str.trim().parse().ok()?;
        Some((num * multiplier as f64) as u64)
    } else {
        s.parse().ok()
    }
}

/// Summary of sync plan operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPlan {
    pub name: String,
    pub url: String,
    pub dest: String,
    pub has_deletions: bool,
    pub stats: RsyncStats,
}

#[cfg(test)]
mod tests {
    use super::*;

    // === RsyncStats::default() tests ===

    #[test]
    fn test_empty_creates_zero_values() {
        let stats = RsyncStats::default();
        assert_eq!(stats, RsyncStats::default());
        assert_eq!(stats.num_files, 0);
        assert_eq!(stats.total_size, 0);
        assert_eq!(stats.created, 0);
        assert_eq!(stats.deleted, 0);
        assert_eq!(stats.transferred, 0);
    }

    // === RsyncStats::has_changes() tests ===

    #[test]
    fn test_has_changes_with_created() {
        let stats = RsyncStats {
            created: 10,
            ..Default::default()
        };
        assert!(stats.has_changes());
    }

    #[test]
    fn test_has_changes_with_deleted() {
        let stats = RsyncStats {
            deleted: 5,
            ..Default::default()
        };
        assert!(stats.has_changes());
    }

    #[test]
    fn test_has_changes_with_transferred() {
        let stats = RsyncStats {
            transferred: 100,
            ..Default::default()
        };
        assert!(stats.has_changes());
    }

    #[test]
    fn test_has_changes_returns_false_when_no_changes() {
        let stats = RsyncStats::default();
        assert!(!stats.has_changes());
    }

    // === format_size() tests ===

    #[test]
    fn test_format_size_zero() {
        assert_eq!(format_size(0), "0");
    }

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(100), "100");
        assert_eq!(format_size(1023), "1023");
    }

    #[test]
    fn test_format_size_kilobytes() {
        assert_eq!(format_size(1024), "1.0K");
        assert_eq!(format_size(1536), "1.5K");
        assert_eq!(format_size(10 * 1024), "10.0K");
    }

    #[test]
    fn test_format_size_megabytes() {
        assert_eq!(format_size(1_048_576), "1.0M");
        assert_eq!(format_size(1_572_864), "1.5M");
        assert_eq!(format_size(10 * 1_048_576), "10.0M");
    }

    #[test]
    fn test_format_size_gigabytes() {
        assert_eq!(format_size(1_073_741_824), "1.0G");
        assert_eq!(format_size(1_610_612_736), "1.5G");
    }

    #[test]
    fn test_format_size_terabytes() {
        assert_eq!(format_size(1_099_511_627_776), "1.0T");
        assert_eq!(format_size(1_649_267_441_664), "1.5T");
    }

    #[test]
    fn test_format_size_petabytes() {
        assert_eq!(format_size(1_125_899_906_842_624), "1.0P");
    }

    #[test]
    fn test_rsync_stats_format_size() {
        let stats = RsyncStats {
            total_size: 1_572_864, // 1.5 MB
            ..Default::default()
        };
        assert_eq!(stats.format_size(), "1.5M");
    }

    // === parse_number() tests ===

    #[test]
    fn test_parse_number_plain() {
        assert_eq!(parse_number("123"), Some(123));
        assert_eq!(parse_number("0"), Some(0));
        assert_eq!(parse_number("999999"), Some(999999));
    }

    #[test]
    fn test_parse_number_with_commas() {
        assert_eq!(parse_number("1,234"), Some(1234));
        assert_eq!(parse_number("1,000,000"), Some(1_000_000));
        assert_eq!(parse_number("999,999,999"), Some(999_999_999));
    }

    #[test]
    fn test_parse_number_invalid() {
        assert_eq!(parse_number(""), None);
        assert_eq!(parse_number("abc"), None);
        // Note: "12,34" is treated as 1234 (commas are simply removed)
        // This is a reasonable interpretation for handling various number formats
        assert_eq!(parse_number("12,34"), Some(1234));
        assert_eq!(parse_number("1,,2"), Some(12)); // Multiple commas
    }

    // === parse_human_size() tests ===

    #[test]
    fn test_parse_human_size_bytes() {
        assert_eq!(parse_human_size("100"), Some(100));
        assert_eq!(parse_human_size("1024"), Some(1024));
    }

    #[test]
    fn test_parse_human_size_kilobytes() {
        assert_eq!(parse_human_size("1K"), Some(1024));
        assert_eq!(parse_human_size("1.5K"), Some(1536));
        assert_eq!(parse_human_size("100K"), Some(102_400));
        assert_eq!(parse_human_size("1k"), Some(1024)); // lowercase
    }

    #[test]
    fn test_parse_human_size_megabytes() {
        assert_eq!(parse_human_size("1M"), Some(1_048_576));
        assert_eq!(parse_human_size("1.5M"), Some(1_572_864));
        assert_eq!(parse_human_size("100M"), Some(104_857_600));
        assert_eq!(parse_human_size("1m"), Some(1_048_576)); // lowercase
    }

    #[test]
    fn test_parse_human_size_gigabytes() {
        assert_eq!(parse_human_size("1G"), Some(1_073_741_824));
        assert_eq!(parse_human_size("1.5G"), Some(1_610_612_736));
        assert_eq!(parse_human_size("1g"), Some(1_073_741_824)); // lowercase
    }

    #[test]
    fn test_parse_human_size_terabytes() {
        assert_eq!(parse_human_size("1T"), Some(1_099_511_627_776));
        assert_eq!(parse_human_size("1.5T"), Some(1_649_267_441_664));
    }

    #[test]
    fn test_parse_human_size_petabytes() {
        assert_eq!(parse_human_size("1P"), Some(1_125_899_906_842_624));
        assert_eq!(parse_human_size("1.5P"), Some(1_688_849_860_263_936));
    }

    #[test]
    fn test_parse_human_size_empty() {
        assert_eq!(parse_human_size(""), Some(0));
        assert_eq!(parse_human_size("   "), Some(0));
    }

    #[test]
    fn test_parse_human_size_invalid() {
        assert_eq!(parse_human_size("abc"), None);
        assert_eq!(parse_human_size("1X"), None); // Invalid unit
    }

    // === parse_num_files() tests ===

    #[test]
    fn test_parse_num_files_simple() {
        let mut stats = RsyncStats::default();
        parse_num_files("123", &mut stats);
        assert_eq!(stats.num_files, 123);
    }

    #[test]
    fn test_parse_num_files_with_commas() {
        let mut stats = RsyncStats::default();
        parse_num_files("1,234", &mut stats);
        assert_eq!(stats.num_files, 1234);
    }

    #[test]
    fn test_parse_num_files_with_components() {
        let mut stats = RsyncStats::default();
        parse_num_files("1,234 (reg: 1,100, dir: 100, link: 34)", &mut stats);
        assert_eq!(stats.num_files, 1234);
        assert_eq!(stats.reg_files, 1100);
        assert_eq!(stats.dirs, 100);
        assert_eq!(stats.symlinks, 34);
    }

    #[test]
    fn test_parse_num_files_partial_components() {
        let mut stats = RsyncStats::default();
        parse_num_files("100 (reg: 80, dir: 20)", &mut stats);
        assert_eq!(stats.num_files, 100);
        assert_eq!(stats.reg_files, 80);
        assert_eq!(stats.dirs, 20);
    }

    // === parse_rsync_stats() tests ===

    #[test]
    fn test_parse_complete_stats() {
        let stderr = r#"Number of files: 1,234 (reg: 1,100, dir: 100, link: 34)
Total file size: 1.5G bytes
Total transferred file size: 500M bytes
Literal data: 400M bytes
Matched data: 100M bytes
File list size: 2.5M bytes
Total bytes sent: 1.2M
Total bytes received: 3.4M
created: 100
deleted: 50
transferred: 200
"#;

        let stats = parse_rsync_stats(stderr).unwrap();
        assert_eq!(stats.num_files, 1234);
        assert_eq!(stats.reg_files, 1100);
        assert_eq!(stats.dirs, 100);
        assert_eq!(stats.symlinks, 34);
        assert_eq!(stats.total_size, 1_610_612_736); // 1.5G
        assert_eq!(stats.literal_data, 419_430_400); // 400M
        assert_eq!(stats.matched_data, 104_857_600); // 100M
        assert_eq!(stats.file_list_size, 2_621_440); // 2.5M
        assert_eq!(stats.bytes_sent, 1_258_291); // 1.2M
        assert_eq!(stats.bytes_received, 3_565_158); // 3.4M
        assert_eq!(stats.created, 100);
        assert_eq!(stats.deleted, 50);
        assert_eq!(stats.transferred, 200);
    }

    #[test]
    fn test_parse_partial_stats() {
        let stderr = r#"Number of files: 100
Total file size: 1G bytes
"#;

        let stats = parse_rsync_stats(stderr).unwrap();
        assert_eq!(stats.num_files, 100);
        assert_eq!(stats.total_size, 1_073_741_824); // 1G
        assert_eq!(stats.created, 0); // Default value
        assert_eq!(stats.deleted, 0); // Default value
    }

    #[test]
    fn test_parse_empty_stats() {
        let stderr = "";
        let stats = parse_rsync_stats(stderr).unwrap();
        assert_eq!(stats, RsyncStats::default());
    }

    #[test]
    fn test_parse_stats_no_changes() {
        let stderr = r#"Number of files: 1,000
Total file size: 1G bytes
created: 0
deleted: 0
transferred: 0
"#;

        let stats = parse_rsync_stats(stderr).unwrap();
        assert!(!stats.has_changes());
    }

    // === RsyncStats serialization tests ===

    #[test]
    fn test_rsync_stats_json_roundtrip() {
        let original = RsyncStats {
            num_files: 1234,
            total_size: 1_500_000,
            created: 100,
            deleted: 50,
            transferred: 200,
            ..Default::default()
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: RsyncStats = serde_json::from_str(&json).unwrap();

        assert_eq!(original, deserialized);
    }

    // === SyncPlan tests ===

    #[test]
    fn test_sync_plan_has_deletions() {
        let plan = SyncPlan {
            name: "test".to_string(),
            url: "rsync://example.com".to_string(),
            dest: "/data".to_string(),
            has_deletions: true,
            stats: RsyncStats::default(),
        };
        assert!(plan.has_deletions);
    }

    #[test]
    fn test_sync_plan_json_serialization() {
        let plan = SyncPlan {
            name: "structures".to_string(),
            url: "rsync://example.com::structures".to_string(),
            dest: "structures".to_string(),
            has_deletions: false,
            stats: RsyncStats {
                num_files: 250000,
                total_size: 912_680_550_400,
                created: 0,
                deleted: 0,
                transferred: 0,
                ..Default::default()
            },
        };

        let json = serde_json::to_string_pretty(&plan).unwrap();
        assert!(json.contains("\"name\": \"structures\""));
        assert!(json.contains("\"has_deletions\": false"));
        assert!(json.contains("\"num_files\": 250000"));

        // Roundtrip
        let deserialized: SyncPlan = serde_json::from_str(&json).unwrap();
        assert_eq!(plan.name, deserialized.name);
        assert_eq!(plan.stats.num_files, deserialized.stats.num_files);
    }

    // === RsyncStats::parse() wrapper tests ===

    #[test]
    fn test_rsync_stats_parse_wrapper() {
        let stderr = r#"Number of files: 500
Total file size: 100M bytes
created: 10
deleted: 5
"#;

        let stats = RsyncStats::parse(stderr).unwrap();
        assert_eq!(stats.num_files, 500);
        assert_eq!(stats.total_size, 104_857_600); // 100M
        assert_eq!(stats.created, 10);
        assert_eq!(stats.deleted, 5);
        assert!(stats.has_changes());
    }
}
