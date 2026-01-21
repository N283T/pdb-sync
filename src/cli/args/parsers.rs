//! Custom value parsers and validators for CLI arguments.
//!
//! This module provides validation functions and custom parsers for CLI argument values.

/// Validate resolution filter (must be in range 0.0-100.0).
///
/// # Examples
///
/// ```
/// use pdb_sync::cli::args::parsers::validate_resolution;
///
/// assert!(validate_resolution("1.5").is_ok());
/// assert!(validate_resolution("0.0").is_ok());
/// assert!(validate_resolution("100.0").is_ok());
/// assert!(validate_resolution("-0.1").is_err());
/// assert!(validate_resolution("100.1").is_err());
/// assert!(validate_resolution("abc").is_err());
/// assert!(validate_resolution("inf").is_err());
/// assert!(validate_resolution("NaN").is_err());
/// ```
pub fn validate_resolution(s: &str) -> Result<f64, String> {
    let value: f64 = s.parse().map_err(|_| format!("Invalid number: {}", s))?;

    // Reject special float values (inf, -inf, NaN)
    if !value.is_finite() {
        return Err(format!("Resolution must be a finite number, got {}", s));
    }

    if !(0.0..=100.0).contains(&value) {
        return Err(format!(
            "Resolution must be between 0.0 and 100.0, got {}",
            value
        ));
    }
    Ok(value)
}

/// Validate organism filter string (max 200 chars, alphanumeric + basic punctuation).
///
/// Allowed characters: alphanumeric, space, hyphen, period, underscore, parentheses
///
/// # Examples
///
/// ```
/// use pdb_sync::cli::args::parsers::validate_organism;
///
/// assert!(validate_organism("Homo sapiens").is_ok());
/// assert!(validate_organism("Escherichia coli").is_ok());
/// assert!(validate_organism("Mus musculus (mouse)").is_ok());
/// assert!(validate_organism("test@invalid").is_err());
/// assert!(validate_organism("").is_err());
/// ```
pub fn validate_organism(s: &str) -> Result<String, String> {
    const MAX_LEN: usize = 200;

    if s.is_empty() {
        return Err("Organism name cannot be empty".into());
    }

    if s.len() > MAX_LEN {
        return Err(format!(
            "Organism name too long ({} chars, max {})",
            s.len(),
            MAX_LEN
        ));
    }

    // Allow alphanumeric, spaces, hyphens, periods, underscores, parentheses
    if s.chars()
        .all(|c| c.is_alphanumeric() || " -._()".contains(c))
    {
        Ok(s.to_string())
    } else {
        Err(
            "Organism name contains invalid characters (allowed: alphanumeric, space, -._())"
                .into(),
        )
    }
}

/// Validate interval string (e.g., "1h", "30m", "1d", "90s").
///
/// Valid formats:
/// - `Ns` - seconds (e.g., "30s")
/// - `Nm` - minutes (e.g., "5m")
/// - `Nh` - hours (e.g., "2h")
/// - `Nd` - days (e.g., "1d")
///
/// # Examples
///
/// ```
/// use pdb_sync::cli::args::parsers::validate_interval;
///
/// assert!(validate_interval("1h").is_ok());
/// assert!(validate_interval("30m").is_ok());
/// assert!(validate_interval("90s").is_ok());
/// assert!(validate_interval("1d").is_ok());
/// assert!(validate_interval("invalid").is_err());
/// assert!(validate_interval("1").is_err());
/// assert!(validate_interval("1x").is_err());
/// assert!(validate_interval("0s").is_err());
/// ```
pub fn validate_interval(s: &str) -> Result<String, String> {
    if s.len() < 2 {
        return Err("Interval too short (format: Ns, Nm, Nh, or Nd)".into());
    }

    let (num_part, unit) = s.split_at(s.len() - 1);

    // Validate numeric part
    let num: u64 = num_part
        .parse()
        .map_err(|_| format!("Invalid interval number: {}", num_part))?;

    // Reject zero intervals (would cause infinite loops in polling)
    if num == 0 {
        return Err("Interval must be greater than 0".into());
    }

    // Validate unit
    match unit {
        "s" | "m" | "h" | "d" => Ok(s.to_string()),
        _ => Err(format!(
            "Invalid interval unit: {} (must be s, m, h, or d)",
            unit
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_resolution() {
        // Valid resolutions
        assert!(validate_resolution("0.0").is_ok());
        assert!(validate_resolution("1.5").is_ok());
        assert!(validate_resolution("100.0").is_ok());

        // Invalid resolutions
        assert!(validate_resolution("-0.1").is_err());
        assert!(validate_resolution("100.1").is_err());
        assert!(validate_resolution("abc").is_err());

        // Special float values (should be rejected)
        assert!(validate_resolution("inf").is_err());
        assert!(validate_resolution("-inf").is_err());
        assert!(validate_resolution("NaN").is_err());
        assert!(validate_resolution("infinity").is_err());
    }

    #[test]
    fn test_validate_organism() {
        // Valid organisms
        assert!(validate_organism("Homo sapiens").is_ok());
        assert!(validate_organism("Escherichia coli").is_ok());
        assert!(validate_organism("Mus musculus (mouse)").is_ok());

        // Too long
        let long_name = "a".repeat(201);
        let result = validate_organism(&long_name);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("201 chars"));

        // Invalid characters
        assert!(validate_organism("test@invalid").is_err());
        assert!(validate_organism("test;injection").is_err());

        // Empty string
        assert!(validate_organism("").is_err());
    }

    #[test]
    fn test_validate_interval() {
        // Valid intervals
        assert!(validate_interval("30s").is_ok());
        assert!(validate_interval("5m").is_ok());
        assert!(validate_interval("2h").is_ok());
        assert!(validate_interval("1d").is_ok());
        assert!(validate_interval("90s").is_ok());

        // Invalid intervals
        assert!(validate_interval("invalid").is_err());
        assert!(validate_interval("1").is_err()); // No unit
        assert!(validate_interval("1x").is_err()); // Invalid unit
        assert!(validate_interval("").is_err()); // Empty
        assert!(validate_interval("s").is_err()); // No number
        assert!(validate_interval("-1m").is_err()); // Negative

        // Zero intervals (should be rejected)
        assert!(validate_interval("0s").is_err());
        assert!(validate_interval("0m").is_err());
        assert!(validate_interval("0h").is_err());
        assert!(validate_interval("0d").is_err());
    }
}
