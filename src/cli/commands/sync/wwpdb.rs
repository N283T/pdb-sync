//! Custom rsync sync handler.

use std::process::Stdio;

use tokio::process::Command;

use crate::cli::args::SyncArgs;
use crate::context::AppContext;
use crate::error::{PdbSyncError, Result};

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
    if flags.delete {
        cmd.arg("--delete");
    }
    flags.apply_to_command(&mut cmd); // Apply merged user flags
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
fn validate_rsync_url(url: &str) -> Result<()> {
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
