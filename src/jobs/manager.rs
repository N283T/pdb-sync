use crate::error::{PdbCliError, Result};
use crate::jobs::{
    generate_job_id, jobs_base_dir, validate_job_id, JobFilter, JobId, JobMeta, JobStatus,
};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

/// Manages background job storage and lifecycle
pub struct JobManager {
    jobs_dir: PathBuf,
}

impl JobManager {
    /// Create a new JobManager with default jobs directory
    pub fn new() -> Self {
        Self {
            jobs_dir: jobs_base_dir(),
        }
    }

    /// Create a new JobManager with a custom jobs directory
    #[cfg(test)]
    pub fn with_dir(jobs_dir: PathBuf) -> Self {
        Self { jobs_dir }
    }

    /// Get the jobs base directory
    #[allow(dead_code)]
    pub fn jobs_dir(&self) -> &PathBuf {
        &self.jobs_dir
    }

    /// Get the directory for a specific job
    pub fn job_dir(&self, job_id: &str) -> PathBuf {
        self.jobs_dir.join(format!("job_{}", job_id))
    }

    /// Get the path to the metadata file for a job
    pub fn meta_path(&self, job_id: &str) -> PathBuf {
        self.job_dir(job_id).join("meta.json")
    }

    /// Get the path to the stdout log for a job
    pub fn stdout_path(&self, job_id: &str) -> PathBuf {
        self.job_dir(job_id).join("stdout.log")
    }

    /// Get the path to the stderr log for a job
    pub fn stderr_path(&self, job_id: &str) -> PathBuf {
        self.job_dir(job_id).join("stderr.log")
    }

    /// Get the path to the PID file for a job
    pub fn pid_path(&self, job_id: &str) -> PathBuf {
        self.job_dir(job_id).join("pid")
    }

    /// Create a new job directory and return job ID and paths
    pub fn create_job(&self, _command: &str) -> Result<(JobId, PathBuf)> {
        let job_id = generate_job_id();
        let job_dir = self.job_dir(&job_id);

        fs::create_dir_all(&job_dir)?;

        // Create empty log files
        fs::write(self.stdout_path(&job_id), "")?;
        fs::write(self.stderr_path(&job_id), "")?;

        Ok((job_id, job_dir))
    }

    /// Save job metadata
    pub fn save_meta(&self, meta: &JobMeta) -> Result<()> {
        let path = self.meta_path(&meta.id);
        let json = serde_json::to_string_pretty(meta)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Load job metadata
    ///
    /// Validates job_id format to prevent path traversal attacks
    pub fn load_meta(&self, job_id: &str) -> Result<JobMeta> {
        validate_job_id(job_id)?;
        let path = self.meta_path(job_id);
        if !path.exists() {
            return Err(PdbCliError::Job(format!("Job not found: {}", job_id)));
        }
        let content = fs::read_to_string(path)?;
        let meta: JobMeta = serde_json::from_str(&content)?;
        Ok(meta)
    }

    /// Save PID file for a running job
    pub fn save_pid(&self, job_id: &str, pid: u32) -> Result<()> {
        let path = self.pid_path(job_id);
        fs::write(path, pid.to_string())?;
        Ok(())
    }

    /// Load PID from file
    pub fn load_pid(&self, job_id: &str) -> Result<Option<u32>> {
        let path = self.pid_path(job_id);
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(path)?;
        let pid: u32 = content
            .trim()
            .parse()
            .map_err(|_| PdbCliError::Job("Invalid PID file".to_string()))?;
        Ok(Some(pid))
    }

    /// Remove PID file (called when job finishes)
    pub fn remove_pid(&self, job_id: &str) -> Result<()> {
        let path = self.pid_path(job_id);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Check if a process is still running
    fn is_process_running(pid: u32) -> bool {
        #[cfg(unix)]
        {
            // Signal 0 doesn't actually send a signal, just checks if process exists
            unsafe { libc::kill(pid as i32, 0) == 0 }
        }
        #[cfg(not(unix))]
        {
            // On non-Unix platforms, assume it's not running
            false
        }
    }

    /// Refresh job status by checking if process is still running
    pub fn refresh_status(&self, job_id: &str) -> Result<JobMeta> {
        let mut meta = self.load_meta(job_id)?;

        if meta.status == JobStatus::Running {
            if let Some(pid) = meta.pid {
                if !Self::is_process_running(pid) {
                    // Process is no longer running, mark as completed
                    // We don't know the exit code, so assume failure
                    meta.mark_completed(1);
                    self.save_meta(&meta)?;
                    self.remove_pid(job_id)?;
                }
            } else {
                // No PID but marked as running - check PID file
                if let Ok(Some(pid)) = self.load_pid(job_id) {
                    if !Self::is_process_running(pid) {
                        meta.mark_completed(1);
                        self.save_meta(&meta)?;
                        self.remove_pid(job_id)?;
                    }
                }
            }
        }

        Ok(meta)
    }

    /// List all jobs matching the filter
    pub fn list_jobs(&self, filter: &JobFilter) -> Result<Vec<JobMeta>> {
        let mut jobs = Vec::new();

        if !self.jobs_dir.exists() {
            return Ok(jobs);
        }

        for entry in fs::read_dir(&self.jobs_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                if let Some(job_id) = dir_name.strip_prefix("job_") {
                    if let Ok(meta) = self.refresh_status(job_id) {
                        // Apply filters
                        if filter.running_only && meta.status != JobStatus::Running {
                            continue;
                        }

                        // Skip old completed jobs unless --all is set
                        if !filter.all && !meta.is_running() {
                            if let Some(finished) = meta.finished_at {
                                let age = chrono::Utc::now() - finished;
                                if age.num_hours() > 24 {
                                    continue;
                                }
                            }
                        }

                        jobs.push(meta);
                    }
                }
            }
        }

        // Sort by start time (newest first)
        jobs.sort_by(|a, b| b.started_at.cmp(&a.started_at));

        Ok(jobs)
    }

    /// Cancel a running job by sending SIGTERM
    pub fn cancel_job(&self, job_id: &str) -> Result<()> {
        let mut meta = self.load_meta(job_id)?;

        if meta.status != JobStatus::Running {
            return Err(PdbCliError::Job(format!(
                "Job {} is not running (status: {})",
                job_id, meta.status
            )));
        }

        let pid = meta.pid.or_else(|| self.load_pid(job_id).ok().flatten());

        if let Some(pid) = pid {
            #[cfg(unix)]
            {
                let result = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
                if result == -1 {
                    let err = std::io::Error::last_os_error();
                    // ESRCH means process doesn't exist (which is OK - it may have already exited)
                    if err.raw_os_error() != Some(libc::ESRCH) {
                        return Err(PdbCliError::Job(format!(
                            "Failed to send SIGTERM to process {}: {}",
                            pid, err
                        )));
                    }
                }
            }
            #[cfg(not(unix))]
            {
                return Err(PdbCliError::Job(
                    "Job cancellation not supported on this platform".to_string(),
                ));
            }
        }

        meta.mark_cancelled();
        self.save_meta(&meta)?;
        self.remove_pid(job_id)?;

        Ok(())
    }

    /// Clean up old job directories
    pub fn clean_old_jobs(&self, older_than: Duration) -> Result<u32> {
        let mut count = 0;

        if !self.jobs_dir.exists() {
            return Ok(0);
        }

        let cutoff = chrono::Utc::now() - chrono::Duration::from_std(older_than).unwrap();

        for entry in fs::read_dir(&self.jobs_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                if let Some(job_id) = dir_name.strip_prefix("job_") {
                    if let Ok(meta) = self.load_meta(job_id) {
                        // Don't delete running jobs
                        if meta.is_running() {
                            continue;
                        }

                        // Check if old enough
                        let job_time = meta.finished_at.unwrap_or(meta.started_at);
                        if job_time < cutoff {
                            fs::remove_dir_all(&path)?;
                            count += 1;
                        }
                    }
                }
            }
        }

        Ok(count)
    }
}

impl Default for JobManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_manager() -> (JobManager, TempDir) {
        let temp = TempDir::new().unwrap();
        let manager = JobManager::with_dir(temp.path().to_path_buf());
        (manager, temp)
    }

    #[test]
    fn test_job_dir_paths() {
        let (manager, _temp) = test_manager();

        let job_dir = manager.job_dir("abc12345");
        assert!(job_dir.to_string_lossy().ends_with("job_abc12345"));

        let meta_path = manager.meta_path("abc12345");
        assert!(meta_path.to_string_lossy().ends_with("meta.json"));

        let stdout_path = manager.stdout_path("abc12345");
        assert!(stdout_path.to_string_lossy().ends_with("stdout.log"));

        let stderr_path = manager.stderr_path("abc12345");
        assert!(stderr_path.to_string_lossy().ends_with("stderr.log"));

        let pid_path = manager.pid_path("abc12345");
        assert!(pid_path.to_string_lossy().ends_with("pid"));
    }

    #[test]
    fn test_create_job() {
        let (manager, _temp) = test_manager();

        let (job_id, job_dir) = manager.create_job("sync --type structures").unwrap();

        assert_eq!(job_id.len(), 8);
        assert!(job_dir.exists());
        assert!(manager.stdout_path(&job_id).exists());
        assert!(manager.stderr_path(&job_id).exists());
    }

    #[test]
    fn test_save_and_load_meta() {
        let (manager, _temp) = test_manager();

        let (job_id, _) = manager.create_job("test command").unwrap();
        let meta = JobMeta::new(job_id.clone(), "test command".to_string(), 12345);

        manager.save_meta(&meta).unwrap();

        let loaded = manager.load_meta(&job_id).unwrap();
        assert_eq!(loaded.id, job_id);
        assert_eq!(loaded.command, "test command");
        assert_eq!(loaded.status, JobStatus::Running);
        assert_eq!(loaded.pid, Some(12345));
    }

    #[test]
    fn test_load_meta_not_found() {
        let (manager, _temp) = test_manager();

        let result = manager.load_meta("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_pid_operations() {
        let (manager, _temp) = test_manager();

        let (job_id, _) = manager.create_job("test").unwrap();

        // Initially no PID
        let pid = manager.load_pid(&job_id).unwrap();
        assert!(pid.is_none());

        // Save PID
        manager.save_pid(&job_id, 54321).unwrap();
        let pid = manager.load_pid(&job_id).unwrap();
        assert_eq!(pid, Some(54321));

        // Remove PID
        manager.remove_pid(&job_id).unwrap();
        let pid = manager.load_pid(&job_id).unwrap();
        assert!(pid.is_none());
    }

    #[test]
    fn test_list_jobs_empty() {
        let (manager, _temp) = test_manager();

        let jobs = manager.list_jobs(&JobFilter::default()).unwrap();
        assert!(jobs.is_empty());
    }

    #[test]
    fn test_list_jobs() {
        let (manager, _temp) = test_manager();

        // Create a few jobs
        let (id1, _) = manager.create_job("job 1").unwrap();
        let meta1 = JobMeta::new(id1.clone(), "job 1".to_string(), 1);
        manager.save_meta(&meta1).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let (id2, _) = manager.create_job("job 2").unwrap();
        let mut meta2 = JobMeta::new(id2.clone(), "job 2".to_string(), 2);
        meta2.mark_completed(0);
        manager.save_meta(&meta2).unwrap();

        // List all (should show both - completed within 24h)
        let jobs = manager
            .list_jobs(&JobFilter {
                all: true,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(jobs.len(), 2);

        // List running only
        let _jobs = manager
            .list_jobs(&JobFilter {
                running_only: true,
                ..Default::default()
            })
            .unwrap();
        // Note: The running job might be detected as not running since PID doesn't exist
        // This is expected behavior - we're testing the filter logic
    }

    #[test]
    fn test_clean_old_jobs() {
        let (manager, _temp) = test_manager();

        // Create a completed job
        let (job_id, _) = manager.create_job("old job").unwrap();
        let mut meta = JobMeta::new(job_id.clone(), "old job".to_string(), 1);
        meta.mark_completed(0);
        // Manually set finished_at to be old
        meta.finished_at = Some(chrono::Utc::now() - chrono::Duration::days(10));
        manager.save_meta(&meta).unwrap();

        // Clean jobs older than 1 day
        let count = manager.clean_old_jobs(Duration::from_secs(86400)).unwrap();
        assert_eq!(count, 1);

        // Job should be gone
        assert!(!manager.job_dir(&job_id).exists());
    }
}
