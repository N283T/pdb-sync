//! rsync-based synchronization for PDB data.

use crate::data_types::{DataType, Layout};
use crate::error::{PdbCliError, Result};
use crate::files::FileFormat;
use crate::mirrors::{Mirror, MirrorId};
use crate::sync::SyncProgress;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// Options for rsync synchronization.
pub struct RsyncOptions {
    pub mirror: MirrorId,
    pub data_types: Vec<DataType>,
    pub formats: Vec<FileFormat>,
    pub layout: Layout,
    pub delete: bool,
    pub bwlimit: Option<u32>,
    pub dry_run: bool,
    pub filters: Vec<String>,
    pub show_progress: bool,
}

impl Default for RsyncOptions {
    fn default() -> Self {
        Self {
            mirror: MirrorId::Rcsb,
            data_types: vec![DataType::Structures],
            formats: vec![FileFormat::Mmcif],
            layout: Layout::Divided,
            delete: false,
            bwlimit: None,
            dry_run: false,
            filters: Vec::new(),
            show_progress: false,
        }
    }
}

/// Result of a single data type synchronization.
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub data_type: DataType,
    pub layout: Layout,
    pub format: Option<FileFormat>,
    pub files_count: u64,
    pub bytes_transferred: u64,
    pub success: bool,
}

pub struct RsyncRunner {
    options: RsyncOptions,
}

impl RsyncRunner {
    pub fn new(options: RsyncOptions) -> Self {
        Self { options }
    }

    /// Run synchronization for all configured data types.
    pub async fn run(&self, dest: &Path) -> Result<Vec<SyncResult>> {
        let mirror = Mirror::get(self.options.mirror);
        let mut results = Vec::new();

        for data_type in &self.options.data_types {
            let applicable_formats = self.applicable_formats(*data_type);

            if applicable_formats.is_empty() {
                // Data type doesn't use format subdirectories (e.g., structure_factors)
                let result = self.sync_data_type(mirror, *data_type, None, dest).await?;
                results.push(result);
            } else {
                // Sync each applicable format
                for format in applicable_formats {
                    let result = self
                        .sync_data_type(mirror, *data_type, Some(format), dest)
                        .await?;
                    results.push(result);
                }
            }
        }

        Ok(results)
    }

    /// Get applicable formats for a data type.
    ///
    /// Only Structures supports format selection (mmCIF or PDB).
    /// For other data types, the format is either:
    /// - Already baked into the rsync_subpath (Assemblies=mmCIF, Biounit=PDB)
    /// - Not applicable (StructureFactors, NMR*, Obsolete)
    fn applicable_formats(&self, data_type: DataType) -> Vec<FileFormat> {
        match data_type {
            // Only Structures supports user-selected formats
            DataType::Structures => self.options.formats.clone(),
            // Format is already in rsync_subpath, no subdirectory needed
            DataType::Assemblies | DataType::Biounit => vec![],
            // No format subdirectories at all
            DataType::StructureFactors
            | DataType::NmrChemicalShifts
            | DataType::NmrRestraints
            | DataType::Obsolete => vec![],
        }
    }

    /// Sync a single data type with optional format.
    async fn sync_data_type(
        &self,
        mirror: &Mirror,
        data_type: DataType,
        format: Option<FileFormat>,
        dest: &Path,
    ) -> Result<SyncResult> {
        let source_subpath = self.build_source_subpath(data_type, format);
        let dest_path = self.build_dest_path(dest, data_type, format);
        let source = mirror.rsync_url(&source_subpath);

        std::fs::create_dir_all(&dest_path)?;

        let description = match format {
            Some(f) => format!("Syncing {} ({})", data_type, f.subdir()),
            None => format!("Syncing {}", data_type),
        };

        tracing::info!("Running rsync from {} to {}", source, dest_path.display());

        let (files_count, bytes_transferred, success) = if self.options.show_progress {
            self.run_with_progress(mirror, &source, &dest_path, &description)
                .await?
        } else {
            self.run_simple(mirror, &source, &dest_path).await?
        };

        Ok(SyncResult {
            data_type,
            layout: self.options.layout,
            format,
            files_count,
            bytes_transferred,
            success,
        })
    }

    /// Build source rsync subpath for a data type and format.
    fn build_source_subpath(&self, data_type: DataType, format: Option<FileFormat>) -> String {
        let base = data_type.rsync_subpath(self.options.layout);
        match format {
            Some(f) => format!("{}/{}/", base, f.subdir()),
            None => format!("{}/", base),
        }
    }

    /// Build local destination path for a data type and format.
    fn build_dest_path(
        &self,
        base: &Path,
        data_type: DataType,
        format: Option<FileFormat>,
    ) -> PathBuf {
        // For structures, we use format subdir directly
        // For other types, include data type in path
        match data_type {
            DataType::Structures => match format {
                Some(f) => base.join(f.subdir()),
                None => base.to_path_buf(),
            },
            _ => {
                let type_dir = data_type.to_string().replace('-', "_");
                match format {
                    Some(f) => base.join(&type_dir).join(f.subdir()),
                    None => base.join(&type_dir),
                }
            }
        }
    }

    /// Run rsync with progress display.
    async fn run_with_progress(
        &self,
        mirror: &Mirror,
        source: &str,
        dest: &Path,
        description: &str,
    ) -> Result<(u64, u64, bool)> {
        let mut cmd = self.build_rsync_command(mirror, source, dest);
        let mut progress = SyncProgress::new(description);

        let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Some(line) = lines.next_line().await? {
                progress.parse_line(&line);
            }
        }

        let status = child.wait().await?;
        let (files, bytes) = progress.stats();

        if status.success() {
            progress.finish(&format!(
                "Completed: {} files, {}",
                files,
                human_bytes(bytes)
            ));
        } else {
            progress.finish(&format!("Failed with status {}", status));
        }

        if !status.success() {
            return Err(PdbCliError::Rsync(format!(
                "rsync exited with status {}",
                status
            )));
        }

        Ok((files, bytes, true))
    }

    /// Run rsync without progress display (simpler output).
    async fn run_simple(
        &self,
        mirror: &Mirror,
        source: &str,
        dest: &Path,
    ) -> Result<(u64, u64, bool)> {
        let mut cmd = self.build_rsync_command(mirror, source, dest);

        let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Some(line) = lines.next_line().await? {
                println!("{}", line);
            }
        }

        let status = child.wait().await?;

        if !status.success() {
            return Err(PdbCliError::Rsync(format!(
                "rsync exited with status {}",
                status
            )));
        }

        // Simple mode doesn't track precise stats
        Ok((0, 0, true))
    }

    /// Build the rsync command with all options.
    fn build_rsync_command(&self, mirror: &Mirror, source: &str, dest: &Path) -> Command {
        let mut cmd = Command::new("rsync");

        // Base options
        cmd.arg("-avz").arg("--progress");

        // Port specification for RCSB
        if let Some(port) = mirror.rsync_port {
            cmd.arg(format!("--port={}", port));
        }

        // Delete option
        if self.options.delete {
            cmd.arg("--delete");
        }

        // Bandwidth limit
        if let Some(limit) = self.options.bwlimit {
            if limit > 0 {
                cmd.arg(format!("--bwlimit={}", limit));
            }
        }

        // Dry run
        if self.options.dry_run {
            cmd.arg("--dry-run");
        }

        // Include filters
        for filter in &self.options.filters {
            cmd.arg(format!("--include=**/{}*", filter));
        }

        // Source and destination
        cmd.arg(source);
        cmd.arg(dest.to_string_lossy().as_ref());

        tracing::debug!("Command: {:?}", cmd);

        cmd
    }

    /// Build the rsync command for display purposes (dry-run preview).
    pub fn build_command_string(&self, dest: &Path) -> Vec<String> {
        let mirror = Mirror::get(self.options.mirror);
        let mut args = vec![
            "rsync".to_string(),
            "-avz".to_string(),
            "--progress".to_string(),
        ];

        if let Some(port) = mirror.rsync_port {
            args.push(format!("--port={}", port));
        }

        if self.options.delete {
            args.push("--delete".to_string());
        }

        if let Some(limit) = self.options.bwlimit {
            if limit > 0 {
                args.push(format!("--bwlimit={}", limit));
            }
        }

        if self.options.dry_run {
            args.push("--dry-run".to_string());
        }

        for filter in &self.options.filters {
            args.push(format!("--include=**/{}*", filter));
        }

        // Add source paths for each data type
        for data_type in &self.options.data_types {
            let applicable_formats = self.applicable_formats(*data_type);

            if applicable_formats.is_empty() {
                let subpath = self.build_source_subpath(*data_type, None);
                args.push(mirror.rsync_url(&subpath));
            } else {
                for format in applicable_formats {
                    let subpath = self.build_source_subpath(*data_type, Some(format));
                    args.push(mirror.rsync_url(&subpath));
                }
            }
        }

        args.push(dest.to_string_lossy().to_string());

        args
    }
}

/// Format bytes as human-readable string.
fn human_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options() {
        let opts = RsyncOptions::default();
        assert_eq!(opts.mirror, MirrorId::Rcsb);
        assert_eq!(opts.data_types, vec![DataType::Structures]);
        assert_eq!(opts.formats, vec![FileFormat::Mmcif]);
        assert_eq!(opts.layout, Layout::Divided);
    }

    #[test]
    fn test_applicable_formats_structures() {
        let opts = RsyncOptions {
            formats: vec![FileFormat::Mmcif, FileFormat::Pdb],
            ..Default::default()
        };
        let runner = RsyncRunner::new(opts);
        assert_eq!(
            runner.applicable_formats(DataType::Structures),
            vec![FileFormat::Mmcif, FileFormat::Pdb]
        );
    }

    #[test]
    fn test_applicable_formats_assemblies() {
        // Assemblies has format baked into rsync_subpath, no additional format dir
        let runner = RsyncRunner::new(RsyncOptions::default());
        assert!(runner.applicable_formats(DataType::Assemblies).is_empty());
    }

    #[test]
    fn test_applicable_formats_biounit() {
        // Biounit has format baked into rsync_subpath, no additional format dir
        let runner = RsyncRunner::new(RsyncOptions::default());
        assert!(runner.applicable_formats(DataType::Biounit).is_empty());
    }

    #[test]
    fn test_applicable_formats_no_format() {
        let runner = RsyncRunner::new(RsyncOptions::default());
        assert!(runner
            .applicable_formats(DataType::StructureFactors)
            .is_empty());
        assert!(runner
            .applicable_formats(DataType::NmrChemicalShifts)
            .is_empty());
    }

    #[test]
    fn test_build_source_subpath_structures() {
        let runner = RsyncRunner::new(RsyncOptions::default());
        assert_eq!(
            runner.build_source_subpath(DataType::Structures, Some(FileFormat::Mmcif)),
            "structures/divided/mmCIF/"
        );
    }

    #[test]
    fn test_build_source_subpath_assemblies() {
        // Assemblies path already includes format, no additional format needed
        let runner = RsyncRunner::new(RsyncOptions::default());
        assert_eq!(
            runner.build_source_subpath(DataType::Assemblies, None),
            "assemblies/mmCIF/divided/"
        );
    }

    #[test]
    fn test_build_source_subpath_biounit() {
        let runner = RsyncRunner::new(RsyncOptions::default());
        assert_eq!(
            runner.build_source_subpath(DataType::Biounit, None),
            "biounit/coordinates/divided/"
        );
    }

    #[test]
    fn test_build_source_subpath_structure_factors() {
        let runner = RsyncRunner::new(RsyncOptions::default());
        assert_eq!(
            runner.build_source_subpath(DataType::StructureFactors, None),
            "structures/divided/structure_factors/"
        );
    }

    #[test]
    fn test_build_dest_path_structures() {
        let runner = RsyncRunner::new(RsyncOptions::default());
        let base = Path::new("/tmp/pdb");
        assert_eq!(
            runner.build_dest_path(base, DataType::Structures, Some(FileFormat::Mmcif)),
            PathBuf::from("/tmp/pdb/mmCIF")
        );
    }

    #[test]
    fn test_build_dest_path_assemblies() {
        let runner = RsyncRunner::new(RsyncOptions::default());
        let base = Path::new("/tmp/pdb");
        assert_eq!(
            runner.build_dest_path(base, DataType::Assemblies, None),
            PathBuf::from("/tmp/pdb/assemblies")
        );
    }

    #[test]
    fn test_build_dest_path_structure_factors() {
        let runner = RsyncRunner::new(RsyncOptions::default());
        let base = Path::new("/tmp/pdb");
        assert_eq!(
            runner.build_dest_path(base, DataType::StructureFactors, None),
            PathBuf::from("/tmp/pdb/structure_factors")
        );
    }
}
