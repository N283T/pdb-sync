use crate::cli::args::{JobsAction, JobsArgs};
use crate::error::Result;
use crate::jobs::{manager::JobManager, JobFilter, JobStatus};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::time::Duration;

/// Run the jobs command
pub async fn run_jobs(args: JobsArgs) -> Result<()> {
    let manager = JobManager::new();

    match args.action {
        Some(JobsAction::Status { job_id }) => {
            show_job_status(&manager, &job_id)?;
        }
        Some(JobsAction::Log { job_id, follow }) => {
            show_job_log(&manager, &job_id, follow).await?;
        }
        Some(JobsAction::Cancel { job_id }) => {
            cancel_job(&manager, &job_id)?;
        }
        Some(JobsAction::Clean { older_than }) => {
            clean_jobs(&manager, &older_than)?;
        }
        None => {
            // Default: list jobs
            list_jobs(&manager, args.all, args.running)?;
        }
    }

    Ok(())
}

/// List all jobs matching the filter
fn list_jobs(manager: &JobManager, all: bool, running_only: bool) -> Result<()> {
    let filter = JobFilter { all, running_only };

    let jobs = manager.list_jobs(&filter)?;

    if jobs.is_empty() {
        if running_only {
            println!("No running jobs.");
        } else {
            println!("No jobs found.");
        }
        return Ok(());
    }

    // Print header
    println!(
        "{:<10} {:<12} {:<20} COMMAND",
        "JOB ID", "STATUS", "STARTED"
    );
    println!("{}", "-".repeat(70));

    for job in jobs {
        let status_str = match job.status {
            JobStatus::Running => "\x1b[33mrunning\x1b[0m",
            JobStatus::Completed => "\x1b[32mcompleted\x1b[0m",
            JobStatus::Failed => "\x1b[31mfailed\x1b[0m",
            JobStatus::Cancelled => "\x1b[90mcancelled\x1b[0m",
        };

        let started = job.started_at.format("%Y-%m-%d %H:%M:%S").to_string();

        // Truncate command if too long
        let command = if job.command.len() > 30 {
            format!("{}...", &job.command[..27])
        } else {
            job.command.clone()
        };

        println!(
            "{:<10} {:<21} {:<20} {}",
            job.id, status_str, started, command
        );
    }

    Ok(())
}

/// Show detailed status of a specific job
fn show_job_status(manager: &JobManager, job_id: &str) -> Result<()> {
    let meta = manager.refresh_status(job_id)?;

    println!("Job ID:    {}", meta.id);
    println!("Command:   {}", meta.command);
    println!("Status:    {}", meta.status);
    println!(
        "Started:   {}",
        meta.started_at.format("%Y-%m-%d %H:%M:%S UTC")
    );

    if let Some(finished) = meta.finished_at {
        println!("Finished:  {}", finished.format("%Y-%m-%d %H:%M:%S UTC"));
    }

    if let Some(exit_code) = meta.exit_code {
        println!("Exit code: {}", exit_code);
    }

    if let Some(pid) = meta.pid {
        println!("PID:       {}", pid);
    }

    if let Some(duration) = meta.duration() {
        let secs = duration.num_seconds();
        if secs < 60 {
            println!("Duration:  {}s", secs);
        } else if secs < 3600 {
            println!("Duration:  {}m {}s", secs / 60, secs % 60);
        } else {
            println!(
                "Duration:  {}h {}m {}s",
                secs / 3600,
                (secs % 3600) / 60,
                secs % 60
            );
        }
    }

    // Show log file paths
    println!();
    println!("Logs:");
    println!("  stdout: {}", manager.stdout_path(job_id).display());
    println!("  stderr: {}", manager.stderr_path(job_id).display());

    Ok(())
}

/// Show logs for a job
async fn show_job_log(manager: &JobManager, job_id: &str, follow: bool) -> Result<()> {
    let stdout_path = manager.stdout_path(job_id);
    let stderr_path = manager.stderr_path(job_id);

    if !stdout_path.exists() {
        return Err(crate::error::PdbCliError::Job(format!(
            "Job not found: {}",
            job_id
        )));
    }

    if follow {
        // Follow mode: tail -f style
        follow_log(&stdout_path).await?;
    } else {
        // Print entire log
        if stdout_path.exists() {
            let content = std::fs::read_to_string(&stdout_path)?;
            if !content.is_empty() {
                print!("{}", content);
            }
        }

        // Also show stderr if it has content
        if stderr_path.exists() {
            let content = std::fs::read_to_string(&stderr_path)?;
            if !content.is_empty() {
                eprintln!("\n--- stderr ---");
                eprint!("{}", content);
            }
        }
    }

    Ok(())
}

/// Follow a log file (tail -f style)
async fn follow_log(path: &std::path::Path) -> Result<()> {
    use tokio::select;
    use tokio::time::sleep;

    let file = std::fs::File::open(path)?;
    let mut reader = BufReader::new(file);

    // Seek to end
    reader.seek(SeekFrom::End(0))?;

    // Create ctrl+c signal handler
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c);

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => {
                // No new data, wait a bit or exit on Ctrl+C
                select! {
                    _ = &mut ctrl_c => {
                        break;
                    }
                    _ = sleep(Duration::from_millis(100)) => {
                        // Continue reading
                    }
                }
            }
            Ok(_) => {
                print!("{}", line);
            }
            Err(e) => {
                return Err(crate::error::PdbCliError::Io(e));
            }
        }
    }

    Ok(())
}

/// Cancel a running job
fn cancel_job(manager: &JobManager, job_id: &str) -> Result<()> {
    manager.cancel_job(job_id)?;
    println!("Job {} cancelled.", job_id);
    Ok(())
}

/// Clean up old job directories
fn clean_jobs(manager: &JobManager, older_than: &str) -> Result<()> {
    let duration = parse_duration(older_than)?;
    let count = manager.clean_old_jobs(duration)?;

    if count == 0 {
        println!("No old jobs to clean.");
    } else {
        println!("Cleaned {} old job(s).", count);
    }

    Ok(())
}

/// Parse a duration string like "7d", "24h", "30m"
fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim();
    if s.is_empty() {
        return Err(crate::error::PdbCliError::InvalidInput(
            "Empty duration string".to_string(),
        ));
    }

    let (num_str, unit) = s.split_at(s.len() - 1);
    let num: u64 = num_str
        .parse()
        .map_err(|_| crate::error::PdbCliError::InvalidInput(format!("Invalid duration: {}", s)))?;

    let secs = match unit {
        "s" => num,
        "m" => num * 60,
        "h" => num * 3600,
        "d" => num * 86400,
        "w" => num * 604800,
        _ => {
            return Err(crate::error::PdbCliError::InvalidInput(format!(
                "Invalid duration unit '{}'. Use s, m, h, d, or w.",
                unit
            )))
        }
    };

    Ok(Duration::from_secs(secs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
        assert_eq!(parse_duration("5m").unwrap(), Duration::from_secs(300));
        assert_eq!(parse_duration("2h").unwrap(), Duration::from_secs(7200));
        assert_eq!(parse_duration("7d").unwrap(), Duration::from_secs(604800));
        assert_eq!(parse_duration("1w").unwrap(), Duration::from_secs(604800));
    }

    #[test]
    fn test_parse_duration_invalid() {
        assert!(parse_duration("").is_err());
        assert!(parse_duration("abc").is_err());
        assert!(parse_duration("5x").is_err());
    }
}
