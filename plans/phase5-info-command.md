# Phase 5: Info Command

## 目標

PDBエントリのメタデータ表示 (RCSB API連携)

## 依存

なし (独立して実装可能)

## 実装内容

### 1. CLI定義: `src/cli/args.rs`

```rust
#[derive(Subcommand)]
pub enum Commands {
    // ... existing commands

    /// Show information about a PDB entry
    Info(InfoArgs),
}

#[derive(Parser)]
pub struct InfoArgs {
    /// PDB ID to query
    pub pdb_id: String,

    /// Show local file info only (no network request)
    #[arg(long)]
    pub local: bool,

    /// Output format
    #[arg(long, value_enum, default_value = "text")]
    pub output: OutputFormat,

    /// Show all available fields
    #[arg(short, long)]
    pub all: bool,
}
```

### 2. 新規モジュール: `src/api/mod.rs`

```rust
pub mod rcsb;
pub use rcsb::RcsbClient;
```

### 3. RCSB API クライアント: `src/api/rcsb.rs`

```rust
use crate::error::{PdbCliError, Result};
use crate::files::PdbId;
use serde::{Deserialize, Serialize};

const RCSB_DATA_API: &str = "https://data.rcsb.org/rest/v1/core/entry";

#[derive(Debug, Serialize, Deserialize)]
pub struct EntryMetadata {
    pub rcsb_id: String,
    pub struct_title: Option<String>,
    pub rcsb_accession_info: Option<AccessionInfo>,
    pub exptl: Option<Vec<ExperimentalMethod>>,
    pub refine: Option<Vec<Refinement>>,
    pub rcsb_entry_info: Option<EntryInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessionInfo {
    pub deposit_date: Option<String>,
    pub initial_release_date: Option<String>,
    pub revision_date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExperimentalMethod {
    pub method: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Refinement {
    pub ls_d_res_high: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EntryInfo {
    pub polymer_entity_count: Option<u32>,
    pub assembly_count: Option<u32>,
    pub molecular_weight: Option<f64>,
}

pub struct RcsbClient {
    client: reqwest::Client,
}

impl RcsbClient {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("pdb-cli")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    pub async fn fetch_entry(&self, pdb_id: &PdbId) -> Result<EntryMetadata> {
        let url = format!("{}/{}", RCSB_DATA_API, pdb_id.as_str().to_uppercase());

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(PdbCliError::Download(format!(
                "RCSB API returned {}",
                response.status()
            )));
        }

        let metadata: EntryMetadata = response.json().await?;
        Ok(metadata)
    }
}

impl Default for RcsbClient {
    fn default() -> Self {
        Self::new()
    }
}
```

### 4. info コマンド: `src/cli/commands/info.rs`

```rust
use crate::api::RcsbClient;
use crate::cli::args::{InfoArgs, OutputFormat};
use crate::context::AppContext;
use crate::error::Result;
use crate::files::PdbId;

pub async fn run_info(args: InfoArgs, ctx: AppContext) -> Result<()> {
    let pdb_id = PdbId::new(&args.pdb_id)?;

    if args.local {
        show_local_info(&pdb_id, &ctx).await?;
    } else {
        let client = RcsbClient::new();
        let metadata = client.fetch_entry(&pdb_id).await?;
        print_metadata(&metadata, args.output, args.all);
    }

    Ok(())
}

async fn show_local_info(pdb_id: &PdbId, ctx: &AppContext) -> Result<()> {
    use crate::files::paths::build_relative_path;
    use crate::files::FileFormat;

    println!("PDB ID: {}", pdb_id);
    println!();

    // Check for local files
    let formats = [
        (FileFormat::CifGz, "mmCIF"),
        (FileFormat::PdbGz, "PDB"),
    ];

    for (format, name) in formats {
        let path = ctx.pdb_dir.join(build_relative_path(pdb_id, format));
        if path.exists() {
            let metadata = tokio::fs::metadata(&path).await?;
            println!("{} file:", name);
            println!("  Path: {}", path.display());
            println!("  Size: {} bytes", metadata.len());
            if let Ok(modified) = metadata.modified() {
                let time = chrono::DateTime::<chrono::Local>::from(modified);
                println!("  Modified: {}", time.format("%Y-%m-%d %H:%M:%S"));
            }
            println!();
        }
    }

    Ok(())
}

fn print_metadata(
    metadata: &crate::api::rcsb::EntryMetadata,
    output: OutputFormat,
    all: bool,
) {
    match output {
        OutputFormat::Text => print_text(metadata, all),
        OutputFormat::Json => print_json(metadata),
        OutputFormat::Csv => print_csv(metadata),
    }
}

fn print_text(metadata: &crate::api::rcsb::EntryMetadata, all: bool) {
    println!("PDB ID: {}", metadata.rcsb_id);

    if let Some(title) = &metadata.struct_title {
        println!("Title:  {}", title);
    }

    if let Some(info) = &metadata.rcsb_accession_info {
        if let Some(date) = &info.deposit_date {
            println!("Deposited: {}", date);
        }
        if let Some(date) = &info.initial_release_date {
            println!("Released:  {}", date);
        }
    }

    if let Some(exptl) = &metadata.exptl {
        let methods: Vec<_> = exptl.iter().map(|e| e.method.as_str()).collect();
        println!("Method: {}", methods.join(", "));
    }

    if let Some(refine) = &metadata.refine {
        if let Some(first) = refine.first() {
            if let Some(res) = first.ls_d_res_high {
                println!("Resolution: {:.2} Å", res);
            }
        }
    }

    if let Some(entry_info) = &metadata.rcsb_entry_info {
        if let Some(count) = entry_info.polymer_entity_count {
            println!("Polymer entities: {}", count);
        }
        if let Some(count) = entry_info.assembly_count {
            println!("Assemblies: {}", count);
        }
        if all {
            if let Some(mw) = entry_info.molecular_weight {
                println!("Molecular weight: {:.2} kDa", mw / 1000.0);
            }
        }
    }
}

fn print_json(metadata: &crate::api::rcsb::EntryMetadata) {
    println!("{}", serde_json::to_string_pretty(metadata).unwrap());
}

fn print_csv(metadata: &crate::api::rcsb::EntryMetadata) {
    // Simple CSV output
    let title = metadata.struct_title.as_deref().unwrap_or("");
    let method = metadata.exptl
        .as_ref()
        .and_then(|e| e.first())
        .map(|e| e.method.as_str())
        .unwrap_or("");
    let resolution = metadata.refine
        .as_ref()
        .and_then(|r| r.first())
        .and_then(|r| r.ls_d_res_high)
        .map(|r| format!("{:.2}", r))
        .unwrap_or_default();

    println!("pdb_id,title,method,resolution");
    println!("{},\"{}\",{},{}", metadata.rcsb_id, title, method, resolution);
}
```

### 5. main.rs 更新

```rust
mod api;

// In match:
Commands::Info(args) => commands::run_info(args, ctx).await,
```

## 使用例

```bash
# 基本情報
pdb-cli info 4hhb

# 全フィールド表示
pdb-cli info 4hhb --all

# ローカル情報のみ
pdb-cli info 4hhb --local

# JSON出力
pdb-cli info 4hhb --output json

# CSV出力 (スクリプト用)
pdb-cli info 4hhb --output csv
```

## 出力例

```
PDB ID: 4HHB
Title:  THE CRYSTAL STRUCTURE OF HUMAN DEOXYHAEMOGLOBIN AT 1.74 ANGSTROMS RESOLUTION
Deposited: 1984-03-07
Released:  1984-07-17
Method: X-RAY DIFFRACTION
Resolution: 1.74 Å
Polymer entities: 2
Assemblies: 1
```

## 完了条件

- [ ] InfoArgs 定義
- [ ] api/rcsb.rs 実装
- [ ] info.rs 実装
- [ ] ローカル情報表示
- [ ] JSON/CSV出力
- [ ] cargo build 成功
- [ ] cargo test 成功

## 工数

2-3日
