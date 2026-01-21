//! Configuration value source tracking.
//!
//! This module provides types for tracking where configuration values originate from,
//! which is useful for debugging and improving error messages.

/// Where a configuration value came from.
///
/// The priority order (highest to lowest) is:
/// 1. Command-line arguments (CliArg)
/// 2. Environment variables (Env)
/// 3. Config file (Config)
/// 4. Default values (Default)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FlagSource {
    /// Default value (hardcoded in the application)
    Default,
    /// Config file value
    Config,
    /// Environment variable
    Env,
    /// Command-line argument
    CliArg,
}

impl FlagSource {
    /// Returns the priority of this source (higher = more important).
    #[allow(dead_code)]
    pub const fn priority(&self) -> u8 {
        match self {
            FlagSource::Default => 0,
            FlagSource::Config => 1,
            FlagSource::Env => 2,
            FlagSource::CliArg => 3,
        }
    }
}

impl std::fmt::Display for FlagSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlagSource::Default => write!(f, "default"),
            FlagSource::Config => write!(f, "config file"),
            FlagSource::Env => write!(f, "environment variable"),
            FlagSource::CliArg => write!(f, "command-line argument"),
        }
    }
}

/// A configuration value with its source.
///
/// This wrapper type tracks both the value and where it came from,
/// enabling better error messages and debugging output.
#[derive(Debug, Clone)]
pub struct SourcedValue<T> {
    /// The configuration value
    pub value: T,
    /// Where this value came from
    pub source: FlagSource,
}

impl<T> SourcedValue<T> {
    /// Create a new sourced value.
    #[allow(dead_code)]
    pub const fn new(value: T, source: FlagSource) -> Self {
        Self { value, source }
    }

    /// Create a sourced value with the default source.
    #[allow(dead_code)]
    pub const fn with_default(value: T) -> Self {
        Self {
            value,
            source: FlagSource::Default,
        }
    }

    /// Create a sourced value from a config file.
    #[allow(dead_code)]
    pub const fn from_config(value: T) -> Self {
        Self {
            value,
            source: FlagSource::Config,
        }
    }

    /// Create a sourced value from an environment variable.
    #[allow(dead_code)]
    pub const fn from_env(value: T) -> Self {
        Self {
            value,
            source: FlagSource::Env,
        }
    }

    /// Create a sourced value from a CLI argument.
    #[allow(dead_code)]
    pub const fn from_cli(value: T) -> Self {
        Self {
            value,
            source: FlagSource::CliArg,
        }
    }

    /// Map the value while preserving the source.
    #[allow(dead_code)]
    pub fn map<U, F>(self, f: F) -> SourcedValue<U>
    where
        F: FnOnce(T) -> U,
    {
        SourcedValue {
            value: f(self.value),
            source: self.source,
        }
    }

    /// Get a reference to the value.
    #[allow(dead_code)]
    pub const fn get(&self) -> &T {
        &self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flag_source_priority() {
        assert_eq!(FlagSource::Default.priority(), 0);
        assert_eq!(FlagSource::Config.priority(), 1);
        assert_eq!(FlagSource::Env.priority(), 2);
        assert_eq!(FlagSource::CliArg.priority(), 3);
    }

    #[test]
    fn test_flag_source_display() {
        assert_eq!(format!("{}", FlagSource::Default), "default");
        assert_eq!(format!("{}", FlagSource::Config), "config file");
        assert_eq!(format!("{}", FlagSource::Env), "environment variable");
        assert_eq!(format!("{}", FlagSource::CliArg), "command-line argument");
    }

    #[test]
    fn test_flag_source_ordering() {
        assert!(FlagSource::CliArg > FlagSource::Env);
        assert!(FlagSource::Env > FlagSource::Config);
        assert!(FlagSource::Config > FlagSource::Default);
    }

    #[test]
    fn test_sourced_value_constructors() {
        let default_val = SourcedValue::with_default(42);
        assert_eq!(default_val.value, 42);
        assert_eq!(default_val.source, FlagSource::Default);

        let config_val = SourcedValue::from_config("test");
        assert_eq!(config_val.value, "test");
        assert_eq!(config_val.source, FlagSource::Config);

        let env_val = SourcedValue::from_env(true);
        assert_eq!(env_val.value, true);
        assert_eq!(env_val.source, FlagSource::Env);

        let cli_val = SourcedValue::from_cli(3.14);
        assert_eq!(cli_val.value, 3.14);
        assert_eq!(cli_val.source, FlagSource::CliArg);
    }

    #[test]
    fn test_sourced_value_map() {
        let val = SourcedValue::new(10, FlagSource::Env);
        let mapped = val.map(|x| x * 2);

        assert_eq!(mapped.value, 20);
        assert_eq!(mapped.source, FlagSource::Env);
    }
}
