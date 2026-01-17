pub mod manager;
pub mod spawn;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

/// Job identifier (8 character hex string)
pub type JobId = String;

/// Status of a background job
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl fmt::Display for JobStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobStatus::Running => write!(f, "running"),
            JobStatus::Completed => write!(f, "completed"),
            JobStatus::Failed => write!(f, "failed"),
            JobStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Metadata for a background job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobMeta {
    /// Unique job identifier
    pub id: JobId,
    /// Command that was executed
    pub command: String,
    /// When the job started
    pub started_at: DateTime<Utc>,
    /// When the job finished (if completed)
    pub finished_at: Option<DateTime<Utc>>,
    /// Current status
    pub status: JobStatus,
    /// Process ID (if running)
    pub pid: Option<u32>,
    /// Exit code (if finished)
    pub exit_code: Option<i32>,
}

impl JobMeta {
    /// Create a new job metadata entry
    pub fn new(id: JobId, command: String, pid: u32) -> Self {
        Self {
            id,
            command,
            started_at: Utc::now(),
            finished_at: None,
            status: JobStatus::Running,
            pid: Some(pid),
            exit_code: None,
        }
    }

    /// Mark the job as completed
    pub fn mark_completed(&mut self, exit_code: i32) {
        self.finished_at = Some(Utc::now());
        self.status = if exit_code == 0 {
            JobStatus::Completed
        } else {
            JobStatus::Failed
        };
        self.exit_code = Some(exit_code);
        self.pid = None;
    }

    /// Mark the job as cancelled
    pub fn mark_cancelled(&mut self) {
        self.finished_at = Some(Utc::now());
        self.status = JobStatus::Cancelled;
        self.pid = None;
    }

    /// Check if the job is still running
    pub fn is_running(&self) -> bool {
        self.status == JobStatus::Running
    }

    /// Get the duration of the job
    pub fn duration(&self) -> Option<chrono::Duration> {
        let end = self.finished_at.unwrap_or_else(Utc::now);
        Some(end - self.started_at)
    }
}

/// Get the base directory for job storage
pub fn jobs_base_dir() -> PathBuf {
    directories::BaseDirs::new()
        .map(|dirs| dirs.cache_dir().join("pdb-cli").join("jobs"))
        .unwrap_or_else(|| PathBuf::from(".pdb-cli-jobs"))
}

/// Generate a unique job ID (8 character hex string)
pub fn generate_job_id() -> JobId {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    // Use lower 32 bits of timestamp XORed with process ID for uniqueness
    let pid = std::process::id();
    let hash = (timestamp as u32) ^ pid;
    format!("{:08x}", hash)
}

/// Filter for listing jobs
#[derive(Debug, Default)]
pub struct JobFilter {
    /// Only show running jobs
    pub running_only: bool,
    /// Show all jobs (including old ones)
    pub all: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_status_display() {
        assert_eq!(JobStatus::Running.to_string(), "running");
        assert_eq!(JobStatus::Completed.to_string(), "completed");
        assert_eq!(JobStatus::Failed.to_string(), "failed");
        assert_eq!(JobStatus::Cancelled.to_string(), "cancelled");
    }

    #[test]
    fn test_job_status_serialization() {
        let json = serde_json::to_string(&JobStatus::Running).unwrap();
        assert_eq!(json, "\"running\"");

        let status: JobStatus = serde_json::from_str("\"completed\"").unwrap();
        assert_eq!(status, JobStatus::Completed);
    }

    #[test]
    fn test_job_meta_new() {
        let meta = JobMeta::new(
            "abc12345".to_string(),
            "sync --type structures".to_string(),
            12345,
        );
        assert_eq!(meta.id, "abc12345");
        assert_eq!(meta.command, "sync --type structures");
        assert_eq!(meta.status, JobStatus::Running);
        assert_eq!(meta.pid, Some(12345));
        assert!(meta.is_running());
        assert!(meta.finished_at.is_none());
        assert!(meta.exit_code.is_none());
    }

    #[test]
    fn test_job_meta_mark_completed() {
        let mut meta = JobMeta::new("abc12345".to_string(), "test".to_string(), 1);

        meta.mark_completed(0);
        assert_eq!(meta.status, JobStatus::Completed);
        assert_eq!(meta.exit_code, Some(0));
        assert!(meta.finished_at.is_some());
        assert!(!meta.is_running());
        assert!(meta.pid.is_none());

        let mut meta2 = JobMeta::new("def67890".to_string(), "test".to_string(), 2);
        meta2.mark_completed(1);
        assert_eq!(meta2.status, JobStatus::Failed);
        assert_eq!(meta2.exit_code, Some(1));
    }

    #[test]
    fn test_job_meta_mark_cancelled() {
        let mut meta = JobMeta::new("abc12345".to_string(), "test".to_string(), 1);

        meta.mark_cancelled();
        assert_eq!(meta.status, JobStatus::Cancelled);
        assert!(meta.finished_at.is_some());
        assert!(!meta.is_running());
        assert!(meta.pid.is_none());
    }

    #[test]
    fn test_job_meta_serialization() {
        let meta = JobMeta::new(
            "abc12345".to_string(),
            "sync --type structures".to_string(),
            12345,
        );
        let json = serde_json::to_string_pretty(&meta).unwrap();

        assert!(json.contains("\"id\": \"abc12345\""));
        assert!(json.contains("\"status\": \"running\""));

        let parsed: JobMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, meta.id);
        assert_eq!(parsed.command, meta.command);
        assert_eq!(parsed.status, meta.status);
    }

    #[test]
    fn test_generate_job_id() {
        let id1 = generate_job_id();
        assert_eq!(id1.len(), 8);
        assert!(id1.chars().all(|c| c.is_ascii_hexdigit()));

        // Generate another ID (may be different due to timing)
        std::thread::sleep(std::time::Duration::from_millis(1));
        let id2 = generate_job_id();
        assert_eq!(id2.len(), 8);
    }

    #[test]
    fn test_jobs_base_dir() {
        let dir = jobs_base_dir();
        // Should end with "pdb-cli/jobs" or be a fallback
        assert!(
            dir.to_string_lossy().ends_with("pdb-cli/jobs")
                || dir.to_string_lossy().contains(".pdb-cli-jobs")
        );
    }

    #[test]
    fn test_job_duration() {
        let meta = JobMeta::new("abc12345".to_string(), "test".to_string(), 1);
        let duration = meta.duration();
        assert!(duration.is_some());
        // Duration should be very small (just created)
        assert!(duration.unwrap().num_seconds() < 1);
    }
}
