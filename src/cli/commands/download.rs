use crate::cli::args::DownloadArgs;
use crate::context::AppContext;
use crate::data_types::DataType;
use crate::download::{DownloadOptions, DownloadResult, DownloadTask, HttpsDownloader};
use crate::error::{PdbCliError, Result};
use crate::files::PdbId;
use std::path::Path;
use std::time::Duration;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};

/// Maximum assembly number to try when assembly=0 (try all)
const MAX_ASSEMBLY_NUMBER: u8 = 60;

pub async fn run_download(args: DownloadArgs, ctx: AppContext) -> Result<()> {
    // Collect PDB IDs from args and/or list file
    let mut pdb_id_strings = args.pdb_ids.clone();

    if let Some(list_path) = &args.list {
        let ids_from_file = read_id_list(list_path).await?;
        pdb_id_strings.extend(ids_from_file);
    }

    if pdb_id_strings.is_empty() {
        return Err(PdbCliError::InvalidInput(
            "No PDB IDs provided. Use positional arguments or --list".into(),
        ));
    }

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

    println!(
        "Downloading {} {} ({} tasks, {} parallel)...",
        pdb_ids.len(),
        args.data_type,
        tasks.len(),
        args.parallel
    );

    // Create downloader and execute
    let downloader = HttpsDownloader::new(options);
    let results = downloader.download_many(tasks, &dest).await;

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

/// Read PDB IDs from a file, one per line.
async fn read_id_list(path: &Path) -> Result<Vec<String>> {
    let file = fs::File::open(path).await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut ids = Vec::new();

    while let Some(line) = lines.next_line().await? {
        let trimmed = line.trim();
        // Skip empty lines and comments
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            ids.push(trimmed.to_string());
        }
    }

    Ok(ids)
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

    #[tokio::test]
    async fn test_read_id_list() {
        use tokio::io::AsyncWriteExt;

        let temp_dir = tempfile::tempdir().unwrap();
        let list_file = temp_dir.path().join("ids.txt");

        let mut file = fs::File::create(&list_file).await.unwrap();
        file.write_all(b"1abc\n2xyz\n# comment\n\n3def\n")
            .await
            .unwrap();
        file.flush().await.unwrap();

        let ids = read_id_list(&list_file).await.unwrap();
        assert_eq!(ids, vec!["1abc", "2xyz", "3def"]);
    }
}
