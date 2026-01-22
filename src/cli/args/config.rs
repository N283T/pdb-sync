//! CLI argument definitions for config subcommand.

use crate::config::{ConfigLoader, ConfigValidator, ValidationResult, ValidationSeverity};
use crate::context::AppContext;
use crate::error::Result;
use clap::{Parser, ValueEnum};
use std::path::PathBuf;

/// Arguments for the `config validate` command.
#[derive(Parser, Clone, Debug)]
pub struct ValidateArgs {
    /// Apply automatic fixes for safe corrections
    #[arg(long)]
    pub fix: bool,

    /// Output format
    #[arg(short, long, default_value = "text")]
    pub format: ValidateOutputFormat,

    /// Path to config file (default: ~/.config/pdb-sync/config.toml)
    #[arg(short, long, env = "PDB_SYNC_CONFIG")]
    pub config: Option<PathBuf>,
}

/// Output format for validation results.
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ValidateOutputFormat {
    /// Human-readable text output
    #[default]
    Text,
    /// JSON output
    Json,
}

/// Run config validate command.
pub async fn run_validate(args: ValidateArgs, _ctx: AppContext) -> Result<()> {
    // Determine config file path
    let config_path = if let Some(ref path) = args.config {
        path.clone()
    } else {
        ConfigLoader::config_path().ok_or_else(|| crate::error::PdbSyncError::Config {
            message: "Could not determine config directory".to_string(),
            key: None,
            source: None,
        })?
    };

    // Load config (use default if file doesn't exist)
    let config = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        toml::from_str(&content)?
    } else {
        // Config file doesn't exist, validate default config
        crate::config::Config::default()
    };

    // Run validation
    let mut validator = ConfigValidator::new(config, Some(config_path));
    let result = validator.validate();

    // Apply fixes if requested
    if args.fix {
        // Check if there are safe issues to fix
        let fixable: Vec<_> = result
            .issues
            .iter()
            .filter(|i| i.severity == ValidationSeverity::Warning)
            .filter(|i| {
                // Only fix certain types of issues
                i.section.contains("data_types")
                    || i.section.contains("rsync_max_size")
                    || i.section.contains("rsync_min_size")
                    || i.section.contains("rsync_chmod")
            })
            .collect();

        if !fixable.is_empty() {
            let changes = validator.fix(&result)?;
            for change in &changes {
                eprintln!("{}", change);
            }
        }
    }

    // Format and display output
    match args.format {
        ValidateOutputFormat::Text => format_text_output(&result),
        ValidateOutputFormat::Json => format_json_output(&result),
    }

    // Exit with error code if validation failed
    if !result.valid {
        std::process::exit(1);
    }

    Ok(())
}

/// Format validation result as human-readable text.
fn format_text_output(result: &ValidationResult) {
    let errors = result.errors();
    let warnings = result.warnings();

    if result.valid {
        if warnings.is_empty() {
            println!("Config validation: OK");
        } else {
            println!(
                "Config validation: OK (with {} warning{})",
                warnings.len(),
                if warnings.len() == 1 { "" } else { "s" }
            );
        }
    } else {
        println!(
            "Config validation: FAILED - {} error{}, {} warning{}",
            errors.len(),
            if errors.len() == 1 { "" } else { "s" },
            warnings.len(),
            if warnings.len() == 1 { "" } else { "s" }
        );
    }

    // Print errors
    if !errors.is_empty() {
        println!("\nErrors:");
        for error in &errors {
            println!("  [{}] {}: {}", error.code, error.section, error.message);
            if let Some(ref suggestion) = error.suggestion {
                println!("         {}", suggestion);
            }
        }
    }

    // Print warnings
    if !warnings.is_empty() {
        if !errors.is_empty() {
            println!();
        }
        println!("Warnings:");
        for warning in &warnings {
            println!(
                "  [{}] {}: {}",
                warning.code, warning.section, warning.message
            );
            if let Some(ref suggestion) = warning.suggestion {
                println!("         {}", suggestion);
            }
        }
    }
}

/// Format validation result as JSON.
fn format_json_output(result: &ValidationResult) {
    use serde_json::json;

    let errors: Vec<_> = result
        .errors()
        .into_iter()
        .map(|e| {
            json!({
                "code": e.code,
                "section": e.section,
                "message": e.message,
                "suggestion": e.suggestion,
            })
        })
        .collect();

    let warnings: Vec<_> = result
        .warnings()
        .into_iter()
        .map(|w| {
            json!({
                "code": w.code,
                "section": w.section,
                "message": w.message,
                "suggestion": w.suggestion,
            })
        })
        .collect();

    let output = json!({
        "valid": result.valid,
        "errors": errors,
        "warnings": warnings,
    });

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
