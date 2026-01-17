# Phase 4: List Command

## 目標

ローカルミラーのファイル一覧表示、パターン検索、統計表示

## 依存

- Phase 1 (DataType)

## 実装内容

### 1. CLI定義: `src/cli/args.rs`

```rust
#[derive(Subcommand)]
pub enum Commands {
    // ... existing commands

    /// List local PDB files
    List(ListArgs),
}

#[derive(Parser)]
pub struct ListArgs {
    /// Pattern to match (supports glob: 1ab*, *abc)
    #[arg(default_value = "*")]
    pub pattern: String,

    /// Data type to list
    #[arg(short = 't', long = "type", value_enum)]
    pub data_type: Option<DataType>,

    /// File format to list
    #[arg(short, long, value_enum)]
    pub format: Option<FileFormat>,

    /// Show file sizes
    #[arg(short, long)]
    pub size: bool,

    /// Show modification times
    #[arg(short = 'T', long)]
    pub time: bool,

    /// Output format
    #[arg(long, value_enum, default_value = "text")]
    pub output: OutputFormat,

    /// Show statistics only
    #[arg(long)]
    pub stats: bool,

    /// Sort by field
    #[arg(long, value_enum, default_value = "name")]
    pub sort: SortField,

    /// Reverse sort order
    #[arg(short, long)]
    pub reverse: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
    Csv,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SortField {
    Name,
    Size,
    Time,
}
```

### 2. 新規ファイル: `src/cli/commands/list.rs`

```rust
use crate::cli::args::{ListArgs, OutputFormat, SortField};
use crate::context::AppContext;
use crate::data_types::DataType;
use crate::error::Result;
use crate::files::FileFormat;
use glob::Pattern;
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug)]
pub struct LocalFile {
    pub pdb_id: String,
    pub path: PathBuf,
    pub size: u64,
    pub modified: std::time::SystemTime,
    pub data_type: DataType,
    pub format: FileFormat,
}

pub async fn run_list(args: ListArgs, ctx: AppContext) -> Result<()> {
    let files = scan_local_files(&ctx.pdb_dir, &args).await?;

    if args.stats {
        print_statistics(&files);
    } else {
        print_file_list(&files, &args);
    }

    Ok(())
}

async fn scan_local_files(base_dir: &Path, args: &ListArgs) -> Result<Vec<LocalFile>> {
    let pattern = Pattern::new(&args.pattern)
        .map_err(|e| crate::error::PdbCliError::InvalidInput(e.to_string()))?;

    let mut files = Vec::new();

    // Scan divided structure
    scan_divided_dir(base_dir, &pattern, args, &mut files).await?;

    // Sort
    sort_files(&mut files, args.sort, args.reverse);

    Ok(files)
}

async fn scan_divided_dir(
    base_dir: &Path,
    pattern: &Pattern,
    args: &ListArgs,
    files: &mut Vec<LocalFile>,
) -> Result<()> {
    // Scan mmCIF directory
    let mmcif_dir = base_dir.join("mmCIF");
    if mmcif_dir.exists() {
        scan_format_dir(&mmcif_dir, pattern, DataType::Structures, FileFormat::CifGz, files).await?;
    }

    // Scan pdb directory
    let pdb_dir = base_dir.join("pdb");
    if pdb_dir.exists() {
        scan_format_dir(&pdb_dir, pattern, DataType::Structures, FileFormat::PdbGz, files).await?;
    }

    // Add more directories as needed...

    Ok(())
}

async fn scan_format_dir(
    dir: &Path,
    pattern: &Pattern,
    data_type: DataType,
    format: FileFormat,
    files: &mut Vec<LocalFile>,
) -> Result<()> {
    let mut entries = fs::read_dir(dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        if path.is_dir() {
            // Hash subdirectory
            Box::pin(scan_format_dir(&path, pattern, data_type, format, files)).await?;
        } else if path.is_file() {
            if let Some(pdb_id) = extract_pdb_id(&path, format) {
                if pattern.matches(&pdb_id) {
                    let metadata = fs::metadata(&path).await?;
                    files.push(LocalFile {
                        pdb_id,
                        path,
                        size: metadata.len(),
                        modified: metadata.modified()?,
                        data_type,
                        format,
                    });
                }
            }
        }
    }

    Ok(())
}

fn extract_pdb_id(path: &Path, format: FileFormat) -> Option<String> {
    let filename = path.file_name()?.to_str()?;

    match format {
        FileFormat::CifGz => {
            // 1abc.cif.gz -> 1abc
            filename.strip_suffix(".cif.gz").map(|s| s.to_string())
        }
        FileFormat::PdbGz => {
            // pdb1abc.ent.gz -> 1abc
            filename
                .strip_prefix("pdb")
                .and_then(|s| s.strip_suffix(".ent.gz"))
                .map(|s| s.to_string())
        }
        _ => None,
    }
}

fn sort_files(files: &mut Vec<LocalFile>, field: SortField, reverse: bool) {
    files.sort_by(|a, b| {
        let cmp = match field {
            SortField::Name => a.pdb_id.cmp(&b.pdb_id),
            SortField::Size => a.size.cmp(&b.size),
            SortField::Time => a.modified.cmp(&b.modified),
        };
        if reverse { cmp.reverse() } else { cmp }
    });
}

fn print_file_list(files: &[LocalFile], args: &ListArgs) {
    match args.output {
        OutputFormat::Text => print_text(files, args),
        OutputFormat::Json => print_json(files),
        OutputFormat::Csv => print_csv(files, args),
    }
}

fn print_text(files: &[LocalFile], args: &ListArgs) {
    for file in files {
        let mut line = file.pdb_id.clone();

        if args.size {
            line.push_str(&format!("\t{}", human_bytes(file.size)));
        }
        if args.time {
            let time = chrono::DateTime::<chrono::Local>::from(file.modified);
            line.push_str(&format!("\t{}", time.format("%Y-%m-%d %H:%M")));
        }

        println!("{}", line);
    }
}

fn print_json(files: &[LocalFile]) {
    let json_files: Vec<_> = files
        .iter()
        .map(|f| serde_json::json!({
            "pdb_id": f.pdb_id,
            "path": f.path,
            "size": f.size,
            "format": f.format.to_string(),
        }))
        .collect();

    println!("{}", serde_json::to_string_pretty(&json_files).unwrap());
}

fn print_csv(files: &[LocalFile], args: &ListArgs) {
    let mut header = "pdb_id".to_string();
    if args.size { header.push_str(",size"); }
    if args.time { header.push_str(",modified"); }
    println!("{}", header);

    for file in files {
        let mut line = file.pdb_id.clone();
        if args.size { line.push_str(&format!(",{}", file.size)); }
        if args.time {
            let time = chrono::DateTime::<chrono::Local>::from(file.modified);
            line.push_str(&format!(",{}", time.format("%Y-%m-%d %H:%M")));
        }
        println!("{}", line);
    }
}

fn print_statistics(files: &[LocalFile]) {
    let total_size: u64 = files.iter().map(|f| f.size).sum();
    let count = files.len();

    println!("Total files: {}", count);
    println!("Total size:  {}", human_bytes(total_size));

    if count > 0 {
        println!("Average:     {}", human_bytes(total_size / count as u64));
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

### 3. main.rs 更新

```rust
Commands::List(args) => commands::run_list(args, ctx).await,
```

### 4. mod.rs 更新

```rust
pub mod list;
pub use list::run_list;
```

## 使用例

```bash
# 全ファイル一覧
pdb-cli list

# パターン検索
pdb-cli list "1ab*"
pdb-cli list "*abc"

# サイズ・時刻表示
pdb-cli list --size --time

# 統計のみ
pdb-cli list --stats

# JSON出力
pdb-cli list --output json > files.json

# CSV出力
pdb-cli list --output csv --size > files.csv

# サイズ順ソート
pdb-cli list --sort size --reverse
```

## 完了条件

- [ ] ListArgs 定義
- [ ] list.rs 実装
- [ ] パターンマッチング (glob)
- [ ] 統計表示
- [ ] JSON/CSV出力
- [ ] ソート機能
- [ ] cargo build 成功
- [ ] cargo test 成功

## 依存追加

```toml
glob = "0.3"
chrono = { version = "0.4", features = ["serde"] }
serde_json = "1.0"
```

## 工数

1-2日
