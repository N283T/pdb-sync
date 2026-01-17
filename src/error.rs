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
}

pub type Result<T> = std::result::Result<T, PdbCliError>;
