//! Error types for pdb-sync.

use std::path::PathBuf;
use thiserror::Error;

/// Main error type for pdb-sync.
#[derive(Error, Debug)]
pub enum PdbSyncError {
    /// Invalid PDB ID format or content.
    #[error("Invalid PDB ID: {input}")]
    InvalidPdbId {
        /// The invalid input string
        input: String,
        /// Underlying error if available
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Configuration-related errors.
    #[error("Configuration error: {message}")]
    Config {
        /// Error message
        message: String,
        /// Config key that caused the error (if applicable)
        key: Option<String>,
        /// Underlying error if available
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// IO errors with context.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Network errors with URL and retriable information.
    #[error("Network error: {url}: {message}")]
    Network {
        /// URL that failed
        url: String,
        /// Error message
        message: String,
        /// Underlying reqwest error
        #[source]
        source: Option<reqwest::Error>,
        /// Whether this error is retriable
        is_retriable: bool,
    },

    /// TOML parsing errors.
    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    /// TOML serialization errors.
    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    /// JSON errors.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// rsync command failures.
    #[error("rsync failed: {command}")]
    Rsync {
        /// Command that was executed
        command: String,
        /// Exit code if available
        exit_code: Option<i32>,
        /// stderr output if available
        stderr: Option<String>,
    },

    /// Unknown mirror ID.
    #[error("Unknown mirror: {0}")]
    UnknownMirror(String),

    /// Path-related errors.
    #[error("Path error: {0}")]
    Path(String),

    /// Download failures with context.
    #[error("Download failed for {pdb_id} from {url}: {message}")]
    Download {
        /// PDB ID being downloaded
        pdb_id: String,
        /// URL that failed
        url: String,
        /// Error message
        message: String,
        /// Whether this error is retriable
        is_retriable: bool,
    },

    /// Entry not found with context.
    #[error("Not found: {pdb_id}")]
    NotFound {
        /// PDB ID that wasn't found
        pdb_id: String,
        /// Mirror that was queried (if applicable)
        mirror: Option<String>,
        /// URLs that were searched
        searched_urls: Vec<String>,
    },

    /// Invalid input.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Checksum mismatch with file information.
    #[allow(dead_code)]
    #[error(
        "Checksum mismatch for {pdb_id}: expected {expected}, got {actual} (file: {file_path})"
    )]
    ChecksumMismatch {
        /// PDB ID
        pdb_id: String,
        /// Expected checksum
        expected: String,
        /// Actual checksum
        actual: String,
        /// Path to file with mismatch
        file_path: PathBuf,
    },

    /// Checksum fetch failure.
    #[error("Checksum fetch failed: {0}")]
    ChecksumFetch(String),

    /// Multiple entries not found.
    #[error("Entries not found: {0} of {1} entries missing")]
    EntriesNotFound(usize, usize),

    /// Watch-related errors.
    #[allow(dead_code)]
    #[error("Watch error: {0}")]
    Watch(String),

    /// Search API errors.
    #[error("Search API error: {0}")]
    SearchApi(String),

    /// State persistence errors.
    #[error("State persistence error: {0}")]
    StatePersistence(String),

    /// Hook execution failures.
    #[error("Hook execution failed: {0}")]
    HookExecution(String),

    /// Notification errors.
    #[error("Notification error: {0}")]
    Notification(String),

    /// Invalid interval format.
    #[error("Invalid interval: {0}")]
    InvalidInterval(String),

    /// aria2c not found in PATH.
    #[allow(dead_code)]
    #[error("aria2c not found in PATH")]
    Aria2cNotFound,

    /// aria2c execution failure.
    #[allow(dead_code)]
    #[error("aria2c execution failed: {0}")]
    Aria2cFailed(String),

    /// Conversion errors.
    #[error("Conversion error: {0}")]
    Conversion(String),

    /// External tool not found.
    #[error("External tool not found: {0}")]
    ToolNotFound(String),

    /// Job-related errors.
    #[error("Job error: {0}")]
    Job(String),
}

impl PdbSyncError {
    /// Check if this error is retriable (e.g., network timeout).
    ///
    /// Returns `true` for transient failures that might succeed on retry.
    #[allow(dead_code)]
    pub fn is_retriable(&self) -> bool {
        match self {
            PdbSyncError::Network { is_retriable, .. } => *is_retriable,
            PdbSyncError::Download { is_retriable, .. } => *is_retriable,
            PdbSyncError::Rsync { exit_code, .. } => {
                // rsync exit codes that indicate temporary failures:
                // 5 - SSH connection failed
                // 10 - Error in socket I/O
                // 30 - Timeout in data send/receive
                matches!(exit_code, Some(5 | 10 | 30))
            }
            _ => false,
        }
    }

    /// Get the PDB ID associated with this error, if any.
    ///
    /// Returns the PDB ID if the error is related to a specific entry.
    #[allow(dead_code)]
    pub fn pdb_id(&self) -> Option<&str> {
        match self {
            PdbSyncError::InvalidPdbId { input, .. } => Some(input),
            PdbSyncError::Download { pdb_id, .. } => Some(pdb_id),
            PdbSyncError::ChecksumMismatch { pdb_id, .. } => Some(pdb_id),
            PdbSyncError::NotFound { pdb_id, .. } => Some(pdb_id),
            _ => None,
        }
    }

    /// Get the URL associated with this error, if any.
    #[allow(dead_code)]
    pub fn url(&self) -> Option<&str> {
        match self {
            PdbSyncError::Network { url, .. } => Some(url),
            PdbSyncError::Download { url, .. } => Some(url),
            _ => None,
        }
    }
}

// ===== Backward compatibility: Legacy error constructors =====

impl From<reqwest::Error> for PdbSyncError {
    fn from(err: reqwest::Error) -> Self {
        // Determine if retriable based on error type
        let is_retriable = err.is_timeout() || err.is_connect();

        PdbSyncError::Network {
            url: err
                .url()
                .map(|u| u.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            message: err.to_string(),
            source: Some(err),
            is_retriable,
        }
    }
}

/// Result type alias for pdb-sync.
pub type Result<T> = std::result::Result<T, PdbSyncError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_retriable_network_timeout() {
        let err = PdbSyncError::Network {
            url: "https://example.com".to_string(),
            message: "timeout".to_string(),
            source: None,
            is_retriable: true,
        };
        assert!(err.is_retriable());
    }

    #[test]
    fn test_is_retriable_network_not_retriable() {
        let err = PdbSyncError::Network {
            url: "https://example.com".to_string(),
            message: "404 Not Found".to_string(),
            source: None,
            is_retriable: false,
        };
        assert!(!err.is_retriable());
    }

    #[test]
    fn test_is_retriable_download() {
        let err = PdbSyncError::Download {
            pdb_id: "1abc".to_string(),
            url: "https://example.com/1abc.cif.gz".to_string(),
            message: "timeout".to_string(),
            is_retriable: true,
        };
        assert!(err.is_retriable());
    }

    #[test]
    fn test_is_retriable_rsync_timeout() {
        let err = PdbSyncError::Rsync {
            command: "rsync -avz src/ dest/".to_string(),
            exit_code: Some(30), // Timeout
            stderr: Some("timeout".to_string()),
        };
        assert!(err.is_retriable());
    }

    #[test]
    fn test_is_retriable_rsync_not_retriable() {
        let err = PdbSyncError::Rsync {
            command: "rsync -avz src/ dest/".to_string(),
            exit_code: Some(1), // Generic error
            stderr: Some("error".to_string()),
        };
        assert!(!err.is_retriable());
    }

    #[test]
    fn test_is_retriable_other_errors() {
        let err = PdbSyncError::InvalidInput("bad input".to_string());
        assert!(!err.is_retriable());
    }

    #[test]
    fn test_pdb_id_extraction() {
        let err = PdbSyncError::Download {
            pdb_id: "1abc".to_string(),
            url: "https://example.com/1abc.cif.gz".to_string(),
            message: "failed".to_string(),
            is_retriable: false,
        };
        assert_eq!(err.pdb_id(), Some("1abc"));
    }

    #[test]
    fn test_pdb_id_not_found() {
        let err = PdbSyncError::Network {
            url: "https://example.com".to_string(),
            message: "error".to_string(),
            source: None,
            is_retriable: false,
        };
        assert_eq!(err.pdb_id(), None);
    }

    #[test]
    fn test_url_extraction() {
        let err = PdbSyncError::Download {
            pdb_id: "1abc".to_string(),
            url: "https://example.com/1abc.cif.gz".to_string(),
            message: "failed".to_string(),
            is_retriable: false,
        };
        assert_eq!(err.url(), Some("https://example.com/1abc.cif.gz"));
    }

    #[test]
    fn test_checksum_mismatch_display() {
        let err = PdbSyncError::ChecksumMismatch {
            pdb_id: "1abc".to_string(),
            expected: "abc123".to_string(),
            actual: "def456".to_string(),
            file_path: PathBuf::from("/path/to/file.cif.gz"),
        };
        let msg = err.to_string();
        assert!(msg.contains("1abc"));
        assert!(msg.contains("abc123"));
        assert!(msg.contains("def456"));
        assert!(msg.contains("/path/to/file.cif.gz"));
    }
}
