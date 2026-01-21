use crate::data_types::DataType;
use crate::download::task::{DownloadResult, DownloadTask};
use crate::error::{PdbSyncError, Result};
use crate::files::{FileFormat, PdbId};
use crate::mirrors::{Mirror, MirrorId};
use async_compression::tokio::bufread::GzipDecoder;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Semaphore;

/// Download configuration options.
#[derive(Clone)]
pub struct DownloadOptions {
    pub mirror: MirrorId,
    pub decompress: bool,
    pub overwrite: bool,
    /// Number of parallel downloads (default: 4)
    pub parallel: usize,
    /// Number of retry attempts (default: 3)
    pub retry_count: u32,
    /// Base delay between retries (exponential backoff)
    pub retry_delay: Duration,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            mirror: MirrorId::Rcsb,
            decompress: false,
            overwrite: false,
            parallel: 4,
            retry_count: 3,
            retry_delay: Duration::from_secs(1),
        }
    }
}

/// HTTPS downloader with parallel download support.
pub struct HttpsDownloader {
    client: reqwest::Client,
    options: DownloadOptions,
    semaphore: Arc<Semaphore>,
}

impl HttpsDownloader {
    pub fn new(options: DownloadOptions) -> Self {
        let semaphore = Arc::new(Semaphore::new(options.parallel));
        let client = reqwest::Client::builder()
            .user_agent("pdb-sync")
            .connect_timeout(Duration::from_secs(30))
            .timeout(Duration::from_secs(300)) // 5 minutes for large files
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            options,
            semaphore,
        }
    }

    /// Download multiple tasks in parallel.
    pub async fn download_many(
        &self,
        tasks: Vec<DownloadTask>,
        dest: &Path,
    ) -> Vec<DownloadResult> {
        let futures: Vec<_> = tasks
            .into_iter()
            .map(|task| self.download_with_semaphore(task, dest.to_path_buf()))
            .collect();

        futures_util::future::join_all(futures).await
    }

    /// Download a task with semaphore-controlled concurrency.
    async fn download_with_semaphore(&self, task: DownloadTask, dest: PathBuf) -> DownloadResult {
        let _permit = self.semaphore.acquire().await.expect("Semaphore closed");
        self.download_with_retry(task, &dest).await
    }

    /// Download a task with exponential backoff retry.
    async fn download_with_retry(&self, task: DownloadTask, dest: &Path) -> DownloadResult {
        let mut last_error = String::new();

        for attempt in 0..=self.options.retry_count {
            if attempt > 0 {
                let delay = self.options.retry_delay * 2u32.pow(attempt - 1);
                tokio::time::sleep(delay).await;
            }

            match self.download_single(&task, dest).await {
                Ok(path) => {
                    return DownloadResult::success(task.pdb_id.clone(), task.data_type, path);
                }
                Err(e) => {
                    // Check for 404 on optional assemblies/biounits (graceful failure)
                    if (task.data_type == DataType::Assemblies
                        || task.data_type == DataType::Biounit)
                        && is_not_found_error(&e)
                    {
                        return DownloadResult::skipped(
                            task.pdb_id.clone(),
                            task.data_type,
                            format!("{} not available (404)", task.data_type),
                        );
                    }
                    last_error = e.to_string();
                    if attempt < self.options.retry_count {
                        eprintln!(
                            "Retry {}/{} for {}: {}",
                            attempt + 1,
                            self.options.retry_count,
                            task.description(),
                            e
                        );
                    }
                }
            }
        }

        DownloadResult::failed(task.pdb_id.clone(), task.data_type, last_error)
    }

    /// Download a single task.
    async fn download_single(&self, task: &DownloadTask, dest: &Path) -> Result<PathBuf> {
        let url = self.build_url_for_task(task);
        let dest_file = self.build_dest_path_for_task(dest, task);

        if dest_file.exists() && !self.options.overwrite {
            return Err(PdbSyncError::Download {
                pdb_id: task.pdb_id.to_string(),
                url: url.clone(),
                message: format!("File already exists: {}", dest_file.display()),
                is_retriable: false,
            });
        }

        if let Some(parent) = dest_file.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        println!("Downloading {} ...", url);

        let response = self.client.get(&url).send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(PdbSyncError::Download {
                pdb_id: task.pdb_id.to_string(),
                url: url.clone(),
                message: format!("HTTP 404 for {}", url),
                is_retriable: false,
            });
        }

        if !response.status().is_success() {
            return Err(PdbSyncError::Download {
                pdb_id: task.pdb_id.to_string(),
                url: url.clone(),
                message: format!("HTTP {} for {}", response.status(), url),
                is_retriable: response.status().is_server_error(),
            });
        }

        let total_size = response.content_length();

        let pb = ProgressBar::new(total_size.unwrap_or(0));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );

        // Download to temporary file first
        let temp_path = dest_file.with_extension("tmp");
        let mut temp_file = File::create(&temp_path).await?;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            temp_file.write_all(&chunk).await?;
            pb.inc(chunk.len() as u64);
        }

        temp_file.flush().await?;
        drop(temp_file);

        pb.finish_with_message("done");

        // Handle compression based on user preference and what mirror provides
        let is_gzipped = self.is_gzipped(&temp_path).await;
        let format_expects_compressed = task.format.is_compressed();

        // Case 1: Downloaded gzipped file, but user wants uncompressed and --decompress is set
        if is_gzipped && !format_expects_compressed && self.options.decompress {
            self.decompress_file(&temp_path, &dest_file).await?;
            tokio::fs::remove_file(&temp_path).await?;
            println!("Saved (decompressed) to: {}", dest_file.display());
        }
        // Case 2: Downloaded gzipped but format expects uncompressed - save with .gz extension
        else if is_gzipped && !format_expects_compressed {
            let corrected_path = dest.join(format!(
                "{}.{}.gz",
                task.pdb_id.as_str(),
                task.format.extension()
            ));
            tokio::fs::rename(&temp_path, &corrected_path).await?;
            println!(
                "Saved to: {} (compressed, use --decompress to extract)",
                corrected_path.display()
            );
            return Ok(corrected_path);
        }
        // Case 3: Normal case - format matches downloaded content
        else {
            tokio::fs::rename(&temp_path, &dest_file).await?;
            println!("Saved to: {}", dest_file.display());
        }

        Ok(dest_file)
    }

    /// Legacy download method for single PDB ID (backward compatibility).
    #[allow(dead_code)]
    pub async fn download(&self, pdb_id: &PdbId, format: FileFormat, dest: &Path) -> Result<()> {
        // Warn if using BinaryCIF with non-RCSB mirror (will fall back to mmCIF)
        if format.base_format() == FileFormat::Bcif && self.options.mirror != MirrorId::Rcsb {
            eprintln!(
                "Warning: BinaryCIF is only available from RCSB. Falling back to mmCIF for {}.",
                self.options.mirror
            );
        }

        let task = DownloadTask::structure(pdb_id.clone(), format);
        let result = self.download_with_retry(task, dest).await;

        match result {
            DownloadResult::Success { .. } => Ok(()),
            DownloadResult::Failed { error, .. } => Err(PdbSyncError::Download {
                pdb_id: pdb_id.to_string(),
                url: format!("{}", self.options.mirror),
                message: error,
                is_retriable: true,
            }),
            DownloadResult::Skipped { reason, .. } => {
                println!("Skipped: {}", reason);
                Ok(())
            }
        }
    }

    async fn is_gzipped(&self, path: &Path) -> bool {
        if let Ok(mut file) = File::open(path).await {
            let mut magic = [0u8; 2];
            if file.read_exact(&mut magic).await.is_ok() {
                return magic == [0x1f, 0x8b];
            }
        }
        false
    }

    async fn decompress_file(&self, src: &Path, dest: &Path) -> Result<()> {
        let file = File::open(src).await?;
        let reader = BufReader::new(file);
        let mut decoder = GzipDecoder::new(reader);

        let mut output = File::create(dest).await?;
        // Use streaming copy instead of loading entire file into memory
        tokio::io::copy(&mut decoder, &mut output).await?;
        output.flush().await?;

        Ok(())
    }

    /// Build URL for a download task based on data type and mirror.
    pub fn build_url_for_task(&self, task: &DownloadTask) -> String {
        let mirror = Mirror::get(self.options.mirror);
        let id = task.pdb_id.as_str();

        match task.data_type {
            DataType::Structures => self.build_structure_url(&task.pdb_id, task.format),
            DataType::Assemblies => {
                self.build_assembly_url(&task.pdb_id, task.assembly_number.unwrap_or(1))
            }
            DataType::StructureFactors => self.build_sf_url(&task.pdb_id),
            DataType::NmrChemicalShifts => self.build_nmr_cs_url(&task.pdb_id),
            DataType::NmrRestraints => self.build_nmr_mr_url(&task.pdb_id),
            DataType::Biounit => {
                // Biounit uses legacy PDB format
                self.build_biounit_url(&task.pdb_id, task.assembly_number.unwrap_or(1))
            }
            DataType::Obsolete => {
                // Obsolete uses same path as structures but in obsolete directory
                format!("{}/obsolete/mmCIF/{}.cif.gz", mirror.https_base, id)
            }
        }
    }

    /// Build URL for structure files.
    ///
    /// Delegates to `Mirror::build_structure_url` for canonical URL construction.
    fn build_structure_url(&self, pdb_id: &PdbId, format: FileFormat) -> String {
        let mirror = Mirror::get(self.options.mirror);
        mirror.build_structure_url(pdb_id, format)
    }

    /// Build URL for assembly files.
    fn build_assembly_url(&self, pdb_id: &PdbId, assembly_num: u8) -> String {
        let id = pdb_id.as_str();
        let middle = pdb_id.middle_chars();

        match self.options.mirror {
            MirrorId::Rcsb => {
                format!(
                    "https://files.rcsb.org/download/{}-assembly{}.cif.gz",
                    id, assembly_num
                )
            }
            MirrorId::Wwpdb => {
                // wwPDB stores assemblies at: assemblies/mmCIF/divided/{middle}/{id}-assembly{n}.cif.gz
                format!(
                    "https://files.wwpdb.org/pub/pdb/data/assemblies/mmCIF/divided/{}/{}-assembly{}.cif.gz",
                    middle, id, assembly_num
                )
            }
            MirrorId::Pdbe => {
                format!(
                    "https://www.ebi.ac.uk/pdbe/entry-files/download/{}-assembly{}.cif.gz",
                    id, assembly_num
                )
            }
            MirrorId::Pdbj => {
                // PDBj may use different format; fallback to RCSB pattern
                format!(
                    "https://files.rcsb.org/download/{}-assembly{}.cif.gz",
                    id, assembly_num
                )
            }
        }
    }

    /// Build URL for structure factor files.
    fn build_sf_url(&self, pdb_id: &PdbId) -> String {
        let id = pdb_id.as_str();
        let middle = pdb_id.middle_chars();

        match self.options.mirror {
            MirrorId::Rcsb => {
                format!("https://files.rcsb.org/download/r{}sf.ent.gz", id)
            }
            MirrorId::Wwpdb => {
                format!(
                    "https://files.wwpdb.org/pub/pdb/data/structures/divided/structure_factors/{}/r{}sf.ent.gz",
                    middle, id
                )
            }
            MirrorId::Pdbe => {
                format!(
                    "https://www.ebi.ac.uk/pdbe/entry-files/download/r{}sf.ent.gz",
                    id
                )
            }
            MirrorId::Pdbj => {
                // Fallback to RCSB
                format!("https://files.rcsb.org/download/r{}sf.ent.gz", id)
            }
        }
    }

    /// Build URL for NMR chemical shift files.
    fn build_nmr_cs_url(&self, pdb_id: &PdbId) -> String {
        let id = pdb_id.as_str();
        let middle = pdb_id.middle_chars();

        match self.options.mirror {
            MirrorId::Rcsb => {
                format!("https://files.rcsb.org/download/{}_cs.str.gz", id)
            }
            MirrorId::Wwpdb => {
                format!(
                    "https://files.wwpdb.org/pub/pdb/data/structures/divided/nmr_chemical_shifts/{}/{}_cs.str.gz",
                    middle, id
                )
            }
            _ => {
                // Fallback to RCSB for other mirrors
                format!("https://files.rcsb.org/download/{}_cs.str.gz", id)
            }
        }
    }

    /// Build URL for NMR restraint files.
    fn build_nmr_mr_url(&self, pdb_id: &PdbId) -> String {
        let id = pdb_id.as_str();
        let middle = pdb_id.middle_chars();

        match self.options.mirror {
            MirrorId::Rcsb => {
                format!("https://files.rcsb.org/download/{}_mr.str.gz", id)
            }
            MirrorId::Wwpdb => {
                format!(
                    "https://files.wwpdb.org/pub/pdb/data/structures/divided/nmr_restraints/{}/{}_mr.str.gz",
                    middle, id
                )
            }
            _ => {
                // Fallback to RCSB for other mirrors
                format!("https://files.rcsb.org/download/{}_mr.str.gz", id)
            }
        }
    }

    /// Build URL for legacy biounit files.
    fn build_biounit_url(&self, pdb_id: &PdbId, unit_num: u8) -> String {
        let id = pdb_id.as_str();
        let middle = pdb_id.middle_chars();

        match self.options.mirror {
            MirrorId::Rcsb => {
                format!("https://files.rcsb.org/download/{}.pdb{}.gz", id, unit_num)
            }
            MirrorId::Wwpdb => {
                format!(
                    "https://files.wwpdb.org/pub/pdb/data/biounit/coordinates/divided/{}/{}.pdb{}.gz",
                    middle, id, unit_num
                )
            }
            _ => {
                // Fallback to RCSB
                format!("https://files.rcsb.org/download/{}.pdb{}.gz", id, unit_num)
            }
        }
    }

    /// Build destination path for a download task.
    pub fn build_dest_path_for_task(&self, dest: &Path, task: &DownloadTask) -> PathBuf {
        let id = task.pdb_id.as_str();

        match task.data_type {
            DataType::Structures => dest.join(format!("{}.{}", id, task.format.extension())),
            DataType::Assemblies => {
                let n = task.assembly_number.unwrap_or(1);
                dest.join(format!("{}-assembly{}.cif.gz", id, n))
            }
            DataType::StructureFactors => dest.join(format!("r{}sf.ent.gz", id)),
            DataType::NmrChemicalShifts => dest.join(format!("{}_cs.str.gz", id)),
            DataType::NmrRestraints => dest.join(format!("{}_mr.str.gz", id)),
            DataType::Biounit => {
                let n = task.assembly_number.unwrap_or(1);
                dest.join(format!("{}.pdb{}.gz", id, n))
            }
            DataType::Obsolete => dest.join(format!("{}.cif.gz", id)),
        }
    }

    // Keep the old build_url for backward compatibility with the download method
    #[allow(dead_code)]
    fn build_url(&self, pdb_id: &PdbId, format: FileFormat) -> String {
        self.build_structure_url(pdb_id, format)
    }

    #[allow(dead_code)]
    fn build_dest_path(
        &self,
        dest: &Path,
        pdb_id: &PdbId,
        format: FileFormat,
    ) -> std::path::PathBuf {
        let id = pdb_id.as_str();
        dest.join(format!("{}.{}", id, format.extension()))
    }
}

/// Check if an error indicates a 404 Not Found.
fn is_not_found_error(err: &PdbSyncError) -> bool {
    match err {
        PdbSyncError::Download { message, .. } => message.contains("404"),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Classic ID URL tests ===

    #[test]
    fn test_build_url_rcsb_classic() {
        let downloader = HttpsDownloader::new(DownloadOptions {
            mirror: MirrorId::Rcsb,
            ..Default::default()
        });
        let pdb_id = PdbId::new("1abc").unwrap();

        assert_eq!(
            downloader.build_url(&pdb_id, FileFormat::Pdb),
            "https://files.rcsb.org/download/1abc.pdb"
        );
        assert_eq!(
            downloader.build_url(&pdb_id, FileFormat::Mmcif),
            "https://files.rcsb.org/download/1abc.cif"
        );
        assert_eq!(
            downloader.build_url(&pdb_id, FileFormat::CifGz),
            "https://files.rcsb.org/download/1abc.cif"
        );
        // BinaryCIF uses models.rcsb.org (different from files.rcsb.org)
        assert_eq!(
            downloader.build_url(&pdb_id, FileFormat::Bcif),
            "https://models.rcsb.org/1abc.bcif"
        );
    }

    #[test]
    fn test_build_url_wwpdb_classic() {
        let downloader = HttpsDownloader::new(DownloadOptions {
            mirror: MirrorId::Wwpdb,
            ..Default::default()
        });
        let pdb_id = PdbId::new("1abc").unwrap();

        // wwPDB provides compressed files via HTTPS
        assert_eq!(
            downloader.build_url(&pdb_id, FileFormat::Mmcif),
            "https://files.wwpdb.org/pub/pdb/data/structures/divided/mmCIF/ab/1abc.cif.gz"
        );
        // Classic IDs have "pdb" prefix in PDB format filename
        assert_eq!(
            downloader.build_url(&pdb_id, FileFormat::Pdb),
            "https://files.wwpdb.org/pub/pdb/data/structures/divided/pdb/ab/pdb1abc.ent.gz"
        );
    }

    // === Extended ID URL tests ===

    #[test]
    fn test_build_url_rcsb_extended() {
        let downloader = HttpsDownloader::new(DownloadOptions {
            mirror: MirrorId::Rcsb,
            ..Default::default()
        });
        let pdb_id = PdbId::new("pdb_00001abc").unwrap();

        // RCSB uses the full ID directly
        assert_eq!(
            downloader.build_url(&pdb_id, FileFormat::Pdb),
            "https://files.rcsb.org/download/pdb_00001abc.pdb"
        );
        assert_eq!(
            downloader.build_url(&pdb_id, FileFormat::Mmcif),
            "https://files.rcsb.org/download/pdb_00001abc.cif"
        );
        assert_eq!(
            downloader.build_url(&pdb_id, FileFormat::Bcif),
            "https://models.rcsb.org/pdb_00001abc.bcif"
        );
    }

    #[test]
    fn test_build_url_wwpdb_extended() {
        let downloader = HttpsDownloader::new(DownloadOptions {
            mirror: MirrorId::Wwpdb,
            ..Default::default()
        });
        let pdb_id = PdbId::new("pdb_00001abc").unwrap();

        // Extended IDs use positions 6-7 for directory partitioning (= "00")
        assert_eq!(
            downloader.build_url(&pdb_id, FileFormat::Mmcif),
            "https://files.wwpdb.org/pub/pdb/data/structures/divided/mmCIF/00/pdb_00001abc.cif.gz"
        );
        // Extended IDs don't have extra "pdb" prefix in filename
        assert_eq!(
            downloader.build_url(&pdb_id, FileFormat::Pdb),
            "https://files.wwpdb.org/pub/pdb/data/structures/divided/pdb/00/pdb_00001abc.ent.gz"
        );
    }

    #[test]
    fn test_build_url_pdbe_extended() {
        let downloader = HttpsDownloader::new(DownloadOptions {
            mirror: MirrorId::Pdbe,
            ..Default::default()
        });
        let pdb_id = PdbId::new("pdb_00001abc").unwrap();

        // Extended IDs don't have extra "pdb" prefix in PDB format
        assert_eq!(
            downloader.build_url(&pdb_id, FileFormat::Pdb),
            "https://www.ebi.ac.uk/pdbe/entry-files/download/pdb_00001abc.ent"
        );
        assert_eq!(
            downloader.build_url(&pdb_id, FileFormat::Mmcif),
            "https://www.ebi.ac.uk/pdbe/entry-files/download/pdb_00001abc.cif"
        );
    }

    #[test]
    fn test_build_url_pdbj_extended() {
        let downloader = HttpsDownloader::new(DownloadOptions {
            mirror: MirrorId::Pdbj,
            ..Default::default()
        });
        let pdb_id = PdbId::new("pdb_00001abc").unwrap();

        // PDBj uses query parameters with full ID
        assert_eq!(
            downloader.build_url(&pdb_id, FileFormat::Pdb),
            "https://pdbj.org/rest/downloadPDBfile?format=pdb&id=pdb_00001abc"
        );
        assert_eq!(
            downloader.build_url(&pdb_id, FileFormat::Mmcif),
            "https://pdbj.org/rest/downloadPDBfile?format=mmcif&id=pdb_00001abc"
        );
    }

    // === Destination path tests ===

    #[test]
    fn test_build_dest_path_classic() {
        let downloader = HttpsDownloader::new(DownloadOptions::default());
        let pdb_id = PdbId::new("1abc").unwrap();

        let path = downloader.build_dest_path(Path::new("/tmp"), &pdb_id, FileFormat::Mmcif);
        assert_eq!(path, std::path::PathBuf::from("/tmp/1abc.cif"));

        let path = downloader.build_dest_path(Path::new("/tmp"), &pdb_id, FileFormat::CifGz);
        assert_eq!(path, std::path::PathBuf::from("/tmp/1abc.cif.gz"));
    }

    #[test]
    fn test_build_assembly_url_rcsb() {
        let downloader = HttpsDownloader::new(DownloadOptions {
            mirror: MirrorId::Rcsb,
            ..Default::default()
        });
        let pdb_id = PdbId::new("4hhb").unwrap();

        let url = downloader.build_assembly_url(&pdb_id, 1);
        assert_eq!(url, "https://files.rcsb.org/download/4hhb-assembly1.cif.gz");
    }

    #[test]
    fn test_build_sf_url_rcsb() {
        let downloader = HttpsDownloader::new(DownloadOptions {
            mirror: MirrorId::Rcsb,
            ..Default::default()
        });
        let pdb_id = PdbId::new("1abc").unwrap();

        let url = downloader.build_sf_url(&pdb_id);
        assert_eq!(url, "https://files.rcsb.org/download/r1abcsf.ent.gz");
    }

    #[test]
    fn test_build_sf_url_wwpdb() {
        let downloader = HttpsDownloader::new(DownloadOptions {
            mirror: MirrorId::Wwpdb,
            ..Default::default()
        });
        let pdb_id = PdbId::new("1abc").unwrap();

        let url = downloader.build_sf_url(&pdb_id);
        assert_eq!(
            url,
            "https://files.wwpdb.org/pub/pdb/data/structures/divided/structure_factors/ab/r1abcsf.ent.gz"
        );
    }

    #[test]
    fn test_build_nmr_urls() {
        let downloader = HttpsDownloader::new(DownloadOptions {
            mirror: MirrorId::Rcsb,
            ..Default::default()
        });
        let pdb_id = PdbId::new("1abc").unwrap();

        let cs_url = downloader.build_nmr_cs_url(&pdb_id);
        assert_eq!(cs_url, "https://files.rcsb.org/download/1abc_cs.str.gz");

        let mr_url = downloader.build_nmr_mr_url(&pdb_id);
        assert_eq!(mr_url, "https://files.rcsb.org/download/1abc_mr.str.gz");
    }

    #[test]
    fn test_build_dest_path_for_task() {
        let downloader = HttpsDownloader::new(DownloadOptions::default());
        let pdb_id = PdbId::new("1abc").unwrap();
        let dest = Path::new("/tmp");

        // Structure
        let task = DownloadTask::structure(pdb_id.clone(), FileFormat::Mmcif);
        let path = downloader.build_dest_path_for_task(dest, &task);
        assert_eq!(path, PathBuf::from("/tmp/1abc.cif"));

        // Assembly
        let task = DownloadTask::assembly(pdb_id.clone(), 2);
        let path = downloader.build_dest_path_for_task(dest, &task);
        assert_eq!(path, PathBuf::from("/tmp/1abc-assembly2.cif.gz"));

        // Structure factors
        let task = DownloadTask::structure_factors(pdb_id.clone());
        let path = downloader.build_dest_path_for_task(dest, &task);
        assert_eq!(path, PathBuf::from("/tmp/r1abcsf.ent.gz"));

        // NMR CS
        let task = DownloadTask::nmr_chemical_shifts(pdb_id.clone());
        let path = downloader.build_dest_path_for_task(dest, &task);
        assert_eq!(path, PathBuf::from("/tmp/1abc_cs.str.gz"));

        // NMR MR
        let task = DownloadTask::nmr_restraints(pdb_id.clone());
        let path = downloader.build_dest_path_for_task(dest, &task);
        assert_eq!(path, PathBuf::from("/tmp/1abc_mr.str.gz"));
    }

    #[test]
    fn test_default_options() {
        let options = DownloadOptions::default();
        assert_eq!(options.parallel, 4);
        assert_eq!(options.retry_count, 3);
        assert_eq!(options.retry_delay, Duration::from_secs(1));
    }

    #[test]
    fn test_build_dest_path_extended() {
        let downloader = HttpsDownloader::new(DownloadOptions::default());
        let pdb_id = PdbId::new("pdb_00001abc").unwrap();

        let path = downloader.build_dest_path(Path::new("/tmp"), &pdb_id, FileFormat::Mmcif);
        assert_eq!(path, std::path::PathBuf::from("/tmp/pdb_00001abc.cif"));

        let path = downloader.build_dest_path(Path::new("/tmp"), &pdb_id, FileFormat::CifGz);
        assert_eq!(path, std::path::PathBuf::from("/tmp/pdb_00001abc.cif.gz"));
    }
}
