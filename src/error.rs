use thiserror::Error;

#[derive(Error, Debug)]
pub enum PdbCliError {
    #[error("Invalid PDB ID: {0}")]
    InvalidPdbId(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("rsync failed: {0}")]
    Rsync(String),

    #[error("Unknown mirror: {0}")]
    UnknownMirror(String),

    #[error("Path error: {0}")]
    Path(String),

    #[error("Download failed: {0}")]
    Download(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[allow(dead_code)]
    #[error("Checksum mismatch for {0}: expected {1}, got {2}")]
    ChecksumMismatch(String, String, String),

    #[error("Checksum fetch failed: {0}")]
    ChecksumFetch(String),

    #[error("Entries not found: {0} of {1} entries missing")]
    EntriesNotFound(usize, usize),
}

pub type Result<T> = std::result::Result<T, PdbCliError>;
