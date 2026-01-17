//! pdb-cli: A CLI tool for managing PDB (Protein Data Bank) files.

pub mod data_types;

// Re-export commonly used types
pub use data_types::{DataType, Layout, PdbeDataType, PdbjDataType};
