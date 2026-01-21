//! State persistence for the watch command.
//!
//! Tracks the last check timestamp and set of downloaded PDB IDs
//! to avoid re-downloading entries across sessions.

use crate::error::{PdbSyncError, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use tokio::fs;

/// State file name
const STATE_FILE: &str = "watch_state.json";

/// Maximum number of downloaded IDs to keep in state
/// This prevents unbounded growth of the state file.
/// With ~300 new entries per week, 10000 covers about 8 months.
const MAX_DOWNLOADED_IDS: usize = 10000;

/// Watch state that persists across sessions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WatchState {
    /// Last check timestamp (YYYY-MM-DD)
    pub last_check: Option<NaiveDate>,
    /// Set of already downloaded PDB IDs (lowercase)
    #[serde(default)]
    pub downloaded_ids: HashSet<String>,
}

impl WatchState {
    /// Get the state file path
    pub fn state_path() -> Result<PathBuf> {
        let cache_dir = directories::BaseDirs::new()
            .ok_or_else(|| PdbSyncError::StatePersistence("Cannot find home directory".into()))?
            .cache_dir()
            .join("pdb-sync");

        Ok(cache_dir.join(STATE_FILE))
    }

    /// Load state from disk, or return default if not found
    pub async fn load_or_init() -> Result<Self> {
        let path = Self::state_path()?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path).await.map_err(|e| {
            PdbSyncError::StatePersistence(format!("Failed to read state file: {}", e))
        })?;

        serde_json::from_str(&content).map_err(|e| {
            PdbSyncError::StatePersistence(format!("Failed to parse state file: {}", e))
        })
    }

    /// Save state to disk (prunes old entries if needed)
    pub async fn save(&mut self) -> Result<()> {
        // Prune before saving to prevent unbounded growth
        self.prune_if_needed();

        let path = Self::state_path()?;

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                PdbSyncError::StatePersistence(format!("Failed to create cache directory: {}", e))
            })?;
        }

        let content = serde_json::to_string_pretty(self).map_err(|e| {
            PdbSyncError::StatePersistence(format!("Failed to serialize state: {}", e))
        })?;

        fs::write(&path, content).await.map_err(|e| {
            PdbSyncError::StatePersistence(format!("Failed to write state file: {}", e))
        })?;

        Ok(())
    }

    /// Check if a PDB ID has been downloaded (case-insensitive)
    pub fn is_downloaded(&self, pdb_id: &str) -> bool {
        self.downloaded_ids.contains(&pdb_id.to_lowercase())
    }

    /// Mark a PDB ID as downloaded (stores lowercase)
    pub fn mark_downloaded(&mut self, pdb_id: &str) {
        self.downloaded_ids.insert(pdb_id.to_lowercase());
    }

    /// Update the last check timestamp
    pub fn update_last_check(&mut self, date: NaiveDate) {
        self.last_check = Some(date);
    }

    /// Prune old entries if the set exceeds the maximum size.
    /// This is called automatically before saving to prevent unbounded growth.
    /// Since we can't track insertion order with HashSet, we simply clear
    /// half the entries when the limit is exceeded (older entries are more
    /// likely to be irrelevant anyway since they're outside the search window).
    pub fn prune_if_needed(&mut self) {
        if self.downloaded_ids.len() > MAX_DOWNLOADED_IDS {
            // Keep roughly half the entries by random sampling
            // This is acceptable because:
            // 1. Entries outside the search window (since last_check) won't be queried again
            // 2. Any accidentally re-downloaded entry just gets skipped (already exists on disk)
            let to_keep = MAX_DOWNLOADED_IDS / 2;
            let ids: Vec<String> = self.downloaded_ids.drain().take(to_keep).collect();
            self.downloaded_ids = ids.into_iter().collect();
            tracing::debug!(
                "Pruned state: kept {} of {} entries",
                self.downloaded_ids.len(),
                MAX_DOWNLOADED_IDS
            );
        }
    }

    /// Get the effective start date for searching
    /// Priority: explicit since date > last_check > 7 days ago
    pub fn effective_start_date(&self, since: Option<NaiveDate>) -> NaiveDate {
        since.unwrap_or_else(|| {
            self.last_check
                .unwrap_or_else(|| chrono::Utc::now().date_naive() - chrono::Duration::days(7))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_downloaded_case_insensitive() {
        let mut state = WatchState::default();
        state.mark_downloaded("1ABC");

        assert!(state.is_downloaded("1abc"));
        assert!(state.is_downloaded("1ABC"));
        assert!(state.is_downloaded("1Abc"));
        assert!(!state.is_downloaded("2xyz"));
    }

    #[test]
    fn test_effective_start_date_priority() {
        let state = WatchState::default();
        let today = chrono::Utc::now().date_naive();
        let seven_days_ago = today - chrono::Duration::days(7);

        // No last_check, no since -> 7 days ago
        let result = state.effective_start_date(None);
        assert_eq!(result, seven_days_ago);

        // With explicit since
        let explicit_date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let result = state.effective_start_date(Some(explicit_date));
        assert_eq!(result, explicit_date);
    }

    #[test]
    fn test_effective_start_date_with_last_check() {
        let mut state = WatchState::default();
        let last_check = NaiveDate::from_ymd_opt(2025, 6, 1).unwrap();
        state.update_last_check(last_check);

        // No explicit since -> use last_check
        let result = state.effective_start_date(None);
        assert_eq!(result, last_check);

        // Explicit since overrides last_check
        let explicit_date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let result = state.effective_start_date(Some(explicit_date));
        assert_eq!(result, explicit_date);
    }

    #[tokio::test]
    async fn test_state_roundtrip() {
        let temp_dir = tempfile::tempdir().unwrap();
        let state_path = temp_dir.path().join("watch_state.json");

        let mut state = WatchState::default();
        state.mark_downloaded("1abc");
        state.mark_downloaded("2xyz");
        state.update_last_check(NaiveDate::from_ymd_opt(2025, 6, 15).unwrap());

        // Manually save to temp location
        let content = serde_json::to_string_pretty(&state).unwrap();
        std::fs::write(&state_path, content).unwrap();

        // Load and verify
        let content = std::fs::read_to_string(&state_path).unwrap();
        let loaded: WatchState = serde_json::from_str(&content).unwrap();

        assert!(loaded.is_downloaded("1abc"));
        assert!(loaded.is_downloaded("2xyz"));
        assert_eq!(
            loaded.last_check,
            Some(NaiveDate::from_ymd_opt(2025, 6, 15).unwrap())
        );
    }
}
