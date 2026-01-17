//! Data types and layouts for PDB archive following wwPDB standard structure.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// PDB archive data types following wwPDB standard structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DataType {
    /// Coordinate files (structures/divided/mmCIF or pdb)
    #[value(alias = "st", alias = "struct")]
    Structures,
    /// Biological assemblies (assemblies/mmCIF/divided)
    #[value(alias = "asm", alias = "assembly")]
    Assemblies,
    /// Legacy biounit format (biounit/coordinates/divided)
    Biounit,
    /// Structure factors - X-ray diffraction data (structures/divided/structure_factors)
    #[value(alias = "sf", alias = "xray")]
    StructureFactors,
    /// NMR chemical shifts (structures/divided/nmr_chemical_shifts)
    #[value(alias = "nmr-cs", alias = "cs")]
    NmrChemicalShifts,
    /// NMR restraints (structures/divided/nmr_restraints)
    #[value(alias = "nmr-r")]
    NmrRestraints,
    /// Obsolete entries (structures/obsolete)
    Obsolete,
}

/// Directory layout options for PDB archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Layout {
    /// Hash-organized structure (e.g., mmCIF/{hash}/{id}.cif.gz)
    /// Files are distributed across subdirectories based on middle 2 chars of PDB ID.
    #[default]
    Divided,
    /// Flat structure (e.g., mmCIF/{id}.cif.gz)
    /// All files in a single directory.
    All,
}

impl DataType {
    /// Get the rsync subpath for this data type and layout.
    ///
    /// # Example
    /// ```
    /// use pdb_cli::data_types::{DataType, Layout};
    ///
    /// assert_eq!(
    ///     DataType::Structures.rsync_subpath(Layout::Divided),
    ///     "structures/divided"
    /// );
    /// ```
    pub fn rsync_subpath(&self, layout: Layout) -> &'static str {
        match (self, layout) {
            (DataType::Structures, Layout::Divided) => "structures/divided",
            (DataType::Structures, Layout::All) => "structures/all",
            (DataType::Assemblies, Layout::Divided) => "assemblies/mmCIF/divided",
            (DataType::Assemblies, Layout::All) => "assemblies/mmCIF/all",
            (DataType::Biounit, Layout::Divided) => "biounit/coordinates/divided",
            (DataType::Biounit, Layout::All) => "biounit/coordinates/all",
            (DataType::StructureFactors, Layout::Divided) => "structures/divided/structure_factors",
            (DataType::StructureFactors, Layout::All) => "structures/all/structure_factors",
            (DataType::NmrChemicalShifts, Layout::Divided) => {
                "structures/divided/nmr_chemical_shifts"
            }
            (DataType::NmrChemicalShifts, Layout::All) => "structures/all/nmr_chemical_shifts",
            (DataType::NmrRestraints, Layout::Divided) => "structures/divided/nmr_restraints",
            (DataType::NmrRestraints, Layout::All) => "structures/all/nmr_restraints",
            (DataType::Obsolete, Layout::Divided) => "structures/obsolete",
            (DataType::Obsolete, Layout::All) => "structures/obsolete",
        }
    }

    /// Get the filename pattern for this data type.
    ///
    /// # Arguments
    /// * `pdb_id` - The PDB ID (lowercase, 4 characters)
    ///
    /// # Returns
    /// Filename pattern (may include wildcards for assemblies/biounit)
    pub fn filename_pattern(&self, pdb_id: &str) -> String {
        match self {
            DataType::Structures => format!("{}.cif.gz", pdb_id),
            DataType::Assemblies => format!("{}-assembly*.cif.gz", pdb_id),
            DataType::Biounit => format!("{}.pdb*.gz", pdb_id),
            DataType::StructureFactors => format!("r{}sf.ent.gz", pdb_id),
            DataType::NmrChemicalShifts => format!("{}_cs.str.gz", pdb_id),
            DataType::NmrRestraints => format!("{}_mr.str.gz", pdb_id),
            DataType::Obsolete => format!("{}.cif.gz", pdb_id),
        }
    }

    /// Get a human-readable description of this data type.
    pub fn description(&self) -> &'static str {
        match self {
            DataType::Structures => "Coordinate files (mmCIF/PDB format)",
            DataType::Assemblies => "Biological assemblies (mmCIF format)",
            DataType::Biounit => "Biological assemblies (legacy PDB format)",
            DataType::StructureFactors => "Structure factors (X-ray diffraction data)",
            DataType::NmrChemicalShifts => "NMR chemical shifts",
            DataType::NmrRestraints => "NMR restraints",
            DataType::Obsolete => "Obsolete entries",
        }
    }

    /// Get all available data types.
    pub fn all() -> &'static [DataType] {
        &[
            DataType::Structures,
            DataType::Assemblies,
            DataType::Biounit,
            DataType::StructureFactors,
            DataType::NmrChemicalShifts,
            DataType::NmrRestraints,
            DataType::Obsolete,
        ]
    }
}

impl std::fmt::Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataType::Structures => write!(f, "structures"),
            DataType::Assemblies => write!(f, "assemblies"),
            DataType::Biounit => write!(f, "biounit"),
            DataType::StructureFactors => write!(f, "structure-factors"),
            DataType::NmrChemicalShifts => write!(f, "nmr-chemical-shifts"),
            DataType::NmrRestraints => write!(f, "nmr-restraints"),
            DataType::Obsolete => write!(f, "obsolete"),
        }
    }
}

impl std::fmt::Display for Layout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Layout::Divided => write!(f, "divided"),
            Layout::All => write!(f, "all"),
        }
    }
}

/// PDBj-specific data types (available only from PDBj mirror).
///
/// These data types are exclusive to the PDBj mirror and require
/// different rsync modules than the standard wwPDB data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PdbjDataType {
    /// EMDB (Electron Microscopy Data Bank) - rsync://rsync.pdbj.org/emdb/
    Emdb,
    /// PDB-IHM (Integrative/Hybrid Methods) - rsync://rsync.pdbj.org/pdb_ihm/
    #[serde(rename = "pdb-ihm")]
    #[value(name = "pdb-ihm")]
    PdbIhm,
    /// Derived data (pre-computed analyses) - rsync://rsync.pdbj.org/ftp_derived/
    Derived,
}

impl PdbjDataType {
    /// Get the rsync module name for this PDBj-specific data type.
    pub fn rsync_module(&self) -> &'static str {
        match self {
            PdbjDataType::Emdb => "emdb",
            PdbjDataType::PdbIhm => "pdb_ihm",
            PdbjDataType::Derived => "ftp_derived",
        }
    }

    /// Get a human-readable description of this data type.
    pub fn description(&self) -> &'static str {
        match self {
            PdbjDataType::Emdb => "EMDB (Electron Microscopy Data Bank)",
            PdbjDataType::PdbIhm => "PDB-IHM (Integrative/Hybrid Methods structures)",
            PdbjDataType::Derived => "Derived data (pre-computed analyses from PDBj)",
        }
    }

    /// Get all available PDBj data types.
    pub fn all() -> &'static [PdbjDataType] {
        &[
            PdbjDataType::Emdb,
            PdbjDataType::PdbIhm,
            PdbjDataType::Derived,
        ]
    }
}

impl std::fmt::Display for PdbjDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PdbjDataType::Emdb => write!(f, "emdb"),
            PdbjDataType::PdbIhm => write!(f, "pdb-ihm"),
            PdbjDataType::Derived => write!(f, "derived"),
        }
    }
}

/// PDBe-specific data types (available only from PDBe mirror).
///
/// These data types are exclusive to the PDBe mirror and use
/// different rsync paths than the standard wwPDB data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PdbeDataType {
    /// SIFTS (Structure Integration with Function, Taxonomy and Sequences)
    Sifts,
    /// PDBeChem v2 (chemical component dictionary)
    Pdbechem,
    /// Foldseek database
    Foldseek,
}

impl PdbeDataType {
    /// Get the rsync path for this PDBe-specific data type.
    pub fn rsync_path(&self) -> &'static str {
        match self {
            PdbeDataType::Sifts => "pub/databases/msd/sifts/",
            PdbeDataType::Pdbechem => "pub/databases/msd/pdbechem_v2/",
            PdbeDataType::Foldseek => "pub/databases/msd/foldseek/",
        }
    }

    /// Get a human-readable description of this data type.
    pub fn description(&self) -> &'static str {
        match self {
            PdbeDataType::Sifts => {
                "SIFTS (Structure Integration with Function, Taxonomy and Sequences)"
            }
            PdbeDataType::Pdbechem => "PDBeChem v2 (chemical component dictionary)",
            PdbeDataType::Foldseek => "Foldseek database for structural similarity search",
        }
    }

    /// Get all available PDBe data types.
    pub fn all() -> &'static [PdbeDataType] {
        &[
            PdbeDataType::Sifts,
            PdbeDataType::Pdbechem,
            PdbeDataType::Foldseek,
        ]
    }
}

impl std::fmt::Display for PdbeDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PdbeDataType::Sifts => write!(f, "sifts"),
            PdbeDataType::Pdbechem => write!(f, "pdbechem"),
            PdbeDataType::Foldseek => write!(f, "foldseek"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rsync_subpath_structures() {
        assert_eq!(
            DataType::Structures.rsync_subpath(Layout::Divided),
            "structures/divided"
        );
        assert_eq!(
            DataType::Structures.rsync_subpath(Layout::All),
            "structures/all"
        );
    }

    #[test]
    fn test_rsync_subpath_assemblies() {
        assert_eq!(
            DataType::Assemblies.rsync_subpath(Layout::Divided),
            "assemblies/mmCIF/divided"
        );
        assert_eq!(
            DataType::Assemblies.rsync_subpath(Layout::All),
            "assemblies/mmCIF/all"
        );
    }

    #[test]
    fn test_rsync_subpath_structure_factors() {
        assert_eq!(
            DataType::StructureFactors.rsync_subpath(Layout::Divided),
            "structures/divided/structure_factors"
        );
    }

    #[test]
    fn test_filename_pattern() {
        assert_eq!(DataType::Structures.filename_pattern("1abc"), "1abc.cif.gz");
        assert_eq!(
            DataType::StructureFactors.filename_pattern("1abc"),
            "r1abcsf.ent.gz"
        );
        assert_eq!(
            DataType::NmrChemicalShifts.filename_pattern("1abc"),
            "1abc_cs.str.gz"
        );
        assert_eq!(
            DataType::NmrRestraints.filename_pattern("1abc"),
            "1abc_mr.str.gz"
        );
    }

    #[test]
    fn test_filename_pattern_wildcards() {
        // Assemblies and biounit have wildcard patterns
        assert!(DataType::Assemblies.filename_pattern("1abc").contains('*'));
        assert!(DataType::Biounit.filename_pattern("1abc").contains('*'));
    }

    #[test]
    fn test_display() {
        assert_eq!(DataType::Structures.to_string(), "structures");
        assert_eq!(DataType::StructureFactors.to_string(), "structure-factors");
        assert_eq!(Layout::Divided.to_string(), "divided");
        assert_eq!(Layout::All.to_string(), "all");
    }

    #[test]
    fn test_default_layout() {
        assert_eq!(Layout::default(), Layout::Divided);
    }

    #[test]
    fn test_all_data_types() {
        let all = DataType::all();
        assert_eq!(all.len(), 7);
        assert!(all.contains(&DataType::Structures));
        assert!(all.contains(&DataType::Obsolete));
    }

    #[test]
    fn test_serde_serialization() {
        let dt = DataType::StructureFactors;
        let json = serde_json::to_string(&dt).unwrap();
        assert_eq!(json, "\"structure-factors\"");

        let layout = Layout::Divided;
        let json = serde_json::to_string(&layout).unwrap();
        assert_eq!(json, "\"divided\"");
    }

    #[test]
    fn test_serde_deserialization() {
        let dt: DataType = serde_json::from_str("\"structure-factors\"").unwrap();
        assert_eq!(dt, DataType::StructureFactors);

        let layout: Layout = serde_json::from_str("\"all\"").unwrap();
        assert_eq!(layout, Layout::All);
    }

    #[test]
    fn test_datatype_aliases() {
        use clap::ValueEnum;

        // Test alias parsing for structures
        assert_eq!(
            DataType::from_str("st", true).unwrap(),
            DataType::Structures
        );
        assert_eq!(
            DataType::from_str("struct", true).unwrap(),
            DataType::Structures
        );

        // Test alias parsing for assemblies
        assert_eq!(
            DataType::from_str("asm", true).unwrap(),
            DataType::Assemblies
        );
        assert_eq!(
            DataType::from_str("assembly", true).unwrap(),
            DataType::Assemblies
        );

        // Test alias parsing for structure-factors
        assert_eq!(
            DataType::from_str("sf", true).unwrap(),
            DataType::StructureFactors
        );
        assert_eq!(
            DataType::from_str("xray", true).unwrap(),
            DataType::StructureFactors
        );

        // Test alias parsing for nmr-chemical-shifts
        assert_eq!(
            DataType::from_str("nmr-cs", true).unwrap(),
            DataType::NmrChemicalShifts
        );
        assert_eq!(
            DataType::from_str("cs", true).unwrap(),
            DataType::NmrChemicalShifts
        );

        // Test alias parsing for nmr-restraints
        assert_eq!(
            DataType::from_str("nmr-r", true).unwrap(),
            DataType::NmrRestraints
        );
    }

    // Tests for PdbjDataType
    #[test]
    fn test_pdbj_data_type_rsync_module() {
        assert_eq!(PdbjDataType::Emdb.rsync_module(), "emdb");
        assert_eq!(PdbjDataType::PdbIhm.rsync_module(), "pdb_ihm");
        assert_eq!(PdbjDataType::Derived.rsync_module(), "ftp_derived");
    }

    #[test]
    fn test_pdbj_data_type_display() {
        assert_eq!(PdbjDataType::Emdb.to_string(), "emdb");
        assert_eq!(PdbjDataType::PdbIhm.to_string(), "pdb-ihm");
        assert_eq!(PdbjDataType::Derived.to_string(), "derived");
    }

    #[test]
    fn test_pdbj_data_type_description() {
        assert!(PdbjDataType::Emdb.description().contains("EMDB"));
        assert!(PdbjDataType::PdbIhm.description().contains("IHM"));
        assert!(PdbjDataType::Derived.description().contains("Derived"));
    }

    #[test]
    fn test_pdbj_data_type_serde() {
        let dt = PdbjDataType::PdbIhm;
        let json = serde_json::to_string(&dt).unwrap();
        assert_eq!(json, "\"pdb-ihm\"");

        let parsed: PdbjDataType = serde_json::from_str("\"emdb\"").unwrap();
        assert_eq!(parsed, PdbjDataType::Emdb);
    }

    #[test]
    fn test_pdbj_data_type_all() {
        let all = PdbjDataType::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&PdbjDataType::Emdb));
        assert!(all.contains(&PdbjDataType::PdbIhm));
        assert!(all.contains(&PdbjDataType::Derived));
    }

    // Tests for PdbeDataType
    #[test]
    fn test_pdbe_data_type_rsync_path() {
        assert_eq!(PdbeDataType::Sifts.rsync_path(), "pub/databases/msd/sifts/");
        assert_eq!(
            PdbeDataType::Pdbechem.rsync_path(),
            "pub/databases/msd/pdbechem_v2/"
        );
        assert_eq!(
            PdbeDataType::Foldseek.rsync_path(),
            "pub/databases/msd/foldseek/"
        );
    }

    #[test]
    fn test_pdbe_data_type_display() {
        assert_eq!(PdbeDataType::Sifts.to_string(), "sifts");
        assert_eq!(PdbeDataType::Pdbechem.to_string(), "pdbechem");
        assert_eq!(PdbeDataType::Foldseek.to_string(), "foldseek");
    }

    #[test]
    fn test_pdbe_data_type_description() {
        assert!(PdbeDataType::Sifts.description().contains("SIFTS"));
        assert!(PdbeDataType::Pdbechem.description().contains("PDBeChem"));
        assert!(PdbeDataType::Foldseek.description().contains("Foldseek"));
    }

    #[test]
    fn test_pdbe_data_type_serde() {
        let dt = PdbeDataType::Sifts;
        let json = serde_json::to_string(&dt).unwrap();
        assert_eq!(json, "\"sifts\"");

        let parsed: PdbeDataType = serde_json::from_str("\"foldseek\"").unwrap();
        assert_eq!(parsed, PdbeDataType::Foldseek);
    }

    #[test]
    fn test_pdbe_data_type_all() {
        let all = PdbeDataType::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&PdbeDataType::Sifts));
        assert!(all.contains(&PdbeDataType::Pdbechem));
        assert!(all.contains(&PdbeDataType::Foldseek));
    }
}
