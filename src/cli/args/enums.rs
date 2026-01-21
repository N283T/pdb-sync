//! ValueEnum types for CLI arguments.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Output format for list command
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum OutputFormat {
    /// Plain text output
    #[default]
    Text,
    /// JSON output
    Json,
    /// CSV output
    Csv,
    /// One ID per line (for piping)
    Ids,
}

/// Sort field for list command
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum SortField {
    /// Sort by PDB ID (alphabetical)
    #[default]
    Name,
    /// Sort by file size
    Size,
    /// Sort by modification time
    Time,
}

/// Notification method for watch command
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum NotifyMethod {
    /// Desktop notification
    Desktop,
    /// Email notification
    Email,
}

/// Experimental method filter for watch command
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExperimentalMethod {
    /// X-ray crystallography
    #[serde(rename = "X-RAY DIFFRACTION")]
    #[value(name = "xray")]
    Xray,
    /// Nuclear Magnetic Resonance
    #[serde(rename = "SOLUTION NMR")]
    #[value(name = "nmr")]
    Nmr,
    /// Electron Microscopy
    #[serde(rename = "ELECTRON MICROSCOPY")]
    #[value(name = "em")]
    Em,
    /// Neutron diffraction
    #[serde(rename = "NEUTRON DIFFRACTION")]
    #[value(name = "neutron")]
    Neutron,
    /// Other methods
    #[serde(rename = "OTHER")]
    #[value(name = "other")]
    Other,
}

impl ExperimentalMethod {
    /// Get the RCSB API value for this method
    pub fn as_rcsb_value(&self) -> &str {
        match self {
            ExperimentalMethod::Xray => "X-RAY DIFFRACTION",
            ExperimentalMethod::Nmr => "SOLUTION NMR",
            ExperimentalMethod::Em => "ELECTRON MICROSCOPY",
            ExperimentalMethod::Neutron => "NEUTRON DIFFRACTION",
            ExperimentalMethod::Other => "OTHER",
        }
    }
}

/// Sync format options
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum SyncFormat {
    /// Legacy PDB format (.pdb)
    Pdb,
    /// mmCIF format (.cif)
    Mmcif,
    /// Both PDB and mmCIF
    Both,
}

impl SyncFormat {
    pub fn to_file_formats(self) -> Vec<crate::files::FileFormat> {
        match self {
            SyncFormat::Pdb => vec![crate::files::FileFormat::Pdb],
            SyncFormat::Mmcif => vec![crate::files::FileFormat::Mmcif],
            SyncFormat::Both => {
                vec![
                    crate::files::FileFormat::Pdb,
                    crate::files::FileFormat::Mmcif,
                ]
            }
        }
    }
}
