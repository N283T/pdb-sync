//! Format conversion utilities using external tools (gemmi).

use crate::error::{PdbSyncError, Result};
use crate::files::FileFormat;
use std::path::Path;
use tokio::process::Command;

/// Check if the gemmi CLI tool is available.
pub async fn check_gemmi_available() -> bool {
    Command::new("gemmi")
        .arg("--version")
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Convert a file between PDB and mmCIF formats using gemmi.
///
/// Note: This function assumes gemmi availability has already been checked
/// by the caller (via `check_gemmi_available()`).
pub async fn convert_with_gemmi(src: &Path, dest: &Path, _to_format: FileFormat) -> Result<()> {
    let mut cmd = Command::new("gemmi");
    // Use "--" to separate options from positional arguments to prevent
    // filenames starting with "-" from being interpreted as flags
    cmd.arg("convert").arg("--").arg(src).arg(dest);

    let output = cmd.output().await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PdbSyncError::Conversion(format!(
            "gemmi failed: {}",
            stderr
        )));
    }
    Ok(())
}

/// Detect the format of a file based on its extension.
pub fn detect_format_from_path(path: &Path) -> Option<FileFormat> {
    let name = path.file_name()?.to_str()?;
    let name_lower = name.to_lowercase();

    if name_lower.ends_with(".cif.gz") {
        Some(FileFormat::CifGz)
    } else if name_lower.ends_with(".ent.gz") || name_lower.ends_with(".pdb.gz") {
        Some(FileFormat::PdbGz)
    } else if name_lower.ends_with(".bcif.gz") {
        Some(FileFormat::BcifGz)
    } else if name_lower.ends_with(".cif") {
        Some(FileFormat::Mmcif)
    } else if name_lower.ends_with(".ent") || name_lower.ends_with(".pdb") {
        Some(FileFormat::Pdb)
    } else if name_lower.ends_with(".bcif") {
        Some(FileFormat::Bcif)
    } else if name_lower.ends_with(".gz") {
        // Generic .gz - could be either format, default to mmCIF
        Some(FileFormat::CifGz)
    } else {
        None
    }
}

/// Get the base filename without compression extension.
#[allow(dead_code)]
pub fn strip_compression_extension(path: &Path) -> std::path::PathBuf {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    if let Some(stripped) = name.strip_suffix(".gz") {
        path.with_file_name(stripped)
    } else {
        path.to_path_buf()
    }
}

/// Build the output filename for a converted file.
pub fn build_output_filename(src: &Path, to_format: FileFormat) -> std::path::PathBuf {
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("output");

    // Handle double extensions like .cif.gz
    let stem = if stem.ends_with(".cif")
        || stem.ends_with(".pdb")
        || stem.ends_with(".ent")
        || stem.ends_with(".bcif")
    {
        stem.rsplit('.').nth(1).unwrap_or(stem)
    } else {
        stem
    };

    let extension = to_format.extension();
    std::path::PathBuf::from(format!("{}.{}", stem, extension))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_format_cif_gz() {
        let path = Path::new("1abc.cif.gz");
        assert_eq!(detect_format_from_path(path), Some(FileFormat::CifGz));
    }

    #[test]
    fn test_detect_format_pdb_gz() {
        let path = Path::new("pdb1abc.ent.gz");
        assert_eq!(detect_format_from_path(path), Some(FileFormat::PdbGz));
    }

    #[test]
    fn test_detect_format_cif() {
        let path = Path::new("1abc.cif");
        assert_eq!(detect_format_from_path(path), Some(FileFormat::Mmcif));
    }

    #[test]
    fn test_detect_format_pdb() {
        let path = Path::new("1abc.pdb");
        assert_eq!(detect_format_from_path(path), Some(FileFormat::Pdb));

        let path = Path::new("pdb1abc.ent");
        assert_eq!(detect_format_from_path(path), Some(FileFormat::Pdb));
    }

    #[test]
    fn test_detect_format_bcif() {
        let path = Path::new("1abc.bcif");
        assert_eq!(detect_format_from_path(path), Some(FileFormat::Bcif));

        let path = Path::new("1abc.bcif.gz");
        assert_eq!(detect_format_from_path(path), Some(FileFormat::BcifGz));
    }

    #[test]
    fn test_detect_format_unknown() {
        let path = Path::new("1abc.txt");
        assert_eq!(detect_format_from_path(path), None);
    }

    #[test]
    fn test_strip_compression_extension() {
        assert_eq!(
            strip_compression_extension(Path::new("1abc.cif.gz")),
            std::path::PathBuf::from("1abc.cif")
        );
        assert_eq!(
            strip_compression_extension(Path::new("1abc.cif")),
            std::path::PathBuf::from("1abc.cif")
        );
    }

    #[test]
    fn test_build_output_filename() {
        let src = Path::new("1abc.cif");
        assert_eq!(
            build_output_filename(src, FileFormat::Pdb),
            std::path::PathBuf::from("1abc.pdb")
        );

        let src = Path::new("1abc.cif.gz");
        assert_eq!(
            build_output_filename(src, FileFormat::Pdb),
            std::path::PathBuf::from("1abc.pdb")
        );

        let src = Path::new("pdb1abc.ent.gz");
        assert_eq!(
            build_output_filename(src, FileFormat::Mmcif),
            std::path::PathBuf::from("pdb1abc.cif")
        );
    }

    #[tokio::test]
    async fn test_gemmi_not_found() {
        // This test verifies the error handling when gemmi is not available
        // We can't reliably test this if gemmi IS installed, so we just verify
        // the function runs without panicking
        let _ = check_gemmi_available().await;
    }
}
