//! Custom rsync sync handler.

use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use tokio::process::Command;
use tokio::sync::Semaphore;

use crate::cli::args::SyncArgs;
use crate::context::AppContext;
use crate::error::{PdbSyncError, Result};
use crate::sync::{parse_rsync_stats, SyncPlan};

use super::common::validate_subpath;

/// Calculate retry delay in seconds.
/// If fixed_delay is Some, use that value.
/// Otherwise, use exponential backoff: 2^attempt capped at 30 seconds.
fn calculate_retry_delay(attempt: u32, fixed_delay: Option<u32>) -> u32 {
    if let Some(delay) = fixed_delay {
        delay
    } else {
        // Exponential backoff: 1, 2, 4, 8, 16, 30, 30...
        // Cap exponent at 5 to prevent overflow (2^5 = 32, then min(32, 30) = 30)
        let exponent = attempt.min(5);
        2_u32.pow(exponent).min(30)
    }
}

/// Execute an async operation with retry logic.
async fn execute_with_retry<F, Fut>(
    mut execute_fn: F,
    max_retries: u32,
    retry_delay: Option<u32>,
    name: &str,
) -> Result<()>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    let mut last_error = None;

    for attempt in 0..=max_retries {
        match execute_fn().await {
            Ok(()) => return Ok(()),
            Err(e) => {
                // Check if error is retriable and we haven't exhausted retries
                if e.is_retriable() && attempt < max_retries {
                    last_error = Some(e);
                    let delay = calculate_retry_delay(attempt, retry_delay);
                    eprintln!(
                        "[{}] Retry {}/{} after {}s...",
                        name,
                        attempt + 1,
                        max_retries,
                        delay
                    );
                    tokio::time::sleep(Duration::from_secs(delay as u64)).await;
                } else {
                    // Either not retriable or exhausted retries
                    return Err(e);
                }
            }
        }
    }

    // This should never be reached since we either return Ok or Err in the loop
    Err(last_error
        .unwrap_or_else(|| PdbSyncError::Job("Retry loop exhausted without error".to_string())))
}

/// Run custom rsync sync by name.
pub async fn run_custom(name: String, args: SyncArgs, ctx: AppContext) -> Result<()> {
    let dest = args.dest.clone().unwrap_or_else(|| ctx.pdb_dir.clone());

    // Find custom config by name
    let custom_config = ctx
        .config
        .sync
        .custom
        .get(&name)
        .ok_or_else(|| PdbSyncError::Config {
            message: format!("Custom sync config '{}' not found", name),
            key: Some("custom".to_string()),
            source: None,
        })?;

    println!("Syncing custom config: {}", name);
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
    let cli_overrides = args.to_rsync_overrides();
    let flags = config_flags.merge_with_overrides(&cli_overrides);
    flags.validate()?;

    // Build destination path
    let dest_path = dest.join(&custom_config.dest);

    // Handle plan mode - show what would change without executing
    if args.plan {
        println!("\nPlan mode - analyzing changes...");
        let mut cmd = Command::new("rsync");
        cmd.arg("-ah")
            .arg("--dry-run")
            .arg("--stats")
            .arg("--itemize-changes");
        flags.apply_to_command(&mut cmd);
        cmd.arg(&custom_config.url).arg(&dest_path);

        // Capture output for parsing
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::inherit());

        let output = cmd.output().await?;

        if !output.status.success() {
            let mut cmd_args = vec![
                "-ah".to_string(),
                "--dry-run".to_string(),
                "--stats".to_string(),
                "--itemize-changes".to_string(),
            ];
            cmd_args.extend(flags.to_args());
            return Err(PdbSyncError::Rsync {
                command: format!(
                    "rsync {} {} {}",
                    cmd_args.join(" "),
                    custom_config.url,
                    dest_path.display()
                ),
                exit_code: output.status.code(),
                stderr: None,
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stats = parse_rsync_stats(&stdout)?;

        let plan = SyncPlan {
            name: name.clone(),
            url: custom_config.url.clone(),
            dest: custom_config.dest.clone(),
            has_deletions: flags.delete,
            stats,
        };

        println!();
        plan.print();

        return Ok(());
    }

    // Handle dry-run mode - show command without executing
    if flags.dry_run {
        println!("\nDry run - would execute:");
        let mut cmd_args = vec!["-ah".to_string(), "--info=progress2".to_string()];
        cmd_args.extend(flags.to_args());
        println!(
            "rsync {} {} {}",
            cmd_args.join(" "),
            custom_config.url,
            dest_path.display()
        );
        return Ok(());
    }

    // Create destination directory
    tokio::fs::create_dir_all(&dest_path).await?;

    // Prepare rsync command arguments for execution
    let rsync_execute = || async {
        // Build rsync command with base options and merged flags
        let mut cmd = Command::new("rsync");
        cmd.arg("-ah"); // Base archive options
        flags.apply_to_command(&mut cmd); // Apply merged user flags (includes --delete if set)
        cmd.arg("--info=progress2")
            .arg(custom_config.url.clone())
            .arg(dest_path.clone());

        // Execute rsync with real-time output
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        let status = cmd.spawn()?.wait().await?;

        if !status.success() {
            let mut cmd_args = vec!["-ah".to_string(), "--info=progress2".to_string()];
            cmd_args.extend(flags.to_args());
            return Err(PdbSyncError::Rsync {
                command: format!(
                    "rsync {} {} {}",
                    cmd_args.join(" "),
                    custom_config.url,
                    dest_path.display()
                ),
                exit_code: status.code(),
                stderr: None,
            });
        }

        Ok(())
    };

    // Execute with retry if requested, otherwise execute directly
    if args.retry > 0 {
        execute_with_retry(rsync_execute, args.retry, args.retry_delay, &name).await?;
    } else {
        rsync_execute().await?;
    }

    println!();
    println!("{}: completed", name);

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

/// List all custom rsync configs.
pub fn list_custom(ctx: &AppContext) {
    let custom_configs = &ctx.config.sync.custom;
    if custom_configs.is_empty() {
        println!("No custom sync configs found.");
        return;
    }

    println!("Custom sync configs ({}):", custom_configs.len());
    println!();
    for (name, custom_config) in custom_configs {
        println!("Name: {}", name);
        if let Some(ref desc) = custom_config.description {
            println!("  Description: {}", desc);
        }
        println!("  URL: {}", custom_config.url);
        println!("  Dest: {}", custom_config.dest);
        println!();
    }
}

/// Run all custom rsync configs.
pub async fn run_custom_all(args: SyncArgs, ctx: AppContext) -> Result<()> {
    let custom_configs = ctx.config.sync.custom.clone();

    if custom_configs.is_empty() {
        println!("No custom sync configs found.");
        return Ok(());
    }

    println!("Syncing {} custom configs...", custom_configs.len());
    println!();

    // If parallel is set, run concurrent with semaphore
    if let Some(parallel_count) = args.parallel {
        return run_custom_all_parallel(&custom_configs, args, ctx, parallel_count).await;
    }

    // Otherwise run sequentially
    let mut all_success = true;

    // Sort keys for deterministic execution order
    let mut names: Vec<_> = custom_configs.keys().collect();
    names.sort();

    for name in names {
        let result = run_custom(name.clone(), args.clone(), ctx.clone()).await;

        match result {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error syncing '{}': {}", name.clone(), e);
                all_success = false;
                if args.fail_fast {
                    return Err(e);
                }
            }
        }
    }

    println!();
    if all_success {
        println!("All custom configs synced successfully.");
        Ok(())
    } else {
        println!("Some custom configs failed to sync.");
        Err(PdbSyncError::Job(
            "One or more custom sync configs failed".to_string(),
        ))
    }
}

/// Run all custom configs in parallel with semaphore-based concurrency limiting.
async fn run_custom_all_parallel(
    custom_configs: &std::collections::HashMap<String, crate::config::schema::CustomRsyncConfig>,
    args: SyncArgs,
    ctx: AppContext,
    parallel_count: usize,
) -> Result<()> {
    use tokio::task::JoinSet;

    // Create semaphore for concurrency limiting
    let semaphore = Arc::new(Semaphore::new(parallel_count));
    let fail_fast = Arc::new(std::sync::atomic::AtomicBool::new(false));

    // Use JoinSet for better task management
    let mut join_set = JoinSet::new();

    // Sort keys for deterministic execution order
    let mut names: Vec<_> = custom_configs.keys().collect();
    names.sort();

    for name in names {
        let name = name.clone();
        let args_clone = args.clone();
        let ctx_clone = ctx.clone();
        let semaphore_clone = semaphore.clone();
        let fail_fast_clone = fail_fast.clone();

        join_set.spawn(async move {
            // Capture fail_fast before moving args_clone
            let fail_fast_flag = args_clone.fail_fast;

            // Acquire semaphore permit FIRST (before checking fail_fast to avoid TOCTOU race)
            let permit = match semaphore_clone.acquire().await {
                Ok(p) => p,
                Err(_) => {
                    return (
                        name.clone(),
                        Err(PdbSyncError::Job(
                            "Semaphore closed unexpectedly".to_string(),
                        )),
                    );
                }
            };

            // Check if we should fail fast AFTER acquiring permit
            if fail_fast_clone.load(std::sync::atomic::Ordering::Relaxed) {
                return (
                    name.clone(),
                    Err(PdbSyncError::Job(
                        "Skipped due to previous failure".to_string(),
                    )),
                );
            }

            // Run the sync with output prefixing
            let result = run_custom_with_prefix(&name, args_clone, ctx_clone).await;

            // Update fail_fast if this failed
            if result.is_err() && fail_fast_flag {
                fail_fast_clone.store(true, std::sync::atomic::Ordering::Relaxed);
            }

            // Keep permit alive until here
            drop(permit);

            (name, result)
        });
    }

    // Collect and report results
    let mut all_success = true;
    while let Some(task_result) = join_set.join_next().await {
        match task_result {
            Ok((name, result)) => {
                if let Err(e) = result {
                    eprintln!("Error syncing '{}': {}", name, e);
                    all_success = false;
                }
            }
            Err(e) => {
                eprintln!("Task error: {}", e);
                all_success = false;
            }
        }
    }

    println!();
    if all_success {
        println!("All custom configs synced successfully.");
        Ok(())
    } else {
        println!("Some custom configs failed to sync.");
        Err(PdbSyncError::Job(
            "One or more custom sync configs failed".to_string(),
        ))
    }
}

/// Run a single custom sync with output prefixing for parallel execution.
async fn run_custom_with_prefix(name: &str, args: SyncArgs, ctx: AppContext) -> Result<()> {
    let dest = args.dest.clone().unwrap_or_else(|| ctx.pdb_dir.clone());

    // Find custom config by name
    let custom_config = ctx
        .config
        .sync
        .custom
        .get(name)
        .ok_or_else(|| PdbSyncError::Config {
            message: format!("Custom sync config '{}' not found", name),
            key: Some("custom".to_string()),
            source: None,
        })?;

    println!("[{}]", name);

    // Validate destination path to prevent path traversal
    super::common::validate_subpath(&custom_config.dest)
        .map_err(|e| PdbSyncError::InvalidInput(format!("Invalid dest path: {}", e)))?;

    // Validate rsync URL format
    validate_rsync_url(&custom_config.url)?;

    // Merge config defaults with CLI overrides
    let config_flags = custom_config.to_rsync_flags();
    let cli_overrides = args.to_rsync_overrides();
    let flags = config_flags.merge_with_overrides(&cli_overrides);
    flags.validate()?;

    // Handle plan mode
    if args.plan {
        println!("[{}] Plan mode - analyzing changes...", name);
        let mut cmd = Command::new("rsync");
        cmd.arg("-ah")
            .arg("--dry-run")
            .arg("--stats")
            .arg("--itemize-changes");
        flags.apply_to_command(&mut cmd);
        cmd.arg(&custom_config.url)
            .arg(dest.join(&custom_config.dest));

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::inherit());

        let output = cmd.output().await?;

        if !output.status.success() {
            return Err(PdbSyncError::Rsync {
                command: format!(
                    "rsync --dry-run {} {}",
                    custom_config.url,
                    dest.join(&custom_config.dest).display()
                ),
                exit_code: output.status.code(),
                stderr: None,
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stats = crate::sync::parse_rsync_stats(&stdout)?;

        let plan = crate::sync::SyncPlan {
            name: name.to_string(),
            url: custom_config.url.clone(),
            dest: custom_config.dest.clone(),
            has_deletions: flags.delete,
            stats,
        };

        println!();
        plan.print();
        return Ok(());
    }

    // Handle dry-run mode
    if flags.dry_run {
        println!("[{}] Dry run - would execute:", name);
        let mut cmd_args = vec!["-ah".to_string(), "--info=progress2".to_string()];
        cmd_args.extend(flags.to_args());
        println!(
            "rsync {} {} {}",
            cmd_args.join(" "),
            custom_config.url,
            dest.join(&custom_config.dest).display()
        );
        return Ok(());
    }

    // Build destination path
    let dest_path = dest.join(&custom_config.dest);

    // Create destination directory
    tokio::fs::create_dir_all(&dest_path).await?;

    // Prepare rsync command arguments for execution
    let rsync_execute = || async {
        // Build rsync command
        let mut cmd = Command::new("rsync");
        cmd.arg("-ah");
        flags.apply_to_command(&mut cmd);
        cmd.arg("--info=progress2")
            .arg(custom_config.url.clone())
            .arg(dest_path.clone());

        // Execute rsync with real-time output
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        let status = cmd.spawn()?.wait().await?;

        if !status.success() {
            return Err(PdbSyncError::Rsync {
                command: format!("rsync {} {}", custom_config.url, dest_path.display()),
                exit_code: status.code(),
                stderr: None,
            });
        }

        Ok(())
    };

    // Execute with retry if requested, otherwise execute directly
    if args.retry > 0 {
        execute_with_retry(rsync_execute, args.retry, args.retry_delay, name).await?;
    } else {
        rsync_execute().await?;
    }

    println!("[{}]: completed", name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_calculate_retry_delay_exponential() {
        // Exponential backoff: 1, 2, 4, 8, 16, 30, 30...
        assert_eq!(calculate_retry_delay(0, None), 1);
        assert_eq!(calculate_retry_delay(1, None), 2);
        assert_eq!(calculate_retry_delay(2, None), 4);
        assert_eq!(calculate_retry_delay(3, None), 8);
        assert_eq!(calculate_retry_delay(4, None), 16);
        assert_eq!(calculate_retry_delay(5, None), 30); // capped at 30
        assert_eq!(calculate_retry_delay(6, None), 30); // still 30
    }

    #[test]
    fn test_calculate_retry_delay_fixed() {
        // Fixed delay override
        assert_eq!(calculate_retry_delay(0, Some(5)), 5);
        assert_eq!(calculate_retry_delay(1, Some(5)), 5);
        assert_eq!(calculate_retry_delay(10, Some(5)), 5);
    }

    #[test]
    fn test_calculate_retry_delay_exponential_cap() {
        // Verify cap at 30 seconds
        assert_eq!(calculate_retry_delay(10, None), 30);
        assert_eq!(calculate_retry_delay(100, None), 30);
    }
}
