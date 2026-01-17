//! Statistics data structures.

use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Statistics about local PDB collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalStats {
    /// Total number of files
    pub total_entries: usize,
    /// Number of unique PDB IDs
    pub unique_pdb_ids: usize,
    /// Total size of all files in bytes
    pub total_size: u64,
    /// Statistics by file format
    pub by_format: BTreeMap<String, FormatStats>,
    /// Statistics by data type
    pub by_data_type: BTreeMap<String, DataTypeStats>,
    /// Detailed statistics (if requested)
    pub detailed: Option<DetailedStats>,
    /// Last sync timestamp
    pub last_sync: Option<DateTime<Utc>>,
    /// Last download timestamp
    pub last_download: Option<DateTime<Utc>>,
}

/// Statistics for a specific file format.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FormatStats {
    /// Number of files in this format
    pub count: usize,
    /// Total size of files in this format
    pub size: u64,
}

/// Statistics for a specific data type.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DataTypeStats {
    /// Number of files of this data type
    pub count: usize,
    /// Total size of files of this data type
    pub size: u64,
    /// Number of unique PDB IDs for this data type
    pub unique_pdb_ids: usize,
}

/// Detailed statistics about the collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedStats {
    /// Smallest file size
    pub min_size: u64,
    /// Largest file size
    pub max_size: u64,
    /// Average file size
    pub avg_size: f64,
    /// Information about the smallest file
    pub smallest_file: Option<FileInfo>,
    /// Information about the largest file
    pub largest_file: Option<FileInfo>,
    /// Information about the oldest file
    pub oldest_file: Option<FileInfo>,
    /// Information about the newest file
    pub newest_file: Option<FileInfo>,
    /// Size distribution
    pub size_distribution: SizeDistribution,
}

/// Information about a specific file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// PDB ID
    pub pdb_id: String,
    /// File path
    pub path: PathBuf,
    /// File size in bytes
    pub size: u64,
    /// Last modification time
    pub modified: DateTime<Local>,
}

/// Distribution of file sizes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SizeDistribution {
    /// Files under 1 KB
    pub under_1kb: usize,
    /// Files 1-10 KB
    pub kb_1_10: usize,
    /// Files 10-100 KB
    pub kb_10_100: usize,
    /// Files 100 KB - 1 MB
    pub kb_100_1mb: usize,
    /// Files 1-10 MB
    pub mb_1_10: usize,
    /// Files over 10 MB
    pub over_10mb: usize,
}

impl SizeDistribution {
    /// Categorize a file size into the appropriate bucket.
    pub fn add(&mut self, size: u64) {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;

        if size < KB {
            self.under_1kb += 1;
        } else if size < 10 * KB {
            self.kb_1_10 += 1;
        } else if size < 100 * KB {
            self.kb_10_100 += 1;
        } else if size < MB {
            self.kb_100_1mb += 1;
        } else if size < 10 * MB {
            self.mb_1_10 += 1;
        } else {
            self.over_10mb += 1;
        }
    }
}

/// Statistics from remote PDB archive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteStats {
    /// Total number of entries in the archive
    pub total_entries: usize,
    /// When the stats were fetched
    pub fetched_at: DateTime<Utc>,
}

/// Comparison between local and remote statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comparison {
    /// Number of entries in local collection
    pub local_count: usize,
    /// Number of entries in remote archive
    pub remote_count: usize,
    /// Coverage percentage (local/remote * 100)
    pub coverage_percent: f64,
}

impl Comparison {
    /// Create a new comparison.
    pub fn new(local_count: usize, remote_count: usize) -> Self {
        let coverage_percent = if remote_count > 0 {
            (local_count as f64 / remote_count as f64) * 100.0
        } else {
            0.0
        };

        Self {
            local_count,
            remote_count,
            coverage_percent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_distribution_add() {
        let mut dist = SizeDistribution::default();

        dist.add(500); // under 1KB
        dist.add(2048); // 1-10 KB
        dist.add(50 * 1024); // 10-100 KB
        dist.add(500 * 1024); // 100KB-1MB
        dist.add(5 * 1024 * 1024); // 1-10 MB
        dist.add(20 * 1024 * 1024); // over 10 MB

        assert_eq!(dist.under_1kb, 1);
        assert_eq!(dist.kb_1_10, 1);
        assert_eq!(dist.kb_10_100, 1);
        assert_eq!(dist.kb_100_1mb, 1);
        assert_eq!(dist.mb_1_10, 1);
        assert_eq!(dist.over_10mb, 1);
    }

    #[test]
    fn test_comparison_new() {
        let cmp = Comparison::new(1000, 200000);
        assert_eq!(cmp.local_count, 1000);
        assert_eq!(cmp.remote_count, 200000);
        assert!((cmp.coverage_percent - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_comparison_zero_remote() {
        let cmp = Comparison::new(100, 0);
        assert_eq!(cmp.coverage_percent, 0.0);
    }

    #[test]
    fn test_format_stats_default() {
        let stats = FormatStats::default();
        assert_eq!(stats.count, 0);
        assert_eq!(stats.size, 0);
    }

    #[test]
    fn test_data_type_stats_default() {
        let stats = DataTypeStats::default();
        assert_eq!(stats.count, 0);
        assert_eq!(stats.size, 0);
        assert_eq!(stats.unique_pdb_ids, 0);
    }
}
