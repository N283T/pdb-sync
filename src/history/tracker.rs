//! History tracking for pdb-cli operations.

use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// History tracker for recording operation timestamps.
pub struct HistoryTracker {
    #[allow(dead_code)] // Will be used by sync/download commands
    path: PathBuf,
    history: OperationHistory,
}

/// Stored operation history.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OperationHistory {
    /// Last sync operation timestamp
    pub last_sync: Option<DateTime<Utc>>,
    /// Last download operation timestamp
    pub last_download: Option<DateTime<Utc>>,
}

impl HistoryTracker {
    /// Load history from the default location (~/.cache/pdb-cli/history.json).
    pub fn load() -> Result<Self> {
        let path = Self::default_path()?;
        Self::load_from(&path)
    }

    /// Load history from a specific path.
    pub fn load_from(path: &PathBuf) -> Result<Self> {
        let history = if path.exists() {
            let content = std::fs::read_to_string(path)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            OperationHistory::default()
        };

        Ok(Self {
            path: path.clone(),
            history,
        })
    }

    /// Get the default history file path.
    fn default_path() -> Result<PathBuf> {
        let cache_dir = directories::ProjectDirs::from("", "", "pdb-cli")
            .map(|dirs| dirs.cache_dir().to_path_buf())
            .unwrap_or_else(|| {
                directories::BaseDirs::new()
                    .map(|dirs| dirs.home_dir().join(".cache").join("pdb-cli"))
                    .unwrap_or_else(|| PathBuf::from(".cache/pdb-cli"))
            });

        Ok(cache_dir.join("history.json"))
    }

    /// Save history to disk.
    #[allow(dead_code)] // Will be used by sync/download commands
    pub fn save(&self) -> Result<()> {
        // Create parent directory if needed
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(&self.history)?;
        std::fs::write(&self.path, content)?;
        Ok(())
    }

    /// Record a sync operation.
    #[allow(dead_code)] // Will be used by sync command
    pub fn record_sync(&mut self, timestamp: DateTime<Utc>) {
        self.history.last_sync = Some(timestamp);
    }

    /// Record a download operation.
    #[allow(dead_code)] // Will be used by download command
    pub fn record_download(&mut self, timestamp: DateTime<Utc>) {
        self.history.last_download = Some(timestamp);
    }

    /// Get the last sync timestamp.
    pub fn last_sync(&self) -> Option<DateTime<Utc>> {
        self.history.last_sync
    }

    /// Get the last download timestamp.
    pub fn last_download(&self) -> Option<DateTime<Utc>> {
        self.history.last_download
    }

    /// Get the underlying history data.
    #[allow(dead_code)] // May be used for JSON output
    pub fn history(&self) -> &OperationHistory {
        &self.history
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("history.json");

        let tracker = HistoryTracker::load_from(&path).unwrap();
        assert!(tracker.last_sync().is_none());
        assert!(tracker.last_download().is_none());
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("subdir").join("history.json");

        let now = Utc::now();

        // Create and save
        let mut tracker = HistoryTracker::load_from(&path).unwrap();
        tracker.record_sync(now);
        tracker.save().unwrap();

        // Load again
        let tracker2 = HistoryTracker::load_from(&path).unwrap();
        assert!(tracker2.last_sync().is_some());
        assert!(tracker2.last_download().is_none());
    }

    #[test]
    fn test_record_operations() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("history.json");

        let mut tracker = HistoryTracker::load_from(&path).unwrap();
        let sync_time = Utc::now();
        let download_time = Utc::now();

        tracker.record_sync(sync_time);
        tracker.record_download(download_time);

        assert_eq!(tracker.last_sync(), Some(sync_time));
        assert_eq!(tracker.last_download(), Some(download_time));
    }

    #[test]
    fn test_history_serialization() {
        let mut history = OperationHistory::default();
        history.last_sync = Some(Utc::now());

        let json = serde_json::to_string(&history).unwrap();
        let parsed: OperationHistory = serde_json::from_str(&json).unwrap();

        assert!(parsed.last_sync.is_some());
    }
}
