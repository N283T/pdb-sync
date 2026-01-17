use crate::cli::args::DownloadArgs;
use crate::context::AppContext;
use crate::data_types::DataType;
use crate::download::{
    aria2c, Aria2cDownloader, DownloadOptions, DownloadResult, DownloadTask, EngineType,
    HttpsDownloader,
};
use crate::error::{PdbCliError, Result};
use crate::files::PdbId;
use crate::utils::IdSource;
use std::time::Duration;
use tokio::fs;

/// Maximum assembly number to try when assembly=0 (try all)
const MAX_ASSEMBLY_NUMBER: u8 = 60;

pub async fn run_download(args: DownloadArgs, ctx: AppContext) -> Result<()> {
    // Collect PDB IDs from args, list file, and/or stdin
    let id_source =
        IdSource::collect(args.pdb_ids.clone(), args.list.as_deref(), args.stdin).await?;

    if id_source.is_empty() {
        return Err(PdbCliError::InvalidInput(
            "No PDB IDs provided. Use positional arguments, --list, or --stdin".into(),
        ));
    }

    let pdb_id_strings = id_source.ids;

    // Parse PDB IDs
    let mut pdb_ids = Vec::new();
    let mut parse_errors = Vec::new();

    for id_str in &pdb_id_strings {
        match PdbId::new(id_str) {
            Ok(pdb_id) => pdb_ids.push(pdb_id),
            Err(e) => {
                eprintln!("Invalid PDB ID '{}': {}", id_str, e);
                parse_errors.push((id_str.clone(), e));
            }
        }
    }

    if pdb_ids.is_empty() {
        return Err(PdbCliError::InvalidInput(
            "No valid PDB IDs provided".into(),
        ));
    }

    // Default to current directory for download
    let dest = args
        .dest
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| ctx.pdb_dir.clone()));
    let mirror = args.mirror.unwrap_or(ctx.mirror);

    // Create destination directory
    fs::create_dir_all(&dest).await?;

    // Build download options
    let options = DownloadOptions {
        mirror,
        decompress: args.decompress || ctx.config.download.auto_decompress,
        overwrite: args.overwrite,
        parallel: args.parallel as usize,
        retry_count: args.retry,
        retry_delay: Duration::from_secs(1),
    };

    // Build download tasks
    let tasks = build_tasks(&pdb_ids, args.data_type, args.format, args.assembly);

    // Determine engine type (CLI arg overrides config)
    let engine = args
        .engine
        .or_else(|| ctx.config.download.engine.parse::<EngineType>().ok())
        .unwrap_or(EngineType::Builtin);

    // Warn if --decompress is used with aria2c (aria2c downloads raw files)
    if engine == EngineType::Aria2c && (args.decompress || ctx.config.download.auto_decompress) {
        eprintln!(
            "Warning: --decompress is not supported with aria2c engine; files will remain compressed"
        );
    }

    // Handle export option
    if args.export_aria2c {
        let content = aria2c::generate_export_input(&tasks, &dest, &options);
        print!("{}", content);
        return Ok(());
    }

    println!(
        "Downloading {} {} ({} tasks, {} parallel)...",
        pdb_ids.len(),
        args.data_type,
        tasks.len(),
        args.parallel
    );

    // Execute download based on engine type
    let results = match engine {
        EngineType::Builtin => {
            let downloader = HttpsDownloader::new(options);
            downloader.download_many(tasks, &dest).await
        }
        EngineType::Aria2c => {
            // Get aria2c-specific options
            let connections = if args.connections != 4 {
                args.connections
            } else {
                ctx.config.download.aria2c_connections
            };
            let split = if args.split != 1 {
                args.split
            } else {
                ctx.config.download.aria2c_split
            };

            match Aria2cDownloader::new(options.clone(), connections, split) {
                Some(downloader) => downloader.download_many(tasks, &dest).await,
                None => {
                    eprintln!(
                        "Warning: aria2c not found in PATH, falling back to built-in downloader"
                    );
                    let downloader = HttpsDownloader::new(options);
                    downloader.download_many(tasks, &dest).await
                }
            }
        }
    };

    // Count results
    let success_count = results.iter().filter(|r| r.is_success()).count();
    let failed_count = results.iter().filter(|r| r.is_failed()).count();
    let skipped_count = results.iter().filter(|r| r.is_skipped()).count();

    // Print failures
    for result in &results {
        if let DownloadResult::Failed {
            pdb_id,
            data_type,
            error,
        } = result
        {
            eprintln!("Failed: {} ({}): {}", pdb_id, data_type, error);
        }
    }

    // Print summary
    println!(
        "\nDownload complete: {} success, {} failed, {} skipped",
        success_count, failed_count, skipped_count
    );
    println!("Destination: {}", dest.display());

    if !parse_errors.is_empty() {
        eprintln!("{} PDB ID(s) were invalid", parse_errors.len());
    }

    Ok(())
}

/// Build download tasks based on data type and assembly options.
fn build_tasks(
    pdb_ids: &[PdbId],
    data_type: DataType,
    format: crate::files::FileFormat,
    assembly: Option<u8>,
) -> Vec<DownloadTask> {
    let mut tasks = Vec::new();

    for pdb_id in pdb_ids {
        match data_type {
            DataType::Structures => {
                tasks.push(DownloadTask::structure(pdb_id.clone(), format));
            }
            DataType::Assemblies => {
                match assembly {
                    Some(0) => {
                        // Try all assemblies 1-60
                        for n in 1..=MAX_ASSEMBLY_NUMBER {
                            tasks.push(DownloadTask::assembly(pdb_id.clone(), n));
                        }
                    }
                    Some(n) => {
                        // Specific assembly number
                        tasks.push(DownloadTask::assembly(pdb_id.clone(), n));
                    }
                    None => {
                        // Default to assembly 1
                        tasks.push(DownloadTask::assembly(pdb_id.clone(), 1));
                    }
                }
            }
            DataType::Biounit => {
                match assembly {
                    Some(0) => {
                        // Try all biounits 1-60
                        for n in 1..=MAX_ASSEMBLY_NUMBER {
                            tasks.push(DownloadTask {
                                pdb_id: pdb_id.clone(),
                                data_type: DataType::Biounit,
                                format: crate::files::FileFormat::PdbGz,
                                assembly_number: Some(n),
                            });
                        }
                    }
                    Some(n) => {
                        tasks.push(DownloadTask {
                            pdb_id: pdb_id.clone(),
                            data_type: DataType::Biounit,
                            format: crate::files::FileFormat::PdbGz,
                            assembly_number: Some(n),
                        });
                    }
                    None => {
                        tasks.push(DownloadTask {
                            pdb_id: pdb_id.clone(),
                            data_type: DataType::Biounit,
                            format: crate::files::FileFormat::PdbGz,
                            assembly_number: Some(1),
                        });
                    }
                }
            }
            DataType::StructureFactors => {
                tasks.push(DownloadTask::structure_factors(pdb_id.clone()));
            }
            DataType::NmrChemicalShifts => {
                tasks.push(DownloadTask::nmr_chemical_shifts(pdb_id.clone()));
            }
            DataType::NmrRestraints => {
                tasks.push(DownloadTask::nmr_restraints(pdb_id.clone()));
            }
            DataType::Obsolete => {
                tasks.push(DownloadTask {
                    pdb_id: pdb_id.clone(),
                    data_type: DataType::Obsolete,
                    format: crate::files::FileFormat::CifGz,
                    assembly_number: None,
                });
            }
        }
    }

    tasks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::FileFormat;

    #[test]
    fn test_build_tasks_structures() {
        let pdb_ids = vec![PdbId::new("1abc").unwrap(), PdbId::new("2xyz").unwrap()];
        let tasks = build_tasks(&pdb_ids, DataType::Structures, FileFormat::Mmcif, None);

        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].data_type, DataType::Structures);
        assert_eq!(tasks[1].data_type, DataType::Structures);
    }

    #[test]
    fn test_build_tasks_assembly_specific() {
        let pdb_ids = vec![PdbId::new("4hhb").unwrap()];
        let tasks = build_tasks(&pdb_ids, DataType::Assemblies, FileFormat::CifGz, Some(1));

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].data_type, DataType::Assemblies);
        assert_eq!(tasks[0].assembly_number, Some(1));
    }

    #[test]
    fn test_build_tasks_assembly_all() {
        let pdb_ids = vec![PdbId::new("4hhb").unwrap()];
        let tasks = build_tasks(&pdb_ids, DataType::Assemblies, FileFormat::CifGz, Some(0));

        // Should generate tasks for assemblies 1-60
        assert_eq!(tasks.len(), 60);
        assert_eq!(tasks[0].assembly_number, Some(1));
        assert_eq!(tasks[59].assembly_number, Some(60));
    }

    #[test]
    fn test_build_tasks_structure_factors() {
        let pdb_ids = vec![PdbId::new("1abc").unwrap(), PdbId::new("2xyz").unwrap()];
        let tasks = build_tasks(
            &pdb_ids,
            DataType::StructureFactors,
            FileFormat::Mmcif,
            None,
        );

        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].data_type, DataType::StructureFactors);
        assert_eq!(tasks[1].data_type, DataType::StructureFactors);
    }
}
