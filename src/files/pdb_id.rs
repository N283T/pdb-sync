use crate::error::{PdbSyncError, Result};
use regex::Regex;
use std::fmt;
use std::str::FromStr;
use std::sync::LazyLock;

/// Classic 4-character PDB ID: starts with digit, followed by 3 alphanumeric
static CLASSIC_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[0-9][a-zA-Z0-9]{3}$").unwrap());

/// Extended 12-character PDB ID: "pdb_" prefix followed by 8 alphanumeric characters
static EXTENDED_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^pdb_[0-9a-zA-Z]{8}$").unwrap());

/// Represents a PDB identifier, supporting both classic (4-char) and extended (12-char) formats.
///
/// # Examples
///
/// ```
/// use pdb_sync::files::PdbId;
///
/// // Classic format: 4 characters starting with a digit
/// let classic = PdbId::new("1abc").unwrap();
/// assert!(classic.is_classic());
/// assert_eq!(classic.middle_chars(), "ab");
///
/// // Extended format: "pdb_" prefix + 8 alphanumeric characters
/// let extended = PdbId::new("pdb_00001abc").unwrap();
/// assert!(!extended.is_classic());
/// assert_eq!(extended.middle_chars(), "00");  // positions 6-7
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PdbId {
    /// Classic 4-character PDB ID (e.g., "1abc")
    Classic(String),
    /// Extended 12-character PDB ID (e.g., "pdb_00001abc")
    Extended(String),
}

impl PdbId {
    /// Create a new PdbId from a string, auto-detecting the format.
    ///
    /// The input is trimmed and normalized to lowercase.
    ///
    /// # Arguments
    ///
    /// * `id` - A string that should be either:
    ///   - Classic format: 4 characters starting with a digit (e.g., "1abc")
    ///   - Extended format: "pdb_" prefix + 8 alphanumeric characters (e.g., "pdb_00001abc")
    ///
    /// # Errors
    ///
    /// Returns `PdbSyncError::InvalidPdbId` if the input doesn't match either format.
    pub fn new(id: &str) -> Result<Self> {
        let id = id.trim().to_lowercase();

        if CLASSIC_REGEX.is_match(&id) {
            Ok(Self::Classic(id))
        } else if EXTENDED_REGEX.is_match(&id) {
            Ok(Self::Extended(id))
        } else {
            Err(PdbSyncError::InvalidPdbId {
                input: id,
                source: None,
            })
        }
    }

    /// Returns the full PDB ID as a string slice.
    pub fn as_str(&self) -> &str {
        match self {
            PdbId::Classic(id) | PdbId::Extended(id) => id,
        }
    }

    /// Returns the middle two characters used for directory partitioning.
    ///
    /// - Classic IDs: characters at positions 1-2 (e.g., "1abc" → "ab")
    /// - Extended IDs: characters at positions 6-7 (e.g., "pdb_00001abc" → "01")
    pub fn middle_chars(&self) -> &str {
        match self {
            PdbId::Classic(id) => &id[1..3],
            PdbId::Extended(id) => &id[6..8],
        }
    }

    /// Returns true if this is a classic 4-character PDB ID.
    pub fn is_classic(&self) -> bool {
        matches!(self, PdbId::Classic(_))
    }
}

impl fmt::Display for PdbId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for PdbId {
    type Err = PdbSyncError;

    fn from_str(s: &str) -> Result<Self> {
        Self::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Classic format tests ===

    #[test]
    fn test_valid_classic_pdb_ids() {
        assert!(PdbId::new("1abc").is_ok());
        assert!(PdbId::new("1ABC").is_ok());
        assert!(PdbId::new("9xyz").is_ok());
        assert!(PdbId::new("4hhb").is_ok());
    }

    #[test]
    fn test_classic_is_classic() {
        let id = PdbId::new("1abc").unwrap();
        assert!(id.is_classic());
    }

    #[test]
    fn test_classic_normalization() {
        let id = PdbId::new("1ABC").unwrap();
        assert_eq!(id.as_str(), "1abc");
    }

    #[test]
    fn test_classic_middle_chars() {
        let id = PdbId::new("1abc").unwrap();
        assert_eq!(id.middle_chars(), "ab");

        let id2 = PdbId::new("4hhb").unwrap();
        assert_eq!(id2.middle_chars(), "hh");
    }

    #[test]
    fn test_classic_display() {
        let id = PdbId::new("1abc").unwrap();
        assert_eq!(format!("{}", id), "1abc");
    }

    // === Extended format tests ===

    #[test]
    fn test_valid_extended_pdb_ids() {
        assert!(PdbId::new("pdb_00001abc").is_ok());
        assert!(PdbId::new("PDB_00001ABC").is_ok());
        assert!(PdbId::new("pdb_12345678").is_ok());
        assert!(PdbId::new("pdb_abcdefgh").is_ok());
    }

    #[test]
    fn test_extended_normalization() {
        let id = PdbId::new("PDB_00001ABC").unwrap();
        assert_eq!(id.as_str(), "pdb_00001abc");
    }

    #[test]
    fn test_extended_middle_chars() {
        // "pdb_00001abc" → positions 6-7 (0-indexed) = "00"
        // Index: 0  1  2  3  4  5  6  7  8  9 10 11
        // Char:  p  d  b  _  0  0  0  0  1  a  b  c
        let id = PdbId::new("pdb_00001abc").unwrap();
        assert_eq!(id.middle_chars(), "00");

        // "pdb_12345678" → positions 6-7 = "34"
        // Index: 0  1  2  3  4  5  6  7  8  9 10 11
        // Char:  p  d  b  _  1  2  3  4  5  6  7  8
        let id2 = PdbId::new("pdb_12345678").unwrap();
        assert_eq!(id2.middle_chars(), "34");

        // "pdb_abcdefgh" → positions 6-7 = "cd"
        // Index: 0  1  2  3  4  5  6  7  8  9 10 11
        // Char:  p  d  b  _  a  b  c  d  e  f  g  h
        let id3 = PdbId::new("pdb_abcdefgh").unwrap();
        assert_eq!(id3.middle_chars(), "cd");
    }

    #[test]
    fn test_extended_display() {
        let id = PdbId::new("pdb_00001abc").unwrap();
        assert_eq!(format!("{}", id), "pdb_00001abc");
    }

    // === Invalid format tests ===

    #[test]
    fn test_invalid_pdb_ids() {
        // Too short
        assert!(PdbId::new("abc").is_err());
        assert!(PdbId::new("1ab").is_err());

        // Too long for classic, wrong format for extended
        assert!(PdbId::new("12345").is_err());

        // Classic must start with digit
        assert!(PdbId::new("abcd").is_err());

        // Empty
        assert!(PdbId::new("").is_err());

        // Invalid extended: wrong prefix
        assert!(PdbId::new("pdb123456789").is_err());

        // Invalid extended: too few characters after prefix
        assert!(PdbId::new("pdb_123").is_err());

        // Invalid extended: too many characters after prefix
        assert!(PdbId::new("pdb_123456789").is_err());
    }

    // === FromStr tests ===

    #[test]
    fn test_from_str_classic() {
        let id: PdbId = "1abc".parse().unwrap();
        assert!(id.is_classic());
        assert_eq!(id.as_str(), "1abc");
    }

    #[test]
    fn test_from_str_extended() {
        let id: PdbId = "pdb_00001abc".parse().unwrap();
        assert!(!id.is_classic());
        assert_eq!(id.as_str(), "pdb_00001abc");
    }

    #[test]
    fn test_from_str_invalid() {
        let result: std::result::Result<PdbId, _> = "invalid".parse();
        assert!(result.is_err());
    }

    // === Whitespace handling ===

    #[test]
    fn test_whitespace_trimming() {
        let id = PdbId::new("  1abc  ").unwrap();
        assert_eq!(id.as_str(), "1abc");

        let id2 = PdbId::new("\tpdb_00001abc\n").unwrap();
        assert_eq!(id2.as_str(), "pdb_00001abc");
    }
}
