use crate::error::{PdbCliError, Result};
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
    type Err = PdbCliError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "rcsb" | "us" => Ok(MirrorId::Rcsb),
            "pdbj" | "jp" => Ok(MirrorId::Pdbj),
            "pdbe" | "uk" | "eu" => Ok(MirrorId::Pdbe),
            "wwpdb" | "global" => Ok(MirrorId::Wwpdb),
            _ => Err(PdbCliError::UnknownMirror(s.to_string())),
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
        match self.rsync_port {
            Some(port) => format!(
                "{}:{}/{}/{}",
                self.rsync_host, port, self.rsync_module, subpath
            ),
            None => format!("{}/{}/{}", self.rsync_host, self.rsync_module, subpath),
        }
    }

    /// Get rsync port arguments if needed
    #[allow(dead_code)]
    pub fn rsync_port_args(&self) -> Option<String> {
        self.rsync_port.map(|p| format!("--port={}", p))
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

// PDBj: rsync://rsync.pdbj.org/ftp_data/structures/divided/
static PDBJ_MIRROR: Mirror = Mirror {
    id: MirrorId::Pdbj,
    name: "PDBj",
    region: "Japan",
    rsync_host: "rsync://rsync.pdbj.org",
    rsync_module: "ftp_data",
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
            "rsync://rsync.pdbj.org/ftp_data/structures/divided/mmCIF/"
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
}
