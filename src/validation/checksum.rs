//! Checksum verification for PDB files.

use crate::error::{PdbCliError, Result};
use crate::mirrors::{Mirror, MirrorId};
use md5::{Digest, Md5};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

/// Result of verifying a file's checksum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyResult {
    /// File matches expected checksum.
    Valid,
    /// File checksum doesn't match.
    Invalid { expected: String, actual: String },
    /// Local file doesn't exist.
    Missing,
    /// No checksum available from mirror.
    NoChecksum,
}

impl VerifyResult {
    #[allow(dead_code)]
    pub fn is_valid(&self) -> bool {
        matches!(self, VerifyResult::Valid)
    }

    #[allow(dead_code)]
    pub fn is_error(&self) -> bool {
        matches!(self, VerifyResult::Invalid { .. } | VerifyResult::Missing)
    }
}

/// Verifier for PDB file checksums.
pub struct ChecksumVerifier {
    client: reqwest::Client,
    /// Cache of checksums by mirror and subpath.
    /// Key: (mirror, subpath), Value: filename -> checksum map
    cache: HashMap<(MirrorId, String), HashMap<String, String>>,
}

impl ChecksumVerifier {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("pdb-cli")
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            cache: HashMap::new(),
        }
    }

    /// Verify a local file against the mirror's checksum.
    pub async fn verify(
        &mut self,
        local_path: &Path,
        mirror: MirrorId,
        subpath: &str,
        filename: &str,
    ) -> Result<VerifyResult> {
        // Check if file exists
        if !local_path.exists() {
            return Ok(VerifyResult::Missing);
        }

        // Get expected checksum from mirror
        let expected = match self.get_checksum(mirror, subpath, filename).await {
            Some(checksum) => checksum,
            None => return Ok(VerifyResult::NoChecksum),
        };

        // Calculate actual checksum
        let actual = calculate_md5(local_path).await?;

        if actual.eq_ignore_ascii_case(&expected) {
            Ok(VerifyResult::Valid)
        } else {
            Ok(VerifyResult::Invalid { expected, actual })
        }
    }

    /// Get checksum for a specific file from cache or fetch it.
    async fn get_checksum(
        &mut self,
        mirror: MirrorId,
        subpath: &str,
        filename: &str,
    ) -> Option<String> {
        let key = (mirror, subpath.to_string());

        // Check cache first
        if let Some(checksums) = self.cache.get(&key) {
            return checksums.get(filename).cloned();
        }

        // Fetch checksums for this directory
        match self.fetch_checksums(mirror, subpath).await {
            Ok(checksums) => {
                let result = checksums.get(filename).cloned();
                self.cache.insert(key, checksums);
                result
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to fetch checksums for {}/{}: {}",
                    mirror,
                    subpath,
                    e
                );
                None
            }
        }
    }

    /// Fetch CHECKSUMS file from a mirror for a given subpath.
    async fn fetch_checksums(
        &self,
        mirror: MirrorId,
        subpath: &str,
    ) -> Result<HashMap<String, String>> {
        let url = build_checksums_url(mirror, subpath);
        tracing::debug!("Fetching checksums from: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| PdbCliError::ChecksumFetch(e.to_string()))?;

        if !response.status().is_success() {
            return Err(PdbCliError::ChecksumFetch(format!(
                "HTTP {} from {}",
                response.status(),
                url
            )));
        }

        let content = response
            .text()
            .await
            .map_err(|e| PdbCliError::ChecksumFetch(e.to_string()))?;

        Ok(parse_checksums(&content))
    }
}

impl Default for ChecksumVerifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the URL for a CHECKSUMS file on a mirror.
fn build_checksums_url(mirror: MirrorId, subpath: &str) -> String {
    let mirror_info = Mirror::get(mirror);

    // wwPDB archive structure: https://files.wwpdb.org/pub/pdb/data/
    // For structures/divided/mmCIF/ab/, the CHECKSUMS file is in the same directory.
    match mirror {
        MirrorId::Wwpdb => {
            format!("{}/data/{}/CHECKSUMS", mirror_info.https_base, subpath)
        }
        MirrorId::Rcsb => {
            // RCSB uses similar structure
            format!("https://files.rcsb.org/pub/pdb/data/{}/CHECKSUMS", subpath)
        }
        MirrorId::Pdbj => {
            // PDBj mirrors wwPDB structure
            format!("https://ftp.pdbj.org/pub/pdb/data/{}/CHECKSUMS", subpath)
        }
        MirrorId::Pdbe => {
            // PDBe mirrors wwPDB structure
            format!(
                "https://ftp.ebi.ac.uk/pub/databases/pdb/data/{}/CHECKSUMS",
                subpath
            )
        }
    }
}

/// Parse CHECKSUMS file content into a filename -> checksum map.
///
/// Supports two common formats:
/// - Format 1: `MD5 (filename) = hash`
/// - Format 2: `hash  filename` (hash followed by two spaces)
pub fn parse_checksums(content: &str) -> HashMap<String, String> {
    let mut checksums = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Try Format 1: MD5 (filename) = hash
        if line.starts_with("MD5") {
            if let Some((filename, hash)) = parse_md5_format(line) {
                checksums.insert(filename, hash);
                continue;
            }
        }

        // Try Format 2: hash  filename (32-char hash followed by two spaces)
        if let Some((filename, hash)) = parse_hash_filename_format(line) {
            checksums.insert(filename, hash);
        }
    }

    checksums
}

/// Validate filename to prevent path traversal attacks.
fn is_safe_filename(filename: &str) -> bool {
    !filename.contains('/')
        && !filename.contains('\\')
        && !filename.contains("..")
        && !filename.is_empty()
}

/// Parse Format 1: `MD5 (filename) = hash`
fn parse_md5_format(line: &str) -> Option<(String, String)> {
    // Example: "MD5 (1abc.cif.gz) = d41d8cd98f00b204e9800998ecf8427e"
    let line = line.strip_prefix("MD5")?;
    let line = line.trim();
    let line = line.strip_prefix('(')?;

    let paren_end = line.find(')')?;
    let filename = line[..paren_end].trim().to_string();

    // Validate filename to prevent path traversal
    if !is_safe_filename(&filename) {
        return None;
    }

    let rest = line[paren_end + 1..].trim();
    let rest = rest.strip_prefix('=')?;
    let hash = rest.trim().to_string();

    if hash.len() == 32 && hash.chars().all(|c| c.is_ascii_hexdigit()) {
        Some((filename, hash))
    } else {
        None
    }
}

/// Parse Format 2: `hash  filename` (hash followed by two spaces)
fn parse_hash_filename_format(line: &str) -> Option<(String, String)> {
    // Example: "d41d8cd98f00b204e9800998ecf8427e  1abc.cif.gz"
    // MD5 hash is 32 hex characters
    if line.len() < 34 {
        return None;
    }

    let hash = &line[..32];
    if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }

    // After hash, expect two spaces or a space then optional mode char (binary/text)
    // Formats: "  filename", " *filename" (binary mode), " filename"
    let rest = &line[32..];
    let filename = rest
        .strip_prefix("  ")
        .or_else(|| rest.strip_prefix(" *"))
        .or_else(|| rest.strip_prefix(' '))
        .map(str::trim)?;

    // Validate filename to prevent path traversal
    if !is_safe_filename(filename) {
        return None;
    }

    Some((filename.to_string(), hash.to_string()))
}

/// Calculate MD5 checksum of a file asynchronously.
pub async fn calculate_md5(path: &Path) -> Result<String> {
    let mut file = File::open(path).await?;
    let mut hasher = Md5::new();
    let mut buffer = [0u8; 8192]; // 8KB buffer

    loop {
        let bytes_read = file.read(&mut buffer).await?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_calculate_md5() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"hello world").unwrap();
        temp.flush().unwrap();

        let checksum = calculate_md5(temp.path()).await.unwrap();
        // MD5 of "hello world" is "5eb63bbbe01eeed093cb22bb8f5acdc3"
        assert_eq!(checksum, "5eb63bbbe01eeed093cb22bb8f5acdc3");
    }

    #[tokio::test]
    async fn test_calculate_md5_empty_file() {
        let temp = NamedTempFile::new().unwrap();

        let checksum = calculate_md5(temp.path()).await.unwrap();
        // MD5 of empty file is "d41d8cd98f00b204e9800998ecf8427e"
        assert_eq!(checksum, "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn test_parse_checksum_format1() {
        let content = r#"
MD5 (1abc.cif.gz) = d41d8cd98f00b204e9800998ecf8427e
MD5 (2xyz.cif.gz) = 5eb63bbbe01eeed093cb22bb8f5acdc3
# This is a comment
"#;

        let checksums = parse_checksums(content);
        assert_eq!(checksums.len(), 2);
        assert_eq!(
            checksums.get("1abc.cif.gz"),
            Some(&"d41d8cd98f00b204e9800998ecf8427e".to_string())
        );
        assert_eq!(
            checksums.get("2xyz.cif.gz"),
            Some(&"5eb63bbbe01eeed093cb22bb8f5acdc3".to_string())
        );
    }

    #[test]
    fn test_parse_checksum_format2() {
        let content = r#"
d41d8cd98f00b204e9800998ecf8427e  1abc.cif.gz
5eb63bbbe01eeed093cb22bb8f5acdc3  2xyz.cif.gz
"#;

        let checksums = parse_checksums(content);
        assert_eq!(checksums.len(), 2);
        assert_eq!(
            checksums.get("1abc.cif.gz"),
            Some(&"d41d8cd98f00b204e9800998ecf8427e".to_string())
        );
        assert_eq!(
            checksums.get("2xyz.cif.gz"),
            Some(&"5eb63bbbe01eeed093cb22bb8f5acdc3".to_string())
        );
    }

    #[test]
    fn test_parse_checksum_mixed_formats() {
        let content = r#"
MD5 (format1.cif.gz) = d41d8cd98f00b204e9800998ecf8427e
5eb63bbbe01eeed093cb22bb8f5acdc3  format2.cif.gz
"#;

        let checksums = parse_checksums(content);
        assert_eq!(checksums.len(), 2);
        assert!(checksums.contains_key("format1.cif.gz"));
        assert!(checksums.contains_key("format2.cif.gz"));
    }

    #[test]
    fn test_parse_checksum_binary_mode() {
        // Binary mode uses * before filename
        let content = "d41d8cd98f00b204e9800998ecf8427e *binary.cif.gz\n";

        let checksums = parse_checksums(content);
        assert_eq!(checksums.len(), 1);
        assert!(checksums.contains_key("binary.cif.gz"));
    }

    #[test]
    fn test_parse_checksum_invalid_lines() {
        let content = r#"
This is not a checksum line
MD5 (missing_equals) hash
shortline
"#;

        let checksums = parse_checksums(content);
        assert!(checksums.is_empty());
    }

    #[test]
    fn test_verify_result_methods() {
        assert!(VerifyResult::Valid.is_valid());
        assert!(!VerifyResult::Missing.is_valid());

        assert!(VerifyResult::Missing.is_error());
        assert!(VerifyResult::Invalid {
            expected: "a".into(),
            actual: "b".into()
        }
        .is_error());
        assert!(!VerifyResult::Valid.is_error());
        assert!(!VerifyResult::NoChecksum.is_error());
    }

    #[test]
    fn test_parse_checksum_path_traversal_rejected() {
        // Path traversal attempts should be rejected
        let content = r#"
d41d8cd98f00b204e9800998ecf8427e  ../../../etc/passwd
d41d8cd98f00b204e9800998ecf8427e  foo/bar.txt
d41d8cd98f00b204e9800998ecf8427e  foo\bar.txt
MD5 (../secret.txt) = d41d8cd98f00b204e9800998ecf8427e
MD5 (path/to/file.txt) = d41d8cd98f00b204e9800998ecf8427e
"#;

        let checksums = parse_checksums(content);
        // All path traversal attempts should be rejected
        assert!(checksums.is_empty());
    }
}
