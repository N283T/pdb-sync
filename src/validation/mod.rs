//! Validation module for verifying PDB files against checksums.

pub mod checksum;

#[allow(unused_imports)]
pub use checksum::{calculate_md5, parse_checksums, ChecksumVerifier, VerifyResult};
