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

    // === File Comparison Methods ===
    /// Use checksum for file comparison (-c) instead of mod time/size
    pub checksum: bool,
    /// Compare by size only, ignore timestamps (--size-only)
    pub size_only: bool,
    /// Always transfer files, ignoring timestamps (--ignore-times)
    pub ignore_times: bool,
    /// Timestamp tolerance in seconds (--modify-window=SECONDS)
    pub modify_window: Option<u32>,

    // === Partial Transfer ===
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

/// CLI overrides for rsync flags.
///
/// Uses `Option` to distinguish "not provided" from "explicitly set".
#[derive(Debug, Clone, Default)]
pub struct RsyncFlagOverrides {
    pub delete: Option<bool>,
    pub bwlimit: Option<u32>,
    pub dry_run: Option<bool>,

    pub compress: Option<bool>,
    pub checksum: Option<bool>,
    pub size_only: Option<bool>,
    pub ignore_times: Option<bool>,
    pub modify_window: Option<u32>,
    pub partial: Option<bool>,
    pub partial_dir: Option<String>,

    pub max_size: Option<String>,
    pub min_size: Option<String>,

    pub timeout: Option<u32>,
    pub contimeout: Option<u32>,

    pub backup: Option<bool>,
    pub backup_dir: Option<String>,

    pub chmod: Option<String>,

    pub exclude: Option<Vec<String>>,
    pub include: Option<Vec<String>>,
    pub exclude_from: Option<String>,
    pub include_from: Option<String>,

    pub verbose: Option<bool>,
    pub quiet: Option<bool>,
    pub itemize_changes: Option<bool>,
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

        // Validate conflicting comparison options
        if self.checksum && self.size_only {
            return Err(PdbSyncError::InvalidInput(
                "checksum and size_only are mutually exclusive".to_string(),
            ));
        }

        if self.size_only && self.ignore_times {
            return Err(PdbSyncError::InvalidInput(
                "size_only and ignore_times are mutually exclusive".to_string(),
            ));
        }

        // Validate chmod format (basic validation)
        if let Some(ref chmod) = self.chmod {
            validate_chmod_string(chmod)?;
        }

        Ok(())
    }

    /// Apply options from RsyncOptionsConfig, overriding existing values.
    ///
    /// This is used to apply global defaults, preset flags, or custom options
    /// to an existing RsyncFlags instance.
    pub fn apply_options(&mut self, options: &crate::config::schema::RsyncOptionsConfig) {
        // Boolean fields
        if let Some(delete) = options.delete {
            self.delete = delete;
        }
        if let Some(compress) = options.compress {
            self.compress = compress;
        }
        if let Some(checksum) = options.checksum {
            self.checksum = checksum;
        }
        if let Some(size_only) = options.size_only {
            self.size_only = size_only;
        }
        if let Some(ignore_times) = options.ignore_times {
            self.ignore_times = ignore_times;
        }
        if let Some(partial) = options.partial {
            self.partial = partial;
        }
        if let Some(backup) = options.backup {
            self.backup = backup;
        }
        if let Some(verbose) = options.verbose {
            self.verbose = verbose;
        }
        if let Some(quiet) = options.quiet {
            self.quiet = quiet;
        }
        if let Some(itemize_changes) = options.itemize_changes {
            self.itemize_changes = itemize_changes;
        }

        // Option fields
        if options.modify_window.is_some() {
            self.modify_window = options.modify_window;
        }
        if options.partial_dir.is_some() {
            self.partial_dir = options.partial_dir.clone();
        }
        if options.max_size.is_some() {
            self.max_size = options.max_size.clone();
        }
        if options.min_size.is_some() {
            self.min_size = options.min_size.clone();
        }
        if options.timeout.is_some() {
            self.timeout = options.timeout;
        }
        if options.contimeout.is_some() {
            self.contimeout = options.contimeout;
        }
        if options.backup_dir.is_some() {
            self.backup_dir = options.backup_dir.clone();
        }
        if options.chmod.is_some() {
            self.chmod = options.chmod.clone();
        }
        if options.exclude_from.is_some() {
            self.exclude_from = options.exclude_from.clone();
        }
        if options.include_from.is_some() {
            self.include_from = options.include_from.clone();
        }

        // Vec fields: non-empty overrides
        if !options.exclude.is_empty() {
            self.exclude = options.exclude.clone();
        }
        if !options.include.is_empty() {
            self.include = options.include.clone();
        }
    }

    /// Apply flags from another RsyncFlags, overriding existing values.
    ///
    /// This is used to apply preset flags. Unlike merge_with(), this copies
    /// all fields directly, including false boolean values.
    /// For Vec fields: only non-empty Vecs override (preserves existing non-empty Vecs).
    pub fn apply_flags(&mut self, other: &RsyncFlags) {
        // Boolean fields: direct copy
        self.delete = other.delete;
        self.compress = other.compress;
        self.checksum = other.checksum;
        self.size_only = other.size_only;
        self.ignore_times = other.ignore_times;
        self.partial = other.partial;
        self.backup = other.backup;
        self.verbose = other.verbose;
        self.quiet = other.quiet;
        self.itemize_changes = other.itemize_changes;

        // Option fields: copy all
        self.modify_window = other.modify_window;
        self.partial_dir = other.partial_dir.clone();
        self.max_size = other.max_size.clone();
        self.min_size = other.min_size.clone();
        self.timeout = other.timeout;
        self.contimeout = other.contimeout;
        self.backup_dir = other.backup_dir.clone();
        self.chmod = other.chmod.clone();

        // Vec fields: non-empty overrides
        if !other.exclude.is_empty() {
            self.exclude = other.exclude.clone();
        }
        if !other.include.is_empty() {
            self.include = other.include.clone();
        }

        // Option<PathBuf> fields
        if other.exclude_from.is_some() {
            self.exclude_from = other.exclude_from.clone();
        }
        if other.include_from.is_some() {
            self.include_from = other.include_from.clone();
        }

        // Note: bwlimit and dry_run are NOT copied (CLI-only flags)
    }

    /// Merge two RsyncFlags, with `other` taking priority over `self`.
    ///
    /// For `Option` fields: `other` Some values override, None preserves `self`.
    /// For `bool` fields: `other` true overrides `self`.
    /// For `Vec` fields: Non-empty `other` overrides, empty preserves `self`.
    ///
    /// Note: This method is currently only used in tests. For production code,
    /// use `apply_flags()` or `apply_options()` instead, which provide clearer
    /// semantics for the priority chain. This method is kept for backward
    /// compatibility and potential future use.
    #[allow(dead_code)]
    pub fn merge_with(&self, other: &RsyncFlags) -> RsyncFlags {
        RsyncFlags {
            // Boolean flags: other=true takes priority
            delete: if other.delete { true } else { self.delete },
            compress: if other.compress { true } else { self.compress },
            checksum: if other.checksum { true } else { self.checksum },
            size_only: if other.size_only {
                true
            } else {
                self.size_only
            },
            ignore_times: if other.ignore_times {
                true
            } else {
                self.ignore_times
            },
            partial: if other.partial { true } else { self.partial },
            backup: if other.backup { true } else { self.backup },
            itemize_changes: if other.itemize_changes {
                true
            } else {
                self.itemize_changes
            },
            verbose: if other.verbose { true } else { self.verbose },
            quiet: if other.quiet { true } else { self.quiet },
            dry_run: other.dry_run || self.dry_run,

            // Option fields: other Some overrides, None preserves self
            bwlimit: other.bwlimit.or(self.bwlimit),
            modify_window: other.modify_window.or(self.modify_window),
            partial_dir: other
                .partial_dir
                .clone()
                .or_else(|| self.partial_dir.clone()),
            max_size: other.max_size.clone().or_else(|| self.max_size.clone()),
            min_size: other.min_size.clone().or_else(|| self.min_size.clone()),
            timeout: other.timeout.or(self.timeout),
            contimeout: other.contimeout.or(self.contimeout),
            backup_dir: other.backup_dir.clone().or_else(|| self.backup_dir.clone()),
            chmod: other.chmod.clone().or_else(|| self.chmod.clone()),
            exclude_from: other
                .exclude_from
                .clone()
                .or_else(|| self.exclude_from.clone()),
            include_from: other
                .include_from
                .clone()
                .or_else(|| self.include_from.clone()),

            // Vec fields: non-empty other overrides, empty preserves self
            exclude: if !other.exclude.is_empty() {
                other.exclude.clone()
            } else {
                self.exclude.clone()
            },
            include: if !other.include.is_empty() {
                other.include.clone()
            } else {
                self.include.clone()
            },
        }
    }

    /// Merge CLI overrides over config defaults.
    ///
    /// CLI overrides take priority over config defaults. `Option` fields allow
    /// distinguishing "not provided" from "explicitly set".
    pub fn merge_with_overrides(&self, overrides: &RsyncFlagOverrides) -> RsyncFlags {
        RsyncFlags {
            // Boolean flags: overrides take priority, otherwise use config values.
            delete: overrides.delete.unwrap_or(self.delete),
            compress: overrides.compress.unwrap_or(self.compress),
            checksum: overrides.checksum.unwrap_or(self.checksum),
            size_only: overrides.size_only.unwrap_or(self.size_only),
            ignore_times: overrides.ignore_times.unwrap_or(self.ignore_times),
            partial: overrides.partial.unwrap_or(self.partial),
            backup: overrides.backup.unwrap_or(self.backup),
            itemize_changes: overrides.itemize_changes.unwrap_or(self.itemize_changes),
            verbose: overrides.verbose.unwrap_or(self.verbose),
            quiet: overrides.quiet.unwrap_or(self.quiet),

            // dry_run is additive (true from either source means true)
            dry_run: overrides.dry_run.unwrap_or(false) || self.dry_run,

            // Option types: CLI Some value overrides, None falls back to config
            bwlimit: overrides.bwlimit.or(self.bwlimit),
            modify_window: overrides.modify_window.or(self.modify_window),
            partial_dir: overrides
                .partial_dir
                .clone()
                .or_else(|| self.partial_dir.clone()),
            max_size: overrides.max_size.clone().or_else(|| self.max_size.clone()),
            min_size: overrides.min_size.clone().or_else(|| self.min_size.clone()),
            timeout: overrides.timeout.or(self.timeout),
            contimeout: overrides.contimeout.or(self.contimeout),
            backup_dir: overrides
                .backup_dir
                .clone()
                .or_else(|| self.backup_dir.clone()),
            chmod: overrides.chmod.clone().or_else(|| self.chmod.clone()),

            // Vec types: CLI overrides if provided
            exclude: overrides
                .exclude
                .clone()
                .unwrap_or_else(|| self.exclude.clone()),
            include: overrides
                .include
                .clone()
                .unwrap_or_else(|| self.include.clone()),
            exclude_from: overrides
                .exclude_from
                .clone()
                .or_else(|| self.exclude_from.clone()),
            include_from: overrides
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

        // File comparison methods
        if self.checksum {
            cmd.arg("-c"); // --checksum
        }

        if self.size_only {
            cmd.arg("--size-only");
        }

        if self.ignore_times {
            cmd.arg("--ignore-times");
        }

        if let Some(window) = self.modify_window {
            cmd.arg(format!("--modify-window={}", window));
        }

        // Partial transfer
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

        // Filtering (include before exclude for proper rsync rule matching)
        for pattern in &self.include {
            cmd.arg(format!("--include={}", pattern));
        }

        for pattern in &self.exclude {
            cmd.arg(format!("--exclude={}", pattern));
        }

        if let Some(ref file) = self.include_from {
            cmd.arg(format!("--include-from={}", file));
        }

        if let Some(ref file) = self.exclude_from {
            cmd.arg(format!("--exclude-from={}", file));
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

        // File comparison methods
        if self.checksum {
            args.push("-c".to_string());
        }

        if self.size_only {
            args.push("--size-only".to_string());
        }

        if self.ignore_times {
            args.push("--ignore-times".to_string());
        }

        if let Some(window) = self.modify_window {
            args.push(format!("--modify-window={}", window));
        }

        // Partial transfer
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

        // Filtering (include before exclude for proper rsync rule matching)
        for pattern in &self.include {
            args.push(format!("--include={}", pattern));
        }

        for pattern in &self.exclude {
            args.push(format!("--exclude={}", pattern));
        }

        if let Some(ref file) = self.include_from {
            args.push(format!("--include-from={}", file));
        }

        if let Some(ref file) = self.exclude_from {
            args.push(format!("--exclude-from={}", file));
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
fn validate_size_string(s: &str) -> Result<()> {
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
fn validate_chmod_string(s: &str) -> Result<()> {
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
    fn test_validate_checksum_size_only_conflict() {
        let flags = RsyncFlags {
            checksum: true,
            size_only: true,
            ..Default::default()
        };
        assert!(flags.validate().is_err());
    }

    #[test]
    fn test_validate_size_only_ignore_times_conflict() {
        let flags = RsyncFlags {
            size_only: true,
            ignore_times: true,
            ..Default::default()
        };
        assert!(flags.validate().is_err());
    }

    #[test]
    fn test_to_args_with_comparison_options() {
        let flags = RsyncFlags {
            size_only: true,
            modify_window: Some(5),
            ..Default::default()
        };
        let args = flags.to_args();
        assert!(args.contains(&"--size-only".to_string()));
        assert!(args.contains(&"--modify-window=5".to_string()));
    }

    #[test]
    fn test_merge_with_overrides() {
        let config = RsyncFlags {
            delete: false,
            compress: true,
            bwlimit: Some(1000),
            exclude: vec!["*.tmp".to_string()],
            ..Default::default()
        };

        let overrides = RsyncFlagOverrides {
            delete: Some(true),
            compress: Some(false),
            max_size: Some("1G".to_string()),
            ..Default::default()
        };

        let merged = config.merge_with_overrides(&overrides);

        assert!(merged.delete); // CLI override
        assert!(!merged.compress); // CLI override (false explicitly set)
        assert_eq!(merged.bwlimit, Some(1000)); // Config preserved
        assert_eq!(merged.max_size, Some("1G".to_string())); // CLI set
        assert_eq!(merged.exclude, vec!["*.tmp".to_string()]); // Config preserved
    }

    #[test]
    fn test_merge_with_overrides_vecs() {
        let config = RsyncFlags {
            exclude: vec!["*.tmp".to_string(), "*.log".to_string()],
            include: vec!["*.cif".to_string()],
            ..Default::default()
        };

        let overrides = RsyncFlagOverrides {
            exclude: Some(vec!["*.bak".to_string()]),
            include: None,
            ..Default::default()
        };

        let merged = config.merge_with_overrides(&overrides);

        // CLI overrides non-empty vec
        assert_eq!(merged.exclude, vec!["*.bak".to_string()]);
        // Config preserved when overrides not provided
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

    #[test]
    fn test_include_before_exclude_ordering() {
        let flags = RsyncFlags {
            include: vec!["*/".to_string(), "*.cif".to_string()],
            exclude: vec!["*".to_string()],
            include_from: Some("/path/to/include.txt".to_string()),
            exclude_from: Some("/path/to/exclude.txt".to_string()),
            ..Default::default()
        };

        let args = flags.to_args();

        // Find indices of include and exclude args
        let include_idx = args
            .iter()
            .position(|a| a.starts_with("--include="))
            .unwrap();
        let exclude_idx = args
            .iter()
            .position(|a| a.starts_with("--exclude="))
            .unwrap();
        let include_from_idx = args
            .iter()
            .position(|a| a.starts_with("--include-from="))
            .unwrap();
        let exclude_from_idx = args
            .iter()
            .position(|a| a.starts_with("--exclude-from="))
            .unwrap();

        // Include patterns must come before exclude patterns
        assert!(
            include_idx < exclude_idx,
            "--include should come before --exclude"
        );
        // Include-from must come before exclude-from
        assert!(
            include_from_idx < exclude_from_idx,
            "--include-from should come before --exclude-from"
        );
    }

    #[test]
    fn test_merge_with() {
        let base = RsyncFlags {
            delete: false,
            compress: true,
            checksum: false,
            bwlimit: Some(1000),
            max_size: Some("1G".to_string()),
            exclude: vec!["*.tmp".to_string()],
            ..Default::default()
        };

        let override_flags = RsyncFlags {
            delete: true,
            compress: false, // Note: bool merge takes other=true, so this won't override
            checksum: true,
            max_size: Some("500M".to_string()),
            min_size: Some("1K".to_string()),
            ..Default::default()
        };

        let merged = base.merge_with(&override_flags);

        // Boolean: other=true takes priority
        assert!(merged.delete); // override took priority
        assert!(merged.compress); // base preserved (override was false)
        assert!(merged.checksum); // override set to true

        // Option: other Some overrides
        assert_eq!(merged.bwlimit, Some(1000)); // base preserved (override was None)
        assert_eq!(merged.max_size, Some("500M".to_string())); // override took priority
        assert_eq!(merged.min_size, Some("1K".to_string())); // override set new value

        // Vec: non-empty other overrides
        assert_eq!(merged.exclude, vec!["*.tmp".to_string()]); // base preserved (override was empty)
    }

    #[test]
    fn test_merge_with_vec_override() {
        let base = RsyncFlags {
            exclude: vec!["*.tmp".to_string(), "*.log".to_string()],
            include: vec!["*.cif".to_string()],
            ..Default::default()
        };

        let override_flags = RsyncFlags {
            exclude: vec!["*.bak".to_string()],
            ..Default::default()
        };

        let merged = base.merge_with(&override_flags);

        // Non-empty other vec overrides base
        assert_eq!(merged.exclude, vec!["*.bak".to_string()]);
        // Empty other vec preserves base
        assert_eq!(merged.include, vec!["*.cif".to_string()]);
    }
}
