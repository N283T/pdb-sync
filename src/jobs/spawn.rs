use crate::error::{PdbCliError, Result};
use crate::jobs::{manager::JobManager, JobId, JobMeta};
use std::fs::File;
use std::path::PathBuf;
use std::process::Command;

/// Spawn a background job by re-executing the current binary
///
/// This function:
/// 1. Creates a job directory with unique ID
/// 2. Removes --bg from args and adds --_job-id
/// 3. Spawns a new detached process
/// 4. Returns immediately with job info
pub fn spawn_background(args: &[String]) -> Result<(JobId, PathBuf)> {
    let manager = JobManager::new();

    // Build command string for display (without --bg)
    let command_display = build_command_display(args);

    // Create job directory
    let (job_id, job_dir) = manager.create_job(&command_display)?;

    // Get current executable path
    let exe = std::env::current_exe()
        .map_err(|e| PdbCliError::Job(format!("Failed to get current executable: {}", e)))?;

    // Build new args: remove --bg, add --_job-id
    let new_args = build_background_args(args, &job_id);

    // Open log files
    let stdout_file = File::create(manager.stdout_path(&job_id))?;
    let stderr_file = File::create(manager.stderr_path(&job_id))?;

    // Spawn the background process
    let child = spawn_detached(&exe, &new_args, stdout_file, stderr_file)?;

    let pid = child.id();

    // Save metadata and PID
    let meta = JobMeta::new(job_id.clone(), command_display, pid);
    manager.save_meta(&meta)?;
    manager.save_pid(&job_id, pid)?;

    Ok((job_id, job_dir))
}

/// Build command display string (for logging/display purposes)
fn build_command_display(args: &[String]) -> String {
    args.iter()
        .filter(|arg| *arg != "--bg")
        .cloned()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Build arguments for background process
fn build_background_args(args: &[String], job_id: &str) -> Vec<String> {
    let mut new_args: Vec<String> = args.iter().filter(|arg| *arg != "--bg").cloned().collect();

    // Add job ID as first argument
    new_args.insert(0, format!("--_job-id={}", job_id));

    new_args
}

/// Spawn a detached process (Unix-specific with process group)
#[cfg(unix)]
fn spawn_detached(
    exe: &std::path::Path,
    args: &[String],
    stdout: File,
    stderr: File,
) -> Result<std::process::Child> {
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;

    let child = unsafe {
        Command::new(exe)
            .args(args)
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(stderr)
            // Create new process group so it survives terminal close
            .pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            })
            .spawn()
            .map_err(|e| PdbCliError::Job(format!("Failed to spawn background process: {}", e)))?
    };

    Ok(child)
}

/// Spawn a detached process (non-Unix fallback)
#[cfg(not(unix))]
fn spawn_detached(
    exe: &std::path::Path,
    args: &[String],
    stdout: File,
    stderr: File,
) -> Result<std::process::Child> {
    use std::process::Stdio;

    let child = Command::new(exe)
        .args(args)
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(stderr)
        .spawn()
        .map_err(|e| PdbCliError::Job(format!("Failed to spawn background process: {}", e)))?;

    Ok(child)
}

/// Finalize a job when it completes
///
/// Called by the background process when it finishes
pub fn finalize_job(job_id: &str, exit_code: i32) -> Result<()> {
    let manager = JobManager::new();

    let mut meta = manager.load_meta(job_id)?;
    meta.mark_completed(exit_code);
    manager.save_meta(&meta)?;
    manager.remove_pid(job_id)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_command_display() {
        let args = vec![
            "sync".to_string(),
            "--type".to_string(),
            "structures".to_string(),
            "--bg".to_string(),
            "--mirror".to_string(),
            "pdbj".to_string(),
        ];

        let display = build_command_display(&args);
        assert_eq!(display, "sync --type structures --mirror pdbj");
        assert!(!display.contains("--bg"));
    }

    #[test]
    fn test_build_background_args() {
        let args = vec![
            "sync".to_string(),
            "--type".to_string(),
            "structures".to_string(),
            "--bg".to_string(),
        ];

        let new_args = build_background_args(&args, "abc12345");

        assert_eq!(new_args[0], "--_job-id=abc12345");
        assert!(new_args.contains(&"sync".to_string()));
        assert!(new_args.contains(&"--type".to_string()));
        assert!(!new_args.contains(&"--bg".to_string()));
    }

    #[test]
    fn test_build_background_args_preserves_order() {
        let args = vec![
            "download".to_string(),
            "1abc".to_string(),
            "--bg".to_string(),
            "--mirror".to_string(),
            "rcsb".to_string(),
        ];

        let new_args = build_background_args(&args, "test1234");

        // Job ID should be first
        assert!(new_args[0].starts_with("--_job-id="));
        // Command should follow
        assert_eq!(new_args[1], "download");
        // --bg should be removed
        assert!(!new_args.contains(&"--bg".to_string()));
    }
}
