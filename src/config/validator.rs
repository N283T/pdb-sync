//! Configuration validation module.
//!
//! Provides validation for configuration files including URL validation,
//! path checks, and rsync flag consistency.

use crate::cli::commands::sync::common::validate_subpath;
use crate::config::schema::{Config, CustomRsyncConfig};
use crate::data_types::DataType;
use crate::error::{PdbSyncError, Result};
use serde::{Deserialize, Serialize};
use shellexpand;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

/// Severity level for validation issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationSeverity {
    /// Error - blocks operation
    Error,
    /// Warning - advisory only
    Warning,
}

/// A validation issue found in the configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// Severity level of this issue
    pub severity: ValidationSeverity,
    /// Section path (e.g., "paths.pdb_dir", "sync.custom[0].url")
    pub section: String,
    /// Error/warning code (e.g., "E001", "W001")
    pub code: String,
    /// Human-readable message
    pub message: String,
    /// Optional suggestion for fixing the issue
    pub suggestion: Option<String>,
}

impl ValidationIssue {
    /// Create a new validation issue.
    fn new(
        severity: ValidationSeverity,
        section: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            section: section.into(),
            code: code.into(),
            message: message.into(),
            suggestion: None,
        }
    }

    /// Add a suggestion to this issue.
    fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

/// Result of configuration validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the configuration is valid (no errors)
    pub valid: bool,
    /// All validation issues found
    pub issues: Vec<ValidationIssue>,
}

impl ValidationResult {
    /// Create a new validation result.
    fn new() -> Self {
        Self {
            valid: true,
            issues: Vec::new(),
        }
    }

    /// Add an issue to the result.
    fn add_issue(&mut self, issue: ValidationIssue) {
        if issue.severity == ValidationSeverity::Error {
            self.valid = false;
        }
        self.issues.push(issue);
    }

    /// Get all errors from the result.
    pub fn errors(&self) -> Vec<&ValidationIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == ValidationSeverity::Error)
            .collect()
    }

    /// Get all warnings from the result.
    pub fn warnings(&self) -> Vec<&ValidationIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == ValidationSeverity::Warning)
            .collect()
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration validator.
pub struct ConfigValidator {
    /// Path to the configuration file
    pub config_path: Option<PathBuf>,
    /// The configuration being validated
    config: Config,
    /// Issue counter for generating codes
    error_count: u32,
    warning_count: u32,
}

impl ConfigValidator {
    /// Create a new validator for the given configuration.
    pub fn new(config: Config, config_path: Option<PathBuf>) -> Self {
        Self {
            config,
            config_path,
            error_count: 0,
            warning_count: 0,
        }
    }

    /// Run full validation on the configuration.
    pub fn validate(&mut self) -> ValidationResult {
        let mut result = ValidationResult::new();

        // Validate paths.pdb_dir
        self.validate_pdb_dir(&mut result);

        // Validate sync.data_types
        self.validate_data_types(&mut result);

        // Validate sync.custom entries
        // Clone to avoid borrow checker issues
        let custom_configs: Vec<CustomRsyncConfig> = self.config.sync.custom.clone();
        for (idx, custom) in custom_configs.iter().enumerate() {
            self.validate_custom_config(custom, idx, &mut result);
        }

        result
    }

    /// Validate the pdb_dir path.
    fn validate_pdb_dir(&mut self, result: &mut ValidationResult) {
        if let Some(ref pdb_dir) = self.config.paths.pdb_dir {
            // Expand tilde if present
            let pdb_dir_str = pdb_dir.to_string_lossy().to_string();
            let expanded = shellexpand::tilde(&pdb_dir_str);
            let path = PathBuf::from(expanded.as_ref());

            if !path.exists() {
                result.add_issue(
                    ValidationIssue::new(
                        ValidationSeverity::Warning,
                        "paths.pdb_dir",
                        self.next_code("W"),
                        "Directory does not exist",
                    )
                    .with_suggestion("Create the directory or update the path"),
                );
            }
        }
    }

    /// Validate data types in sync.data_types.
    fn validate_data_types(&mut self, result: &mut ValidationResult) {
        let mut seen = HashSet::new();

        // Collect data types first to avoid borrow checker issues
        let data_types: Vec<String> = self.config.sync.data_types.clone();

        for (idx, data_type_str) in data_types.iter().enumerate() {
            // Check for duplicates
            if seen.contains(data_type_str) {
                result.add_issue(
                    ValidationIssue::new(
                        ValidationSeverity::Warning,
                        format!("sync.data_types[{}]", idx),
                        self.next_code("W"),
                        format!("Duplicate data type: {}", data_type_str),
                    )
                    .with_suggestion("Remove duplicate entries"),
                );
                continue;
            }
            seen.insert(data_type_str.clone());

            // Validate against known DataType enum
            match self.parse_data_type(data_type_str) {
                Ok(_) => {}
                Err(_) => {
                    // Check if it's a close match (alias issue)
                    let suggestion = self.suggest_data_type_fix(data_type_str);
                    result.add_issue(
                        ValidationIssue::new(
                            ValidationSeverity::Error,
                            format!("sync.data_types[{}]", idx),
                            self.next_code("E"),
                            format!("Unknown data type: {}", data_type_str),
                        )
                        .with_suggestion(suggestion),
                    );
                }
            }
        }
    }

    /// Parse a data type string, handling aliases.
    fn parse_data_type(&self, s: &str) -> Result<DataType> {
        parse_data_type(s)
    }

    /// Suggest a fix for an invalid data type.
    fn suggest_data_type_fix(&self, s: &str) -> String {
        let lower = s.to_lowercase();
        let known = vec![
            "structures",
            "assemblies",
            "biounit",
            "structure-factors",
            "nmr-chemical-shifts",
            "nmr-restraints",
            "obsolete",
        ];

        // Check for close matches
        for known_type in &known {
            if lower.len() >= 3
                && (lower.contains(&known_type[..3.min(known_type.len())])
                    || known_type.contains(&lower[..3.min(lower.len())]))
            {
                return format!("Use \"{}\" instead", known_type);
            }
        }

        format!("Valid types: {}", known.join(", "))
    }

    /// Validate a custom rsync configuration.
    fn validate_custom_config(
        &mut self,
        custom: &CustomRsyncConfig,
        idx: usize,
        result: &mut ValidationResult,
    ) {
        let prefix = format!("sync.custom[{}]", idx);

        // Validate URL
        if let Some(issue) = self.validate_url(&custom.url, &prefix) {
            result.add_issue(issue);
        }

        // Validate dest subpath
        if let Err(msg) = validate_subpath(&custom.dest) {
            result.add_issue(
                ValidationIssue::new(
                    ValidationSeverity::Error,
                    format!("{}.dest", prefix),
                    self.next_code("E"),
                    msg,
                )
                .with_suggestion("Use a relative path without '..'"),
            );
        }

        // Validate rsync flag consistency (partial_dir requires partial, etc.)
        // We check consistency here rather than calling flags.validate() to avoid
        // double validation of size/chmod formats
        if custom.rsync_partial_dir.is_some() && !custom.rsync_partial {
            result.add_issue(
                ValidationIssue::new(
                    ValidationSeverity::Error,
                    prefix.clone(),
                    self.next_code("E"),
                    "partial_dir requires partial=true",
                )
                .with_suggestion("Set rsync_partial=true or remove rsync_partial_dir"),
            );
        }

        if custom.rsync_backup_dir.is_some() && !custom.rsync_backup {
            result.add_issue(
                ValidationIssue::new(
                    ValidationSeverity::Error,
                    prefix.clone(),
                    self.next_code("E"),
                    "backup_dir requires backup=true",
                )
                .with_suggestion("Set rsync_backup=true or remove rsync_backup_dir"),
            );
        }

        if custom.rsync_verbose && custom.rsync_quiet {
            result.add_issue(
                ValidationIssue::new(
                    ValidationSeverity::Error,
                    prefix.clone(),
                    self.next_code("E"),
                    "verbose and quiet are mutually exclusive",
                )
                .with_suggestion("Set only one of rsync_verbose or rsync_quiet"),
            );
        }

        // Validate max_size format (warning only - allows typos in config)
        if let Some(ref max_size) = custom.rsync_max_size {
            if let Err(_e) = crate::sync::flags::validate_size_string(max_size) {
                result.add_issue(
                    ValidationIssue::new(
                        ValidationSeverity::Warning,
                        format!("{}.rsync_max_size", prefix),
                        self.next_code("W"),
                        format!("Invalid size format: {}", max_size),
                    )
                    .with_suggestion("Use format like \"1G\" or \"100M\""),
                );
            }
        }

        // Validate min_size format (warning only)
        if let Some(ref min_size) = custom.rsync_min_size {
            if let Err(_e) = crate::sync::flags::validate_size_string(min_size) {
                result.add_issue(
                    ValidationIssue::new(
                        ValidationSeverity::Warning,
                        format!("{}.rsync_min_size", prefix),
                        self.next_code("W"),
                        format!("Invalid size format: {}", min_size),
                    )
                    .with_suggestion("Use format like \"1G\" or \"100M\""),
                );
            }
        }

        // Validate chmod format (warning only)
        if let Some(ref chmod) = custom.rsync_chmod {
            if let Err(_e) = crate::sync::flags::validate_chmod_string(chmod) {
                result.add_issue(
                    ValidationIssue::new(
                        ValidationSeverity::Warning,
                        format!("{}.rsync_chmod", prefix),
                        self.next_code("W"),
                        format!("Invalid chmod format: {}", chmod),
                    )
                    .with_suggestion("Use format like \"D755,F644\" or \"u+rwx,go-r\""),
                );
            }
        }
    }

    /// Validate an rsync URL.
    ///
    /// Only validates format, not connectivity (offline-friendly).
    fn validate_url(&mut self, url: &str, prefix: &str) -> Option<ValidationIssue> {
        let url = url.trim();

        // Empty URL
        if url.is_empty() {
            return Some(ValidationIssue::new(
                ValidationSeverity::Error,
                format!("{}.url", prefix),
                self.next_code("E"),
                "URL is empty",
            ));
        }

        // Check for dangerous characters (command injection)
        for ch in [';', '&', '|', '`', '$', '\0'] {
            if url.contains(ch) {
                return Some(ValidationIssue::new(
                    ValidationSeverity::Error,
                    format!("{}.url", prefix),
                    self.next_code("E"),
                    format!("URL contains dangerous character: '{}'", ch),
                ));
            }
        }

        // Validate rsync URL format
        // Acceptable formats:
        // - host::module/path (rsync daemon format)
        // - rsync://host[:port]/module/path (rsync:// URL format)
        // - user@host::module/path (with username)

        let is_daemon_format = url.contains("::");
        let is_url_format = url.starts_with("rsync://");

        if !is_daemon_format && !is_url_format {
            return Some(
                ValidationIssue::new(
                    ValidationSeverity::Error,
                    format!("{}.url", prefix),
                    self.next_code("E"),
                    "Invalid rsync URL format",
                )
                .with_suggestion("Use format like \"rsync://host/module\" or \"host::module\""),
            );
        }

        // For daemon format, check that there's content after ::
        if is_daemon_format {
            let parts: Vec<&str> = url.split("::").collect();
            if parts.len() != 2 || parts[1].is_empty() {
                return Some(
                    ValidationIssue::new(
                        ValidationSeverity::Error,
                        format!("{}.url", prefix),
                        self.next_code("E"),
                        "Incomplete rsync daemon URL",
                    )
                    .with_suggestion(
                        "Use format like \"host::module/path\" (module path required after ::)",
                    ),
                );
            }
        }

        // For URL format, check that there's content after //
        if is_url_format {
            let after_protocol = &url[8..]; // Skip "rsync://"
            if after_protocol.is_empty() || after_protocol.starts_with('/') {
                return Some(
                    ValidationIssue::new(
                        ValidationSeverity::Error,
                        format!("{}.url", prefix),
                        self.next_code("E"),
                        "Incomplete rsync URL",
                    )
                    .with_suggestion(
                        "Use format like \"rsync://host/module/path\" (host required)",
                    ),
                );
            }
        }

        None
    }

    /// Get the next error code.
    fn next_code(&mut self, prefix: &str) -> String {
        match prefix {
            "E" => {
                self.error_count += 1;
                format!("E{:03}", self.error_count)
            }
            "W" => {
                self.warning_count += 1;
                format!("W{:03}", self.warning_count)
            }
            _ => "???".to_string(),
        }
    }

    /// Apply safe automatic fixes to the configuration.
    ///
    /// Returns a list of changes made.
    pub fn fix(&self, _result: &ValidationResult) -> Result<Vec<String>> {
        let mut changes = Vec::new();

        // Create backup of original config
        if let Some(ref config_path) = self.config_path {
            let backup_path = config_path.with_extension("toml.bak");
            fs::copy(config_path, &backup_path).map_err(|e| PdbSyncError::Config {
                message: format!(
                    "Failed to create backup at {}: {}",
                    backup_path.display(),
                    e
                ),
                key: None,
                source: Some(Box::new(e)),
            })?;
            changes.push(format!("Created backup: {}", backup_path.display()));
        }

        // Apply fixes (placeholder for actual fix logic)
        // Normalize paths
        // Normalize size strings
        // Remove duplicates
        // Fix invalid data_type values

        Ok(changes)
    }
}

/// Parse a data type from a string (helper function).
pub fn parse_data_type(s: &str) -> Result<DataType> {
    let normalized = s.to_lowercase().replace('_', "-");

    match normalized.as_str() {
        "structures" | "st" | "struct" => Ok(DataType::Structures),
        "assemblies" | "asm" | "assembly" => Ok(DataType::Assemblies),
        "biounit" => Ok(DataType::Biounit),
        "structure-factors" | "sf" | "xray" => Ok(DataType::StructureFactors),
        "nmr-chemical-shifts" | "nmr-cs" | "nmrcs" | "cs" => Ok(DataType::NmrChemicalShifts),
        "nmr-restraints" | "nmr-r" | "nmrr" => Ok(DataType::NmrRestraints),
        "obsolete" => Ok(DataType::Obsolete),
        _ => Err(PdbSyncError::InvalidInput(format!(
            "Unknown data type: {}",
            s
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{PathsConfig, SyncConfig};
    use crate::mirrors::MirrorId;

    fn make_test_config() -> Config {
        Config {
            paths: PathsConfig::default(),
            sync: SyncConfig::default(),
            mirror_selection: Default::default(),
        }
    }

    #[test]
    fn test_validator_valid_config() {
        let config = make_test_config();
        let mut validator = ConfigValidator::new(config, None);
        let result = validator.validate();

        assert!(result.valid);
        assert_eq!(result.errors().len(), 0);
        assert_eq!(result.warnings().len(), 0);
    }

    #[test]
    fn test_validate_url_valid_formats() {
        let valid_urls = vec![
            "rsync://host/module/path",
            "rsync://rsync.ebi.ac.uk/pub/databases/msd/sifts/",
            "host::module/path",
            "data.pdbj.org::rsync/pub/emdb/",
            "user@host::module/path",
        ];

        for url in valid_urls {
            let mut validator = ConfigValidator::new(make_test_config(), None);
            assert!(
                validator.validate_url(url, "test").is_none(),
                "URL should be valid: {}",
                url
            );
        }
    }

    #[test]
    fn test_validate_url_invalid_formats() {
        let invalid_urls = vec![
            "",
            "not-an-rsync-url",
            "host::",
            "rsync://",
            "rsync:///",
            "http://example.com",
        ];

        for url in invalid_urls {
            let mut validator = ConfigValidator::new(make_test_config(), None);
            let result = validator.validate_url(url, "test");
            assert!(result.is_some(), "URL should be invalid: {}", url);
            assert_eq!(result.unwrap().severity, ValidationSeverity::Error);
        }
    }

    #[test]
    fn test_validate_url_dangerous_characters() {
        let dangerous_urls = vec![
            "host::mod; echo",
            "host::mod&rm",
            "host::mod|cat",
            "host::mod`whoami`",
            "host::mod$HOME",
        ];

        for url in dangerous_urls {
            let mut validator = ConfigValidator::new(make_test_config(), None);
            let result = validator.validate_url(url, "test");
            assert!(
                result.is_some(),
                "URL with dangerous chars should be invalid: {}",
                url
            );
        }
    }

    #[test]
    fn test_validate_subpath_valid() {
        let valid_paths = vec!["pub/emdb", "pdbe/sifts", "data/2024", "single", ""];

        for path in valid_paths {
            assert!(
                validate_subpath(path).is_ok(),
                "Path should be valid: {}",
                path
            );
        }
    }

    #[test]
    fn test_validate_subpath_path_traversal() {
        let invalid_paths = vec!["../etc/passwd", "foo/../bar", "foo/..", ".."];

        for path in invalid_paths {
            assert!(
                validate_subpath(path).is_err(),
                "Path should be invalid: {}",
                path
            );
        }
    }

    #[test]
    fn test_validate_data_types_valid() {
        let mut config = make_test_config();
        config.sync.data_types = vec![
            "structures".to_string(),
            "assemblies".to_string(),
            "structure-factors".to_string(),
        ];

        let mut validator = ConfigValidator::new(config, None);
        let result = validator.validate();

        assert!(result.valid);
        assert_eq!(result.errors().len(), 0);
    }

    #[test]
    fn test_validate_data_types_with_aliases() {
        let mut config = make_test_config();
        config.sync.data_types = vec!["st".to_string(), "asm".to_string(), "sf".to_string()];

        let mut validator = ConfigValidator::new(config, None);
        let result = validator.validate();

        assert!(result.valid);
        assert_eq!(result.errors().len(), 0);
    }

    #[test]
    fn test_validate_data_types_invalid() {
        let mut config = make_test_config();
        config.sync.data_types = vec!["structures".to_string(), "invalid_type".to_string()];

        let mut validator = ConfigValidator::new(config, None);
        let result = validator.validate();

        assert!(!result.valid);
        assert_eq!(result.errors().len(), 1);
        assert!(result.errors()[0].section.contains("data_types"));
    }

    #[test]
    fn test_validate_data_types_duplicates() {
        let mut config = make_test_config();
        config.sync.data_types = vec!["structures".to_string(), "structures".to_string()];

        let mut validator = ConfigValidator::new(config, None);
        let result = validator.validate();

        // Duplicates are warnings, not errors
        assert!(result.valid);
        assert_eq!(result.warnings().len(), 1);
        assert!(result.warnings()[0].message.contains("Duplicate"));
    }

    #[test]
    fn test_validate_custom_config_valid() {
        let mut config = make_test_config();
        config.sync.custom.push(CustomRsyncConfig {
            name: "test".to_string(),
            url: "rsync://example.com/module".to_string(),
            dest: "pub/data".to_string(),
            description: None,
            ..Default::default()
        });

        let mut validator = ConfigValidator::new(config, None);
        let result = validator.validate();

        assert!(result.valid);
        assert_eq!(result.errors().len(), 0);
    }

    #[test]
    fn test_validate_custom_config_invalid_url() {
        let mut config = make_test_config();
        config.sync.custom.push(CustomRsyncConfig {
            name: "test".to_string(),
            url: "not-an-rsync-url".to_string(),
            dest: "pub/data".to_string(),
            description: None,
            ..Default::default()
        });

        let mut validator = ConfigValidator::new(config, None);
        let result = validator.validate();

        assert!(!result.valid);
        assert_eq!(result.errors().len(), 1);
        assert!(result.errors()[0].section.contains("url"));
    }

    #[test]
    fn test_validate_custom_config_path_traversal() {
        let mut config = make_test_config();
        config.sync.custom.push(CustomRsyncConfig {
            name: "test".to_string(),
            url: "rsync://example.com/module".to_string(),
            dest: "../etc/passwd".to_string(),
            description: None,
            ..Default::default()
        });

        let mut validator = ConfigValidator::new(config, None);
        let result = validator.validate();

        assert!(!result.valid);
        assert_eq!(result.errors().len(), 1);
        assert!(result.errors()[0].section.contains("dest"));
    }

    #[test]
    fn test_validate_custom_config_partial_dir_requires_partial() {
        let mut config = make_test_config();
        config.sync.custom.push(CustomRsyncConfig {
            name: "test".to_string(),
            url: "rsync://example.com/module".to_string(),
            dest: "pub/data".to_string(),
            description: None,
            rsync_partial: false,
            rsync_partial_dir: Some(".partial".to_string()),
            ..Default::default()
        });

        let mut validator = ConfigValidator::new(config, None);
        let result = validator.validate();

        assert!(!result.valid);
        assert_eq!(result.errors().len(), 1);
        assert!(result.errors()[0]
            .message
            .contains("partial_dir requires partial"));
    }

    #[test]
    fn test_validate_custom_config_invalid_max_size() {
        let mut config = make_test_config();
        config.sync.custom.push(CustomRsyncConfig {
            name: "test".to_string(),
            url: "rsync://example.com/module".to_string(),
            dest: "pub/data".to_string(),
            description: None,
            rsync_max_size: Some("invalid".to_string()),
            ..Default::default()
        });

        let mut validator = ConfigValidator::new(config, None);
        let result = validator.validate();

        // Invalid size format is a warning, not an error
        assert!(result.valid);
        assert_eq!(result.warnings().len(), 1);
        assert!(result.warnings()[0].section.contains("max_size"));
    }

    #[test]
    fn test_parse_data_type_valid() {
        assert!(matches!(
            parse_data_type("structures").unwrap(),
            DataType::Structures
        ));
        assert!(matches!(
            parse_data_type("st").unwrap(),
            DataType::Structures
        ));
        assert!(matches!(
            parse_data_type("structure-factors").unwrap(),
            DataType::StructureFactors
        ));
        assert!(matches!(
            parse_data_type("sf").unwrap(),
            DataType::StructureFactors
        ));
    }

    #[test]
    fn test_parse_data_type_invalid() {
        assert!(parse_data_type("invalid").is_err());
        assert!(parse_data_type("").is_err());
    }

    #[test]
    fn test_validation_result_serialization() {
        let mut result = ValidationResult::new();
        result.add_issue(
            ValidationIssue::new(
                ValidationSeverity::Error,
                "test.section",
                "E001",
                "Test error",
            )
            .with_suggestion("Fix it"),
        );

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"valid\":false"));
        assert!(json.contains("\"E001\""));
        assert!(json.contains("Test error"));
        assert!(json.contains("Fix it"));
    }
}
