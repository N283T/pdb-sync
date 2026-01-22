//! pdb-sync: A CLI tool for managing PDB (Protein Data Bank) files.

// Public modules for library tests
pub mod cli;
pub mod config;
pub mod context;
pub mod data_types;
pub mod error;
pub mod files;
pub mod mirrors;
pub mod sync;
pub mod utils;

// Re-export commonly used types
pub use data_types::{DataType, Layout};
