use crate::error::{PdbCliError, Result};
use regex::Regex;
use std::fmt;
use std::str::FromStr;
use std::sync::LazyLock;

static PDB_ID_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[0-9][a-zA-Z0-9]{3}$").unwrap());

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PdbId(String);

impl PdbId {
    pub fn new(id: &str) -> Result<Self> {
        let id = id.trim().to_lowercase();
        if PDB_ID_REGEX.is_match(&id) {
            Ok(Self(id))
        } else {
            Err(PdbCliError::InvalidPdbId(id))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the middle two characters used for directory partitioning
    pub fn middle_chars(&self) -> &str {
        &self.0[1..3]
    }
}

impl fmt::Display for PdbId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for PdbId {
    type Err = PdbCliError;

    fn from_str(s: &str) -> Result<Self> {
        Self::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_pdb_ids() {
        assert!(PdbId::new("1abc").is_ok());
        assert!(PdbId::new("1ABC").is_ok());
        assert!(PdbId::new("9xyz").is_ok());
        assert!(PdbId::new("4hhb").is_ok());
    }

    #[test]
    fn test_invalid_pdb_ids() {
        assert!(PdbId::new("abc").is_err());
        assert!(PdbId::new("12345").is_err());
        assert!(PdbId::new("abcd").is_err());
        assert!(PdbId::new("").is_err());
        assert!(PdbId::new("1ab").is_err());
    }

    #[test]
    fn test_normalization() {
        let id = PdbId::new("1ABC").unwrap();
        assert_eq!(id.as_str(), "1abc");
    }

    #[test]
    fn test_middle_chars() {
        let id = PdbId::new("1abc").unwrap();
        assert_eq!(id.middle_chars(), "ab");
    }
}
