//! Local statistics collection.

use crate::data_types::DataType;
use crate::error::Result;
use crate::files::FileFormat;
use crate::stats::types::*;
use chrono::{DateTime, Local};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Length of classic PDB IDs (e.g., "1abc")
const CLASSIC_PDB_ID_LEN: usize = 4;

/// Length of extended PDB IDs (e.g., "pdb_00001abc")
const EXTENDED_PDB_ID_LEN: usize = 12;

/// Length of "pdb" prefix in filenames like "pdb1abc.ent.gz"
const PDB_PREFIX_LEN: usize = 3;

/// Collector for local PDB statistics.
pub struct LocalStatsCollector {
    pdb_dir: PathBuf,
}

impl LocalStatsCollector {
    /// Create a new collector for the given PDB directory.
    pub fn new(pdb_dir: &Path) -> Self {
        Self {
            pdb_dir: pdb_dir.to_path_buf(),
        }
    }

    /// Collect statistics from the local PDB directory.
    ///
    /// # Arguments
    /// * `detailed` - Whether to collect detailed statistics (size distribution, min/max files)
    /// * `format_filter` - Optional filter for a specific file format
    /// * `data_type_filter` - Optional filter for a specific data type
    pub async fn collect(
        &self,
        detailed: bool,
        format_filter: Option<FileFormat>,
        data_type_filter: Option<DataType>,
    ) -> Result<LocalStats> {
        let mut total_entries = 0usize;
        let mut total_size = 0u64;
        let mut all_pdb_ids: HashSet<String> = HashSet::new();
        let mut by_format: BTreeMap<String, FormatStats> = BTreeMap::new();
        let mut by_data_type: BTreeMap<String, DataTypeStats> = BTreeMap::new();

        // For detailed stats
        let mut size_distribution = SizeDistribution::default();
        let mut min_size: Option<u64> = None;
        let mut max_size: Option<u64> = None;
        let mut min_file: Option<FileInfo> = None;
        let mut max_file: Option<FileInfo> = None;
        let mut oldest_file: Option<FileInfo> = None;
        let mut newest_file: Option<FileInfo> = None;
        let mut total_for_avg: u64 = 0;
        let mut count_for_avg: usize = 0;

        // Scan directories based on format filter
        let format_dirs = self.get_format_directories(format_filter);

        // Track unique PDB IDs per data type
        let mut pdb_ids_by_data_type: HashMap<String, HashSet<String>> = HashMap::new();

        for (format_name, subdir, data_type) in format_dirs {
            // Skip if data type filter is set and doesn't match
            if let Some(ref filter) = data_type_filter {
                if data_type != *filter {
                    continue;
                }
            }

            let dir_path = self.pdb_dir.join(subdir);
            if !dir_path.exists() {
                continue;
            }

            let mut type_pdb_ids: HashSet<String> = HashSet::new();

            // Scan the directory (it may have subdivisions like ab/, cd/, etc.)
            self.scan_directory(
                &dir_path,
                &format_name,
                &mut total_entries,
                &mut total_size,
                &mut all_pdb_ids,
                &mut type_pdb_ids,
                &mut by_format,
                detailed,
                &mut size_distribution,
                &mut min_size,
                &mut max_size,
                &mut min_file,
                &mut max_file,
                &mut oldest_file,
                &mut newest_file,
                &mut total_for_avg,
                &mut count_for_avg,
            )
            .await?;

            // Accumulate PDB IDs per data type
            let type_name = data_type.to_string();
            pdb_ids_by_data_type
                .entry(type_name)
                .or_default()
                .extend(type_pdb_ids);
        }

        // Set unique_pdb_ids from accumulated sets
        for (type_name, pdb_ids) in &pdb_ids_by_data_type {
            let entry = by_data_type.entry(type_name.clone()).or_default();
            entry.unique_pdb_ids = pdb_ids.len();
        }

        // Calculate by_data_type counts and sizes from by_format
        // This is a simplified approach - we map formats to data types
        self.aggregate_data_type_stats(&by_format, &mut by_data_type);

        let detailed_stats = if detailed && count_for_avg > 0 {
            Some(DetailedStats {
                min_size: min_size.unwrap_or(0),
                max_size: max_size.unwrap_or(0),
                avg_size: total_for_avg as f64 / count_for_avg as f64,
                smallest_file: min_file,
                largest_file: max_file,
                oldest_file,
                newest_file,
                size_distribution,
            })
        } else {
            None
        };

        Ok(LocalStats {
            total_entries,
            unique_pdb_ids: all_pdb_ids.len(),
            total_size,
            by_format,
            by_data_type,
            detailed: detailed_stats,
            last_sync: None,     // Will be set from history tracker
            last_download: None, // Will be set from history tracker
        })
    }

    /// Get the list of format directories to scan.
    fn get_format_directories(
        &self,
        format_filter: Option<FileFormat>,
    ) -> Vec<(String, &'static str, DataType)> {
        let all_dirs = vec![
            ("cif.gz".to_string(), "mmCIF", DataType::Structures),
            ("ent.gz".to_string(), "pdb", DataType::Structures),
            ("bcif.gz".to_string(), "bcif", DataType::Structures),
            (
                "assembly.cif.gz".to_string(),
                "assemblies",
                DataType::Assemblies,
            ),
            (
                "sf.ent.gz".to_string(),
                "structure_factors",
                DataType::StructureFactors,
            ),
        ];

        if let Some(filter) = format_filter {
            let filter_subdir = filter.subdir();
            all_dirs
                .into_iter()
                .filter(|(_, subdir, _)| *subdir == filter_subdir)
                .collect()
        } else {
            all_dirs
        }
    }

    /// Scan a directory recursively and collect statistics.
    #[allow(clippy::too_many_arguments)]
    async fn scan_directory(
        &self,
        dir_path: &Path,
        format_name: &str,
        total_entries: &mut usize,
        total_size: &mut u64,
        all_pdb_ids: &mut HashSet<String>,
        type_pdb_ids: &mut HashSet<String>,
        by_format: &mut BTreeMap<String, FormatStats>,
        detailed: bool,
        size_distribution: &mut SizeDistribution,
        min_size: &mut Option<u64>,
        max_size: &mut Option<u64>,
        min_file: &mut Option<FileInfo>,
        max_file: &mut Option<FileInfo>,
        oldest_file: &mut Option<FileInfo>,
        newest_file: &mut Option<FileInfo>,
        total_for_avg: &mut u64,
        count_for_avg: &mut usize,
    ) -> Result<()> {
        let mut entries = fs::read_dir(dir_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let metadata = entry.metadata().await?;

            if metadata.is_dir() {
                // Recurse into subdirectories (e.g., hash directories like "ab", "cd")
                Box::pin(self.scan_directory(
                    &path,
                    format_name,
                    total_entries,
                    total_size,
                    all_pdb_ids,
                    type_pdb_ids,
                    by_format,
                    detailed,
                    size_distribution,
                    min_size,
                    max_size,
                    min_file,
                    max_file,
                    oldest_file,
                    newest_file,
                    total_for_avg,
                    count_for_avg,
                ))
                .await?;
            } else if metadata.is_file() {
                let file_name = path.file_name().unwrap_or_default().to_string_lossy();

                // Extract PDB ID from filename
                if let Some(pdb_id) = self.extract_pdb_id(&file_name) {
                    let size = metadata.len();

                    *total_entries += 1;
                    *total_size += size;
                    all_pdb_ids.insert(pdb_id.clone());
                    type_pdb_ids.insert(pdb_id.clone());

                    // Update format stats
                    let format_entry = by_format.entry(format_name.to_string()).or_default();
                    format_entry.count += 1;
                    format_entry.size += size;

                    // Update detailed stats
                    if detailed {
                        size_distribution.add(size);
                        *total_for_avg += size;
                        *count_for_avg += 1;

                        let modified = metadata
                            .modified()
                            .ok()
                            .map(DateTime::<Local>::from)
                            .unwrap_or_else(Local::now);

                        let file_info = FileInfo {
                            pdb_id: pdb_id.clone(),
                            path: path.clone(),
                            size,
                            modified,
                        };

                        // Track min/max sizes
                        if min_size.is_none() || size < min_size.unwrap() {
                            *min_size = Some(size);
                            *min_file = Some(file_info.clone());
                        }
                        if max_size.is_none() || size > max_size.unwrap() {
                            *max_size = Some(size);
                            *max_file = Some(file_info.clone());
                        }

                        // Track oldest/newest files
                        if oldest_file.is_none()
                            || modified < oldest_file.as_ref().unwrap().modified
                        {
                            *oldest_file = Some(file_info.clone());
                        }
                        if newest_file.is_none()
                            || modified > newest_file.as_ref().unwrap().modified
                        {
                            *newest_file = Some(file_info);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Extract PDB ID from a filename.
    fn extract_pdb_id(&self, filename: &str) -> Option<String> {
        // Handle various filename patterns:
        // - 1abc.cif.gz -> 1abc
        // - pdb1abc.ent.gz -> 1abc
        // - pdb_00001abc.cif.gz -> pdb_00001abc
        // - 1abc-assembly1.cif.gz -> 1abc
        // - r1abcsf.ent.gz -> 1abc (structure factors)

        let name = filename.to_lowercase();

        // Extended PDB ID (pdb_XXXXXXXX)
        if name.starts_with("pdb_") && name.len() >= EXTENDED_PDB_ID_LEN {
            let id = &name[0..EXTENDED_PDB_ID_LEN];
            if id
                .chars()
                .skip(CLASSIC_PDB_ID_LEN)
                .all(|c| c.is_alphanumeric())
            {
                return Some(id.to_string());
            }
        }

        // Structure factors (r1abcsf.ent.gz)
        if name.starts_with('r') && name.contains("sf") {
            let id_part = &name[1..];
            if let Some(sf_pos) = id_part.find("sf") {
                let id = &id_part[..sf_pos];
                if id.len() == CLASSIC_PDB_ID_LEN {
                    return Some(id.to_string());
                }
            }
        }

        // PDB format with prefix (pdb1abc.ent.gz)
        if name.starts_with("pdb") && !name.starts_with("pdb_") {
            let id_part = &name[PDB_PREFIX_LEN..];
            if id_part.len() >= CLASSIC_PDB_ID_LEN {
                let id = &id_part[..CLASSIC_PDB_ID_LEN];
                if Self::is_classic_pdb_id(id) {
                    return Some(id.to_string());
                }
            }
        }

        // Standard pattern (1abc.cif.gz) or assembly (1abc-assembly1.cif.gz)
        let id_part = name.split(&['.', '-'][..]).next()?;
        if id_part.len() == CLASSIC_PDB_ID_LEN && Self::is_classic_pdb_id(id_part) {
            return Some(id_part.to_string());
        }

        None
    }

    /// Check if a string matches the classic 4-character PDB ID pattern.
    fn is_classic_pdb_id(s: &str) -> bool {
        if s.len() != CLASSIC_PDB_ID_LEN {
            return false;
        }
        let mut chars = s.chars();
        let first = chars.next().unwrap();
        first.is_ascii_digit() && chars.all(|c| c.is_alphanumeric())
    }

    /// Aggregate data type statistics from format statistics.
    fn aggregate_data_type_stats(
        &self,
        by_format: &BTreeMap<String, FormatStats>,
        by_data_type: &mut BTreeMap<String, DataTypeStats>,
    ) {
        // Map formats to data types
        for (format_name, format_stats) in by_format {
            let data_type = match format_name.as_str() {
                "cif.gz" | "ent.gz" | "bcif.gz" => "structures",
                "assembly.cif.gz" => "assemblies",
                "sf.ent.gz" => "structure-factors",
                _ => "other",
            };

            let entry = by_data_type.entry(data_type.to_string()).or_default();
            entry.count += format_stats.count;
            entry.size += format_stats.size;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs as tokio_fs;

    async fn create_test_structure(temp_dir: &Path) -> Result<()> {
        // Create mmCIF directory structure
        let mmcif_dir = temp_dir.join("mmCIF").join("ab");
        tokio_fs::create_dir_all(&mmcif_dir).await?;

        // Create test files
        tokio_fs::write(mmcif_dir.join("1abc.cif.gz"), b"test content 1").await?;
        tokio_fs::write(mmcif_dir.join("2abc.cif.gz"), b"test content 22").await?;

        // Create pdb directory structure
        let pdb_dir = temp_dir.join("pdb").join("ab");
        tokio_fs::create_dir_all(&pdb_dir).await?;
        tokio_fs::write(pdb_dir.join("pdb1abc.ent.gz"), b"pdb content").await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_collect_basic_stats() {
        let temp_dir = TempDir::new().unwrap();
        create_test_structure(temp_dir.path()).await.unwrap();

        let collector = LocalStatsCollector::new(temp_dir.path());
        let stats = collector.collect(false, None, None).await.unwrap();

        assert_eq!(stats.total_entries, 3);
        assert_eq!(stats.unique_pdb_ids, 2); // 1abc and 2abc
        assert!(stats.total_size > 0);
        assert!(stats.detailed.is_none());
    }

    #[tokio::test]
    async fn test_collect_with_format_filter() {
        let temp_dir = TempDir::new().unwrap();
        create_test_structure(temp_dir.path()).await.unwrap();

        let collector = LocalStatsCollector::new(temp_dir.path());
        let stats = collector
            .collect(false, Some(FileFormat::CifGz), None)
            .await
            .unwrap();

        // Should only count mmCIF files
        assert_eq!(stats.total_entries, 2);
    }

    #[tokio::test]
    async fn test_collect_detailed_stats() {
        let temp_dir = TempDir::new().unwrap();
        create_test_structure(temp_dir.path()).await.unwrap();

        let collector = LocalStatsCollector::new(temp_dir.path());
        let stats = collector.collect(true, None, None).await.unwrap();

        assert!(stats.detailed.is_some());
        let detailed = stats.detailed.unwrap();
        assert!(detailed.min_size > 0);
        assert!(detailed.max_size >= detailed.min_size);
        assert!(detailed.avg_size > 0.0);
        assert!(detailed.smallest_file.is_some());
        assert!(detailed.largest_file.is_some());
    }

    #[test]
    fn test_extract_pdb_id_mmcif() {
        let collector = LocalStatsCollector::new(Path::new("/tmp"));
        assert_eq!(
            collector.extract_pdb_id("1abc.cif.gz"),
            Some("1abc".to_string())
        );
    }

    #[test]
    fn test_extract_pdb_id_pdb_format() {
        let collector = LocalStatsCollector::new(Path::new("/tmp"));
        assert_eq!(
            collector.extract_pdb_id("pdb1abc.ent.gz"),
            Some("1abc".to_string())
        );
    }

    #[test]
    fn test_extract_pdb_id_extended() {
        let collector = LocalStatsCollector::new(Path::new("/tmp"));
        assert_eq!(
            collector.extract_pdb_id("pdb_00001abc.cif.gz"),
            Some("pdb_00001abc".to_string())
        );
    }

    #[test]
    fn test_extract_pdb_id_assembly() {
        let collector = LocalStatsCollector::new(Path::new("/tmp"));
        assert_eq!(
            collector.extract_pdb_id("1abc-assembly1.cif.gz"),
            Some("1abc".to_string())
        );
    }

    #[test]
    fn test_extract_pdb_id_structure_factors() {
        let collector = LocalStatsCollector::new(Path::new("/tmp"));
        assert_eq!(
            collector.extract_pdb_id("r1abcsf.ent.gz"),
            Some("1abc".to_string())
        );
    }

    #[test]
    fn test_is_classic_pdb_id() {
        assert!(LocalStatsCollector::is_classic_pdb_id("1abc"));
        assert!(LocalStatsCollector::is_classic_pdb_id("4hhb"));
        assert!(LocalStatsCollector::is_classic_pdb_id("9xyz"));
        assert!(!LocalStatsCollector::is_classic_pdb_id("abc1")); // Doesn't start with digit
        assert!(!LocalStatsCollector::is_classic_pdb_id("12345")); // Too long
        assert!(!LocalStatsCollector::is_classic_pdb_id("1ab")); // Too short
    }
}
