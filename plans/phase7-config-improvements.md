# Phase 7: Configuration Improvements

## 目標

- データタイプ別保存パス
- デフォルトレイアウト設定
- 自動ミラー選択
- 帯域制限のデフォルト

## 依存

- Phase 1 (DataType, Layout)

## 実装内容

### 1. 設定スキーマ更新: `src/config/schema.rs`

```rust
use crate::data_types::{DataType, Layout};
use crate::files::FileFormat;
use crate::mirrors::MirrorId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub paths: PathsConfig,
    pub sync: SyncConfig,
    pub download: DownloadConfig,
    pub mirror_selection: MirrorSelectionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PathsConfig {
    /// Base directory for all PDB data
    pub pdb_dir: Option<PathBuf>,

    /// Per-data-type directories (override pdb_dir)
    #[serde(default)]
    pub data_type_dirs: HashMap<String, PathBuf>,
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self {
            pdb_dir: None,
            data_type_dirs: HashMap::new(),
        }
    }
}

impl PathsConfig {
    /// Get directory for a specific data type
    pub fn dir_for(&self, data_type: DataType) -> Option<&PathBuf> {
        let key = data_type.to_string();
        self.data_type_dirs.get(&key)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncConfig {
    #[serde(with = "mirror_id_serde")]
    pub mirror: MirrorId,

    /// Bandwidth limit in KB/s (0 = unlimited)
    pub bwlimit: u32,

    /// Delete files not present on remote
    pub delete: bool,

    /// Default directory layout
    #[serde(with = "layout_serde")]
    pub layout: Layout,

    /// Default data types to sync
    #[serde(default = "default_data_types")]
    pub data_types: Vec<String>,
}

fn default_data_types() -> Vec<String> {
    vec!["structures".to_string()]
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            mirror: MirrorId::Rcsb,
            bwlimit: 0,
            delete: false,
            layout: Layout::Divided,
            data_types: default_data_types(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DownloadConfig {
    /// Default file format
    pub default_format: String,

    /// Automatically decompress downloaded files
    pub auto_decompress: bool,

    /// Number of parallel downloads
    pub parallel: usize,

    /// Retry count on failure
    pub retry_count: u32,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            default_format: "mmcif".to_string(),
            auto_decompress: true,
            parallel: 4,
            retry_count: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MirrorSelectionConfig {
    /// Enable automatic mirror selection based on latency
    pub auto_select: bool,

    /// Preferred region (us, jp, eu, global)
    pub preferred_region: Option<String>,

    /// Cache latency test results (seconds)
    pub latency_cache_ttl: u64,
}

impl Default for MirrorSelectionConfig {
    fn default() -> Self {
        Self {
            auto_select: false,
            preferred_region: None,
            latency_cache_ttl: 3600, // 1 hour
        }
    }
}

// Serde helpers for enums
mod mirror_id_serde {
    use super::*;

    pub fn serialize<S>(mirror: &MirrorId, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&mirror.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<MirrorId, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

mod layout_serde {
    use super::*;

    pub fn serialize<S>(layout: &Layout, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&layout.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Layout, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "divided" => Ok(Layout::Divided),
            "all" => Ok(Layout::All),
            _ => Err(serde::de::Error::custom("invalid layout")),
        }
    }
}
```

### 2. 自動ミラー選択: `src/mirrors/auto_select.rs`

```rust
use crate::mirrors::{Mirror, MirrorId};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Cached latency results
static LATENCY_CACHE: RwLock<Option<LatencyCache>> = RwLock::const_new(None);

struct LatencyCache {
    results: HashMap<MirrorId, Duration>,
    timestamp: Instant,
    ttl: Duration,
}

impl LatencyCache {
    fn is_valid(&self) -> bool {
        self.timestamp.elapsed() < self.ttl
    }
}

/// Select the best mirror based on latency
pub async fn select_best_mirror(
    preferred_region: Option<&str>,
    cache_ttl: Duration,
) -> MirrorId {
    // Check cache first
    {
        let cache = LATENCY_CACHE.read().await;
        if let Some(ref c) = *cache {
            if c.is_valid() {
                return find_best_from_cache(&c.results, preferred_region);
            }
        }
    }

    // Test all mirrors
    let results = test_all_mirrors().await;

    // Update cache
    {
        let mut cache = LATENCY_CACHE.write().await;
        *cache = Some(LatencyCache {
            results: results.clone(),
            timestamp: Instant::now(),
            ttl: cache_ttl,
        });
    }

    find_best_from_cache(&results, preferred_region)
}

async fn test_all_mirrors() -> HashMap<MirrorId, Duration> {
    let mirrors = [
        MirrorId::Rcsb,
        MirrorId::Pdbj,
        MirrorId::Pdbe,
        MirrorId::Wwpdb,
    ];

    let mut results = HashMap::new();

    for mirror_id in mirrors {
        if let Some(latency) = test_mirror_latency(mirror_id).await {
            results.insert(mirror_id, latency);
        }
    }

    results
}

async fn test_mirror_latency(mirror_id: MirrorId) -> Option<Duration> {
    let mirror = Mirror::get(mirror_id);
    let url = format!("{}/1abc.cif", mirror.https_base);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;

    let start = Instant::now();

    // HEAD request to test latency
    let result = client.head(&url).send().await;

    if result.is_ok() {
        Some(start.elapsed())
    } else {
        None
    }
}

fn find_best_from_cache(
    results: &HashMap<MirrorId, Duration>,
    preferred_region: Option<&str>,
) -> MirrorId {
    // If preferred region is set, try to use it
    if let Some(region) = preferred_region {
        let preferred = match region.to_lowercase().as_str() {
            "us" | "rcsb" => MirrorId::Rcsb,
            "jp" | "pdbj" | "japan" => MirrorId::Pdbj,
            "eu" | "uk" | "pdbe" | "europe" => MirrorId::Pdbe,
            "global" | "wwpdb" => MirrorId::Wwpdb,
            _ => MirrorId::Rcsb,
        };

        // Use preferred if it's available and reasonably fast
        if let Some(latency) = results.get(&preferred) {
            // Accept if within 2x of best latency
            let best_latency = results.values().min().copied().unwrap_or(*latency);
            if *latency < best_latency * 2 {
                return preferred;
            }
        }
    }

    // Otherwise return fastest
    results
        .iter()
        .min_by_key(|(_, latency)| *latency)
        .map(|(id, _)| *id)
        .unwrap_or(MirrorId::Rcsb)
}

/// Test mirror and print results
pub async fn print_mirror_latencies() {
    println!("Testing mirror latencies...");

    let results = test_all_mirrors().await;

    let mut sorted: Vec<_> = results.iter().collect();
    sorted.sort_by_key(|(_, latency)| *latency);

    for (mirror_id, latency) in sorted {
        println!("  {}: {:?}", mirror_id, latency);
    }
}
```

### 3. context.rs 更新

```rust
impl AppContext {
    pub async fn new() -> Result<Self> {
        let config = ConfigLoader::load()?;

        // Auto-select mirror if enabled
        let mirror = if config.mirror_selection.auto_select {
            let ttl = Duration::from_secs(config.mirror_selection.latency_cache_ttl);
            auto_select::select_best_mirror(
                config.mirror_selection.preferred_region.as_deref(),
                ttl,
            ).await
        } else {
            config.sync.mirror
        };

        // ... rest of initialization
    }
}
```

### 4. 設定例

```toml
# ~/.config/pdb-cli/config.toml

[paths]
pdb_dir = "/data/pdb"

# Optional per-data-type directories
[paths.data_type_dirs]
structures = "/data/pdb/structures"
assemblies = "/data/pdb/assemblies"
structure-factors = "/data/pdb/experimental"

[sync]
mirror = "pdbj"
bwlimit = 10000  # 10 MB/s
delete = false
layout = "divided"
data_types = ["structures", "assemblies"]

[download]
default_format = "mmcif"
auto_decompress = true
parallel = 8
retry_count = 3

[mirror_selection]
auto_select = true
preferred_region = "jp"
latency_cache_ttl = 3600
```

### 5. config show 更新

```rust
pub fn run_config_show(config: &Config) {
    println!("Current configuration:");
    println!();

    println!("[paths]");
    if let Some(ref dir) = config.paths.pdb_dir {
        println!("  pdb_dir = \"{}\"", dir.display());
    }
    for (dtype, dir) in &config.paths.data_type_dirs {
        println!("  {} = \"{}\"", dtype, dir.display());
    }

    println!();
    println!("[sync]");
    println!("  mirror = \"{}\"", config.sync.mirror);
    println!("  bwlimit = {}", config.sync.bwlimit);
    println!("  layout = \"{}\"", config.sync.layout);
    println!("  data_types = {:?}", config.sync.data_types);

    println!();
    println!("[download]");
    println!("  default_format = \"{}\"", config.download.default_format);
    println!("  parallel = {}", config.download.parallel);
    println!("  retry_count = {}", config.download.retry_count);

    println!();
    println!("[mirror_selection]");
    println!("  auto_select = {}", config.mirror_selection.auto_select);
    if let Some(ref region) = config.mirror_selection.preferred_region {
        println!("  preferred_region = \"{}\"", region);
    }
}
```

## 使用例

```bash
# 設定表示
pdb-cli config show

# 設定変更
pdb-cli config set sync.mirror pdbj
pdb-cli config set download.parallel 16
pdb-cli config set mirror_selection.auto_select true

# ミラーレイテンシテスト
pdb-cli config test-mirrors
```

## 完了条件

- [ ] PathsConfig に data_type_dirs 追加
- [ ] SyncConfig に layout, data_types 追加
- [ ] DownloadConfig に parallel, retry_count 追加
- [ ] MirrorSelectionConfig 追加
- [ ] auto_select.rs 実装
- [ ] config show 更新
- [ ] cargo build 成功
- [ ] cargo test 成功

## 工数

2日
