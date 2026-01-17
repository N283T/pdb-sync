//! Remote statistics provider for RCSB PDB.

use crate::api::RcsbClient;
use crate::error::Result;
use crate::stats::types::{Comparison, RemoteStats};
use chrono::Utc;

/// Provider for remote PDB archive statistics.
pub struct RemoteStatsProvider;

impl RemoteStatsProvider {
    /// Fetch statistics from the remote PDB archive.
    pub async fn fetch_stats(client: &RcsbClient) -> Result<RemoteStats> {
        let total_entries = client.get_total_entry_count().await?;

        Ok(RemoteStats {
            total_entries,
            fetched_at: Utc::now(),
        })
    }

    /// Compare local statistics with remote statistics.
    pub fn compare(local_count: usize, remote_stats: &RemoteStats) -> Comparison {
        Comparison::new(local_count, remote_stats.total_entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare() {
        let remote = RemoteStats {
            total_entries: 200000,
            fetched_at: Utc::now(),
        };

        let comparison = RemoteStatsProvider::compare(10000, &remote);
        assert_eq!(comparison.local_count, 10000);
        assert_eq!(comparison.remote_count, 200000);
        assert!((comparison.coverage_percent - 5.0).abs() < 0.01);
    }
}
