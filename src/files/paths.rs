use crate::files::PdbId;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum FileFormat {
    /// Legacy PDB format (decompressed)
    Pdb,
    /// mmCIF format (decompressed)
    Mmcif,
    /// BinaryCIF format
    Bcif,
    /// Compressed PDB format (.ent.gz)
    #[clap(name = "pdb-gz")]
    PdbGz,
    /// Compressed mmCIF format (.cif.gz)
    #[clap(name = "cif-gz")]
    CifGz,
    /// Compressed BinaryCIF format (.bcif.gz)
    #[clap(name = "bcif-gz")]
    BcifGz,
}

impl FileFormat {
    #[allow(dead_code)]
    pub fn extension(&self) -> &'static str {
        match self {
            FileFormat::Pdb => "pdb",
            FileFormat::Mmcif => "cif",
            FileFormat::Bcif => "bcif",
            FileFormat::PdbGz => "ent.gz",
            FileFormat::CifGz => "cif.gz",
            FileFormat::BcifGz => "bcif.gz",
        }
    }

    pub fn subdir(&self) -> &'static str {
        match self {
            FileFormat::Pdb | FileFormat::PdbGz => "pdb",
            FileFormat::Mmcif | FileFormat::CifGz => "mmCIF",
            FileFormat::Bcif | FileFormat::BcifGz => "bcif",
        }
    }

    /// Whether this format should be downloaded compressed
    pub fn is_compressed(&self) -> bool {
        matches!(
            self,
            FileFormat::PdbGz | FileFormat::CifGz | FileFormat::BcifGz
        )
    }

    /// Get the base format (uncompressed version)
    pub fn base_format(&self) -> FileFormat {
        match self {
            FileFormat::Pdb | FileFormat::PdbGz => FileFormat::Pdb,
            FileFormat::Mmcif | FileFormat::CifGz => FileFormat::Mmcif,
            FileFormat::Bcif | FileFormat::BcifGz => FileFormat::Bcif,
        }
    }
}

impl std::fmt::Display for FileFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileFormat::Pdb => write!(f, "pdb"),
            FileFormat::Mmcif => write!(f, "mmcif"),
            FileFormat::Bcif => write!(f, "bcif"),
            FileFormat::PdbGz => write!(f, "pdb-gz"),
            FileFormat::CifGz => write!(f, "cif-gz"),
            FileFormat::BcifGz => write!(f, "bcif-gz"),
        }
    }
}

impl std::str::FromStr for FileFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pdb" => Ok(FileFormat::Pdb),
            "mmcif" | "cif" => Ok(FileFormat::Mmcif),
            "bcif" => Ok(FileFormat::Bcif),
            "pdb-gz" | "pdbgz" | "ent.gz" => Ok(FileFormat::PdbGz),
            "cif-gz" | "cifgz" | "cif.gz" | "mmcif-gz" => Ok(FileFormat::CifGz),
            "bcif-gz" | "bcifgz" | "bcif.gz" => Ok(FileFormat::BcifGz),
            _ => Err(format!("Unknown format: {}", s)),
        }
    }
}

/// Build the relative path for a PDB file in the standard directory structure.
///
/// # Path patterns
///
/// ## Classic IDs (e.g., "1abc")
/// - PDB format: `pdb/ab/pdb1abc.ent.gz` (has "pdb" prefix in filename)
/// - mmCIF format: `mmCIF/ab/1abc.cif.gz`
/// - BCIF format: `bcif/ab/1abc.bcif.gz`
///
/// ## Extended IDs (e.g., "pdb_00001abc")
/// - PDB format: `pdb/01/pdb_00001abc.ent.gz` (no extra "pdb" prefix)
/// - mmCIF format: `mmCIF/01/pdb_00001abc.cif.gz`
/// - BCIF format: `bcif/01/pdb_00001abc.bcif.gz`
#[allow(dead_code)]
pub fn build_relative_path(pdb_id: &PdbId, format: FileFormat) -> PathBuf {
    let middle = pdb_id.middle_chars();
    let id = pdb_id.as_str();

    match format {
        FileFormat::Pdb => {
            // Classic IDs get "pdb" prefix in filename, extended IDs don't
            if pdb_id.is_classic() {
                PathBuf::from(format!("pdb/{}/pdb{}.pdb", middle, id))
            } else {
                PathBuf::from(format!("pdb/{}/{}.pdb", middle, id))
            }
        }
        FileFormat::Mmcif => PathBuf::from(format!("mmCIF/{}/{}.cif", middle, id)),
        FileFormat::Bcif => PathBuf::from(format!("bcif/{}/{}.bcif", middle, id)),
        FileFormat::PdbGz => {
            // Classic IDs get "pdb" prefix in filename, extended IDs don't
            if pdb_id.is_classic() {
                PathBuf::from(format!("pdb/{}/pdb{}.ent.gz", middle, id))
            } else {
                PathBuf::from(format!("pdb/{}/{}.ent.gz", middle, id))
            }
        }
        FileFormat::CifGz => PathBuf::from(format!("mmCIF/{}/{}.cif.gz", middle, id)),
        FileFormat::BcifGz => PathBuf::from(format!("bcif/{}/{}.bcif.gz", middle, id)),
    }
}

/// Build the full path for a PDB file given a base directory
#[allow(dead_code)]
pub fn build_full_path(base_dir: &std::path::Path, pdb_id: &PdbId, format: FileFormat) -> PathBuf {
    base_dir.join(build_relative_path(pdb_id, format))
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Classic ID path tests ===

    #[test]
    fn test_build_relative_path_classic_pdb() {
        let id = PdbId::new("1abc").unwrap();
        let path = build_relative_path(&id, FileFormat::PdbGz);
        assert_eq!(path, PathBuf::from("pdb/ab/pdb1abc.ent.gz"));
    }

    #[test]
    fn test_build_relative_path_classic_pdb_uncompressed() {
        let id = PdbId::new("1abc").unwrap();
        let path = build_relative_path(&id, FileFormat::Pdb);
        assert_eq!(path, PathBuf::from("pdb/ab/pdb1abc.pdb"));
    }

    #[test]
    fn test_build_relative_path_classic_mmcif() {
        let id = PdbId::new("1abc").unwrap();
        let path = build_relative_path(&id, FileFormat::CifGz);
        assert_eq!(path, PathBuf::from("mmCIF/ab/1abc.cif.gz"));
    }

    #[test]
    fn test_build_relative_path_classic_bcif() {
        let id = PdbId::new("1abc").unwrap();
        let path = build_relative_path(&id, FileFormat::BcifGz);
        assert_eq!(path, PathBuf::from("bcif/ab/1abc.bcif.gz"));
    }

    // === Extended ID path tests ===

    #[test]
    fn test_build_relative_path_extended_pdb() {
        let id = PdbId::new("pdb_00001abc").unwrap();
        let path = build_relative_path(&id, FileFormat::PdbGz);
        // Extended IDs don't get extra "pdb" prefix in filename
        // middle_chars for "pdb_00001abc" is "00" (positions 6-7)
        assert_eq!(path, PathBuf::from("pdb/00/pdb_00001abc.ent.gz"));
    }

    #[test]
    fn test_build_relative_path_extended_pdb_uncompressed() {
        let id = PdbId::new("pdb_00001abc").unwrap();
        let path = build_relative_path(&id, FileFormat::Pdb);
        assert_eq!(path, PathBuf::from("pdb/00/pdb_00001abc.pdb"));
    }

    #[test]
    fn test_build_relative_path_extended_mmcif() {
        let id = PdbId::new("pdb_00001abc").unwrap();
        let path = build_relative_path(&id, FileFormat::CifGz);
        assert_eq!(path, PathBuf::from("mmCIF/00/pdb_00001abc.cif.gz"));
    }

    #[test]
    fn test_build_relative_path_extended_bcif() {
        let id = PdbId::new("pdb_00001abc").unwrap();
        let path = build_relative_path(&id, FileFormat::BcifGz);
        assert_eq!(path, PathBuf::from("bcif/00/pdb_00001abc.bcif.gz"));
    }

    #[test]
    fn test_build_relative_path_extended_middle_chars() {
        // "pdb_12345678" → middle chars are positions 6-7 = "34"
        let id = PdbId::new("pdb_12345678").unwrap();
        let path = build_relative_path(&id, FileFormat::CifGz);
        assert_eq!(path, PathBuf::from("mmCIF/34/pdb_12345678.cif.gz"));
    }

    // === FileFormat tests ===

    #[test]
    fn test_format_from_str() {
        assert_eq!("cif-gz".parse::<FileFormat>().unwrap(), FileFormat::CifGz);
        assert_eq!("mmcif".parse::<FileFormat>().unwrap(), FileFormat::Mmcif);
        assert_eq!("pdb".parse::<FileFormat>().unwrap(), FileFormat::Pdb);
    }

    #[test]
    fn test_is_compressed() {
        assert!(!FileFormat::Pdb.is_compressed());
        assert!(!FileFormat::Mmcif.is_compressed());
        assert!(FileFormat::CifGz.is_compressed());
        assert!(FileFormat::PdbGz.is_compressed());
    }
}
