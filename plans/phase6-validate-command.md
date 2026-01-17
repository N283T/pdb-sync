# Phase 6: Validate Command

## 目標

ローカルファイルのチェックサム検証、破損検出、自動修復

## 依存

- Phase 1 (DataType)

## 実装内容

### 1. CLI定義: `src/cli/args.rs`

```rust
#[derive(Subcommand)]
pub enum Commands {
    // ... existing commands

    /// Validate local files against checksums
    Validate(ValidateArgs),
}

#[derive(Parser)]
pub struct ValidateArgs {
    /// PDB IDs to validate (empty = all local files)
    pub pdb_ids: Vec<String>,

    /// Data type to validate
    #[arg(short = 't', long = "type", value_enum)]
    pub data_type: Option<DataType>,

    /// File format to validate
    #[arg(short, long, value_enum)]
    pub format: Option<FileFormat>,

    /// Fix corrupted files by re-downloading
    #[arg(long)]
    pub fix: bool,

    /// Show progress
    #[arg(short = 'P', long)]
    pub progress: bool,

    /// Only show corrupted files
    #[arg(long)]
    pub errors_only: bool,
}
```

### 2. 新規モジュール: `src/validation/mod.rs`

```rust
pub mod checksum;
pub use checksum::{verify_file, ChecksumVerifier};
```

### 3. チェックサム検証: `src/validation/checksum.rs`

```rust
use crate::error::{PdbCliError, Result};
use crate::mirrors::{Mirror, MirrorId};
use md5::{Md5, Digest};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub struct ChecksumVerifier {
    client: reqwest::Client,
    checksums: HashMap<String, String>,
}

impl ChecksumVerifier {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("pdb-cli")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            checksums: HashMap::new(),
        }
    }

    /// Fetch checksums from mirror
    /// PDB mirrors provide CHECKSUMS files in each directory
    pub async fn fetch_checksums(
        &mut self,
        mirror: MirrorId,
        subpath: &str,
    ) -> Result<()> {
        let mirror_info = Mirror::get(mirror);
        // Example: https://files.rcsb.org/pub/pdb/data/structures/divided/mmCIF/ab/CHECKSUMS
        let url = format!("{}/{}/CHECKSUMS", mirror_info.https_base, subpath);

        let response = self.client.get(&url).send().await;

        if let Ok(resp) = response {
            if resp.status().is_success() {
                let text = resp.text().await?;
                self.parse_checksums(&text);
            }
        }

        Ok(())
    }

    fn parse_checksums(&mut self, content: &str) {
        // CHECKSUMS format: "MD5 (filename) = hash" or "hash  filename"
        for line in content.lines() {
            if let Some((hash, filename)) = self.parse_checksum_line(line) {
                self.checksums.insert(filename, hash);
            }
        }
    }

    fn parse_checksum_line(&self, line: &str) -> Option<(String, String)> {
        // Format 1: MD5 (1abc.cif.gz) = d41d8cd98f00b204e9800998ecf8427e
        if line.starts_with("MD5") {
            let parts: Vec<&str> = line.split(" = ").collect();
            if parts.len() == 2 {
                let filename = parts[0]
                    .trim_start_matches("MD5 (")
                    .trim_end_matches(")");
                return Some((parts[1].to_string(), filename.to_string()));
            }
        }

        // Format 2: d41d8cd98f00b204e9800998ecf8427e  1abc.cif.gz
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() == 2 && parts[0].len() == 32 {
            return Some((parts[0].to_string(), parts[1].to_string()));
        }

        None
    }

    pub fn get_expected_hash(&self, filename: &str) -> Option<&str> {
        self.checksums.get(filename).map(|s| s.as_str())
    }
}

/// Calculate MD5 hash of a file
pub async fn calculate_md5(path: &Path) -> Result<String> {
    let mut file = File::open(path).await?;
    let mut hasher = Md5::new();
    let mut buffer = vec![0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer).await?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Verify a file against expected hash
pub async fn verify_file(path: &Path, expected_hash: &str) -> Result<bool> {
    let actual_hash = calculate_md5(path).await?;
    Ok(actual_hash == expected_hash)
}

#[derive(Debug)]
pub enum VerifyResult {
    Valid,
    Invalid { expected: String, actual: String },
    Missing,
    NoChecksum,
}
```

### 4. validate コマンド: `src/cli/commands/validate.rs`

```rust
use crate::cli::args::ValidateArgs;
use crate::context::AppContext;
use crate::download::{DownloadOptions, HttpsDownloader};
use crate::error::Result;
use crate::files::{paths::build_relative_path, FileFormat, PdbId};
use crate::validation::{calculate_md5, ChecksumVerifier, VerifyResult};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

pub async fn run_validate(args: ValidateArgs, ctx: AppContext) -> Result<()> {
    let format = args.format.unwrap_or(FileFormat::CifGz);

    // Collect files to validate
    let files = if args.pdb_ids.is_empty() {
        scan_local_files(&ctx.pdb_dir, format).await?
    } else {
        args.pdb_ids
            .iter()
            .filter_map(|id| PdbId::new(id).ok())
            .map(|pdb_id| {
                let path = ctx.pdb_dir.join(build_relative_path(&pdb_id, format));
                (pdb_id, path)
            })
            .collect()
    };

    let total = files.len();
    println!("Validating {} files...", total);

    let pb = if args.progress {
        let pb = ProgressBar::new(total as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] [{bar:40}] {pos}/{len} {msg}")
                .unwrap()
        );
        Some(pb)
    } else {
        None
    };

    let mut valid_count = 0;
    let mut invalid_count = 0;
    let mut missing_count = 0;
    let mut corrupted_files = Vec::new();

    // Create verifier (would fetch checksums from mirror)
    let mut verifier = ChecksumVerifier::new();

    for (pdb_id, path) in &files {
        if let Some(ref pb) = pb {
            pb.set_message(pdb_id.to_string());
        }

        let result = validate_file(&path, &verifier).await;

        match result {
            VerifyResult::Valid => {
                valid_count += 1;
                if !args.errors_only {
                    println!("✓ {} OK", pdb_id);
                }
            }
            VerifyResult::Invalid { expected, actual } => {
                invalid_count += 1;
                corrupted_files.push((pdb_id.clone(), path.clone()));
                println!("✗ {} CORRUPTED (expected: {}, got: {})", pdb_id, expected, actual);
            }
            VerifyResult::Missing => {
                missing_count += 1;
                println!("? {} MISSING", pdb_id);
            }
            VerifyResult::NoChecksum => {
                valid_count += 1; // Assume valid if no checksum available
                if !args.errors_only {
                    println!("- {} (no checksum)", pdb_id);
                }
            }
        }

        if let Some(ref pb) = pb {
            pb.inc(1);
        }
    }

    if let Some(pb) = pb {
        pb.finish_and_clear();
    }

    // Summary
    println!();
    println!("Validation complete:");
    println!("  Valid:    {}", valid_count);
    println!("  Invalid:  {}", invalid_count);
    println!("  Missing:  {}", missing_count);

    // Fix corrupted files if requested
    if args.fix && !corrupted_files.is_empty() {
        println!();
        println!("Re-downloading {} corrupted files...", corrupted_files.len());

        let options = DownloadOptions {
            mirror: ctx.mirror,
            overwrite: true,
            ..Default::default()
        };
        let downloader = HttpsDownloader::new(options);

        for (pdb_id, _) in corrupted_files {
            match downloader.download(&pdb_id, format, &ctx.pdb_dir).await {
                Ok(_) => println!("  ✓ {} re-downloaded", pdb_id),
                Err(e) => println!("  ✗ {} failed: {}", pdb_id, e),
            }
        }
    }

    Ok(())
}

async fn validate_file(path: &Path, verifier: &ChecksumVerifier) -> VerifyResult {
    if !path.exists() {
        return VerifyResult::Missing;
    }

    let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

    if let Some(expected) = verifier.get_expected_hash(filename) {
        match calculate_md5(path).await {
            Ok(actual) => {
                if actual == expected {
                    VerifyResult::Valid
                } else {
                    VerifyResult::Invalid {
                        expected: expected.to_string(),
                        actual,
                    }
                }
            }
            Err(_) => VerifyResult::Invalid {
                expected: expected.to_string(),
                actual: "read error".to_string(),
            },
        }
    } else {
        VerifyResult::NoChecksum
    }
}

async fn scan_local_files(
    base_dir: &Path,
    format: FileFormat,
) -> Result<Vec<(PdbId, std::path::PathBuf)>> {
    // Scan local directory for files
    // Similar to list command implementation
    todo!("Implement local file scanning")
}
```

### 5. main.rs 更新

```rust
mod validation;

// In match:
Commands::Validate(args) => commands::run_validate(args, ctx).await,
```

## 使用例

```bash
# 全ファイル検証
pdb-cli validate --progress

# 特定IDのみ
pdb-cli validate 1abc 2xyz 3def

# エラーのみ表示
pdb-cli validate --errors-only

# 破損ファイルを修復
pdb-cli validate --fix

# 特定フォーマットのみ
pdb-cli validate --format pdb-gz
```

## 出力例

```
Validating 1234 files...
✓ 1abc OK
✓ 2xyz OK
✗ 3def CORRUPTED (expected: a1b2c3d4..., got: e5f6g7h8...)
? 4ghi MISSING

Validation complete:
  Valid:    1231
  Invalid:  2
  Missing:  1
```

## 完了条件

- [ ] ValidateArgs 定義
- [ ] validation/checksum.rs 実装
- [ ] validate.rs 実装
- [ ] MD5計算
- [ ] プログレス表示
- [ ] --fix オプション
- [ ] cargo build 成功
- [ ] cargo test 成功

## 依存追加

```toml
md-5 = "0.10"
```

## 工数

2-3日
