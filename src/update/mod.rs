//! Update checking and downloading for local PDB files.

use crate::download::{DownloadOptions, HttpsDownloader};
use crate::error::Result;
use crate::files::{paths::build_relative_path, FileFormat, PdbId};
use crate::mirrors::{Mirror, MirrorId};
use crate::validation::ChecksumVerifier;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tokio::sync::Semaphore;

/// Tolerance in seconds for timestamp comparison.
/// Files modified within this window are considered up-to-date.
const TIMESTAMP_TOLERANCE_SECS: i64 = 5;

/// Status of a file compared to the remote mirror.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum UpdateStatus {
    /// File is up-to-date (local mtime >= remote mtime within tolerance).
    UpToDate,
    /// File is outdated (remote is newer).
    Outdated {
        local_time: DateTime<Utc>,
        remote_time: DateTime<Utc>,
    },
    /// Remote file not found (possibly obsolete entry).
    Missing,
    /// Could not determine status.
    Unknown { reason: String },
    /// Successfully updated (after download).
    Updated,
    /// Update failed.
    UpdateFailed { error: String },
}

impl UpdateStatus {
    pub fn is_outdated(&self) -> bool {
        matches!(self, UpdateStatus::Outdated { .. })
    }

    pub fn is_up_to_date(&self) -> bool {
        matches!(self, UpdateStatus::UpToDate)
    }

    #[allow(dead_code)]
    pub fn is_missing(&self) -> bool {
        matches!(self, UpdateStatus::Missing)
    }

    #[allow(dead_code)]
    pub fn is_error(&self) -> bool {
        matches!(
            self,
            UpdateStatus::UpdateFailed { .. } | UpdateStatus::Unknown { .. }
        )
    }
}

/// Result of checking a single file for updates.
#[derive(Debug, Clone, Serialize)]
pub struct UpdateResult {
    pub pdb_id: String,
    pub status: UpdateStatus,
    pub local_path: PathBuf,
}

impl UpdateResult {
    pub fn new(pdb_id: &PdbId, status: UpdateStatus, local_path: PathBuf) -> Self {
        Self {
            pdb_id: pdb_id.to_string(),
            status,
            local_path,
        }
    }
}

/// Checker for file update status with parallel support.
pub struct UpdateChecker {
    client: reqwest::Client,
    semaphore: Arc<Semaphore>,
}

impl UpdateChecker {
    /// Create a new update checker with the specified parallelism.
    pub fn new(parallel: usize) -> Self {
        let semaphore = Arc::new(Semaphore::new(parallel));
        let client = reqwest::Client::builder()
            .user_agent("pdb-cli")
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30)) // Shorter timeout for HEAD requests
            .build()
            .expect("Failed to create HTTP client");

        Self { client, semaphore }
    }

    /// Check update status using HTTP HEAD request (fast, default).
    ///
    /// Compares local file mtime with remote Last-Modified header.
    pub async fn check_status_head(
        &self,
        pdb_id: &PdbId,
        local_path: &Path,
        mirror: MirrorId,
        format: FileFormat,
    ) -> UpdateStatus {
        // Get local file mtime
        let local_mtime = match get_local_mtime(local_path).await {
            Ok(mtime) => mtime,
            Err(e) => {
                return UpdateStatus::Unknown {
                    reason: format!("Failed to get local mtime: {}", e),
                }
            }
        };

        // Build URL for this file
        let url = build_structure_url(pdb_id, format, mirror);

        // Send HEAD request
        let response = match self.client.head(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                return UpdateStatus::Unknown {
                    reason: format!("HEAD request failed: {}", e),
                }
            }
        };

        // Handle response status
        match response.status() {
            status if status.is_success() => {
                // Parse Last-Modified header
                if let Some(last_modified) = response.headers().get("last-modified") {
                    match parse_http_date(last_modified.to_str().unwrap_or("")) {
                        Some(remote_mtime) => compare_timestamps(local_mtime, remote_mtime),
                        None => UpdateStatus::Unknown {
                            reason: "Failed to parse Last-Modified header".to_string(),
                        },
                    }
                } else {
                    UpdateStatus::Unknown {
                        reason: "No Last-Modified header in response".to_string(),
                    }
                }
            }
            reqwest::StatusCode::NOT_FOUND => UpdateStatus::Missing,
            reqwest::StatusCode::METHOD_NOT_ALLOWED => UpdateStatus::Unknown {
                reason: "HEAD method not allowed by server".to_string(),
            },
            status => UpdateStatus::Unknown {
                reason: format!("HTTP {}", status),
            },
        }
    }

    /// Check update status using checksum verification (accurate but slower).
    ///
    /// Compares local file checksum with remote checksum from CHECKSUMS file.
    pub async fn check_status_checksum(
        &self,
        pdb_id: &PdbId,
        local_path: &Path,
        verifier: &mut ChecksumVerifier,
        format: FileFormat,
        mirror: MirrorId,
    ) -> UpdateStatus {
        // Build the subpath for checksum lookup
        let subpath = build_checksum_subpath(format, pdb_id);
        let filename = local_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        match verifier
            .verify(local_path, mirror, &subpath, &filename)
            .await
        {
            Ok(result) => match result {
                crate::validation::VerifyResult::Valid => UpdateStatus::UpToDate,
                crate::validation::VerifyResult::Invalid { .. } => {
                    // If checksum doesn't match, the file needs updating
                    // We don't have timestamps here, so we just mark as outdated
                    // with dummy timestamps
                    UpdateStatus::Outdated {
                        local_time: Utc::now(),
                        remote_time: Utc::now(),
                    }
                }
                crate::validation::VerifyResult::Missing => UpdateStatus::Unknown {
                    reason: "Local file missing".to_string(),
                },
                crate::validation::VerifyResult::NoChecksum => UpdateStatus::Unknown {
                    reason: "No checksum available for this file".to_string(),
                },
            },
            Err(e) => UpdateStatus::Unknown {
                reason: format!("Checksum verification failed: {}", e),
            },
        }
    }

    /// Check many files in parallel.
    pub async fn check_many(
        &self,
        files: Vec<(PdbId, PathBuf)>,
        mirror: MirrorId,
        format: FileFormat,
        verify: bool,
    ) -> Vec<UpdateResult> {
        if verify {
            // For checksum mode, we need to share the verifier
            // Process sequentially to share the checksum cache
            let mut verifier = ChecksumVerifier::new();
            let mut results = Vec::with_capacity(files.len());

            for (pdb_id, local_path) in files {
                let status = self
                    .check_status_checksum(&pdb_id, &local_path, &mut verifier, format, mirror)
                    .await;
                results.push(UpdateResult::new(&pdb_id, status, local_path));
            }

            results
        } else {
            // For HEAD mode, check in parallel
            let futures: Vec<_> = files
                .into_iter()
                .map(|(pdb_id, local_path)| {
                    let semaphore = Arc::clone(&self.semaphore);
                    let client = self.client.clone();
                    async move {
                        let _permit = semaphore.acquire().await.expect("Semaphore closed");
                        let checker = UpdateChecker {
                            client,
                            semaphore: Arc::new(Semaphore::new(1)), // Dummy, not used
                        };
                        let status = checker
                            .check_status_head(&pdb_id, &local_path, mirror, format)
                            .await;
                        UpdateResult::new(&pdb_id, status, local_path)
                    }
                })
                .collect();

            futures_util::future::join_all(futures).await
        }
    }
}

/// Download updated files.
pub async fn download_updates(
    outdated: &[&UpdateResult],
    mirror: MirrorId,
    format: FileFormat,
    pdb_dir: &Path,
) -> Result<Vec<UpdateResult>> {
    if outdated.is_empty() {
        return Ok(Vec::new());
    }

    let downloader = HttpsDownloader::new(DownloadOptions {
        mirror,
        decompress: false,
        overwrite: true,
        parallel: 4,
        ..Default::default()
    });

    let mut results = Vec::with_capacity(outdated.len());

    for item in outdated {
        let pdb_id = match PdbId::new(&item.pdb_id) {
            Ok(id) => id,
            Err(_) => {
                results.push(UpdateResult {
                    pdb_id: item.pdb_id.clone(),
                    status: UpdateStatus::UpdateFailed {
                        error: "Invalid PDB ID".to_string(),
                    },
                    local_path: item.local_path.clone(),
                });
                continue;
            }
        };

        // Build the destination directory for this format/hash
        let relative_path = build_relative_path(&pdb_id, format);
        let full_path = pdb_dir.join(&relative_path);

        // Delete the old file
        if full_path.exists() {
            if let Err(e) = fs::remove_file(&full_path).await {
                results.push(UpdateResult {
                    pdb_id: item.pdb_id.clone(),
                    status: UpdateStatus::UpdateFailed {
                        error: format!("Failed to remove old file: {}", e),
                    },
                    local_path: item.local_path.clone(),
                });
                continue;
            }
        }

        // Ensure parent directory exists
        if let Some(parent) = full_path.parent() {
            if let Err(e) = fs::create_dir_all(parent).await {
                results.push(UpdateResult {
                    pdb_id: item.pdb_id.clone(),
                    status: UpdateStatus::UpdateFailed {
                        error: format!("Failed to create directory: {}", e),
                    },
                    local_path: item.local_path.clone(),
                });
                continue;
            }
        }

        // Download fresh copy
        let dest = full_path.parent().unwrap_or(pdb_dir);
        match downloader.download(&pdb_id, format, dest).await {
            Ok(()) => {
                results.push(UpdateResult {
                    pdb_id: item.pdb_id.clone(),
                    status: UpdateStatus::Updated,
                    local_path: full_path,
                });
            }
            Err(e) => {
                results.push(UpdateResult {
                    pdb_id: item.pdb_id.clone(),
                    status: UpdateStatus::UpdateFailed {
                        error: e.to_string(),
                    },
                    local_path: item.local_path.clone(),
                });
            }
        }
    }

    Ok(results)
}

/// Get the modification time of a local file as UTC timestamp.
async fn get_local_mtime(path: &Path) -> Result<DateTime<Utc>> {
    let metadata = fs::metadata(path).await?;
    let mtime = metadata.modified()?;
    Ok(DateTime::<Utc>::from(mtime))
}

/// Parse HTTP date format (RFC 2822 / RFC 7231).
///
/// Example: "Wed, 21 Oct 2015 07:28:00 GMT"
fn parse_http_date(date_str: &str) -> Option<DateTime<Utc>> {
    // Try RFC 2822 format first
    if let Ok(dt) = DateTime::parse_from_rfc2822(date_str) {
        return Some(dt.with_timezone(&Utc));
    }

    // Try HTTP date format (RFC 7231)
    // Format: "Wed, 21 Oct 2015 07:28:00 GMT"
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(date_str, "%a, %d %b %Y %H:%M:%S GMT") {
        return Some(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
    }

    // Try alternative format without timezone
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(date_str, "%a, %d %b %Y %T") {
        return Some(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
    }

    None
}

/// Compare local and remote timestamps with tolerance.
fn compare_timestamps(local: DateTime<Utc>, remote: DateTime<Utc>) -> UpdateStatus {
    let diff_secs = (remote - local).num_seconds();

    if diff_secs > TIMESTAMP_TOLERANCE_SECS {
        // Remote is newer by more than tolerance
        UpdateStatus::Outdated {
            local_time: local,
            remote_time: remote,
        }
    } else {
        // Local is up-to-date (same or newer)
        UpdateStatus::UpToDate
    }
}

/// Build URL for structure files.
///
/// Replicates logic from HttpsDownloader::build_structure_url.
fn build_structure_url(pdb_id: &PdbId, format: FileFormat, mirror: MirrorId) -> String {
    let mirror_info = Mirror::get(mirror);
    let id = pdb_id.as_str();
    let base = format.base_format();

    match mirror {
        MirrorId::Rcsb => match base {
            FileFormat::Pdb => format!("{}/{}.pdb", mirror_info.https_base, id),
            FileFormat::Mmcif => format!("{}/{}.cif", mirror_info.https_base, id),
            FileFormat::Bcif => format!("https://models.rcsb.org/{}.bcif", id),
            _ => unreachable!(),
        },
        MirrorId::Pdbj => match base {
            FileFormat::Pdb => format!("{}?format=pdb&id={}", mirror_info.https_base, id),
            FileFormat::Mmcif => format!("{}?format=mmcif&id={}", mirror_info.https_base, id),
            FileFormat::Bcif => format!("{}?format=mmcif&id={}", mirror_info.https_base, id),
            _ => unreachable!(),
        },
        MirrorId::Pdbe => match base {
            FileFormat::Pdb => {
                if pdb_id.is_classic() {
                    format!("{}/pdb{}.ent", mirror_info.https_base, id)
                } else {
                    format!("{}/{}.ent", mirror_info.https_base, id)
                }
            }
            FileFormat::Mmcif => format!("{}/{}.cif", mirror_info.https_base, id),
            FileFormat::Bcif => format!("{}/{}.cif", mirror_info.https_base, id),
            _ => unreachable!(),
        },
        MirrorId::Wwpdb => {
            let middle = pdb_id.middle_chars();
            match base {
                FileFormat::Pdb => {
                    if pdb_id.is_classic() {
                        format!(
                            "{}/divided/pdb/{}/pdb{}.ent.gz",
                            mirror_info.https_base, middle, id
                        )
                    } else {
                        format!(
                            "{}/divided/pdb/{}/{}.ent.gz",
                            mirror_info.https_base, middle, id
                        )
                    }
                }
                FileFormat::Mmcif => {
                    format!(
                        "{}/divided/mmCIF/{}/{}.cif.gz",
                        mirror_info.https_base, middle, id
                    )
                }
                FileFormat::Bcif => {
                    format!(
                        "{}/divided/mmCIF/{}/{}.cif.gz",
                        mirror_info.https_base, middle, id
                    )
                }
                _ => unreachable!(),
            }
        }
    }
}

/// Build the checksum subpath for a given format and PDB ID.
fn build_checksum_subpath(format: FileFormat, pdb_id: &PdbId) -> String {
    let middle = pdb_id.middle_chars();
    match format {
        FileFormat::Pdb | FileFormat::PdbGz => {
            format!("structures/divided/pdb/{}", middle)
        }
        FileFormat::Mmcif | FileFormat::CifGz => {
            format!("structures/divided/mmCIF/{}", middle)
        }
        FileFormat::Bcif | FileFormat::BcifGz => {
            format!("structures/divided/bcif/{}", middle)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === UpdateStatus tests ===

    #[test]
    fn test_update_status_is_outdated() {
        let outdated = UpdateStatus::Outdated {
            local_time: Utc::now(),
            remote_time: Utc::now(),
        };
        assert!(outdated.is_outdated());
        assert!(!UpdateStatus::UpToDate.is_outdated());
    }

    #[test]
    fn test_update_status_is_up_to_date() {
        assert!(UpdateStatus::UpToDate.is_up_to_date());
        assert!(!UpdateStatus::Missing.is_up_to_date());
    }

    #[test]
    fn test_update_status_is_error() {
        assert!(UpdateStatus::Unknown {
            reason: "test".into()
        }
        .is_error());
        assert!(UpdateStatus::UpdateFailed {
            error: "test".into()
        }
        .is_error());
        assert!(!UpdateStatus::UpToDate.is_error());
    }

    #[test]
    fn test_update_status_serialization() {
        let status = UpdateStatus::UpToDate;
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"status\":\"up_to_date\""));

        let outdated = UpdateStatus::Outdated {
            local_time: DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            remote_time: DateTime::parse_from_rfc3339("2024-01-02T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
        };
        let json = serde_json::to_string(&outdated).unwrap();
        assert!(json.contains("\"status\":\"outdated\""));
        assert!(json.contains("\"local_time\""));
        assert!(json.contains("\"remote_time\""));
    }

    // === HTTP date parsing tests ===

    #[test]
    fn test_parse_http_date_rfc7231() {
        let date_str = "Wed, 21 Oct 2015 07:28:00 GMT";
        let parsed = parse_http_date(date_str);
        assert!(parsed.is_some());
        let dt = parsed.unwrap();
        assert_eq!(dt.year(), 2015);
        assert_eq!(dt.month(), 10);
        assert_eq!(dt.day(), 21);
    }

    #[test]
    fn test_parse_http_date_invalid() {
        assert!(parse_http_date("invalid date").is_none());
        assert!(parse_http_date("").is_none());
    }

    // === Timestamp comparison tests ===

    #[test]
    fn test_compare_timestamps_outdated() {
        let local = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let remote = DateTime::parse_from_rfc3339("2024-01-02T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let status = compare_timestamps(local, remote);
        assert!(status.is_outdated());
    }

    #[test]
    fn test_compare_timestamps_up_to_date() {
        let local = DateTime::parse_from_rfc3339("2024-01-02T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let remote = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let status = compare_timestamps(local, remote);
        assert!(status.is_up_to_date());
    }

    #[test]
    fn test_compare_timestamps_within_tolerance() {
        let local = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let remote = DateTime::parse_from_rfc3339("2024-01-01T00:00:03Z")
            .unwrap()
            .with_timezone(&Utc);

        // 3 seconds difference is within 5-second tolerance
        let status = compare_timestamps(local, remote);
        assert!(status.is_up_to_date());
    }

    // === URL building tests ===

    #[test]
    fn test_build_structure_url_rcsb() {
        let pdb_id = PdbId::new("1abc").unwrap();
        let url = build_structure_url(&pdb_id, FileFormat::Mmcif, MirrorId::Rcsb);
        assert_eq!(url, "https://files.rcsb.org/download/1abc.cif");
    }

    #[test]
    fn test_build_structure_url_wwpdb() {
        let pdb_id = PdbId::new("1abc").unwrap();
        let url = build_structure_url(&pdb_id, FileFormat::CifGz, MirrorId::Wwpdb);
        assert_eq!(
            url,
            "https://files.wwpdb.org/pub/pdb/data/structures/divided/mmCIF/ab/1abc.cif.gz"
        );
    }

    #[test]
    fn test_build_structure_url_extended() {
        let pdb_id = PdbId::new("pdb_00001abc").unwrap();
        let url = build_structure_url(&pdb_id, FileFormat::CifGz, MirrorId::Wwpdb);
        assert_eq!(
            url,
            "https://files.wwpdb.org/pub/pdb/data/structures/divided/mmCIF/00/pdb_00001abc.cif.gz"
        );
    }

    // === Checksum subpath tests ===

    #[test]
    fn test_build_checksum_subpath_mmcif() {
        let pdb_id = PdbId::new("1abc").unwrap();
        let subpath = build_checksum_subpath(FileFormat::CifGz, &pdb_id);
        assert_eq!(subpath, "structures/divided/mmCIF/ab");
    }

    #[test]
    fn test_build_checksum_subpath_pdb() {
        let pdb_id = PdbId::new("1abc").unwrap();
        let subpath = build_checksum_subpath(FileFormat::PdbGz, &pdb_id);
        assert_eq!(subpath, "structures/divided/pdb/ab");
    }

    // === chrono import for year(), month(), day() ===
    use chrono::Datelike;
}
