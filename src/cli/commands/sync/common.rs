//! Shared utilities for sync command handlers.

use std::path::{Component, Path};

/// Validate a subpath to prevent path traversal attacks.
///
/// Returns an error if the subpath contains dangerous patterns like `..`
/// or absolute path components.
pub fn validate_subpath(subpath: &str) -> Result<(), &'static str> {
    // Check for null bytes
    if subpath.contains('\0') {
        return Err("Subpath cannot contain null bytes");
    }

    let path = Path::new(subpath);
    if path.is_absolute() {
        return Err("Subpath must be relative");
    }

    for component in path.components() {
        match component {
            Component::ParentDir => {
                return Err("Subpath cannot contain '..' (path traversal)");
            }
            Component::Prefix(_) => {
                return Err("Subpath cannot contain Windows drive prefixes");
            }
            Component::RootDir => {
                return Err("Subpath cannot be absolute");
            }
            Component::CurDir | Component::Normal(_) => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_subpath_valid() {
        assert!(validate_subpath("foo/bar").is_ok());
        assert!(validate_subpath("data/2024").is_ok());
        assert!(validate_subpath("").is_ok());
    }

    #[test]
    fn test_validate_subpath_path_traversal() {
        assert!(validate_subpath("../etc/passwd").is_err());
        assert!(validate_subpath("foo/../bar").is_err());
        assert!(validate_subpath("foo/..").is_err());
    }

    #[test]
    fn test_validate_subpath_null_byte() {
        assert!(validate_subpath("foo\0bar").is_err());
    }

    #[test]
    fn test_validate_subpath_absolute() {
        assert!(validate_subpath("/etc/passwd").is_err());
    }
}
