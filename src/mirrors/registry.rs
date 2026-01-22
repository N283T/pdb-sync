use crate::data_types::{PdbeDataType, PdbjDataType};
use crate::error::{PdbSyncError, Result};
use crate::files::{FileFormat, PdbId};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, clap::ValueEnum)]
pub enum MirrorId {
    /// RCSB (US)
    #[value(alias = "us")]
    Rcsb,
    /// PDBj (Japan)
    #[value(alias = "jp")]
    Pdbj,
    /// PDBe (Europe/UK)
    #[value(alias = "uk", alias = "eu")]
    Pdbe,
    /// wwPDB (Global)
    #[value(alias = "global")]
    Wwpdb,
}

impl MirrorId {
    #[allow(dead_code)]
    pub fn all() -> &'static [MirrorId] {
        &[
            MirrorId::Rcsb,
            MirrorId::Pdbj,
            MirrorId::Pdbe,
            MirrorId::Wwpdb,
        ]
    }
}

impl std::fmt::Display for MirrorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MirrorId::Rcsb => write!(f, "rcsb"),
            MirrorId::Pdbj => write!(f, "pdbj"),
            MirrorId::Pdbe => write!(f, "pdbe"),
            MirrorId::Wwpdb => write!(f, "wwpdb"),
        }
    }
}

impl FromStr for MirrorId {
    type Err = PdbSyncError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "rcsb" | "us" => Ok(MirrorId::Rcsb),
            "pdbj" | "jp" => Ok(MirrorId::Pdbj),
            "pdbe" | "uk" | "eu" => Ok(MirrorId::Pdbe),
            "wwpdb" | "global" => Ok(MirrorId::Wwpdb),
            _ => Err(PdbSyncError::UnknownMirror(s.to_string())),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Mirror {
    pub id: MirrorId,
    pub name: &'static str,
    pub region: &'static str,
    /// Base rsync URL (e.g., rsync://rsync.rcsb.org)
    pub rsync_host: &'static str,
    /// rsync module name (e.g., ftp_data)
    pub rsync_module: &'static str,
    /// Custom port for rsync (None = default 873)
    pub rsync_port: Option<u16>,
    /// Base URL for HTTPS downloads
    pub https_base: &'static str,
}

impl Mirror {
    pub fn get(id: MirrorId) -> &'static Mirror {
        match id {
            MirrorId::Rcsb => &RCSB_MIRROR,
            MirrorId::Pdbj => &PDBJ_MIRROR,
            MirrorId::Pdbe => &PDBE_MIRROR,
            MirrorId::Wwpdb => &WWPDB_MIRROR,
        }
    }

    /// Build the rsync source URL for a given subpath
    pub fn rsync_url(&self, subpath: &str) -> String {
        match self.id {
            MirrorId::Pdbj => {
                // PDBj standard wwPDB data is under pub/pdb/data/
                format!("{}::rsync/pub/pdb/data/{}", self.rsync_host, subpath)
            }
            _ => match self.rsync_port {
                Some(port) => format!(
                    "{}:{}/{}/{}",
                    self.rsync_host, port, self.rsync_module, subpath
                ),
                None => format!("{}/{}/{}", self.rsync_host, self.rsync_module, subpath),
            },
        }
    }

    /// Get rsync port arguments if needed
    #[allow(dead_code)]
    pub fn rsync_port_args(&self) -> Option<String> {
        self.rsync_port.map(|p| format!("--port={}", p))
    }

    /// Build HTTPS URL for structure file downloads.
    ///
    /// This is the canonical URL construction for structure files across all mirrors.
    /// Used by both `HttpsDownloader` and `UpdateChecker`.
    pub fn build_structure_url(&self, pdb_id: &PdbId, format: FileFormat) -> String {
        let id = pdb_id.as_str();
        let base = format.base_format();

        match self.id {
            MirrorId::Rcsb => match base {
                FileFormat::Pdb => format!("{}/{}.pdb", self.https_base, id),
                FileFormat::Mmcif => format!("{}/{}.cif", self.https_base, id),
                FileFormat::Bcif => format!("https://models.rcsb.org/{}.bcif", id),
                _ => unreachable!(),
            },
            MirrorId::Pdbj => match base {
                FileFormat::Pdb => format!("{}?format=pdb&id={}", self.https_base, id),
                FileFormat::Mmcif => format!("{}?format=mmcif&id={}", self.https_base, id),
                FileFormat::Bcif => format!("{}?format=mmcif&id={}", self.https_base, id),
                _ => unreachable!(),
            },
            MirrorId::Pdbe => match base {
                // Classic IDs: pdb{id}.ent, Extended IDs: {id}.ent (no extra "pdb" prefix)
                FileFormat::Pdb => {
                    if pdb_id.is_classic() {
                        format!("{}/pdb{}.ent", self.https_base, id)
                    } else {
                        format!("{}/{}.ent", self.https_base, id)
                    }
                }
                FileFormat::Mmcif => format!("{}/{}.cif", self.https_base, id),
                FileFormat::Bcif => format!("{}/{}.cif", self.https_base, id),
                _ => unreachable!(),
            },
            MirrorId::Wwpdb => {
                let middle = pdb_id.middle_chars();
                match base {
                    // Classic IDs: pdb{id}.ent.gz, Extended IDs: {id}.ent.gz
                    FileFormat::Pdb => {
                        if pdb_id.is_classic() {
                            format!(
                                "{}/divided/pdb/{}/pdb{}.ent.gz",
                                self.https_base, middle, id
                            )
                        } else {
                            format!("{}/divided/pdb/{}/{}.ent.gz", self.https_base, middle, id)
                        }
                    }
                    FileFormat::Mmcif => {
                        format!("{}/divided/mmCIF/{}/{}.cif.gz", self.https_base, middle, id)
                    }
                    FileFormat::Bcif => {
                        format!("{}/divided/mmCIF/{}/{}.cif.gz", self.https_base, middle, id)
                    }
                    _ => unreachable!(),
                }
            }
        }
    }

    /// Get PDBj-specific rsync URL (only valid for PDBj mirror).
    ///
    /// Returns `None` if called on a non-PDBj mirror.
    ///
    /// Uses data.pdbj.org::rsync/{path}/ format where:
    /// - pub/ data: data.pdbj.org::rsync/pub/{module}/
    /// - pdbj/ data: data.pdbj.org::rsync/{directory}/
    pub fn pdbj_rsync_url(&self, data_type: PdbjDataType) -> Option<String> {
        if self.id != MirrorId::Pdbj {
            return None;
        }
        // data.pdbj.org::rsync/{path}/
        let host = self.rsync_host; // "data.pdbj.org"
        let module = self.rsync_module; // "rsync"

        // Build path based on data type
        let path = if data_type.is_pub_data() {
            // pub/ data: pub/emdb/, pub/pdb_ihm/, pub/derived_data/
            format!("{}/", data_type.dest_subdir())
        } else {
            // pdbj/ data: bsma/, efsite/, etc.
            format!("{}/", data_type.dest_subdir())
        };

        Some(format!("{}::{}/{}", host, module, path))
    }

    /// Get PDBe-specific rsync URL (only valid for PDBe mirror).
    ///
    /// Returns `None` if called on a non-PDBe mirror.
    pub fn pdbe_rsync_url(&self, data_type: PdbeDataType) -> Option<String> {
        if self.id != MirrorId::Pdbe {
            return None;
        }
        // Extract host from rsync_host (which has format "rsync://host")
        let host = self.rsync_host.trim_start_matches("rsync://");
        Some(format!("rsync://{}/{}", host, data_type.rsync_path()))
    }
}

// RCSB: rsync://rsync.rcsb.org:33444/ftp_data/structures/divided/
static RCSB_MIRROR: Mirror = Mirror {
    id: MirrorId::Rcsb,
    name: "RCSB PDB",
    region: "US",
    rsync_host: "rsync://rsync.rcsb.org",
    rsync_module: "ftp_data",
    rsync_port: Some(33444),
    https_base: "https://files.rcsb.org/download",
};

// PDBj: data.pdbj.org::rsync/{path}/
// Root contains: bsma, efsite, pdb_nextgen, pdb_versioned, pdbjplus, promode, pub, uniprot, xrda
// pub/ contains: emdb, pdb (wwPDB common data), pdb_ihm
static PDBJ_MIRROR: Mirror = Mirror {
    id: MirrorId::Pdbj,
    name: "PDBj",
    region: "Japan",
    rsync_host: "data.pdbj.org",
    rsync_module: "rsync",
    rsync_port: None,
    https_base: "https://pdbj.org/rest/downloadPDBfile",
};

// PDBe: rsync://rsync.ebi.ac.uk/pub/databases/pdb/data/structures/divided/
static PDBE_MIRROR: Mirror = Mirror {
    id: MirrorId::Pdbe,
    name: "PDBe",
    region: "Europe",
    rsync_host: "rsync://rsync.ebi.ac.uk",
    rsync_module: "pub/databases/pdb/data",
    rsync_port: None,
    https_base: "https://www.ebi.ac.uk/pdbe/entry-files/download",
};

// wwPDB: rsync://rsync.wwpdb.org/ftp_data/structures/divided/
static WWPDB_MIRROR: Mirror = Mirror {
    id: MirrorId::Wwpdb,
    name: "wwPDB",
    region: "Global",
    rsync_host: "rsync://rsync.wwpdb.org",
    rsync_module: "ftp_data",
    rsync_port: None,
    https_base: "https://files.wwpdb.org/pub/pdb/data/structures",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mirror_id_from_str() {
        assert_eq!(MirrorId::from_str("rcsb").unwrap(), MirrorId::Rcsb);
        assert_eq!(MirrorId::from_str("us").unwrap(), MirrorId::Rcsb);
        assert_eq!(MirrorId::from_str("pdbj").unwrap(), MirrorId::Pdbj);
        assert_eq!(MirrorId::from_str("jp").unwrap(), MirrorId::Pdbj);
    }

    #[test]
    fn test_rsync_url() {
        let rcsb = Mirror::get(MirrorId::Rcsb);
        assert_eq!(
            rcsb.rsync_url("structures/divided/mmCIF/"),
            "rsync://rsync.rcsb.org:33444/ftp_data/structures/divided/mmCIF/"
        );

        let pdbj = Mirror::get(MirrorId::Pdbj);
        assert_eq!(
            pdbj.rsync_url("structures/divided/mmCIF/"),
            "data.pdbj.org::rsync/pub/pdb/data/structures/divided/mmCIF/"
        );

        let pdbe = Mirror::get(MirrorId::Pdbe);
        assert_eq!(
            pdbe.rsync_url("structures/divided/mmCIF/"),
            "rsync://rsync.ebi.ac.uk/pub/databases/pdb/data/structures/divided/mmCIF/"
        );
    }

    #[test]
    fn test_mirror_aliases_clap() {
        use clap::ValueEnum;

        // Test alias parsing via clap ValueEnum
        assert_eq!(
            <MirrorId as ValueEnum>::from_str("us", true).unwrap(),
            MirrorId::Rcsb
        );
        assert_eq!(
            <MirrorId as ValueEnum>::from_str("jp", true).unwrap(),
            MirrorId::Pdbj
        );
        assert_eq!(
            <MirrorId as ValueEnum>::from_str("uk", true).unwrap(),
            MirrorId::Pdbe
        );
        assert_eq!(
            <MirrorId as ValueEnum>::from_str("eu", true).unwrap(),
            MirrorId::Pdbe
        );
        assert_eq!(
            <MirrorId as ValueEnum>::from_str("global", true).unwrap(),
            MirrorId::Wwpdb
        );
    }

    #[test]
    fn test_mirror_aliases_fromstr() {
        // Test eu alias via FromStr
        assert_eq!(
            <MirrorId as FromStr>::from_str("eu").unwrap(),
            MirrorId::Pdbe
        );
    }

    #[test]
    fn test_build_structure_url_rcsb_classic() {
        let mirror = Mirror::get(MirrorId::Rcsb);
        let pdb_id = PdbId::new("1abc").unwrap();

        assert_eq!(
            mirror.build_structure_url(&pdb_id, FileFormat::Pdb),
            "https://files.rcsb.org/download/1abc.pdb"
        );
        assert_eq!(
            mirror.build_structure_url(&pdb_id, FileFormat::Mmcif),
            "https://files.rcsb.org/download/1abc.cif"
        );
        assert_eq!(
            mirror.build_structure_url(&pdb_id, FileFormat::CifGz),
            "https://files.rcsb.org/download/1abc.cif"
        );
        assert_eq!(
            mirror.build_structure_url(&pdb_id, FileFormat::Bcif),
            "https://models.rcsb.org/1abc.bcif"
        );
    }

    // Tests for PDBj-specific rsync URL builder
    #[test]
    fn test_pdbj_rsync_url_returns_url_for_pdbj() {
        use crate::data_types::PdbjDataType;

        let pdbj = Mirror::get(MirrorId::Pdbj);

        // pub/ data (wwPDB common)
        assert_eq!(
            pdbj.pdbj_rsync_url(PdbjDataType::Emdb),
            Some("data.pdbj.org::rsync/pub/emdb/".to_string())
        );
        assert_eq!(
            pdbj.pdbj_rsync_url(PdbjDataType::PdbIhm),
            Some("data.pdbj.org::rsync/pub/pdb_ihm/".to_string())
        );
        assert_eq!(
            pdbj.pdbj_rsync_url(PdbjDataType::Derived),
            Some("data.pdbj.org::rsync/pub/derived_data/".to_string())
        );

        // pdbj/ data (PDBj-specific)
        assert_eq!(
            pdbj.pdbj_rsync_url(PdbjDataType::Bsma),
            Some("data.pdbj.org::rsync/pdbj/bsma/".to_string())
        );
        assert_eq!(
            pdbj.pdbj_rsync_url(PdbjDataType::Efsite),
            Some("data.pdbj.org::rsync/pdbj/efsite/".to_string())
        );
        assert_eq!(
            pdbj.pdbj_rsync_url(PdbjDataType::PdbNextgen),
            Some("data.pdbj.org::rsync/pdbj/pdb_nextgen/".to_string())
        );
        assert_eq!(
            pdbj.pdbj_rsync_url(PdbjDataType::PdbVersioned),
            Some("data.pdbj.org::rsync/pdbj/pdb_versioned/".to_string())
        );
    }

    #[test]
    fn test_build_structure_url_wwpdb_classic() {
        let mirror = Mirror::get(MirrorId::Wwpdb);
        let pdb_id = PdbId::new("1abc").unwrap();

        assert_eq!(
            mirror.build_structure_url(&pdb_id, FileFormat::Mmcif),
            "https://files.wwpdb.org/pub/pdb/data/structures/divided/mmCIF/ab/1abc.cif.gz"
        );
        assert_eq!(
            mirror.build_structure_url(&pdb_id, FileFormat::Pdb),
            "https://files.wwpdb.org/pub/pdb/data/structures/divided/pdb/ab/pdb1abc.ent.gz"
        );
    }

    #[test]
    fn test_pdbj_rsync_url_returns_none_for_other_mirrors() {
        use crate::data_types::PdbjDataType;

        // Should return None for non-PDBj mirrors
        assert_eq!(
            Mirror::get(MirrorId::Rcsb).pdbj_rsync_url(PdbjDataType::Emdb),
            None
        );
        assert_eq!(
            Mirror::get(MirrorId::Pdbe).pdbj_rsync_url(PdbjDataType::Emdb),
            None
        );
        assert_eq!(
            Mirror::get(MirrorId::Wwpdb).pdbj_rsync_url(PdbjDataType::Emdb),
            None
        );
    }

    // Tests for PDBe-specific rsync URL builder
    #[test]
    fn test_pdbe_rsync_url_returns_url_for_pdbe() {
        use crate::data_types::PdbeDataType;

        let pdbe = Mirror::get(MirrorId::Pdbe);

        assert_eq!(
            pdbe.pdbe_rsync_url(PdbeDataType::Sifts),
            Some("rsync://rsync.ebi.ac.uk/pub/databases/msd/sifts/".to_string())
        );
        assert_eq!(
            pdbe.pdbe_rsync_url(PdbeDataType::Pdbechem),
            Some("rsync://rsync.ebi.ac.uk/pub/databases/msd/pdbechem_v2/".to_string())
        );
        assert_eq!(
            pdbe.pdbe_rsync_url(PdbeDataType::Foldseek),
            Some("rsync://rsync.ebi.ac.uk/pub/databases/msd/foldseek/".to_string())
        );
    }

    #[test]
    fn test_build_structure_url_rcsb_extended() {
        let mirror = Mirror::get(MirrorId::Rcsb);
        let pdb_id = PdbId::new("pdb_00001abc").unwrap();

        assert_eq!(
            mirror.build_structure_url(&pdb_id, FileFormat::Pdb),
            "https://files.rcsb.org/download/pdb_00001abc.pdb"
        );
        assert_eq!(
            mirror.build_structure_url(&pdb_id, FileFormat::Mmcif),
            "https://files.rcsb.org/download/pdb_00001abc.cif"
        );
        assert_eq!(
            mirror.build_structure_url(&pdb_id, FileFormat::Bcif),
            "https://models.rcsb.org/pdb_00001abc.bcif"
        );
    }

    #[test]
    fn test_build_structure_url_wwpdb_extended() {
        let mirror = Mirror::get(MirrorId::Wwpdb);
        let pdb_id = PdbId::new("pdb_00001abc").unwrap();

        // Extended IDs use positions 6-7 for directory partitioning (= "00")
        assert_eq!(
            mirror.build_structure_url(&pdb_id, FileFormat::Mmcif),
            "https://files.wwpdb.org/pub/pdb/data/structures/divided/mmCIF/00/pdb_00001abc.cif.gz"
        );
        // Extended IDs don't have extra "pdb" prefix in filename
        assert_eq!(
            mirror.build_structure_url(&pdb_id, FileFormat::Pdb),
            "https://files.wwpdb.org/pub/pdb/data/structures/divided/pdb/00/pdb_00001abc.ent.gz"
        );
    }

    #[test]
    fn test_build_structure_url_pdbe_extended() {
        let mirror = Mirror::get(MirrorId::Pdbe);
        let pdb_id = PdbId::new("pdb_00001abc").unwrap();

        // Extended IDs don't have extra "pdb" prefix in PDB format
        assert_eq!(
            mirror.build_structure_url(&pdb_id, FileFormat::Pdb),
            "https://www.ebi.ac.uk/pdbe/entry-files/download/pdb_00001abc.ent"
        );
        assert_eq!(
            mirror.build_structure_url(&pdb_id, FileFormat::Mmcif),
            "https://www.ebi.ac.uk/pdbe/entry-files/download/pdb_00001abc.cif"
        );
    }

    #[test]
    fn test_build_structure_url_pdbj_extended() {
        let mirror = Mirror::get(MirrorId::Pdbj);
        let pdb_id = PdbId::new("pdb_00001abc").unwrap();

        // PDBj uses query parameters with full ID
        assert_eq!(
            mirror.build_structure_url(&pdb_id, FileFormat::Pdb),
            "https://pdbj.org/rest/downloadPDBfile?format=pdb&id=pdb_00001abc"
        );
        assert_eq!(
            mirror.build_structure_url(&pdb_id, FileFormat::Mmcif),
            "https://pdbj.org/rest/downloadPDBfile?format=mmcif&id=pdb_00001abc"
        );
    }

    #[test]
    fn test_pdbe_rsync_url_returns_none_for_other_mirrors() {
        use crate::data_types::PdbeDataType;

        // Should return None for non-PDBe mirrors
        assert_eq!(
            Mirror::get(MirrorId::Rcsb).pdbe_rsync_url(PdbeDataType::Sifts),
            None
        );
        assert_eq!(
            Mirror::get(MirrorId::Pdbj).pdbe_rsync_url(PdbeDataType::Sifts),
            None
        );
        assert_eq!(
            Mirror::get(MirrorId::Wwpdb).pdbe_rsync_url(PdbeDataType::Sifts),
            None
        );
    }
}
