//! Automatic mirror selection based on latency testing.
//!
//! Note: This module is currently unused after removing sync.mirror field,
//! but kept for potential future use with mirror auto-selection.

#![allow(dead_code)]

use crate::mirrors::{Mirror, MirrorId};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Thread-safe cache for mirror latencies.
pub struct LatencyCache {
    /// Map of mirror ID to (latency, timestamp)
    cache: RwLock<HashMap<MirrorId, (Duration, Instant)>>,
    /// Cache TTL
    ttl: Duration,
}

impl LatencyCache {
    /// Create a new latency cache with the given TTL.
    pub fn new(ttl: Duration) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            ttl,
        }
    }

    /// Get cached latency for a mirror if still valid.
    #[allow(dead_code)]
    pub async fn get(&self, mirror_id: MirrorId) -> Option<Duration> {
        let cache = self.cache.read().await;
        cache.get(&mirror_id).and_then(|(latency, timestamp)| {
            if timestamp.elapsed() < self.ttl {
                Some(*latency)
            } else {
                None
            }
        })
    }

    /// Store latency for a mirror.
    pub async fn set(&self, mirror_id: MirrorId, latency: Duration) {
        let mut cache = self.cache.write().await;
        cache.insert(mirror_id, (latency, Instant::now()));
    }

    /// Get all cached entries that are still valid.
    pub async fn get_all_valid(&self) -> HashMap<MirrorId, Duration> {
        let cache = self.cache.read().await;
        cache
            .iter()
            .filter_map(|(id, (latency, timestamp))| {
                if timestamp.elapsed() < self.ttl {
                    Some((*id, *latency))
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Global latency cache.
///
/// Note: The cache is lazily initialized on first use. The TTL is only set during
/// initialization, so subsequent calls with different TTL values will use the
/// originally configured TTL. This is by design for CLI usage where configuration
/// is typically constant throughout a session.
static LATENCY_CACHE: std::sync::OnceLock<LatencyCache> = std::sync::OnceLock::new();

fn get_cache(ttl: Duration) -> &'static LatencyCache {
    LATENCY_CACHE.get_or_init(|| LatencyCache::new(ttl))
}

/// Select the best mirror based on latency testing.
///
/// # Arguments
/// * `preferred_region` - Optional preferred region (e.g., "us", "jp", "europe")
/// * `cache_ttl` - TTL for latency cache
///
/// # Returns
/// The mirror ID with the lowest latency (or preferred region within 2x tolerance)
pub async fn select_best_mirror(preferred_region: Option<&str>, cache_ttl: Duration) -> MirrorId {
    let cache = get_cache(cache_ttl);

    // Check if we have valid cached results for all mirrors
    let cached = cache.get_all_valid().await;
    let results = if cached.len() == MirrorId::all().len() {
        cached
    } else {
        // Test all mirrors and cache results
        let fresh = test_all_mirrors().await;
        for (&id, &latency) in &fresh {
            cache.set(id, latency).await;
        }
        fresh
    };

    find_best_from_results(&results, preferred_region)
}

/// Test latency to all mirrors.
///
/// Returns a map of mirror ID to latency (only successful tests).
pub async fn test_all_mirrors() -> HashMap<MirrorId, Duration> {
    let handles: Vec<_> = MirrorId::all()
        .iter()
        .map(|&id| {
            tokio::spawn(async move {
                let latency = test_mirror_latency(id).await;
                (id, latency)
            })
        })
        .collect();

    let mut results = HashMap::new();
    for handle in handles {
        match handle.await {
            Ok((id, Some(latency))) => {
                results.insert(id, latency);
            }
            Ok((id, None)) => {
                tracing::debug!("Mirror {} did not respond", id);
            }
            Err(e) => {
                tracing::warn!("Task for mirror latency testing panicked: {}", e);
            }
        }
    }
    results
}

/// Test latency to a single mirror using a HEAD request.
///
/// Returns the latency if successful, None otherwise.
pub async fn test_mirror_latency(mirror_id: MirrorId) -> Option<Duration> {
    let mirror = Mirror::get(mirror_id);
    let url = mirror.https_base;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .ok()?;

    let start = Instant::now();
    let result = client.head(url).send().await;
    let elapsed = start.elapsed();

    match result {
        Ok(resp) if resp.status().is_success() || resp.status().is_redirection() => Some(elapsed),
        _ => None,
    }
}

/// Find the best mirror from test results.
///
/// If a preferred region is specified, prefer mirrors in that region
/// if their latency is within 2x of the best latency.
pub fn find_best_from_results(
    results: &HashMap<MirrorId, Duration>,
    preferred_region: Option<&str>,
) -> MirrorId {
    if results.is_empty() {
        return MirrorId::Rcsb; // Default fallback
    }

    // Find the absolute best latency
    let (best_id, best_latency) = results
        .iter()
        .min_by_key(|(_, &latency)| latency)
        .map(|(&id, &latency)| (id, latency))
        .unwrap();

    // If no preferred region, return the absolute best
    let Some(preferred) = preferred_region else {
        return best_id;
    };

    // Normalize region names for comparison
    let normalize_region = |s: &str| s.to_lowercase();
    let preferred_normalized = normalize_region(preferred);

    // Find mirrors matching the preferred region within 2x latency tolerance
    let tolerance = best_latency * 2;

    for (&id, &latency) in results {
        if latency <= tolerance {
            let mirror = Mirror::get(id);
            let region_normalized = normalize_region(mirror.region);

            // Check if region matches (partial match for flexibility)
            if region_normalized.contains(&preferred_normalized)
                || preferred_normalized.contains(&region_normalized)
                || matches_region_alias(&preferred_normalized, &region_normalized)
            {
                return id;
            }
        }
    }

    // No preferred region mirror within tolerance, return absolute best
    best_id
}

/// Check if a region alias matches.
///
/// Both inputs should be normalized to lowercase before calling this function.
/// Maps common region aliases to mirror regions:
/// - "us", "usa", "america" -> "us" (RCSB)
/// - "jp", "japan" -> "japan" (PDBj)
/// - "eu", "europe", "uk" -> "europe" (PDBe)
fn matches_region_alias(preferred: &str, mirror_region: &str) -> bool {
    matches!(
        (preferred, mirror_region),
        ("us" | "usa" | "america", "us")
            | ("jp" | "japan", "japan")
            | ("eu" | "europe" | "uk", "europe")
    )
}

/// Print latency test results for all mirrors.
#[allow(dead_code)]
pub async fn print_mirror_latencies() {
    println!("Testing mirror latencies...\n");

    let results = test_all_mirrors().await;

    // Sort by latency
    let mut sorted: Vec<_> = results.iter().collect();
    sorted.sort_by_key(|(_, latency)| *latency);

    println!("{:<10} {:<15} {:>10}", "Mirror", "Region", "Latency");
    println!("{}", "-".repeat(37));

    for (&id, latency) in &sorted {
        let mirror = Mirror::get(id);
        println!(
            "{:<10} {:<15} {:>7.0} ms",
            id.to_string(),
            mirror.region,
            latency.as_secs_f64() * 1000.0
        );
    }

    // Show any mirrors that failed
    let tested: std::collections::HashSet<_> = results.keys().collect();
    let failed: Vec<_> = MirrorId::all()
        .iter()
        .filter(|id| !tested.contains(id))
        .collect();

    if !failed.is_empty() {
        println!("\nFailed to reach:");
        for id in failed {
            let mirror = Mirror::get(*id);
            println!("  {} ({})", id, mirror.region);
        }
    }

    // Recommendation
    if let Some((&best_id, _)) = sorted.first() {
        println!("\nRecommended: {} (lowest latency)", best_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_best_from_results_empty() {
        let results = HashMap::new();
        let best = find_best_from_results(&results, None);
        assert_eq!(best, MirrorId::Rcsb);
    }

    #[test]
    fn test_find_best_from_results_no_preference() {
        let mut results = HashMap::new();
        results.insert(MirrorId::Rcsb, Duration::from_millis(100));
        results.insert(MirrorId::Pdbj, Duration::from_millis(50));
        results.insert(MirrorId::Pdbe, Duration::from_millis(150));

        let best = find_best_from_results(&results, None);
        assert_eq!(best, MirrorId::Pdbj);
    }

    #[test]
    fn test_find_best_from_results_with_preference_within_tolerance() {
        let mut results = HashMap::new();
        results.insert(MirrorId::Rcsb, Duration::from_millis(100)); // US
        results.insert(MirrorId::Pdbj, Duration::from_millis(50)); // Japan
        results.insert(MirrorId::Pdbe, Duration::from_millis(80)); // Europe

        // Prefer US, and RCSB (100ms) is within 2x of best (50ms * 2 = 100ms)
        let best = find_best_from_results(&results, Some("us"));
        assert_eq!(best, MirrorId::Rcsb);
    }

    #[test]
    fn test_find_best_from_results_with_preference_outside_tolerance() {
        let mut results = HashMap::new();
        results.insert(MirrorId::Rcsb, Duration::from_millis(200)); // US
        results.insert(MirrorId::Pdbj, Duration::from_millis(50)); // Japan
        results.insert(MirrorId::Pdbe, Duration::from_millis(80)); // Europe

        // Prefer US, but RCSB (200ms) is outside 2x of best (50ms * 2 = 100ms)
        let best = find_best_from_results(&results, Some("us"));
        assert_eq!(best, MirrorId::Pdbj);
    }

    #[test]
    fn test_find_best_from_results_region_alias() {
        let mut results = HashMap::new();
        results.insert(MirrorId::Rcsb, Duration::from_millis(60)); // US
        results.insert(MirrorId::Pdbj, Duration::from_millis(50)); // Japan

        // "jp" should match "Japan"
        let best = find_best_from_results(&results, Some("jp"));
        assert_eq!(best, MirrorId::Pdbj);
    }

    #[tokio::test]
    async fn test_latency_cache_basic() {
        let cache = LatencyCache::new(Duration::from_secs(60));

        // Initially empty
        assert!(cache.get(MirrorId::Rcsb).await.is_none());

        // Set and get
        cache.set(MirrorId::Rcsb, Duration::from_millis(100)).await;
        assert_eq!(
            cache.get(MirrorId::Rcsb).await,
            Some(Duration::from_millis(100))
        );
    }

    #[tokio::test]
    async fn test_latency_cache_expiration() {
        let cache = LatencyCache::new(Duration::from_millis(10));

        cache.set(MirrorId::Rcsb, Duration::from_millis(100)).await;
        assert!(cache.get(MirrorId::Rcsb).await.is_some());

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(cache.get(MirrorId::Rcsb).await.is_none());
    }
}
