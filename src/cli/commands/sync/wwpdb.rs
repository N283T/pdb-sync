//! Custom rsync sync handler.

use std::process::Stdio;

use tokio::process::Command;

use crate::cli::args::OutputFormat;
use crate::cli::args::SyncArgs;
use crate::context::AppContext;
use crate::error::{PdbSyncError, Result};
use crate::sync::{RsyncStats, SyncPlan};

use super::common::validate_subpath;

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

    // Build destination path (needed for plan mode and regular sync)
    let dest_path = dest.join(&custom_config.dest);

    // Plan mode: show what would change by running rsync with --dry-run --stats
    if args.is_plan_mode() {
        return run_plan_mode(custom_config, &dest_path, &flags, &args).await;
    }

    if flags.dry_run {
        println!("\nDry run - would execute:");
        let delete_flag = if flags.delete { " --delete" } else { "" };
        println!(
            "rsync -ah{} --info=progress2 {} {}",
            delete_flag,
            custom_config.url,
            dest_path.display()
        );
        return Ok(());
    }

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

/// Run plan mode: execute rsync with --dry-run --stats and display results.
async fn run_plan_mode(
    custom_config: &crate::config::schema::CustomRsyncConfig,
    dest_path: &std::path::Path,
    flags: &crate::sync::RsyncFlags,
    args: &SyncArgs,
) -> Result<()> {
    // Build rsync command with --dry-run --stats --itemize-changes
    let mut cmd = Command::new("rsync");
    cmd.arg("-ah")
        .arg("--dry-run")
        .arg("--stats")
        .arg("--itemize-changes");

    // Apply flags (but exclude --itemize-changes if already added)
    flags.apply_to_command(&mut cmd);
    cmd.arg("--info=progress2")
        .arg(&custom_config.url)
        .arg(dest_path);

    // Capture output instead of inheriting
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let output = cmd.output().await?;

    if !output.status.success() {
        return Err(PdbSyncError::Rsync {
            command: format!(
                "rsync -ah --dry-run --stats --itemize-changes {} {}",
                custom_config.url,
                dest_path.display()
            ),
            exit_code: output.status.code(),
            stderr: Some(String::from_utf8_lossy(&output.stderr).to_string()),
        });
    }

    // Parse stderr for stats
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stats = RsyncStats::parse(&stderr)?;

    let plan = SyncPlan {
        name: custom_config.name.clone(),
        url: custom_config.url.clone(),
        dest: custom_config.dest.clone(),
        has_deletions: stats.deleted > 0,
        stats,
    };

    // Display based on output format
    match args.output {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&plan)?);
        }
        OutputFormat::Text => {
            print_text_plan(&plan);
        }
        OutputFormat::Csv | OutputFormat::Ids => {
            return Err(PdbSyncError::InvalidInput(
                "Output format not supported for plan mode. Use --output text or --output json"
                    .to_string(),
            ));
        }
    }

    Ok(())
}

/// Print plan in human-readable text format.
fn print_text_plan(plan: &SyncPlan) {
    println!("Sync Plan: {}", plan.name);
    println!("  Source: {}", plan.url);
    println!("  Destination: {}", plan.dest);
    println!();
    println!("  Files: {}", plan.stats.num_files);
    println!("  Total size: {}", plan.stats.format_size());

    if plan.stats.has_changes() {
        println!();
        println!("  Changes:");
        if plan.stats.created > 0 {
            println!("    Created: {} files", plan.stats.created);
        }
        if plan.stats.deleted > 0 {
            println!("    Deleted: {} files", plan.stats.deleted);
        }
        if plan.stats.transferred > 0 {
            println!("    Transferred: {} files", plan.stats.transferred);
        }
    } else {
        println!("  No changes - already up to date");
    }
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

    // Plan mode: collect all plans and output together
    if args.is_plan_mode() {
        return run_all_plan_mode(&custom_configs, &args, &ctx).await;
    }

    println!("Syncing {} custom configs...", custom_configs.len());
    println!();

    let mut all_success = true;

    for custom_config in &custom_configs {
        let name = custom_config.name.clone();
        let result = run_custom(name.clone(), args.clone(), ctx.clone()).await;

        match result {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error syncing '{}': {}", name.clone(), e);
                all_success = false;
            }
        }
    }

    println!();
    if all_success {
        println!("All custom configs synced successfully.");
    } else {
        println!("Some custom configs failed to sync.");
    }

    Ok(())
}

/// Run plan mode for all custom configs.
async fn run_all_plan_mode(
    custom_configs: &[crate::config::schema::CustomRsyncConfig],
    args: &SyncArgs,
    ctx: &AppContext,
) -> Result<()> {
    let mut plans = Vec::new();

    for custom_config in custom_configs {
        let name = custom_config.name.clone();
        let dest = args.dest.clone().unwrap_or_else(|| ctx.pdb_dir.clone());
        let dest_path = dest.join(&custom_config.dest);

        // Validate destination path
        validate_subpath(&custom_config.dest)
            .map_err(|e| PdbSyncError::InvalidInput(format!("Invalid dest path: {}", e)))?;

        // Validate rsync URL
        validate_rsync_url(&custom_config.url)?;

        // Merge flags
        let config_flags = custom_config.to_rsync_flags();
        let cli_flags = args.to_rsync_flags();
        let flags = config_flags.merge_with_cli(&cli_flags);
        flags.validate()?;

        // Run plan mode for this config
        let mut cmd = Command::new("rsync");
        cmd.arg("-ah")
            .arg("--dry-run")
            .arg("--stats")
            .arg("--itemize-changes");
        flags.apply_to_command(&mut cmd);
        cmd.arg("--info=progress2")
            .arg(&custom_config.url)
            .arg(&dest_path);

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let output = cmd.output().await?;

        if !output.status.success() {
            eprintln!(
                "Error planning '{}': {}",
                name,
                String::from_utf8_lossy(&output.stderr)
            );
            continue;
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stats = RsyncStats::parse(&stderr)?;

        plans.push(SyncPlan {
            name: custom_config.name.clone(),
            url: custom_config.url.clone(),
            dest: custom_config.dest.clone(),
            has_deletions: stats.deleted > 0,
            stats,
        });
    }

    // Display based on output format
    match args.output {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&plans)?);
        }
        OutputFormat::Text => {
            for plan in &plans {
                print_text_plan(plan);
                println!();
            }
        }
        OutputFormat::Csv | OutputFormat::Ids => {
            return Err(PdbSyncError::InvalidInput(
                "Output format not supported for plan mode. Use --output text or --output json"
                    .to_string(),
            ));
        }
    }

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
}
