# PDB-CLI Implementation Plan

## Overview

Protein Data Bankのファイル管理CLI。rsync同期、ファイルダウンロード、ローカルファイル管理を提供。

## CLI Commands

```
pdb-cli <COMMAND>

Commands:
  sync       rsyncでミラーから同期
  download   HTTPSで個別ファイルをダウンロード
  copy       ローカルファイルのコピー
  config     設定管理
  env        環境変数管理
```

### sync

```bash
pdb-cli sync [OPTIONS] [FILTER]...
  -m, --mirror <REGION>  ミラー選択: us, jp, uk, global
  -f, --format <FORMAT>  フォーマット: pdb, mmcif, both
  -d, --dest <DIR>       保存先ディレクトリ
  --delete               リモートにないファイルを削除
  --bwlimit <KBPS>       帯域制限
  -n, --dry-run          実行せず確認のみ
```

### download

```bash
pdb-cli download [OPTIONS] <PDB_ID>...
  -f, --format <FORMAT>  フォーマット: pdb, mmcif, bcif
  -d, --dest <DIR>       保存先
  --decompress           gzを解凍
```

### copy

```bash
pdb-cli copy [OPTIONS] <SOURCE> <DEST>
  --flatten              フラットにコピー
  --symlink              シンボリックリンク作成
```

### config / env

```bash
pdb-cli config init|show|set|get
pdb-cli env show|export|set
```

## Mirror Registry

| ID | Region | rsync URL |
|----|--------|-----------|
| rcsb | US | rsync://rsync.rcsb.org (port 33444) |
| pdbj | JP | rsync://rsync.pdbj.org |
| pdbe | UK | rsync://rsync.ebi.ac.uk/pub/databases/pdb/ |
| wwpdb | Global | rsync://rsync.wwpdb.org |

## Configuration (TOML)

```toml
# ~/.config/pdb-cli/config.toml

[paths]
pdb_dir = "/data/pdb"

[sync]
mirror = "us"
bwlimit = 0

[download]
auto_decompress = true
parallel = 4
```

## Environment Variables

- `PDB_DIR` - ベースディレクトリ
- `PDB_CLI_CONFIG` - 設定ファイルパス
- `PDB_CLI_MIRROR` - デフォルトミラー

## Module Structure

```
src/
├── main.rs              # エントリポイント
├── cli/
│   ├── args.rs          # Clap定義
│   └── commands/
│       ├── sync.rs
│       ├── download.rs
│       ├── copy.rs
│       ├── config.rs
│       └── env.rs
├── config/
│   ├── loader.rs        # TOML読み込み
│   └── schema.rs        # 設定構造体
├── mirrors/
│   └── registry.rs      # ミラー定義
├── sync/
│   └── rsync.rs         # rsyncラッパー
├── download/
│   └── https.rs         # HTTPSダウンロード
├── files/
│   ├── pdb_id.rs        # PDB ID検証
│   └── paths.rs         # パス構築
├── context.rs           # アプリケーションコンテキスト
└── error.rs             # エラー型
```

## Dependencies

```toml
[dependencies]
clap = { version = "4.5", features = ["derive", "env"] }
toml = "0.8"
serde = { version = "1.0", features = ["derive"] }
directories = "5.0"
thiserror = "2.0"
anyhow = "1.0"
reqwest = { version = "0.12", features = ["stream", "rustls-tls"] }
tokio = { version = "1.0", features = ["rt-multi-thread", "macros", "fs"] }
indicatif = "0.17"
tracing = "0.1"
tracing-subscriber = "0.3"
regex = "1.10"
url = "2.5"
```

## Implementation Phases

### Phase 1: Foundation
1. プロジェクト構造作成
2. CLI引数パース (clap)
3. 設定ファイル読み込み
4. PDB ID検証

### Phase 2: Download
1. HTTPSダウンロード実装
2. プログレスバー表示
3. リトライロジック

### Phase 3: Sync
1. rsyncコマンドラッパー
2. ミラー選択
3. フィルタリング

### Phase 4: File Management
1. コピーコマンド
2. 環境変数管理
3. シェル補完

## Critical Files

- `src/cli/args.rs` - CLI定義の中心
- `src/config/schema.rs` - 設定構造体
- `src/files/pdb_id.rs` - PDB IDドメインロジック
- `src/context.rs` - 設定統合

## Verification

1. `cargo build` - ビルド確認
2. `cargo test` - ユニットテスト
3. `pdb-cli config init` - 設定初期化テスト
4. `pdb-cli download 1abc --dry-run` - ダウンロードテスト
5. `pdb-cli sync --mirror jp --dry-run` - rsync同期テスト
