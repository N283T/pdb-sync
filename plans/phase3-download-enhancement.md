# Phase 3: Download Command Enhancement

## 目標

- 並列ダウンロード
- リトライロジック
- アセンブリ/構造因子ダウンロード
- データタイプ選択

## 依存

- Phase 1 (DataType)

## 実装内容

### 1. CLI引数更新: `src/cli/args.rs`

```rust
#[derive(Parser)]
pub struct DownloadArgs {
    /// PDB IDs to download
    #[arg(required_unless_present = "list")]
    pub pdb_ids: Vec<String>,

    /// Data type to download
    #[arg(short = 't', long = "type", value_enum, default_value = "structures")]
    pub data_type: DataType,

    /// File format (for structures)
    #[arg(short, long, value_enum, default_value = "mmcif")]
    pub format: FileFormat,

    /// Assembly number (for assemblies, 0 = all)
    #[arg(short, long)]
    pub assembly: Option<u8>,

    /// Destination directory
    #[arg(short, long)]
    pub dest: Option<PathBuf>,

    /// Decompress downloaded files
    #[arg(long)]
    pub decompress: bool,

    /// Mirror to download from
    #[arg(short, long, value_enum)]
    pub mirror: Option<MirrorId>,

    /// Overwrite existing files
    #[arg(long)]
    pub overwrite: bool,

    /// Number of parallel downloads
    #[arg(short, long, default_value = "4")]
    pub parallel: usize,

    /// Retry count on failure
    #[arg(long, default_value = "3")]
    pub retry: u32,

    /// Read PDB IDs from file
    #[arg(short, long)]
    pub list: Option<PathBuf>,
}
```

### 2. ダウンロードオプション更新: `src/download/https.rs`

```rust
pub struct DownloadOptions {
    pub mirror: MirrorId,
    pub decompress: bool,
    pub overwrite: bool,
    pub parallel: usize,
    pub retry_count: u32,
    pub retry_delay: Duration,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            mirror: MirrorId::Rcsb,
            decompress: false,
            overwrite: false,
            parallel: 4,
            retry_count: 3,
            retry_delay: Duration::from_secs(1),
        }
    }
}
```

### 3. 並列ダウンロード: `src/download/https.rs`

```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

pub struct HttpsDownloader {
    client: reqwest::Client,
    options: DownloadOptions,
    semaphore: Arc<Semaphore>,
}

impl HttpsDownloader {
    pub fn new(options: DownloadOptions) -> Self {
        let semaphore = Arc::new(Semaphore::new(options.parallel));
        let client = reqwest::Client::builder()
            .user_agent("pdb-cli")
            .timeout(Duration::from_secs(300))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, options, semaphore }
    }

    pub async fn download_many(
        &self,
        tasks: Vec<DownloadTask>,
        dest: &Path,
    ) -> Vec<DownloadResult> {
        let futures: Vec<_> = tasks
            .into_iter()
            .map(|task| self.download_with_semaphore(task, dest))
            .collect();

        futures::future::join_all(futures).await
    }

    async fn download_with_semaphore(
        &self,
        task: DownloadTask,
        dest: &Path,
    ) -> DownloadResult {
        let _permit = self.semaphore.acquire().await.unwrap();
        self.download_with_retry(task, dest).await
    }

    async fn download_with_retry(
        &self,
        task: DownloadTask,
        dest: &Path,
    ) -> DownloadResult {
        let mut last_error = None;

        for attempt in 0..=self.options.retry_count {
            if attempt > 0 {
                tokio::time::sleep(self.options.retry_delay).await;
                eprintln!("Retry {}/{} for {}", attempt, self.options.retry_count, task.pdb_id);
            }

            match self.download_single(&task, dest).await {
                Ok(path) => return DownloadResult::Success { pdb_id: task.pdb_id, path },
                Err(e) => last_error = Some(e),
            }
        }

        DownloadResult::Failed {
            pdb_id: task.pdb_id,
            error: last_error.unwrap(),
        }
    }
}
```

### 4. ダウンロードタスク: `src/download/task.rs`

```rust
use crate::data_types::DataType;
use crate::files::{FileFormat, PdbId};

#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub pdb_id: PdbId,
    pub data_type: DataType,
    pub format: FileFormat,
    pub assembly_number: Option<u8>,
}

#[derive(Debug)]
pub enum DownloadResult {
    Success {
        pdb_id: PdbId,
        path: PathBuf,
    },
    Failed {
        pdb_id: PdbId,
        error: PdbCliError,
    },
    Skipped {
        pdb_id: PdbId,
        reason: String,
    },
}
```

### 5. URL構築: データタイプ対応

```rust
impl HttpsDownloader {
    fn build_url(&self, task: &DownloadTask) -> String {
        let mirror = Mirror::get(self.options.mirror);
        let id = task.pdb_id.as_str();

        match task.data_type {
            DataType::Structures => self.build_structure_url(mirror, id, task.format),
            DataType::Assemblies => self.build_assembly_url(mirror, id, task.assembly_number),
            DataType::StructureFactors => self.build_sf_url(mirror, id),
            // ...
        }
    }

    fn build_assembly_url(&self, mirror: &Mirror, id: &str, assembly: Option<u8>) -> String {
        match self.options.mirror {
            MirrorId::Rcsb => {
                if let Some(n) = assembly {
                    format!("{}/{}-assembly{}.cif.gz", mirror.https_base, id, n)
                } else {
                    // Assembly 1 as default
                    format!("{}/{}-assembly1.cif.gz", mirror.https_base, id)
                }
            }
            // Other mirrors...
        }
    }

    fn build_sf_url(&self, mirror: &Mirror, id: &str) -> String {
        match self.options.mirror {
            MirrorId::Rcsb => format!("{}/r{}sf.ent.gz", mirror.https_base, id),
            // Other mirrors...
        }
    }
}
```

### 6. download コマンド更新: `src/cli/commands/download.rs`

```rust
pub async fn run_download(args: DownloadArgs, ctx: AppContext) -> Result<()> {
    let dest = args.dest.unwrap_or_else(|| std::env::current_dir().unwrap());
    let mirror = args.mirror.unwrap_or(ctx.mirror);

    // Collect PDB IDs
    let mut pdb_ids = args.pdb_ids.clone();
    if let Some(list_path) = &args.list {
        let ids_from_file = read_id_list(list_path).await?;
        pdb_ids.extend(ids_from_file);
    }

    // Build tasks
    let tasks: Vec<DownloadTask> = pdb_ids
        .iter()
        .filter_map(|id| PdbId::new(id).ok())
        .map(|pdb_id| DownloadTask {
            pdb_id,
            data_type: args.data_type,
            format: args.format,
            assembly_number: args.assembly,
        })
        .collect();

    let options = DownloadOptions {
        mirror,
        decompress: args.decompress,
        overwrite: args.overwrite,
        parallel: args.parallel,
        retry_count: args.retry,
        ..Default::default()
    };

    let downloader = HttpsDownloader::new(options);
    let results = downloader.download_many(tasks, &dest).await;

    // Print summary
    let success = results.iter().filter(|r| matches!(r, DownloadResult::Success { .. })).count();
    let failed = results.iter().filter(|r| matches!(r, DownloadResult::Failed { .. })).count();

    println!("\nDownloaded {} files, {} failed", success, failed);

    Ok(())
}
```

## 使用例

```bash
# 並列ダウンロード
pdb-cli download 1abc 2xyz 3def -p 8

# リトライ付き
pdb-cli download 1abc --retry 5

# アセンブリダウンロード
pdb-cli download 4hhb -t assemblies -a 1

# 全アセンブリ
pdb-cli download 4hhb -t assemblies -a 0

# 構造因子
pdb-cli download 1abc -t structure-factors

# リストから
pdb-cli download -l ids.txt -p 16
```

## 完了条件

- [ ] DownloadArgs 更新 (parallel, retry, data_type, assembly)
- [ ] 並列ダウンロード (Semaphore)
- [ ] リトライロジック
- [ ] アセンブリURL構築
- [ ] 構造因子URL構築
- [ ] DownloadTask, DownloadResult
- [ ] cargo build 成功
- [ ] cargo test 成功

## 工数

3-4日
