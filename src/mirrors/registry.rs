use crate::error::{PdbCliError, Result};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum MirrorId {
    /// RCSB (US)
    Rcsb,
    /// PDBj (Japan)
    Pdbj,
    /// PDBe (Europe/UK)
    Pdbe,
    /// wwPDB (Global)
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
            "pdbe" | "uk" => Ok(MirrorId::Pdbe),
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
    pub rsync_url: &'static str,
    pub rsync_port: Option<u16>,
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

    #[allow(dead_code)]
    pub fn rsync_url_with_port(&self) -> String {
        match self.rsync_port {
            Some(port) => format!("rsync://rsync.rcsb.org:{}", port),
            None => self.rsync_url.to_string(),
        }
    }
}

static RCSB_MIRROR: Mirror = Mirror {
    id: MirrorId::Rcsb,
    name: "RCSB PDB",
    region: "US",
    rsync_url: "rsync://rsync.rcsb.org",
    rsync_port: Some(33444),
    https_base: "https://files.rcsb.org/download",
};

static PDBJ_MIRROR: Mirror = Mirror {
    id: MirrorId::Pdbj,
    name: "PDBj",
    region: "Japan",
    rsync_url: "rsync://rsync.pdbj.org",
    rsync_port: None,
    https_base: "https://pdbj.org/rest/downloadPDBfile",
};

static PDBE_MIRROR: Mirror = Mirror {
    id: MirrorId::Pdbe,
    name: "PDBe",
    region: "Europe",
    rsync_url: "rsync://rsync.ebi.ac.uk/pub/databases/pdb",
    rsync_port: None,
    https_base: "https://www.ebi.ac.uk/pdbe/entry-files/download",
};

static WWPDB_MIRROR: Mirror = Mirror {
    id: MirrorId::Wwpdb,
    name: "wwPDB",
    region: "Global",
    rsync_url: "rsync://rsync.wwpdb.org",
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
    fn test_rsync_url_with_port() {
        let rcsb = Mirror::get(MirrorId::Rcsb);
        assert_eq!(rcsb.rsync_url_with_port(), "rsync://rsync.rcsb.org:33444");

        let pdbj = Mirror::get(MirrorId::Pdbj);
        assert_eq!(pdbj.rsync_url_with_port(), "rsync://rsync.pdbj.org");
    }
}
