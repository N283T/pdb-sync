//! Sync configuration validation.

use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Validation result for a configuration check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub checks: Vec<ValidationCheck>,
}

/// A single validation check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCheck {
    pub name: String,
    pub status: ValidationStatus,
    pub message: String,
    pub fixable: bool,
}

/// Status of a validation check.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationStatus {
    Pass,
    Warning,
    Error,
}

impl ValidationResult {
    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        self.checks
            .iter()
            .any(|c| matches!(c.status, ValidationStatus::Error))
    }

    /// Print validation results in human-readable format.
    pub fn print(&self) {
        if self.valid {
            println!("Configuration is valid!");
            return;
        }

        println!("\nValidation Results:\n");

        for check in &self.checks {
            let status = match check.status {
                ValidationStatus::Pass => "✓",
                ValidationStatus::Warning => "⚠",
                ValidationStatus::Error => "✗",
            };

            println!("{} {}", status, check.name);

            if !check.message.is_empty() {
                println!("  {}", check.message);
            }

            if check.fixable {
                println!("  (This issue can be auto-fixed with --fix)");
            }

            println!();
        }

        if self.has_errors() {
            println!("Validation failed with errors.");
        } else {
            println!("Validation passed with warnings.");
        }
    }
}

/// Validate the entire configuration.
pub fn validate_config(config: &Config) -> ValidationResult {
    let mut checks = Vec::new();

    // Validate custom sync configs
    for custom_config in &config.sync.custom {
        checks.push(validate_custom_config_name(&custom_config.name));
        checks.push(validate_custom_config_url(&custom_config.url));
        checks.push(validate_custom_config_dest(&custom_config.dest));
        checks.push(validate_custom_config_flags(custom_config));
    }

    // Check if there are any custom configs
    if config.sync.custom.is_empty() {
        checks.push(ValidationCheck {
            name: "Custom sync configs".to_string(),
            status: ValidationStatus::Warning,
            message: "No custom sync configs defined".to_string(),
            fixable: false,
        });
    }

    let valid = !checks
        .iter()
        .any(|c| matches!(c.status, ValidationStatus::Error));

    ValidationResult { valid, checks }
}

fn validate_custom_config_name(name: &str) -> ValidationCheck {
    if name.is_empty() {
        return ValidationCheck {
            name: format!("Config name '{}'", name),
            status: ValidationStatus::Error,
            message: "Name cannot be empty".to_string(),
            fixable: false,
        };
    }

    if name.contains(' ') || name.contains('/') || name.contains('\\') {
        return ValidationCheck {
            name: format!("Config name '{}'", name),
            status: ValidationStatus::Error,
            message: "Name cannot contain spaces or path separators".to_string(),
            fixable: false,
        };
    }

    ValidationCheck {
        name: format!("Config name '{}'", name),
        status: ValidationStatus::Pass,
        message: String::new(),
        fixable: false,
    }
}

fn validate_custom_config_url(url: &str) -> ValidationCheck {
    // Use existing rsync URL validation
    use crate::cli::commands::sync::wwpdb::validate_rsync_url;

    match validate_rsync_url(url) {
        Ok(_) => ValidationCheck {
            name: format!("URL '{}'", url),
            status: ValidationStatus::Pass,
            message: String::new(),
            fixable: false,
        },
        Err(e) => ValidationCheck {
            name: format!("URL '{}'", url),
            status: ValidationStatus::Error,
            message: e.to_string(),
            fixable: false,
        },
    }
}

fn validate_custom_config_dest(dest: &str) -> ValidationCheck {
    // Check for path traversal
    if dest.contains("..") {
        return ValidationCheck {
            name: format!("Destination path '{}'", dest),
            status: ValidationStatus::Error,
            message: "Path traversal not allowed (contains '..')".to_string(),
            fixable: false,
        };
    }

    // Check for absolute path (should be relative subpath)
    if Path::new(dest).is_absolute() {
        return ValidationCheck {
            name: format!("Destination path '{}'", dest),
            status: ValidationStatus::Warning,
            message: "Absolute path detected - should be relative subpath".to_string(),
            fixable: false,
        };
    }

    ValidationCheck {
        name: format!("Destination path '{}'", dest),
        status: ValidationStatus::Pass,
        message: String::new(),
        fixable: false,
    }
}

fn validate_custom_config_flags(
    config: &crate::config::schema::CustomRsyncConfig,
) -> ValidationCheck {
    let flags = config.to_rsync_flags();
    let mut issues = Vec::new();

    // Check partial_dir without partial
    if flags.partial_dir.is_some() && !flags.partial {
        issues.push("partial_dir is set but partial is false".to_string());
    }

    // Check backup_dir without backup
    if flags.backup_dir.is_some() && !flags.backup {
        issues.push("backup_dir is set but backup is false".to_string());
    }

    // Check conflicting verbosity
    if flags.verbose && flags.quiet {
        issues.push("verbose and quiet are both true (mutually exclusive)".to_string());
    }

    if issues.is_empty() {
        ValidationCheck {
            name: format!("Flags for '{}'", config.name),
            status: ValidationStatus::Pass,
            message: String::new(),
            fixable: false,
        }
    } else {
        ValidationCheck {
            name: format!("Flags for '{}'", config.name),
            status: ValidationStatus::Error,
            message: issues.join("; "),
            fixable: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_config_name_valid() {
        let check = validate_custom_config_name("test");
        assert!(matches!(check.status, ValidationStatus::Pass));
    }

    #[test]
    fn test_validate_config_name_empty() {
        let check = validate_custom_config_name("");
        assert!(matches!(check.status, ValidationStatus::Error));
    }

    #[test]
    fn test_validate_config_name_invalid_chars() {
        let check = validate_custom_config_name("test name");
        assert!(matches!(check.status, ValidationStatus::Error));

        let check = validate_custom_config_name("test/path");
        assert!(matches!(check.status, ValidationStatus::Error));
    }

    #[test]
    fn test_validate_dest_path_traversal() {
        let check = validate_custom_config_dest("../etc");
        assert!(matches!(check.status, ValidationStatus::Error));
    }

    #[test]
    fn test_validate_dest_absolute_path_warning() {
        let check = validate_custom_config_dest("/absolute/path");
        assert!(matches!(check.status, ValidationStatus::Warning));
    }
}
