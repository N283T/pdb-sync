# Phase 2: Sync Command Enhancement

## 目標

- レイアウト選択 (divided/all)
- データタイプ選択
- プログレス表示改善
- 増分同期サポート

## 依存

- Phase 1 (DataType, Layout)

## 実装内容

### 1. CLI引数更新: `src/cli/args.rs`

```rust
#[derive(Parser)]
pub struct SyncArgs {
    /// Mirror to sync from
    #[arg(short, long, value_enum)]
    pub mirror: Option<MirrorId>,

    /// Data types to sync (can specify multiple)
    #[arg(short = 't', long = "type", value_enum, default_value = "structures")]
    pub data_types: Vec<DataType>,

    /// File format for coordinate files
    #[arg(short, long, value_enum, default_value = "mmcif")]
    pub format: SyncFormat,

    /// Directory layout
    #[arg(short, long, value_enum, default_value = "divided")]
    pub layout: Layout,

    /// Destination directory
    #[arg(short, long)]
    pub dest: Option<PathBuf>,

    /// Delete files not present on the remote
    #[arg(long)]
    pub delete: bool,

    /// Bandwidth limit in KB/s
    #[arg(long)]
    pub bwlimit: Option<u32>,

    /// Perform a dry run without making changes
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Show detailed progress
    #[arg(short = 'P', long)]
    pub progress: bool,

    /// Filter patterns (PDB ID prefixes)
    #[arg(trailing_var_arg = true)]
    pub filters: Vec<String>,
}
```

### 2. rsyncランナー更新: `src/sync/rsync.rs`

```rust
pub struct RsyncOptions {
    pub mirror: MirrorId,
    pub data_types: Vec<DataType>,
    pub formats: Vec<FileFormat>,
    pub layout: Layout,
    pub delete: bool,
    pub bwlimit: Option<u32>,
    pub dry_run: bool,
    pub filters: Vec<String>,
    pub show_progress: bool,
}

impl RsyncRunner {
    async fn sync_data_type(
        &self,
        mirror: &Mirror,
        data_type: DataType,
        format: FileFormat,
        dest: &Path,
    ) -> Result<SyncResult> {
        let subpath = data_type.rsync_subpath(self.options.layout);
        let format_dir = format.subdir();
        let full_subpath = format!("{}/{}/", subpath, format_dir);

        let source = mirror.rsync_url(&full_subpath);
        // ...
    }
}
```

### 3. 新規ファイル: `src/sync/progress.rs`

```rust
use indicatif::{ProgressBar, ProgressStyle};

/// rsync出力をパースしてプログレス表示
pub struct SyncProgress {
    progress_bar: ProgressBar,
    files_transferred: u64,
    bytes_transferred: u64,
}

impl SyncProgress {
    pub fn new() -> Self {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} [{elapsed_precise}] {msg}")
                .unwrap()
        );
        Self {
            progress_bar: pb,
            files_transferred: 0,
            bytes_transferred: 0,
        }
    }

    /// rsyncの出力行をパース
    pub fn parse_line(&mut self, line: &str) {
        // rsync --progress output format:
        // "filename"
        // "     1,234,567 100%   12.34MB/s    0:00:01"
        if let Some(bytes) = self.parse_bytes(line) {
            self.bytes_transferred += bytes;
            self.files_transferred += 1;
            self.update_display();
        }
    }

    fn parse_bytes(&self, line: &str) -> Option<u64> {
        // Parse rsync progress line
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts[1] == "100%" {
            parts[0].replace(",", "").parse().ok()
        } else {
            None
        }
    }

    fn update_display(&self) {
        self.progress_bar.set_message(format!(
            "Files: {} | Transferred: {}",
            self.files_transferred,
            human_bytes(self.bytes_transferred)
        ));
    }
}

fn human_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
```

### 4. sync コマンド更新: `src/cli/commands/sync.rs`

```rust
pub async fn run_sync(args: SyncArgs, ctx: AppContext) -> Result<()> {
    let mirror = args.mirror.unwrap_or(ctx.mirror);
    let dest = args.dest.unwrap_or_else(|| ctx.pdb_dir.clone());

    let options = RsyncOptions {
        mirror,
        data_types: args.data_types.clone(),
        formats: args.format.to_file_formats(),
        layout: args.layout,
        delete: args.delete,
        bwlimit: args.bwlimit.or(Some(ctx.config.sync.bwlimit)).filter(|&b| b > 0),
        dry_run: args.dry_run,
        filters: args.filters.clone(),
        show_progress: args.progress,
    };

    println!("Syncing from {} mirror...", mirror);
    println!("Data types: {:?}", args.data_types);
    println!("Layout: {}", args.layout);
    println!("Destination: {}", dest.display());

    let runner = RsyncRunner::new(options);
    let results = runner.run(&dest).await?;

    // Print summary
    for result in results {
        println!("{}: {} files synced", result.data_type, result.files_count);
    }

    Ok(())
}
```

## 使用例

```bash
# 基本同期 (structures, divided)
pdb-cli sync

# assemblies を flat で同期
pdb-cli sync -t assemblies -l all

# 複数データタイプ
pdb-cli sync -t structures -t assemblies -t structure-factors

# プログレス表示付き
pdb-cli sync -P

# フィルタ付き (1abで始まるIDのみ)
pdb-cli sync 1ab
```

## 完了条件

- [ ] SyncArgs に data_types, layout, progress 追加
- [ ] RsyncRunner が DataType, Layout 対応
- [ ] progress.rs 作成
- [ ] 複数データタイプの同期
- [ ] cargo build 成功
- [ ] cargo test 成功
- [ ] 手動テスト (dry-run)

## 工数

3-5日
