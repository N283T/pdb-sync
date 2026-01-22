//! Sync plan mode - analyze what would change without executing.

use crate::error::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Sync plan summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPlan {
    pub name: String,
    pub url: String,
    pub dest: String,
    pub has_deletions: bool,
    pub stats: RsyncStats,
}

/// Statistics parsed from rsync output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RsyncStats {
    pub files: u64,
    pub total_size: u64,
    pub created: u64,
    pub deleted: u64,
    pub transferred: u64,
}

impl SyncPlan {
    /// Print the plan in human-readable format.
    pub fn print(&self) {
        println!("Sync Plan: {}", self.name);
        println!("  Source: {}", self.url);
        println!("  Destination: {}", self.dest);
        println!();
        println!("  Files: {}", self.stats.files);
        println!("  Total size: {}", format_size(self.stats.total_size));
        println!();

        if self.stats.has_changes() {
            println!("  Changes:");
            if self.stats.created > 0 {
                println!("    Created: {} files", self.stats.created);
            }
            if self.stats.deleted > 0 {
                println!("    Deleted: {} files", self.stats.deleted);
            }
            if self.stats.transferred > 0 {
                println!("    Transferred: {} files", self.stats.transferred);
            }
        } else {
            println!("  No changes - already up to date");
        }
    }
}

impl RsyncStats {
    /// Check if there are any changes.
    pub fn has_changes(&self) -> bool {
        self.created > 0 || self.deleted > 0 || self.transferred > 0
    }
}

/// Parse rsync --stats output.
pub fn parse_rsync_stats(output: &str) -> Result<RsyncStats> {
    let mut stats = RsyncStats::default();

    for line in output.lines() {
        if line.contains("Number of files:") {
            stats.files = parse_number(line).unwrap_or(0);
        } else if line.contains("Total file size:") {
            stats.total_size = parse_human_size(line).unwrap_or(0);
        } else if line.contains("Created:") {
            stats.created = parse_num_files(line).unwrap_or(0);
        } else if line.contains("Deleted:") {
            stats.deleted = parse_num_files(line).unwrap_or(0);
        } else if line.contains("Transferred:") {
            stats.transferred = parse_num_files(line).unwrap_or(0);
        }
    }

    Ok(stats)
}

fn parse_number(line: &str) -> Option<u64> {
    let pos = line.find(':')?;
    let rest = &line[pos + 2..];
    let num_str: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == ',' || *c == '.')
        .collect();
    num_str.replace(',', "").parse().ok()
}

fn parse_human_size(line: &str) -> Option<u64> {
    let re = SIZE_REGEX
        .get_or_init(|| Regex::new(r"(\d+\.?\d*)\s*([KMGTPE]i?B)").expect("invalid regex"));
    let caps = re.captures(line)?;
    let num: f64 = caps.get(1)?.as_str().parse().ok()?;
    let unit = caps.get(2)?.as_str().to_uppercase();

    let multiplier = match unit.as_str() {
        "B" => 1,
        "KB" | "KIB" => 1024,
        "MB" | "MIB" => 1024 * 1024,
        "GB" | "GIB" => 1024 * 1024 * 1024,
        "TB" | "TIB" => 1024u64.pow(4),
        "PB" | "PIB" => 1024u64.pow(5),
        _ => return None,
    };

    Some((num * multiplier as f64) as u64)
}

/// Cached regex for parsing human-readable sizes.
static SIZE_REGEX: OnceLock<Regex> = OnceLock::new();

fn parse_num_files(line: &str) -> Option<u64> {
    // Find the number in "Created: 100 files" or "Deleted: 1,000 files"
    let parts: Vec<&str> = line.split_whitespace().collect();
    for part in parts {
        // Check if part looks like a number (possibly with commas)
        let cleaned = part.replace(',', "");
        if cleaned.parse::<u64>().is_ok() {
            return cleaned.parse().ok();
        }
    }
    None
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[(&str, u64)] = &[
        ("PB", 1024u64.pow(5)),
        ("TB", 1024u64.pow(4)),
        ("GB", 1024u64.pow(3)),
        ("MB", 1024u64.pow(2)),
        ("KB", 1024),
        ("B", 1),
    ];

    for &(unit, size_in_bytes) in UNITS {
        if bytes >= size_in_bytes {
            let value = bytes as f64 / size_in_bytes as f64;
            // Only show decimal for units larger than B
            if unit == "B" {
                return format!("{}{}", value as u64, unit);
            } else {
                return format!("{:.1}{}", value, unit);
            }
        }
    }
    format!("{}B", bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_number() {
        assert_eq!(parse_number("Number of files: 1,234"), Some(1234));
        assert_eq!(parse_number("Number of files: 100"), Some(100));
    }

    #[test]
    fn test_parse_human_size() {
        assert_eq!(
            parse_human_size("Total file size: 1.5 GB"),
            Some(1_610_612_736)
        );
        assert_eq!(
            parse_human_size("Total file size: 500 MB"),
            Some(524_288_000)
        );
    }

    #[test]
    fn test_parse_num_files() {
        assert_eq!(parse_num_files("Created: 100 files"), Some(100));
        assert_eq!(parse_num_files("Deleted: 1,000 files"), Some(1000));
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(1024), "1.0KB");
        assert_eq!(format_size(1024 * 1024), "1.0MB");
        assert_eq!(format_size(500), "500B");
    }
}
