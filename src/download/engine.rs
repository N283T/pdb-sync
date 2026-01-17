//! Download engine selection and configuration.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Download engine type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EngineType {
    /// Built-in HTTPS downloader using reqwest
    #[default]
    Builtin,
    /// External aria2c downloader for parallel downloads
    Aria2c,
}

impl fmt::Display for EngineType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineType::Builtin => write!(f, "builtin"),
            EngineType::Aria2c => write!(f, "aria2c"),
        }
    }
}

impl FromStr for EngineType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "builtin" => Ok(EngineType::Builtin),
            "aria2c" => Ok(EngineType::Aria2c),
            _ => Err(format!("Unknown engine type: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_type_display() {
        assert_eq!(EngineType::Builtin.to_string(), "builtin");
        assert_eq!(EngineType::Aria2c.to_string(), "aria2c");
    }

    #[test]
    fn test_engine_type_from_str() {
        let parse = |s: &str| s.parse::<EngineType>();
        assert_eq!(parse("builtin").unwrap(), EngineType::Builtin);
        assert_eq!(parse("aria2c").unwrap(), EngineType::Aria2c);
        assert_eq!(parse("BUILTIN").unwrap(), EngineType::Builtin);
        assert_eq!(parse("Aria2c").unwrap(), EngineType::Aria2c);
        assert!(parse("unknown").is_err());
    }

    #[test]
    fn test_engine_type_default() {
        assert_eq!(EngineType::default(), EngineType::Builtin);
    }

    #[test]
    fn test_engine_type_serde() {
        let engine = EngineType::Aria2c;
        let json = serde_json::to_string(&engine).unwrap();
        assert_eq!(json, "\"aria2c\"");

        let deserialized: EngineType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, EngineType::Aria2c);
    }
}
