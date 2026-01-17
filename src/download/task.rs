//! Download task types for parallel download orchestration.

use crate::data_types::DataType;
use crate::files::{FileFormat, PdbId};
use std::path::PathBuf;

/// A single download task specification.
#[derive(Debug, Clone)]
pub struct DownloadTask {
    /// The PDB ID to download
    pub pdb_id: PdbId,
    /// Type of data to download (structures, assemblies, etc.)
    pub data_type: DataType,
    /// File format for the download
    pub format: FileFormat,
    /// Assembly number (for assemblies only, None for other types)
    pub assembly_number: Option<u8>,
}

impl DownloadTask {
    /// Create a new download task for structures.
    pub fn structure(pdb_id: PdbId, format: FileFormat) -> Self {
        Self {
            pdb_id,
            data_type: DataType::Structures,
            format,
            assembly_number: None,
        }
    }

    /// Create a new download task for a specific assembly.
    pub fn assembly(pdb_id: PdbId, assembly_number: u8) -> Self {
        Self {
            pdb_id,
            data_type: DataType::Assemblies,
            format: FileFormat::CifGz, // Assemblies are always mmCIF
            assembly_number: Some(assembly_number),
        }
    }

    /// Create a new download task for structure factors.
    pub fn structure_factors(pdb_id: PdbId) -> Self {
        Self {
            pdb_id,
            data_type: DataType::StructureFactors,
            format: FileFormat::PdbGz, // SF files are .ent.gz
            assembly_number: None,
        }
    }

    /// Create a new download task for NMR chemical shifts.
    pub fn nmr_chemical_shifts(pdb_id: PdbId) -> Self {
        Self {
            pdb_id,
            data_type: DataType::NmrChemicalShifts,
            format: FileFormat::CifGz, // CS files are .str.gz (using CifGz as placeholder)
            assembly_number: None,
        }
    }

    /// Create a new download task for NMR restraints.
    pub fn nmr_restraints(pdb_id: PdbId) -> Self {
        Self {
            pdb_id,
            data_type: DataType::NmrRestraints,
            format: FileFormat::CifGz, // MR files are .str.gz (using CifGz as placeholder)
            assembly_number: None,
        }
    }

    /// Get a display-friendly description of this task.
    pub fn description(&self) -> String {
        match self.data_type {
            DataType::Assemblies => {
                if let Some(n) = self.assembly_number {
                    format!("{} assembly {}", self.pdb_id, n)
                } else {
                    format!("{} assemblies", self.pdb_id)
                }
            }
            _ => format!("{} {}", self.pdb_id, self.data_type),
        }
    }
}

/// Result of a download operation.
#[derive(Debug)]
#[allow(dead_code)]
pub enum DownloadResult {
    /// Download completed successfully
    Success {
        pdb_id: PdbId,
        data_type: DataType,
        path: PathBuf,
    },
    /// Download failed with an error
    Failed {
        pdb_id: PdbId,
        data_type: DataType,
        error: String,
    },
    /// Download was skipped (e.g., file exists, 404 for optional assembly)
    Skipped {
        pdb_id: PdbId,
        data_type: DataType,
        reason: String,
    },
}

impl DownloadResult {
    /// Create a success result.
    pub fn success(pdb_id: PdbId, data_type: DataType, path: PathBuf) -> Self {
        Self::Success {
            pdb_id,
            data_type,
            path,
        }
    }

    /// Create a failed result.
    pub fn failed(pdb_id: PdbId, data_type: DataType, error: impl Into<String>) -> Self {
        Self::Failed {
            pdb_id,
            data_type,
            error: error.into(),
        }
    }

    /// Create a skipped result.
    pub fn skipped(pdb_id: PdbId, data_type: DataType, reason: impl Into<String>) -> Self {
        Self::Skipped {
            pdb_id,
            data_type,
            reason: reason.into(),
        }
    }

    /// Check if this is a success.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Check if this is a failure.
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// Check if this was skipped.
    pub fn is_skipped(&self) -> bool {
        matches!(self, Self::Skipped { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_description() {
        let pdb_id = PdbId::new("1abc").unwrap();

        let task = DownloadTask::structure(pdb_id.clone(), FileFormat::Mmcif);
        assert_eq!(task.description(), "1abc structures");

        let task = DownloadTask::assembly(pdb_id.clone(), 1);
        assert_eq!(task.description(), "1abc assembly 1");

        let task = DownloadTask::structure_factors(pdb_id.clone());
        assert_eq!(task.description(), "1abc structure-factors");
    }

    #[test]
    fn test_result_variants() {
        let pdb_id = PdbId::new("1abc").unwrap();

        let result = DownloadResult::success(
            pdb_id.clone(),
            DataType::Structures,
            PathBuf::from("/tmp/1abc.cif"),
        );
        assert!(result.is_success());
        assert!(!result.is_failed());
        assert!(!result.is_skipped());

        let result = DownloadResult::failed(pdb_id.clone(), DataType::Structures, "Network error");
        assert!(!result.is_success());
        assert!(result.is_failed());

        let result = DownloadResult::skipped(pdb_id.clone(), DataType::Structures, "File exists");
        assert!(!result.is_success());
        assert!(result.is_skipped());
    }
}
