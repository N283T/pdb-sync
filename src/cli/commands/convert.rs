//! Convert command handler.

use crate::cli::args::ConvertArgs;
use crate::context::AppContext;
use crate::convert::{
    build_dest_path, check_gemmi_available, detect_format_from_path, ConvertOperation,
    ConvertResult, ConvertTask, Converter,
};
use crate::error::{PdbSyncError, Result};
use std::io::{self, BufRead};
use std::path::PathBuf;

/// Run the convert command.
pub async fn run_convert(args: ConvertArgs, _ctx: AppContext) -> Result<()> {
    // Collect input files
    let mut files = collect_input_files(&args).await?;

    if files.is_empty() {
        return Err(PdbSyncError::InvalidInput(
            "No files to convert. Provide file paths, glob patterns, or use --stdin".into(),
        ));
    }

    // Filter by source format if specified
    if let Some(from_format) = args.from {
        files.retain(|f| {
            detect_format_from_path(f)
                .map(|detected| format_matches(detected, from_format))
                .unwrap_or(false)
        });

        if files.is_empty() {
            return Err(PdbSyncError::InvalidInput(format!(
                "No files match the source format filter: {}",
                from_format
            )));
        }
    }

    // Determine the operation
    let operation = determine_operation(&args)?;

    // Check gemmi availability for format conversion
    if let ConvertOperation::ConvertFormat(_) = operation {
        if !check_gemmi_available().await {
            return Err(PdbSyncError::ToolNotFound(
                "gemmi not found. Install with: pip install gemmi".into(),
            ));
        }
    }

    // Build tasks
    let dest_dir = args.dest.as_deref();
    let tasks: Vec<ConvertTask> = files
        .iter()
        .filter_map(|source| {
            let dest = match build_dest_path(source, dest_dir, operation, args.in_place) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Skipping {}: {}", source.display(), e);
                    return None;
                }
            };

            Some(ConvertTask {
                source: source.clone(),
                dest,
                operation,
            })
        })
        .collect();

    if tasks.is_empty() {
        return Err(PdbSyncError::InvalidInput(
            "No valid conversion tasks to perform".into(),
        ));
    }

    println!(
        "Converting {} file(s) ({}, {} parallel)...",
        tasks.len(),
        operation,
        args.parallel
    );

    // Execute conversions
    let converter = Converter::new(args.parallel as usize);
    let results = converter.convert_many(tasks).await;

    // Count results
    let success_count = results.iter().filter(|r| r.is_success()).count();
    let failed_count = results.iter().filter(|r| r.is_failed()).count();
    let skipped_count = results.iter().filter(|r| r.is_skipped()).count();

    // Print results
    for result in &results {
        match result {
            ConvertResult::Success { source, dest, .. } => {
                println!("  {} -> {}", source.display(), dest.display());

                // Delete source if in-place mode
                if args.in_place && source != dest {
                    if let Err(e) = tokio::fs::remove_file(source).await {
                        eprintln!("Warning: Failed to remove original file: {}", e);
                    }
                }
            }
            ConvertResult::Failed { source, error, .. } => {
                eprintln!("Failed: {}: {}", source.display(), error);
            }
            ConvertResult::Skipped { source, reason } => {
                println!("Skipped: {}: {}", source.display(), reason);
            }
        }
    }

    // Print summary
    println!(
        "\nConversion complete: {} success, {} failed, {} skipped",
        success_count, failed_count, skipped_count
    );

    if failed_count > 0 {
        return Err(PdbSyncError::Conversion(format!(
            "{} conversion(s) failed",
            failed_count
        )));
    }

    Ok(())
}

/// Collect input files from args and stdin.
async fn collect_input_files(args: &ConvertArgs) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    // Read from stdin if requested (synchronous, like id_reader.rs)
    if args.stdin {
        let stdin = io::stdin();
        let reader = stdin.lock();

        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                let path = PathBuf::from(trimmed);
                if path.exists() {
                    files.push(path);
                } else {
                    eprintln!("Warning: File not found: {}", trimmed);
                }
            }
        }
    }

    // Process file arguments (may include glob patterns)
    for pattern in &args.files {
        // Check if it's a glob pattern
        if pattern.contains('*') || pattern.contains('?') || pattern.contains('[') {
            match glob::glob(pattern) {
                Ok(paths) => {
                    for entry in paths.flatten() {
                        if entry.is_file() {
                            files.push(entry);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Invalid glob pattern '{}': {}", pattern, e);
                }
            }
        } else {
            let path = PathBuf::from(pattern);
            if path.exists() {
                files.push(path);
            } else {
                eprintln!("Warning: File not found: {}", pattern);
            }
        }
    }

    Ok(files)
}

/// Determine the operation from the arguments.
fn determine_operation(args: &ConvertArgs) -> Result<ConvertOperation> {
    if args.decompress {
        Ok(ConvertOperation::Decompress)
    } else if args.compress {
        Ok(ConvertOperation::Compress)
    } else if let Some(to_format) = args.to {
        Ok(ConvertOperation::ConvertFormat(to_format))
    } else {
        Err(PdbSyncError::InvalidInput(
            "Must specify --decompress, --compress, or --to <format>".into(),
        ))
    }
}

/// Check if a detected format matches the expected format.
fn format_matches(detected: crate::files::FileFormat, expected: crate::files::FileFormat) -> bool {
    // Match base formats (e.g., CifGz matches Mmcif filter)
    detected == expected || detected.base_format() == expected.base_format()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::FileFormat;

    #[test]
    fn test_determine_operation_decompress() {
        let args = ConvertArgs {
            files: vec![],
            decompress: true,
            compress: false,
            to: None,
            from: None,
            dest: None,
            in_place: false,
            stdin: false,
            parallel: 4,
        };

        let op = determine_operation(&args).unwrap();
        assert_eq!(op, ConvertOperation::Decompress);
    }

    #[test]
    fn test_determine_operation_compress() {
        let args = ConvertArgs {
            files: vec![],
            decompress: false,
            compress: true,
            to: None,
            from: None,
            dest: None,
            in_place: false,
            stdin: false,
            parallel: 4,
        };

        let op = determine_operation(&args).unwrap();
        assert_eq!(op, ConvertOperation::Compress);
    }

    #[test]
    fn test_determine_operation_format() {
        let args = ConvertArgs {
            files: vec![],
            decompress: false,
            compress: false,
            to: Some(FileFormat::Pdb),
            from: None,
            dest: None,
            in_place: false,
            stdin: false,
            parallel: 4,
        };

        let op = determine_operation(&args).unwrap();
        assert_eq!(op, ConvertOperation::ConvertFormat(FileFormat::Pdb));
    }

    #[test]
    fn test_determine_operation_none() {
        let args = ConvertArgs {
            files: vec![],
            decompress: false,
            compress: false,
            to: None,
            from: None,
            dest: None,
            in_place: false,
            stdin: false,
            parallel: 4,
        };

        let result = determine_operation(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_format_matches() {
        assert!(format_matches(FileFormat::CifGz, FileFormat::Mmcif));
        assert!(format_matches(FileFormat::Mmcif, FileFormat::CifGz));
        assert!(format_matches(FileFormat::PdbGz, FileFormat::Pdb));
        assert!(!format_matches(FileFormat::Pdb, FileFormat::Mmcif));
    }
}
