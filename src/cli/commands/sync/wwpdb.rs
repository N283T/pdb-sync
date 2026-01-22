//! Custom rsync sync handler.

use std::process::Stdio;
use std::time::{Duration, Instant};

use std::sync::Arc;
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;
use tokio::sync::Semaphore;

use crate::cli::args::SyncArgs;
use crate::config::schema::CustomRsyncConfig;
use crate::context::AppContext;
use crate::error::{PdbSyncError, Result};

use super::common::validate_subpath;

/// Result of a single sync operation.
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub name: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub error: Option<String>,
    pub duration: Duration,
}

impl SyncResult {
    pub fn success(name: String, duration: Duration) -> Self {
        Self {
            name,
            success: true,
            exit_code: Some(0),
            error: None,
            duration,
        }
    }

    pub fn failed(name: String, exit_code: Option<i32>, error: String, duration: Duration) -> Self {
        Self {
            name,
            success: false,
            exit_code,
            error: Some(error),
            duration,
        }
    }

    pub fn display(&self) -> String {
        if self.success {
            format!(
                "[{}] SUCCESS ({:.2}s)",
                self.name,
                self.duration.as_secs_f64()
            )
        } else {
            let exit_info = self
                .exit_code
                .map(|c| format!("exit code {}", c))
                .unwrap_or_else(|| "unknown".to_string());
            format!(
                "[{}] FAILED: {} ({:.2}s)",
                self.name,
                exit_info,
                self.duration.as_secs_f64()
            )
        }
    }
}

/// Run custom rsync sync by name.
pub async fn run_custom(name: String, args: SyncArgs, ctx: AppContext) -> Result<()> {
    let dest = args.dest.clone().unwrap_or_else(|| ctx.pdb_dir.clone());

    // Find custom config by name
    let custom_config = ctx
        .config
        .sync
        .custom
        .iter()
        .find(|c| c.name == name)
        .ok_or_else(|| PdbSyncError::Config {
            message: format!("Custom sync config '{}' not found", name),
            key: Some("custom".to_string()),
            source: None,
        })?;

    println!("Syncing custom config: {}", custom_config.name);
    if let Some(ref desc) = custom_config.description {
        println!("Description: {}", desc);
    }
    println!("URL: {}", custom_config.url);
    println!("Destination: {}/{}", dest.display(), custom_config.dest);

    // Validate destination path to prevent path traversal
    validate_subpath(&custom_config.dest)
        .map_err(|e| PdbSyncError::InvalidInput(format!("Invalid dest path: {}", e)))?;

    // Validate rsync URL format
    validate_rsync_url(&custom_config.url)?;

    // Merge config defaults with CLI overrides
    let config_flags = custom_config.to_rsync_flags();
    let cli_flags = args.to_rsync_flags();
    let flags = config_flags.merge_with_cli(&cli_flags);
    flags.validate()?;

    if flags.dry_run {
        println!("\nDry run - would execute:");
        let delete_flag = if flags.delete { " --delete" } else { "" };
        println!(
            "rsync -ah{} --info=progress2 {} {}",
            delete_flag,
            custom_config.url,
            dest.join(&custom_config.dest).display()
        );
        return Ok(());
    }

    // Build destination path
    let dest_path = dest.join(&custom_config.dest);

    // Create destination directory
    tokio::fs::create_dir_all(&dest_path).await?;

    // Build rsync command with base options and merged flags
    let mut cmd = Command::new("rsync");
    cmd.arg("-ah"); // Base archive options
    flags.apply_to_command(&mut cmd); // Apply merged user flags (includes --delete if set)
    cmd.arg("--info=progress2")
        .arg(&custom_config.url)
        .arg(&dest_path);

    // Execute rsync with real-time output
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    let status = cmd.spawn()?.wait().await?;

    if !status.success() {
        let delete_flag = if flags.delete { " --delete" } else { "" };
        return Err(PdbSyncError::Rsync {
            command: format!(
                "rsync -ah{} --info=progress2 {} {}",
                delete_flag,
                custom_config.url,
                dest_path.display()
            ),
            exit_code: status.code(),
            stderr: None,
        });
    }

    println!();
    println!("{}: completed", custom_config.name);

    Ok(())
}

/// Validate rsync URL format to prevent injection or unintended behavior.
pub fn validate_rsync_url(url: &str) -> Result<()> {
    // Check for command injection patterns
    let dangerous_chars = [';', '&', '|', '`', '$', '\n', '\r', '\t'];
    for ch in dangerous_chars {
        if url.contains(ch) {
            return Err(PdbSyncError::InvalidInput(format!(
                "Invalid character '{}' in rsync URL",
                ch
            )));
        }
    }

    // Reject shell metacharacters or embedded options
    if url.contains("--") || url.contains('\'') || url.contains('"') {
        return Err(PdbSyncError::InvalidInput(
            "Invalid characters in rsync URL".to_string(),
        ));
    }

    // Reject path traversal attempts in URL
    if url.contains("..") || url.contains('\\') {
        return Err(PdbSyncError::InvalidInput(
            "Path traversal not allowed in rsync URL".to_string(),
        ));
    }

    // Validate URL format: either host::module/path or rsync://host:port/module/path
    let is_standard_rsync = url.contains("::");
    let is_url_rsync = url.starts_with("rsync://");

    if !is_standard_rsync && !is_url_rsync {
        return Err(PdbSyncError::InvalidInput(
            "Invalid rsync URL format (expected host::module/path or rsync://host:port/module/path)".to_string(),
        ));
    }

    Ok(())
}

/// Run all custom rsync configs.
pub async fn run_custom_all(args: SyncArgs, ctx: AppContext) -> Result<()> {
    let custom_configs = ctx.config.sync.custom.clone();

    if custom_configs.is_empty() {
        println!("No custom sync configs found.");
        return Ok(());
    }

    // Validate parallel flag
    args.validate_parallel()?;

    println!("Syncing {} custom configs...", custom_configs.len());
    println!();

    let results = if args.parallel == 1 {
        run_sequential(&custom_configs, args, ctx).await?
    } else {
        let parallel = args.parallel;
        run_parallel(&custom_configs, args, ctx, parallel).await?
    };

    print_summary(&results);
    Ok(())
}

/// Run syncs sequentially (one at a time).
async fn run_sequential(
    configs: &[CustomRsyncConfig],
    args: SyncArgs,
    ctx: AppContext,
) -> Result<Vec<SyncResult>> {
    let mut results = Vec::new();

    for config in configs {
        let name = config.name.clone();
        let start = Instant::now();

        match run_custom(name.clone(), args.clone(), ctx.clone()).await {
            Ok(_) => {
                results.push(SyncResult::success(name, start.elapsed()));
            }
            Err(e) => {
                results.push(SyncResult::failed(
                    name,
                    None,
                    e.to_string(),
                    start.elapsed(),
                ));
            }
        }
    }

    Ok(results)
}

/// Run syncs in parallel with concurrency limit.
async fn run_parallel(
    configs: &[CustomRsyncConfig],
    args: SyncArgs,
    ctx: AppContext,
    parallel: usize,
) -> Result<Vec<SyncResult>> {
    let semaphore = Arc::new(Semaphore::new(parallel));
    let mut tasks = Vec::new();

    for config in configs {
        let semaphore = semaphore.clone();
        let name = config.name.clone();
        let args_clone = args.clone();
        let ctx_clone = ctx.clone();

        let task = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            let start = Instant::now();

            match run_sync_with_prefix(&name, args_clone, ctx_clone).await {
                Ok(_) => SyncResult::success(name, start.elapsed()),
                Err(e) => SyncResult::failed(name, None, e.to_string(), start.elapsed()),
            }
        });

        tasks.push(task);
    }

    // Collect results
    let mut results = Vec::new();
    for task in tasks {
        results.push(task.await?);
    }

    Ok(results)
}

/// Run a single sync with output prefixing.
async fn run_sync_with_prefix(name: &str, args: SyncArgs, ctx: AppContext) -> Result<()> {
    let dest = args.dest.clone().unwrap_or_else(|| ctx.pdb_dir.clone());

    // Find custom config by name
    let custom_config = ctx
        .config
        .sync
        .custom
        .iter()
        .find(|c| c.name == name)
        .ok_or_else(|| PdbSyncError::Config {
            message: format!("Custom sync config '{}' not found", name),
            key: Some("custom".to_string()),
            source: None,
        })?;

    println!("[{}] Starting sync...", name);
    println!("[{}] URL: {}", name, custom_config.url);
    println!(
        "[{}] Destination: {}/{}",
        name,
        dest.display(),
        custom_config.dest
    );

    // Validate destination path to prevent path traversal
    validate_subpath(&custom_config.dest)
        .map_err(|e| PdbSyncError::InvalidInput(format!("Invalid dest path: {}", e)))?;

    // Validate rsync URL format
    validate_rsync_url(&custom_config.url)?;

    // Merge config defaults with CLI overrides
    let config_flags = custom_config.to_rsync_flags();
    let cli_flags = args.to_rsync_flags();
    let flags = config_flags.merge_with_cli(&cli_flags);
    flags.validate()?;

    if flags.dry_run {
        println!("[{}] Dry run - would execute:", name);
        let delete_flag = if flags.delete { " --delete" } else { "" };
        println!(
            "[{}] rsync -ah{} --info=progress2 {} {}",
            name,
            delete_flag,
            custom_config.url,
            dest.join(&custom_config.dest).display()
        );
        return Ok(());
    }

    // Build destination path
    let dest_path = dest.join(&custom_config.dest);

    // Create destination directory
    tokio::fs::create_dir_all(&dest_path).await?;

    // Build rsync command with base options and merged flags
    let mut cmd = Command::new("rsync");
    cmd.arg("-ah"); // Base archive options
    flags.apply_to_command(&mut cmd); // Apply merged user flags (includes --delete if set)
    cmd.arg("--info=progress2")
        .arg(&custom_config.url)
        .arg(&dest_path);

    // Capture stdout and stderr for prefixing
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn()?;

    let stdout = child.stdout.take().expect("Failed to get stdout");
    let stderr = child.stderr.take().expect("Failed to get stderr");

    // Handle stdout
    let name_stdout = name.to_string();
    let stdout_task = tokio::spawn(async move {
        let reader = tokio::io::BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            println!("[{}] {}", name_stdout, line);
        }
    });

    // Handle stderr
    let name_stderr = name.to_string();
    let stderr_task = tokio::spawn(async move {
        let reader = tokio::io::BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            eprintln!("[{}] {}", name_stderr, line);
        }
    });

    // Wait for both output handlers
    stdout_task.await?;
    stderr_task.await?;

    let status = child.wait().await?;

    if !status.success() {
        let delete_flag = if flags.delete { " --delete" } else { "" };
        return Err(PdbSyncError::Rsync {
            command: format!(
                "rsync -ah{} --info=progress2 {} {}",
                delete_flag,
                custom_config.url,
                dest_path.display()
            ),
            exit_code: status.code(),
            stderr: None,
        });
    }

    println!("[{}] Completed", name);

    Ok(())
}

/// Print summary of sync results.
fn print_summary(results: &[SyncResult]) {
    println!();
    println!("=== Sync Summary ===");

    let total = results.len();
    let success = results.iter().filter(|r| r.success).count();
    let failed = total - success;

    println!(
        "Total: {} | Success: {} | Failed: {}",
        total, success, failed
    );

    if failed > 0 {
        println!();
        println!("Failed syncs:");
        for result in results.iter().filter(|r| !r.success) {
            println!("  - {}", result.display());
            if let Some(ref error) = result.error {
                println!("    Error: {}", error);
            }
        }
    } else {
        println!("All syncs completed successfully.");
    }

    // Timing info
    let total_duration: Duration = results.iter().map(|r| r.duration).sum();
    let avg_duration = if total > 0 {
        total_duration / total as u32
    } else {
        Duration::ZERO
    };

    println!(
        "Total time: {:.2}s | Average: {:.2}s",
        total_duration.as_secs_f64(),
        avg_duration.as_secs_f64()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_result_success() {
        let result = SyncResult::success("test-config".to_string(), Duration::from_secs(5));
        assert!(result.success);
        assert_eq!(result.name, "test-config");
        assert_eq!(result.exit_code, Some(0));
        assert!(result.error.is_none());
        assert_eq!(result.duration, Duration::from_secs(5));
    }

    #[test]
    fn test_sync_result_failed() {
        let result = SyncResult::failed(
            "test-config".to_string(),
            Some(1),
            "Connection failed".to_string(),
            Duration::from_secs(2),
        );
        assert!(!result.success);
        assert_eq!(result.name, "test-config");
        assert_eq!(result.exit_code, Some(1));
        assert_eq!(result.error, Some("Connection failed".to_string()));
        assert_eq!(result.duration, Duration::from_secs(2));
    }

    #[test]
    fn test_sync_result_display_success() {
        let result = SyncResult::success("test-config".to_string(), Duration::from_secs(5));
        let display = result.display();
        assert!(display.contains("[test-config]"));
        assert!(display.contains("SUCCESS"));
        assert!(display.contains("5.00"));
    }

    #[test]
    fn test_sync_result_display_failed() {
        let result = SyncResult::failed(
            "test-config".to_string(),
            Some(1),
            "Connection failed".to_string(),
            Duration::from_secs(2),
        );
        let display = result.display();
        assert!(display.contains("[test-config]"));
        assert!(display.contains("FAILED"));
        assert!(display.contains("exit code 1"));
        assert!(display.contains("2.00"));
    }

    #[test]
    fn test_validate_standard_rsync() {
        assert!(validate_rsync_url("rsync.example.com::module/path").is_ok());
        assert!(validate_rsync_url("data.pdbj.org::rsync/pub/emdb/").is_ok());
        assert!(validate_rsync_url("rsync.ebi.ac.uk::pdbe/data").is_ok());
    }

    #[test]
    fn test_validate_rsync_url_format() {
        assert!(validate_rsync_url("rsync://example.com:873/module/path").is_ok());
        assert!(validate_rsync_url("rsync://rsync.wwpdb.org:873/ftp_data/").is_ok());
    }

    #[test]
    fn test_validate_dangerous_chars() {
        assert!(validate_rsync_url("rsync://ex;ample.com::module").is_err());
        assert!(validate_rsync_url("rsync://example.com::mod`ule").is_err());
        assert!(validate_rsync_url("rsync://example.com&evil.com::module").is_err());
        assert!(validate_rsync_url("rsync://example.com::module|pipe").is_err());
        assert!(validate_rsync_url("rsync://example.com::mod$ule").is_err());
    }

    #[test]
    fn test_validate_shell_metachars() {
        assert!(validate_rsync_url("rsync://example.com::module--delete").is_err());
        assert!(validate_rsync_url("rsync://example.com::mod'ule").is_err());
        assert!(validate_rsync_url("rsync://example.com::mod\"ule").is_err());
    }

    #[test]
    fn test_validate_path_traversal() {
        assert!(validate_rsync_url("rsync://example.com::module/../etc").is_err());
        assert!(validate_rsync_url("rsync://example.com::module\\..\\etc").is_err());
        assert!(validate_rsync_url("rsync://example.com::module/../../etc").is_err());
    }

    #[test]
    fn test_validate_invalid_format() {
        assert!(validate_rsync_url("not-a-valid-url").is_err());
        assert!(validate_rsync_url("http://example.com").is_err());
        assert!(validate_rsync_url("ftp://example.com").is_err());
    }
}
