use crate::files::PdbId;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum FileFormat {
    Pdb,
    Mmcif,
    Bcif,
}

impl FileFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            FileFormat::Pdb => "ent.gz",
            FileFormat::Mmcif => "cif.gz",
            FileFormat::Bcif => "bcif.gz",
        }
    }

    pub fn subdir(&self) -> &'static str {
        match self {
            FileFormat::Pdb => "pdb",
            FileFormat::Mmcif => "mmCIF",
            FileFormat::Bcif => "bcif",
        }
    }
}

impl std::fmt::Display for FileFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileFormat::Pdb => write!(f, "pdb"),
            FileFormat::Mmcif => write!(f, "mmcif"),
            FileFormat::Bcif => write!(f, "bcif"),
        }
    }
}

/// Build the relative path for a PDB file in the standard directory structure
pub fn build_relative_path(pdb_id: &PdbId, format: FileFormat) -> PathBuf {
    let middle = pdb_id.middle_chars();
    let id = pdb_id.as_str();

    match format {
        FileFormat::Pdb => PathBuf::from(format!("pdb/{}/pdb{}.ent.gz", middle, id)),
        FileFormat::Mmcif => PathBuf::from(format!("mmCIF/{}/{}.cif.gz", middle, id)),
        FileFormat::Bcif => PathBuf::from(format!("bcif/{}/{}.bcif.gz", middle, id)),
    }
}

/// Build the full path for a PDB file given a base directory
pub fn build_full_path(base_dir: &std::path::Path, pdb_id: &PdbId, format: FileFormat) -> PathBuf {
    base_dir.join(build_relative_path(pdb_id, format))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_relative_path_pdb() {
        let id = PdbId::new("1abc").unwrap();
        let path = build_relative_path(&id, FileFormat::Pdb);
        assert_eq!(path, PathBuf::from("pdb/ab/pdb1abc.ent.gz"));
    }

    #[test]
    fn test_build_relative_path_mmcif() {
        let id = PdbId::new("1abc").unwrap();
        let path = build_relative_path(&id, FileFormat::Mmcif);
        assert_eq!(path, PathBuf::from("mmCIF/ab/1abc.cif.gz"));
    }

    #[test]
    fn test_build_relative_path_bcif() {
        let id = PdbId::new("1abc").unwrap();
        let path = build_relative_path(&id, FileFormat::Bcif);
        assert_eq!(path, PathBuf::from("bcif/ab/1abc.bcif.gz"));
    }
}
