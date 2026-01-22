#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum FileFormat {
    /// Legacy PDB format (decompressed)
    Pdb,
    /// mmCIF format (decompressed)
    #[value(alias = "cif")]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_from_str() {
        assert_eq!("cif-gz".parse::<FileFormat>().unwrap(), FileFormat::CifGz);
        assert_eq!("mmcif".parse::<FileFormat>().unwrap(), FileFormat::Mmcif);
        assert_eq!("pdb".parse::<FileFormat>().unwrap(), FileFormat::Pdb);
    }

    #[test]
    fn test_format_alias_cif() {
        // "cif" should be an alias for "mmcif"
        use std::str::FromStr;
        assert_eq!(FileFormat::from_str("cif").unwrap(), FileFormat::Mmcif);
    }
}
