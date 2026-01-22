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
    /// use pdb_sync::data_types::{DataType, Layout};
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
///
/// Data stored in pub/ (wwPDB common):
/// - emdb, pdb-ihm, derived
///
/// Data stored in pdbj/ (PDBj-specific):
/// - bsma, efsite, pdb-nextgen, pdb-versioned, pdbjplus, promode, uniprot, xrda
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PdbjDataType {
    /// EMDB (Electron Microscopy Data Bank) - rsync://rsync.pdbj.org/emdb/ → pub/emdb/
    Emdb,
    /// PDB-IHM (Integrative/Hybrid Methods) - rsync://rsync.pdbj.org/pdb_ihm/ → pub/pdb_ihm/
    #[serde(rename = "pdb-ihm")]
    #[value(name = "pdb-ihm")]
    PdbIhm,
    /// Derived data (pre-computed analyses) - rsync://rsync.pdbj.org/ftp_derived/ → pub/derived_data/
    Derived,

    // PDBj-specific directories (stored in pdbj/)
    /// BSM-Arc (Binding Site Matrix archive)
    Bsma,
    /// eF-site (electrostatic potential field database)
    Efsite,
    /// PDB NextGen archive
    #[serde(rename = "pdb-nextgen")]
    #[value(name = "pdb-nextgen")]
    PdbNextgen,
    /// PDB Versioned archive
    #[serde(rename = "pdb-versioned")]
    #[value(name = "pdb-versioned")]
    PdbVersioned,
    /// PDBjPlus (data generated by PDBj for wwPDB data)
    Pdbjplus,
    /// Promode (protein model database)
    Promode,
    /// UniProt integration data
    Uniprot,
    /// XRDA (X-ray Data Analysis)
    Xrda,
}

impl PdbjDataType {
    /// Whether this data type should be stored in pub/ (wwPDB common) or pdbj/ (PDBj-specific)
    pub fn is_pub_data(&self) -> bool {
        matches!(
            self,
            PdbjDataType::Emdb | PdbjDataType::PdbIhm | PdbjDataType::Derived
        )
    }

    /// Get the destination subdirectory relative to base
    /// For pub/ data: returns "pub/emdb", "pub/pdb_ihm", "pub/derived_data"
    /// For pdbj/ data: returns "pdbj/bsma", "pdbj/efsite", etc.
    pub fn dest_subdir(&self) -> &'static str {
        match self {
            PdbjDataType::Emdb => "pub/emdb",
            PdbjDataType::PdbIhm => "pub/pdb_ihm",
            PdbjDataType::Derived => "pub/derived_data",
            PdbjDataType::Bsma => "pdbj/bsma",
            PdbjDataType::Efsite => "pdbj/efsite",
            PdbjDataType::PdbNextgen => "pdbj/pdb_nextgen",
            PdbjDataType::PdbVersioned => "pdbj/pdb_versioned",
            PdbjDataType::Pdbjplus => "pdbj/pdbjplus",
            PdbjDataType::Promode => "pdbj/promode",
            PdbjDataType::Uniprot => "pdbj/uniprot",
            PdbjDataType::Xrda => "pdbj/xrda",
        }
    }

    /// Get the rsync subpath for this PDBj data type.
    ///
    /// Returns the path component for `data.pdbj.org::rsync/{path}/`
    /// Note: The rsync module is always "rsync", only the path varies.
    pub fn rsync_module(&self) -> &'static str {
        match self {
            // pub/ data (wwPDB common): path under pub/
            PdbjDataType::Emdb => "pub/emdb",
            PdbjDataType::PdbIhm => "pub/pdb_ihm",
            PdbjDataType::Derived => "pub/derived_data",
            // pdbj/ data (PDBj-specific): path is the directory name
            PdbjDataType::Bsma => "pdbj/bsma",
            PdbjDataType::Efsite => "pdbj/efsite",
            PdbjDataType::PdbNextgen => "pdbj/pdb_nextgen",
            PdbjDataType::PdbVersioned => "pdbj/pdb_versioned",
            PdbjDataType::Pdbjplus => "pdbj/pdbjplus",
            PdbjDataType::Promode => "pdbj/promode",
            PdbjDataType::Uniprot => "pdbj/uniprot",
            PdbjDataType::Xrda => "pdbj/xrda",
        }
    }

    /// Get a human-readable description of this data type.
    pub fn description(&self) -> &'static str {
        match self {
            PdbjDataType::Emdb => "EMDB (Electron Microscopy Data Bank)",
            PdbjDataType::PdbIhm => "PDB-IHM (Integrative/Hybrid Methods structures)",
            PdbjDataType::Derived => "Derived data (pre-computed analyses from PDBj)",
            PdbjDataType::Bsma => "BSM-Arc (Binding Site Matrix archive)",
            PdbjDataType::Efsite => "eF-site (electrostatic potential field database)",
            PdbjDataType::PdbNextgen => "PDB NextGen archive",
            PdbjDataType::PdbVersioned => "PDB Versioned archive",
            PdbjDataType::Pdbjplus => "PDBjPlus (data generated by PDBj)",
            PdbjDataType::Promode => "Promode (protein model database)",
            PdbjDataType::Uniprot => "UniProt integration data",
            PdbjDataType::Xrda => "XRDA (X-ray Data Analysis)",
        }
    }

    /// Get all available PDBj data types.
    pub fn all() -> &'static [PdbjDataType] {
        &[
            PdbjDataType::Emdb,
            PdbjDataType::PdbIhm,
            PdbjDataType::Derived,
            PdbjDataType::Bsma,
            PdbjDataType::Efsite,
            PdbjDataType::PdbNextgen,
            PdbjDataType::PdbVersioned,
            PdbjDataType::Pdbjplus,
            PdbjDataType::Promode,
            PdbjDataType::Uniprot,
            PdbjDataType::Xrda,
        ]
    }
}

impl std::fmt::Display for PdbjDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PdbjDataType::Emdb => write!(f, "emdb"),
            PdbjDataType::PdbIhm => write!(f, "pdb-ihm"),
            PdbjDataType::Derived => write!(f, "derived"),
            PdbjDataType::Bsma => write!(f, "bsma"),
            PdbjDataType::Efsite => write!(f, "efsite"),
            PdbjDataType::PdbNextgen => write!(f, "pdb-nextgen"),
            PdbjDataType::PdbVersioned => write!(f, "pdb-versioned"),
            PdbjDataType::Pdbjplus => write!(f, "pdbjplus"),
            PdbjDataType::Promode => write!(f, "promode"),
            PdbjDataType::Uniprot => write!(f, "uniprot"),
            PdbjDataType::Xrda => write!(f, "xrda"),
        }
    }
}

/// PDBe-specific data types (available only from PDBe mirror).
///
/// These data types are exclusive to the PDBe mirror and use
/// different rsync paths than the standard wwPDB data.
///
/// All 11 directories match the init template and are available at:
/// rsync://rsync.ebi.ac.uk/pub/databases/msd/
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PdbeDataType {
    /// Assemblies data
    Assemblies,
    /// Foldseek database
    Foldseek,
    /// Fragment screening data
    #[serde(rename = "fragment-screening")]
    #[value(name = "fragment-screening")]
    FragmentScreening,
    /// GraphDB data
    Graphdb,
    /// NMR data
    Nmr,
    /// PDB assemblies analysis
    #[serde(rename = "pdb-assemblies-analysis")]
    #[value(name = "pdb-assemblies-analysis")]
    PdbAssembliesAnalysis,
    /// Uncompressed PDB files
    #[serde(rename = "pdb-uncompressed")]
    #[value(name = "pdb-uncompressed")]
    PdbUncompressed,
    /// PDBeChem v2 (chemical component dictionary)
    Pdbechem,
    /// SIFTS (Structure Integration with Function, Taxonomy and Sequences)
    Sifts,
    /// Status files
    Status,
    /// Updated mmCIF files
    #[serde(rename = "updated-mmcif")]
    #[value(name = "updated-mmcif")]
    UpdatedMmcif,
}

impl PdbeDataType {
    /// Get the rsync path for this PDBe-specific data type.
    pub fn rsync_path(&self) -> &'static str {
        match self {
            PdbeDataType::Assemblies => "pub/databases/msd/assemblies/",
            PdbeDataType::Foldseek => "pub/databases/msd/foldseek/",
            PdbeDataType::FragmentScreening => "pub/databases/msd/fragment_screening/",
            PdbeDataType::Graphdb => "pub/databases/msd/graphdb/",
            PdbeDataType::Nmr => "pub/databases/msd/nmr/",
            PdbeDataType::PdbAssembliesAnalysis => "pub/databases/msd/pdb-assemblies-analysis/",
            PdbeDataType::PdbUncompressed => "pub/databases/msd/pdb_uncompressed/",
            PdbeDataType::Pdbechem => "pub/databases/msd/pdbechem_v2/",
            PdbeDataType::Sifts => "pub/databases/msd/sifts/",
            PdbeDataType::Status => "pub/databases/msd/status/",
            PdbeDataType::UpdatedMmcif => "pub/databases/msd/updated_mmcif/",
        }
    }

    /// Get the destination subdirectory name (for pdbe/)
    pub fn dest_subdir(&self) -> &'static str {
        match self {
            PdbeDataType::Assemblies => "assemblies",
            PdbeDataType::Foldseek => "foldseek",
            PdbeDataType::FragmentScreening => "fragment_screening",
            PdbeDataType::Graphdb => "graphdb",
            PdbeDataType::Nmr => "nmr",
            PdbeDataType::PdbAssembliesAnalysis => "pdb-assemblies-analysis",
            PdbeDataType::PdbUncompressed => "pdb_uncompressed",
            PdbeDataType::Pdbechem => "pdbechem_v2",
            PdbeDataType::Sifts => "sifts",
            PdbeDataType::Status => "status",
            PdbeDataType::UpdatedMmcif => "updated_mmcif",
        }
    }

    /// Get a human-readable description of this data type.
    pub fn description(&self) -> &'static str {
        match self {
            PdbeDataType::Assemblies => "Assemblies data",
            PdbeDataType::Foldseek => "Foldseek database for structural similarity search",
            PdbeDataType::FragmentScreening => "Fragment screening data",
            PdbeDataType::Graphdb => "GraphDB data",
            PdbeDataType::Nmr => "NMR data",
            PdbeDataType::PdbAssembliesAnalysis => "PDB assemblies analysis",
            PdbeDataType::PdbUncompressed => "Uncompressed PDB files",
            PdbeDataType::Pdbechem => "PDBeChem v2 (chemical component dictionary)",
            PdbeDataType::Sifts => {
                "SIFTS (Structure Integration with Function, Taxonomy and Sequences)"
            }
            PdbeDataType::Status => "Status files",
            PdbeDataType::UpdatedMmcif => "Updated mmCIF files",
        }
    }

    /// Get all available PDBe data types.
    pub fn all() -> &'static [PdbeDataType] {
        &[
            PdbeDataType::Assemblies,
            PdbeDataType::Foldseek,
            PdbeDataType::FragmentScreening,
            PdbeDataType::Graphdb,
            PdbeDataType::Nmr,
            PdbeDataType::PdbAssembliesAnalysis,
            PdbeDataType::PdbUncompressed,
            PdbeDataType::Pdbechem,
            PdbeDataType::Sifts,
            PdbeDataType::Status,
            PdbeDataType::UpdatedMmcif,
        ]
    }
}

impl std::fmt::Display for PdbeDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PdbeDataType::Assemblies => write!(f, "assemblies"),
            PdbeDataType::Foldseek => write!(f, "foldseek"),
            PdbeDataType::FragmentScreening => write!(f, "fragment-screening"),
            PdbeDataType::Graphdb => write!(f, "graphdb"),
            PdbeDataType::Nmr => write!(f, "nmr"),
            PdbeDataType::PdbAssembliesAnalysis => write!(f, "pdb-assemblies-analysis"),
            PdbeDataType::PdbUncompressed => write!(f, "pdb-uncompressed"),
            PdbeDataType::Pdbechem => write!(f, "pdbechem"),
            PdbeDataType::Sifts => write!(f, "sifts"),
            PdbeDataType::Status => write!(f, "status"),
            PdbeDataType::UpdatedMmcif => write!(f, "updated-mmcif"),
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
        // Returns path component for data.pdbj.org::rsync/{path}/
        assert_eq!(PdbjDataType::Emdb.rsync_module(), "pub/emdb");
        assert_eq!(PdbjDataType::PdbIhm.rsync_module(), "pub/pdb_ihm");
        assert_eq!(PdbjDataType::Derived.rsync_module(), "pub/derived_data");
        assert_eq!(PdbjDataType::Bsma.rsync_module(), "pdbj/bsma");
        assert_eq!(PdbjDataType::Efsite.rsync_module(), "pdbj/efsite");
        assert_eq!(PdbjDataType::PdbNextgen.rsync_module(), "pdbj/pdb_nextgen");
        assert_eq!(
            PdbjDataType::PdbVersioned.rsync_module(),
            "pdbj/pdb_versioned"
        );
        assert_eq!(PdbjDataType::Pdbjplus.rsync_module(), "pdbj/pdbjplus");
        assert_eq!(PdbjDataType::Promode.rsync_module(), "pdbj/promode");
        assert_eq!(PdbjDataType::Uniprot.rsync_module(), "pdbj/uniprot");
        assert_eq!(PdbjDataType::Xrda.rsync_module(), "pdbj/xrda");
    }

    #[test]
    fn test_pdbj_data_type_dest_subdir() {
        // pub/ data (wwPDB common)
        assert_eq!(PdbjDataType::Emdb.dest_subdir(), "pub/emdb");
        assert_eq!(PdbjDataType::PdbIhm.dest_subdir(), "pub/pdb_ihm");
        assert_eq!(PdbjDataType::Derived.dest_subdir(), "pub/derived_data");

        // pdbj/ data (PDBj-specific)
        assert_eq!(PdbjDataType::Bsma.dest_subdir(), "pdbj/bsma");
        assert_eq!(PdbjDataType::Efsite.dest_subdir(), "pdbj/efsite");
        assert_eq!(PdbjDataType::PdbNextgen.dest_subdir(), "pdbj/pdb_nextgen");
        assert_eq!(
            PdbjDataType::PdbVersioned.dest_subdir(),
            "pdbj/pdb_versioned"
        );
        assert_eq!(PdbjDataType::Pdbjplus.dest_subdir(), "pdbj/pdbjplus");
        assert_eq!(PdbjDataType::Promode.dest_subdir(), "pdbj/promode");
        assert_eq!(PdbjDataType::Uniprot.dest_subdir(), "pdbj/uniprot");
        assert_eq!(PdbjDataType::Xrda.dest_subdir(), "pdbj/xrda");
    }

    #[test]
    fn test_pdbj_data_type_is_pub_data() {
        assert!(PdbjDataType::Emdb.is_pub_data());
        assert!(PdbjDataType::PdbIhm.is_pub_data());
        assert!(PdbjDataType::Derived.is_pub_data());

        assert!(!PdbjDataType::Bsma.is_pub_data());
        assert!(!PdbjDataType::Efsite.is_pub_data());
        assert!(!PdbjDataType::PdbNextgen.is_pub_data());
        assert!(!PdbjDataType::PdbVersioned.is_pub_data());
        assert!(!PdbjDataType::Pdbjplus.is_pub_data());
        assert!(!PdbjDataType::Promode.is_pub_data());
        assert!(!PdbjDataType::Uniprot.is_pub_data());
        assert!(!PdbjDataType::Xrda.is_pub_data());
    }

    #[test]
    fn test_pdbj_data_type_display() {
        assert_eq!(PdbjDataType::Emdb.to_string(), "emdb");
        assert_eq!(PdbjDataType::PdbIhm.to_string(), "pdb-ihm");
        assert_eq!(PdbjDataType::Derived.to_string(), "derived");
        assert_eq!(PdbjDataType::Bsma.to_string(), "bsma");
        assert_eq!(PdbjDataType::Efsite.to_string(), "efsite");
        assert_eq!(PdbjDataType::PdbNextgen.to_string(), "pdb-nextgen");
        assert_eq!(PdbjDataType::PdbVersioned.to_string(), "pdb-versioned");
        assert_eq!(PdbjDataType::Pdbjplus.to_string(), "pdbjplus");
        assert_eq!(PdbjDataType::Promode.to_string(), "promode");
        assert_eq!(PdbjDataType::Uniprot.to_string(), "uniprot");
        assert_eq!(PdbjDataType::Xrda.to_string(), "xrda");
    }

    #[test]
    fn test_pdbj_data_type_description() {
        assert!(PdbjDataType::Emdb.description().contains("EMDB"));
        assert!(PdbjDataType::PdbIhm.description().contains("IHM"));
        assert!(PdbjDataType::Derived.description().contains("Derived"));
        assert!(PdbjDataType::Bsma.description().contains("BSM-Arc"));
        assert!(PdbjDataType::Efsite.description().contains("eF-site"));
        assert!(PdbjDataType::PdbNextgen.description().contains("NextGen"));
        assert!(PdbjDataType::PdbVersioned
            .description()
            .contains("Versioned"));
    }

    #[test]
    fn test_pdbj_data_type_serde() {
        let dt = PdbjDataType::PdbIhm;
        let json = serde_json::to_string(&dt).unwrap();
        assert_eq!(json, "\"pdb-ihm\"");

        let parsed: PdbjDataType = serde_json::from_str("\"emdb\"").unwrap();
        assert_eq!(parsed, PdbjDataType::Emdb);

        // Test hyphenated variants
        let parsed_nextgen: PdbjDataType = serde_json::from_str("\"pdb-nextgen\"").unwrap();
        assert_eq!(parsed_nextgen, PdbjDataType::PdbNextgen);

        let parsed_versioned: PdbjDataType = serde_json::from_str("\"pdb-versioned\"").unwrap();
        assert_eq!(parsed_versioned, PdbjDataType::PdbVersioned);
    }

    #[test]
    fn test_pdbj_data_type_all() {
        let all = PdbjDataType::all();
        assert_eq!(all.len(), 11);
        assert!(all.contains(&PdbjDataType::Emdb));
        assert!(all.contains(&PdbjDataType::PdbIhm));
        assert!(all.contains(&PdbjDataType::Derived));
        assert!(all.contains(&PdbjDataType::Bsma));
        assert!(all.contains(&PdbjDataType::Efsite));
        assert!(all.contains(&PdbjDataType::PdbNextgen));
        assert!(all.contains(&PdbjDataType::PdbVersioned));
        assert!(all.contains(&PdbjDataType::Pdbjplus));
        assert!(all.contains(&PdbjDataType::Promode));
        assert!(all.contains(&PdbjDataType::Uniprot));
        assert!(all.contains(&PdbjDataType::Xrda));
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
        assert_eq!(
            PdbeDataType::Assemblies.rsync_path(),
            "pub/databases/msd/assemblies/"
        );
        assert_eq!(
            PdbeDataType::FragmentScreening.rsync_path(),
            "pub/databases/msd/fragment_screening/"
        );
        assert_eq!(
            PdbeDataType::Graphdb.rsync_path(),
            "pub/databases/msd/graphdb/"
        );
        assert_eq!(PdbeDataType::Nmr.rsync_path(), "pub/databases/msd/nmr/");
        assert_eq!(
            PdbeDataType::PdbAssembliesAnalysis.rsync_path(),
            "pub/databases/msd/pdb-assemblies-analysis/"
        );
        assert_eq!(
            PdbeDataType::PdbUncompressed.rsync_path(),
            "pub/databases/msd/pdb_uncompressed/"
        );
        assert_eq!(
            PdbeDataType::Status.rsync_path(),
            "pub/databases/msd/status/"
        );
        assert_eq!(
            PdbeDataType::UpdatedMmcif.rsync_path(),
            "pub/databases/msd/updated_mmcif/"
        );
    }

    #[test]
    fn test_pdbe_data_type_dest_subdir() {
        assert_eq!(PdbeDataType::Assemblies.dest_subdir(), "assemblies");
        assert_eq!(PdbeDataType::Foldseek.dest_subdir(), "foldseek");
        assert_eq!(
            PdbeDataType::FragmentScreening.dest_subdir(),
            "fragment_screening"
        );
        assert_eq!(PdbeDataType::Graphdb.dest_subdir(), "graphdb");
        assert_eq!(PdbeDataType::Nmr.dest_subdir(), "nmr");
        assert_eq!(
            PdbeDataType::PdbAssembliesAnalysis.dest_subdir(),
            "pdb-assemblies-analysis"
        );
        assert_eq!(
            PdbeDataType::PdbUncompressed.dest_subdir(),
            "pdb_uncompressed"
        );
        assert_eq!(PdbeDataType::Pdbechem.dest_subdir(), "pdbechem_v2");
        assert_eq!(PdbeDataType::Sifts.dest_subdir(), "sifts");
        assert_eq!(PdbeDataType::Status.dest_subdir(), "status");
        assert_eq!(PdbeDataType::UpdatedMmcif.dest_subdir(), "updated_mmcif");
    }

    #[test]
    fn test_pdbe_data_type_display() {
        assert_eq!(PdbeDataType::Sifts.to_string(), "sifts");
        assert_eq!(PdbeDataType::Pdbechem.to_string(), "pdbechem");
        assert_eq!(PdbeDataType::Foldseek.to_string(), "foldseek");
        assert_eq!(PdbeDataType::Assemblies.to_string(), "assemblies");
        assert_eq!(
            PdbeDataType::FragmentScreening.to_string(),
            "fragment-screening"
        );
        assert_eq!(PdbeDataType::Graphdb.to_string(), "graphdb");
        assert_eq!(PdbeDataType::Nmr.to_string(), "nmr");
        assert_eq!(
            PdbeDataType::PdbAssembliesAnalysis.to_string(),
            "pdb-assemblies-analysis"
        );
        assert_eq!(
            PdbeDataType::PdbUncompressed.to_string(),
            "pdb-uncompressed"
        );
        assert_eq!(PdbeDataType::Status.to_string(), "status");
        assert_eq!(PdbeDataType::UpdatedMmcif.to_string(), "updated-mmcif");
    }

    #[test]
    fn test_pdbe_data_type_description() {
        assert!(PdbeDataType::Sifts.description().contains("SIFTS"));
        assert!(PdbeDataType::Pdbechem.description().contains("PDBeChem"));
        assert!(PdbeDataType::Foldseek.description().contains("Foldseek"));
        assert!(PdbeDataType::Assemblies
            .description()
            .contains("Assemblies"));
        assert!(PdbeDataType::FragmentScreening
            .description()
            .contains("Fragment"));
        assert!(PdbeDataType::Graphdb.description().contains("GraphDB"));
        assert!(PdbeDataType::Nmr.description().contains("NMR"));
    }

    #[test]
    fn test_pdbe_data_type_serde() {
        let dt = PdbeDataType::Sifts;
        let json = serde_json::to_string(&dt).unwrap();
        assert_eq!(json, "\"sifts\"");

        let parsed: PdbeDataType = serde_json::from_str("\"foldseek\"").unwrap();
        assert_eq!(parsed, PdbeDataType::Foldseek);

        // Test hyphenated variants
        let parsed_fragment: PdbeDataType = serde_json::from_str("\"fragment-screening\"").unwrap();
        assert_eq!(parsed_fragment, PdbeDataType::FragmentScreening);

        let parsed_assemblies: PdbeDataType =
            serde_json::from_str("\"pdb-assemblies-analysis\"").unwrap();
        assert_eq!(parsed_assemblies, PdbeDataType::PdbAssembliesAnalysis);

        let parsed_uncompressed: PdbeDataType =
            serde_json::from_str("\"pdb-uncompressed\"").unwrap();
        assert_eq!(parsed_uncompressed, PdbeDataType::PdbUncompressed);

        let parsed_updated: PdbeDataType = serde_json::from_str("\"updated-mmcif\"").unwrap();
        assert_eq!(parsed_updated, PdbeDataType::UpdatedMmcif);
    }

    #[test]
    fn test_pdbe_data_type_all() {
        let all = PdbeDataType::all();
        assert_eq!(all.len(), 11);
        assert!(all.contains(&PdbeDataType::Assemblies));
        assert!(all.contains(&PdbeDataType::Foldseek));
        assert!(all.contains(&PdbeDataType::FragmentScreening));
        assert!(all.contains(&PdbeDataType::Graphdb));
        assert!(all.contains(&PdbeDataType::Nmr));
        assert!(all.contains(&PdbeDataType::PdbAssembliesAnalysis));
        assert!(all.contains(&PdbeDataType::PdbUncompressed));
        assert!(all.contains(&PdbeDataType::Pdbechem));
        assert!(all.contains(&PdbeDataType::Sifts));
        assert!(all.contains(&PdbeDataType::Status));
        assert!(all.contains(&PdbeDataType::UpdatedMmcif));
    }
}
