//! Common rsync flag definitions shared between CLI and config.

use crate::error::{PdbSyncError, Result};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

/// Common rsync flags shared between configuration and CLI arguments.
///
/// This struct provides a curated subset of commonly-used rsync options
/// for PDB syncing, with validation and merge capabilities.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RsyncFlags {
    // === Existing flags (for backward compatibility) ===
    /// Delete files that don't exist on the remote
    pub delete: bool,
    /// Bandwidth limit in KB/s
    pub bwlimit: Option<u32>,
    /// Dry run - don't make any changes
    pub dry_run: bool,

    // === Compression & Transfer Optimization ===
    /// Compress data during transfer (-z)
    pub compress: bool,
    /// Use checksum for file comparison (-c) instead of mod time/size
    pub checksum: bool,
    /// Keep partially transferred files (--partial)
    pub partial: bool,
    /// Directory for partial files (--partial-dir=DIR)
    pub partial_dir: Option<String>,

    // === Size Limits ===
    /// Maximum file size to transfer (--max-size=SIZE)
    pub max_size: Option<String>,
    /// Minimum file size to transfer (--min-size=SIZE)
    pub min_size: Option<String>,

    // === Timeouts ===
    /// I/O timeout in seconds (--timeout=SECONDS)
    pub timeout: Option<u32>,
    /// Connection timeout in seconds (--contimeout=SECONDS)
    pub contimeout: Option<u32>,

    // === Backup ===
    /// Create backups (--backup)
    pub backup: bool,
    /// Backup directory (--backup-dir=DIR)
    pub backup_dir: Option<String>,

    // === Permissions ===
    /// Change permission flags (--chmod=CHMOD)
    pub chmod: Option<String>,

    // === Filtering ===
    /// Exclude patterns (--exclude)
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Include patterns (--include)
    #[serde(default)]
    pub include: Vec<String>,
    /// File with exclude patterns (--exclude-from=FILE)
    pub exclude_from: Option<String>,
    /// File with include patterns (--include-from=FILE)
    pub include_from: Option<String>,

    // === Verbosity ===
    /// Verbose output (-v, --verbose)
    pub verbose: bool,
    /// Quiet mode (-q, --quiet)
    pub quiet: bool,
    /// Itemize changes (-i, --itemize-changes)
    pub itemize_changes: bool,
}

impl RsyncFlags {
    /// Validate the rsync flags.
    ///
    /// Ensures that flag combinations are valid and values are appropriate.
    pub fn validate(&self) -> Result<()> {
        // Validate size format
        if let Some(ref max_size) = self.max_size {
            validate_size_string(max_size)?;
        }
        if let Some(ref min_size) = self.min_size {
            validate_size_string(min_size)?;
        }

        // Validate that partial_dir implies partial
        if self.partial_dir.is_some() && !self.partial {
            return Err(PdbSyncError::InvalidInput(
                "partial_dir requires partial=true".to_string(),
            ));
        }

        // Validate that backup_dir implies backup
        if self.backup_dir.is_some() && !self.backup {
            return Err(PdbSyncError::InvalidInput(
                "backup_dir requires backup=true".to_string(),
            ));
        }

        // Validate conflicting verbosity options
        if self.verbose && self.quiet {
            return Err(PdbSyncError::InvalidInput(
                "verbose and quiet are mutually exclusive".to_string(),
            ));
        }

        // Validate chmod format (basic validation)
        if let Some(ref chmod) = self.chmod {
            validate_chmod_string(chmod)?;
        }

        Ok(())
    }

    /// Merge CLI flags over config defaults.
    ///
    /// CLI flags take priority over config defaults. For boolean flags, the
    /// CLI value is used directly (can't distinguish between unspecified and
    /// explicit false from clap). Use `--no-*` negation flags to explicitly
    /// disable a config default.
    pub fn merge_with_cli(&self, cli: &RsyncFlags) -> RsyncFlags {
        RsyncFlags {
            // Boolean flags: CLI value takes priority
            // Note: clap bool args can't distinguish "not set" from "false"
            // To override config true to false, use --no-flag if available
            delete: cli.delete,
            compress: cli.compress,
            checksum: cli.checksum,
            partial: cli.partial,
            backup: cli.backup,
            itemize_changes: cli.itemize_changes,
            verbose: cli.verbose,
            quiet: cli.quiet,

            // dry_run is additive (true from either source means true)
            dry_run: cli.dry_run || self.dry_run,

            // Option types: CLI Some value overrides, None falls back to config
            bwlimit: cli.bwlimit.or(self.bwlimit),
            partial_dir: cli.partial_dir.clone().or_else(|| self.partial_dir.clone()),
            max_size: cli.max_size.clone().or_else(|| self.max_size.clone()),
            min_size: cli.min_size.clone().or_else(|| self.min_size.clone()),
            timeout: cli.timeout.or(self.timeout),
            contimeout: cli.contimeout.or(self.contimeout),
            backup_dir: cli.backup_dir.clone().or_else(|| self.backup_dir.clone()),
            chmod: cli.chmod.clone().or_else(|| self.chmod.clone()),

            // Vec types: CLI extends config (or overrides if non-empty)
            exclude: if !cli.exclude.is_empty() {
                cli.exclude.clone()
            } else {
                self.exclude.clone()
            },
            include: if !cli.include.is_empty() {
                cli.include.clone()
            } else {
                self.include.clone()
            },
            exclude_from: cli
                .exclude_from
                .clone()
                .or_else(|| self.exclude_from.clone()),
            include_from: cli
                .include_from
                .clone()
                .or_else(|| self.include_from.clone()),
        }
    }

    /// Apply these flags to a Command builder.
    ///
    /// Adds appropriate rsync command-line arguments based on the flags.
    pub fn apply_to_command(&self, cmd: &mut Command) {
        // Basic flags
        if self.delete {
            cmd.arg("--delete");
        }

        if let Some(limit) = self.bwlimit {
            if limit > 0 {
                cmd.arg(format!("--bwlimit={}", limit));
            }
        }

        if self.dry_run {
            cmd.arg("--dry-run");
        }

        // Compression & optimization
        if self.compress {
            cmd.arg("-z"); // --compress
        }

        if self.checksum {
            cmd.arg("-c"); // --checksum
        }

        if self.partial {
            cmd.arg("--partial");
        }

        if let Some(ref dir) = self.partial_dir {
            cmd.arg(format!("--partial-dir={}", dir));
        }

        // Size limits
        if let Some(ref size) = self.max_size {
            cmd.arg(format!("--max-size={}", size));
        }

        if let Some(ref size) = self.min_size {
            cmd.arg(format!("--min-size={}", size));
        }

        // Timeouts
        if let Some(timeout) = self.timeout {
            cmd.arg(format!("--timeout={}", timeout));
        }

        if let Some(timeout) = self.contimeout {
            cmd.arg(format!("--contimeout={}", timeout));
        }

        // Backup
        if self.backup {
            cmd.arg("--backup");
        }

        if let Some(ref dir) = self.backup_dir {
            cmd.arg(format!("--backup-dir={}", dir));
        }

        // Permissions
        if let Some(ref chmod) = self.chmod {
            cmd.arg(format!("--chmod={}", chmod));
        }

        // Filtering
        for pattern in &self.exclude {
            cmd.arg(format!("--exclude={}", pattern));
        }

        for pattern in &self.include {
            cmd.arg(format!("--include={}", pattern));
        }

        if let Some(ref file) = self.exclude_from {
            cmd.arg(format!("--exclude-from={}", file));
        }

        if let Some(ref file) = self.include_from {
            cmd.arg(format!("--include-from={}", file));
        }

        // Verbosity
        if self.verbose {
            cmd.arg("--verbose");
        }

        if self.quiet {
            cmd.arg("--quiet");
        }

        if self.itemize_changes {
            cmd.arg("--itemize-changes");
        }
    }

    /// Convert these flags to a vector of command-line argument strings.
    ///
    /// This is used for displaying the command that would be run.
    #[allow(dead_code)]
    pub fn to_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        // Basic flags
        if self.delete {
            args.push("--delete".to_string());
        }

        if let Some(limit) = self.bwlimit {
            if limit > 0 {
                args.push(format!("--bwlimit={}", limit));
            }
        }

        if self.dry_run {
            args.push("--dry-run".to_string());
        }

        // Compression & optimization
        if self.compress {
            args.push("-z".to_string());
        }

        if self.checksum {
            args.push("-c".to_string());
        }

        if self.partial {
            args.push("--partial".to_string());
        }

        if let Some(ref dir) = self.partial_dir {
            args.push(format!("--partial-dir={}", dir));
        }

        // Size limits
        if let Some(ref size) = self.max_size {
            args.push(format!("--max-size={}", size));
        }

        if let Some(ref size) = self.min_size {
            args.push(format!("--min-size={}", size));
        }

        // Timeouts
        if let Some(timeout) = self.timeout {
            args.push(format!("--timeout={}", timeout));
        }

        if let Some(timeout) = self.contimeout {
            args.push(format!("--contimeout={}", timeout));
        }

        // Backup
        if self.backup {
            args.push("--backup".to_string());
        }

        if let Some(ref dir) = self.backup_dir {
            args.push(format!("--backup-dir={}", dir));
        }

        // Permissions
        if let Some(ref chmod) = self.chmod {
            args.push(format!("--chmod={}", chmod));
        }

        // Filtering
        for pattern in &self.exclude {
            args.push(format!("--exclude={}", pattern));
        }

        for pattern in &self.include {
            args.push(format!("--include={}", pattern));
        }

        if let Some(ref file) = self.exclude_from {
            args.push(format!("--exclude-from={}", file));
        }

        if let Some(ref file) = self.include_from {
            args.push(format!("--include-from={}", file));
        }

        // Verbosity
        if self.verbose {
            args.push("--verbose".to_string());
        }

        if self.quiet {
            args.push("--quiet".to_string());
        }

        if self.itemize_changes {
            args.push("--itemize-changes".to_string());
        }

        args
    }
}

/// Validate a size string for rsync (--max-size, --min-size).
///
/// Valid formats: "100", "100K", "100M", "1G", etc.
pub fn validate_size_string(s: &str) -> Result<()> {
    let s = s.trim();

    if s.is_empty() {
        return Err(PdbSyncError::InvalidInput(
            "Size string is empty".to_string(),
        ));
    }

    // Check if string ends with valid suffix (optional)
    let suffix = s.chars().last().unwrap();
    if suffix.is_ascii_alphabetic() {
        // Validate suffix
        if !matches!(suffix.to_ascii_uppercase(), 'K' | 'M' | 'G' | 'T' | 'P') {
            return Err(PdbSyncError::InvalidInput(format!(
                "Invalid size suffix: {}. Valid: K, M, G, T, P",
                suffix
            )));
        }
    }

    // Check that the prefix is numeric
    let numeric_part = if suffix.is_ascii_alphabetic() {
        &s[..s.len() - 1]
    } else {
        s
    };

    numeric_part
        .parse::<u64>()
        .map_err(|_| PdbSyncError::InvalidInput(format!("Invalid size number: {}", s)))?;

    Ok(())
}

/// Validate chmod string format.
///
/// Basic validation - checks for common chmod patterns like "D755", "F644", etc.
pub fn validate_chmod_string(s: &str) -> Result<()> {
    let s = s.trim();

    if s.is_empty() {
        return Err(PdbSyncError::InvalidInput(
            "chmod string is empty".to_string(),
        ));
    }

    // Reject dangerous characters
    for ch in s.chars() {
        if ch == ';' || ch == '&' || ch == '|' || ch == '$' || ch == '`' || ch == '\\' {
            return Err(PdbSyncError::InvalidInput(format!(
                "Invalid character in chmod string: '{}'",
                ch
            )));
        }
    }

    // Check for standard chmod format (e.g., "D755,F644", "a+rX", "u+w,go-r")
    // This is a basic validation - rsync accepts more complex formats
    for part in s.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        // Check for valid chmod patterns
        // Pattern 1: D/F + octal (e.g., D755, F644)
        // Pattern 2: u/g/o/a + rwx (e.g., u+rwx,go-r)
        // Pattern 3: Just octal (e.g., 755)

        let has_df_prefix = part.starts_with('D') || part.starts_with('F');
        let has_ugo_prefix = part.starts_with('u')
            || part.starts_with('g')
            || part.starts_with('o')
            || part.starts_with('a');

        // After prefix, should have +-= and/or octal/symbolic modes
        let valid = if has_df_prefix {
            // D755, F644 - rest should be octal
            part.len() > 1 && part[1..].chars().all(|c| c.is_ascii_digit())
        } else if has_ugo_prefix {
            // u+rwx, go-rx - should contain operator and mode chars
            let has_operator = part.contains('+') || part.contains('-') || part.contains('=');
            let has_mode = part.contains('r')
                || part.contains('w')
                || part.contains('x')
                || part.contains('X')
                || part.contains('s')
                || part.contains('t');
            has_operator && has_mode
        } else {
            // Just octal like "755"
            part.chars().all(|c| c.is_ascii_digit())
        };

        if !valid {
            return Err(PdbSyncError::InvalidInput(format!(
                "Invalid chmod format: {}",
                part
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_flags() {
        let flags = RsyncFlags::default();
        assert!(!flags.delete);
        assert!(flags.bwlimit.is_none());
        assert!(!flags.dry_run);
        assert!(!flags.compress);
        assert!(!flags.checksum);
        assert!(flags.exclude.is_empty());
    }

    #[test]
    fn test_validate_size_strings() {
        // Valid sizes
        assert!(validate_size_string("100").is_ok());
        assert!(validate_size_string("100K").is_ok());
        assert!(validate_size_string("100M").is_ok());
        assert!(validate_size_string("1G").is_ok());
        assert!(validate_size_string("2T").is_ok());
        assert!(validate_size_string("1P").is_ok());
        assert!(validate_size_string("100k").is_ok()); // lowercase
        assert!(validate_size_string("100m").is_ok());

        // Invalid sizes
        assert!(validate_size_string("").is_err());
        assert!(validate_size_string("100X").is_err());
        assert!(validate_size_string("abc").is_err());
        assert!(validate_size_string("K100").is_err());
    }

    #[test]
    fn test_validate_chmod_strings() {
        // Valid chmod
        assert!(validate_chmod_string("D755").is_ok());
        assert!(validate_chmod_string("F644").is_ok());
        assert!(validate_chmod_string("D755,F644").is_ok());
        assert!(validate_chmod_string("u+rwx").is_ok());
        assert!(validate_chmod_string("go-rx").is_ok());
        assert!(validate_chmod_string("a+rX").is_ok());
        assert!(validate_chmod_string("u+w,go-r").is_ok());

        // Invalid chmod
        assert!(validate_chmod_string("").is_err());
        assert!(validate_chmod_string("X755").is_err()); // D or F prefix
        assert!(validate_chmod_string("invalid").is_err());
    }

    #[test]
    fn test_validate_partial_dir_requires_partial() {
        let flags = RsyncFlags {
            partial_dir: Some(".partial".to_string()),
            ..Default::default()
        };
        assert!(flags.validate().is_err());

        let flags = RsyncFlags {
            partial: true,
            partial_dir: Some(".partial".to_string()),
            ..Default::default()
        };
        assert!(flags.validate().is_ok());
    }

    #[test]
    fn test_validate_backup_dir_requires_backup() {
        let flags = RsyncFlags {
            backup_dir: Some(".backup".to_string()),
            ..Default::default()
        };
        assert!(flags.validate().is_err());

        let flags = RsyncFlags {
            backup: true,
            backup_dir: Some(".backup".to_string()),
            ..Default::default()
        };
        assert!(flags.validate().is_ok());
    }

    #[test]
    fn test_validate_verbose_quiet_conflict() {
        let flags = RsyncFlags {
            verbose: true,
            quiet: true,
            ..Default::default()
        };
        assert!(flags.validate().is_err());
    }

    #[test]
    fn test_merge_with_cli() {
        let config = RsyncFlags {
            delete: false,
            compress: true,
            bwlimit: Some(1000),
            exclude: vec!["*.tmp".to_string()],
            ..Default::default()
        };

        let cli = RsyncFlags {
            delete: true,
            compress: false,
            bwlimit: None,
            max_size: Some("1G".to_string()),
            ..Default::default()
        };

        let merged = config.merge_with_cli(&cli);

        assert!(merged.delete); // CLI override
        assert!(!merged.compress); // CLI override (false explicitly set)
        assert_eq!(merged.bwlimit, Some(1000)); // Config preserved
        assert_eq!(merged.max_size, Some("1G".to_string())); // CLI set
        assert_eq!(merged.exclude, vec!["*.tmp".to_string()]); // Config preserved
    }

    #[test]
    fn test_merge_with_cli_empty_vecs() {
        let config = RsyncFlags {
            exclude: vec!["*.tmp".to_string(), "*.log".to_string()],
            include: vec!["*.cif".to_string()],
            ..Default::default()
        };

        let cli = RsyncFlags {
            exclude: vec!["*.bak".to_string()],
            include: vec![],
            ..Default::default()
        };

        let merged = config.merge_with_cli(&cli);

        // CLI overrides non-empty vec
        assert_eq!(merged.exclude, vec!["*.bak".to_string()]);
        // Config preserved when CLI vec is empty
        assert_eq!(merged.include, vec!["*.cif".to_string()]);
    }

    #[test]
    fn test_apply_to_command() {
        let flags = RsyncFlags {
            delete: true,
            compress: true,
            bwlimit: Some(500),
            max_size: Some("1G".to_string()),
            exclude: vec!["*.tmp".to_string()],
            ..Default::default()
        };

        // Test to_args() method
        let args = flags.to_args();

        assert!(args.contains(&"--delete".to_string()));
        assert!(args.contains(&"-z".to_string()));
        assert!(args.contains(&"--bwlimit=500".to_string()));
        assert!(args.contains(&"--max-size=1G".to_string()));
        assert!(args.contains(&"--exclude=*.tmp".to_string()));
    }
}
