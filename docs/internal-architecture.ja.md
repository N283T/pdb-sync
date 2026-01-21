# pdb-sync 内部アーキテクチャ

このドキュメントはpdb-syncの内部実装についての技術的な概要です。

## プロジェクト構成

```
pdb-sync/
├── src/
│   ├── main.rs              # エントリーポイント
│   ├── lib.rs               # ライブラルート
│   ├── error.rs             # エラー型定義 (PdbSyncError)
│   ├── context.rs           # アプリケーションコンテキスト
│   ├── data_types.rs        # データ型定義
│   │
│   ├── cli/                 # CLIレイヤー
│   │   ├── args.rs          # コマンドライン引数定義 (clap)
│   │   ├── mod.rs           # CLIモジュールルート
│   │   └── commands/        # 各コマンドのハンドラー
│   │       ├── mod.rs
│   │       ├── sync/        # syncコマンドとサブコマンド
│   │       ├── download.rs  # downloadコマンド
│   │       ├── list.rs      # listコマンド
│   │       ├── find.rs      # findコマンド
│   │       ├── info.rs      # infoコマンド
│   │       ├── validate.rs  # validateコマンド
│   │       ├── update.rs    # updateコマンド
│   │       ├── watch.rs     # watchコマンド
│   │       ├── stats.rs     # statsコマンド
│   │       ├── tree.rs      # treeコマンド
│   │       ├── convert.rs   # convertコマンド
│   │       ├── copy.rs      # copyコマンド
│   │       ├── config.rs    # configコマンド
│   │       ├── jobs.rs      # jobsコマンド
│   │       ├── env.rs       # envコマンド
│   │       └── setup.rs     # セットアップウィザード
│   │
│   ├── config/              # 設定管理
│   │   ├── mod.rs
│   │   ├── loader.rs        # 設定ファイルの読み込み
│   │   └── schema.rs        # 設定スキーマ構造体
│   │
│   ├── mirrors/             # ミラー管理
│   │   ├── mod.rs
│   │   ├── registry.rs      # ミラーレジストリ
│   │   └── auto_select.rs   # 自動ミラー選択（レイテンシー計測）
│   │
│   ├── api/                 # 外部APIクライアント
│   │   ├── mod.rs
│   │   └── rcsb.rs          # RCSB Search API v2クライアント
│   │
│   ├── download/            # ダウンロードエンジン
│   │   ├── mod.rs
│   │   ├── engine.rs        # ダウンロードエンジンtrait
│   │   ├── https.rs         # HTTPSダウンロード実装
│   │   ├── aria2c.rs        # aria2c RPC実装
│   │   └── task.rs          # ダウンロードタスク管理
│   │
│   ├── sync/                # rsync同期
│   │   ├── mod.rs
│   │   ├── rsync.rs         # rsync実行
│   │   └── progress.rs      # rsync進捗パーサー
│   │
│   ├── validation/          # ファイル検証
│   │   ├── mod.rs
│   │   └── checksum.rs      # チェックサム検証
│   │
│   ├── update/              # 更新チェック
│   │   └── mod.rs
│   │
│   ├── watch/               # 新規エントリ監視
│   │   ├── mod.rs
│   │   ├── rcsb.rs          # RCSB Search APIクライアント
│   │   ├── state.rs         # 監視状態管理
│   │   ├── notify.rs        # 通知機能（デスクトップ/メール）
│   │   └── hooks.rs         # フック実行
│   │
│   ├── stats/               # 統計情報
│   │   ├── mod.rs
│   │   ├── collector.rs     # ローカル統計収集
│   │   ├── types.rs         # 統計型定義
│   │   └── remote.rs        # リモート統計取得
│   │
│   ├── tree/                # ディレクトリツリー
│   │   ├── mod.rs
│   │   ├── build.rs         # ツリー構築
│   │   └── render.rs        # ツリー描画
│   │
│   ├── convert/             # フォーマット変換
│   │   ├── mod.rs
│   │   ├── compress.rs      # 圧縮/解凍
│   │   └── format.rs        # フォーマット変換
│   │
│   ├── files/               # ファイル管理
│   │   ├── mod.rs
│   │   ├── paths.rs         # ファイルパス処理
│   │   └── pdb_id.rs        # PDB IDパース・検証
│   │
│   ├── jobs/                # バックグラウンドジョブ
│   │   ├── mod.rs           # ジョブ型定義、ユーティリティ
│   │   ├── manager.rs       # ジョブマネージャー
│   │   └── spawn.rs         # ジョブ生成
│   │
│   ├── history/             # 履歴管理
│   │   ├── mod.rs
│   │   └── tracker.rs       # 操作履歴トラッカー
│   │
│   └── utils/               # ユーティリティ
│       ├── mod.rs
│       ├── id_reader.rs     # PDB IDリーダー（stdin/file/args）
│       └── format.rs        # 出力フォーマット（text/json/csv）
│
├── Cargo.toml               # プロジェクト設定
├── README.md                # 英語README
└── README.ja.md             # 日本語README
```

## モジュール詳細

### CLIレイヤー (`cli/`)

**責任**: コマンドライン引数のパース、各コマンドのルーティング

- **args.rs**: clapを使った引数定義。コマンド、サブコマンド、オプションを定義
- **commands/**: 各コマンドの実装ハンドラー
  - コマンド固有のロジックを含む
  - 下層モジュール（download, sync, etc.）を呼び出す

### 設定管理 (`config/`)

**責任**: 設定ファイルの読み込み、バリデーション、デフォルト値管理

```rust
// 設定ファイルの場所
// デフォルト: ~/.config/pdb-sync/config.toml
// 環境変数: PDB_SYNC_CONFIGで上書き可能

// 主な設定項目
[paths]
pdb_dir = "/data/pdb"           // PDBファイルのベースディレクトリ

[sync]
mirror = "rcsb"                 // デフォルトミラー
data_types = ["structures"]     // 同期するデータタイプ
layout = "divided"              // ディレクトリレイアウト

[download]
default_format = "mmcif"        // デフォルトフォーマット
parallel = 4                    // 並列ダウンロード数
```

### ミラー管理 (`mirrors/`)

**責任**: PDBミラーの定義、URL生成、自動選択

- **registry.rs**: サポートする全ミラーの情報（RCSB, PDBj, PDBe, wwPDB）
- **auto_select.rs**: レイテンシー計測による最適ミラーの自動選択

```rust
// ミラー定義
pub struct Mirror {
    pub id: &'static str,
    pub name: &'static str,
    pub region: &'static str,
    pub rsync_host: &'static str,
    pub https_base: &'static str,
}
```

### APIクライアント (`api/`)

**責任**: 外部APIとの通信

- **rcsb.rs**: RCSB Search API v2 クライアント
  - 新規エントリの検索
  - エントリメタデータの取得
  - フィルタリング（実験手法、分解能、生物種）

```rust
// Search API使用例
let client = RcsbSearchClient::new()?;
let filters = SearchFilters {
    method: Some(ExperimentalMethod::Xray),
    resolution: Some(2.0),
    organism: Some("Homo sapiens".to_string()),
};
let entries = client.search_new_entries(since, &filters).await?;
```

### ダウンロードエンジン (`download/`)

**責任**: ファイルダウンロードの実行

- **engine.rs**: ダウンロードエンジンのtrait定義
- **https.rs**: reqwestを使ったHTTPSダウンロード実装
- **aria2c.rs**: aria2c RPCによる高速ダウンロード（オプション）
- **task.rs**: ダウンロードタスクの管理、進捗追跡

```rust
// ダウンロードエンジンtrait
#[async_trait]
pub trait DownloadEngine: Send + Sync {
    async fn download(&self, task: DownloadTask) -> Result<()>;
    fn concurrent_limit(&self) -> usize;
}
```

### rsync同期 (`sync/`)

**責任**: rsyncによるファイル同期の実行

- **rsync.rs**: rsyncプロセスの実行、引数構築
- **progress.rs**: rsyncの出力から進捗をパース

```rust
// rsync実行例
let rsync = RsyncEngine::new(mirror, options)?;
rsync.sync(data_type, dest, &progress_sender).await?;
```

### ファイル検証 (`validation/`)

**責任**: チェックサムによるファイル整合性検証

- **checksum.rs**: チェックサムの取得、比較、破損ファイルの検出

```rust
// チェックサム検証
let validator = ChecksumValidator::new(mirror)?;
let result = validator.validate(pdb_id, local_file).await?;
match result {
    ValidationStatus::Valid => Ok(()),
    ValidationStatus::Corrupted => Err(...),
    ValidationStatus::Missing => Err(...),
}
```

### 監視 (`watch/`)

**責任**: 新規PDBエントリの監視、自動ダウンロード

- **rcsb.rs**: RCSB Search APIで新規エントリを検索
- **state.rs**: 監視状態の永続化（最終チェック時刻）
- **notify.rs**: 通知機能（デスクトップ通知 via notify-rust、メール via lettre）
- **hooks.rs**: 新規エントリ検出時のスクリプト実行

```rust
// watchループ
let mut watcher = Watcher::new(state, filters)?;
loop {
    let new_entries = watcher.check_for_new_entries().await?;
    for entry in new_entries {
        download_entry(&entry).await?;
        notify(&entry)?;
        run_hook(&entry)?;
    }
    tokio::time::sleep(interval).await;
}
```

### 統計情報 (`stats/`)

**責任**: ローカルコレクションの統計収集、リモートとの比較

- **collector.rs**: ローカルファイルのスキャン、統計計算
- **types.rs**: 統計データ構造
- **remote.rs**: リモートPDBアーカイブの情報取得

```rust
// 統計収集
let collector = StatsCollector::new(pdb_dir)?;
let stats = collector.collect()?.calculate()?;
println!("Total files: {}", stats.total_count);
println!("Total size: {}", stats.total_size);
```

### ディレクトリツリー (`tree/`)

**責任**: ディレクトリ構造の可視化

- **build.rs**: ファイルシステムからツリー構造を構築
- **render.rs**: ツリーのテキスト描画（ASCII art）

```rust
// ツリー構築
let builder = TreeBuilder::new(pdb_dir, depth)?;
let tree = builder.build()?.sort_by(field)?;
// 描画
tree.render(&mut stdout)?;
```

### フォーマット変換 (`convert/`)

**責任**: ファイルフォーマットの変換、圧縮/解凍

- **compress.rs**: gzip圧縮/解凍（async-compression使用）
- **format.rs**: gemmiを使ったフォーマット変換（mmCIF ↔ PDB）

```rust
// 変換パイプライン
let converter = FormatConverter::new();
converter.convert_stream(input, output, from_format, to_format).await?;
```

### ファイル管理 (`files/`)

**責任**: ファイルパス処理、PDB IDのパースと検証

- **paths.rs**: PDBファイルのパス構築（divided/allレイアウト対応）
- **pdb_id.rs**: PDB IDのパース（クラシック4文字、拡張12文字）

```rust
// PDB IDパース
let pdb_id = PdbId::parse("4hhb")?;  // クラシック
let pdb_id = PdbId::parse("pdb_00001hhb")?;  // 拡張

// パス構築
let path = pdb_id.path(divided_layout, mmcif_format)?;
// => /data/pdb/data/structures/divided/pdb/hh/4hhb.cif.gz
```

### バックグラウンドジョブ (`jobs/`)

**責任**: 長時間実行タスクのバックグラウンド実行、管理

- **mod.rs**: ジョブ型定義、ID生成、検証
- **manager.rs**: ジョブの登録、ステータス追跡、クリーンアップ
- **spawn.rs**: 子プロセスとしてジョブを生成

```rust
// ジョブ生成
let job_id = generate_job_id();  // 8文字hex
let mut child = spawn_job(command, args, &job_id)?;
let meta = JobMeta::new(job_id, command_string, child.id());

// ジョブ管理
manager.register(meta)?;
// ジョブ終了を待機
child.wait()?.map(|status| meta.mark_completed(status.code()))?;
manager.save(&meta)?;
```

### 履歴管理 (`history/`)

**責任**: 操作履歴の記録、最終同期/ダウンロード時刻の管理

- **tracker.rs**: JSONファイルへの履歴保存

```rust
// 履歴ファイル
// ~/.cache/pdb-sync/history.json

{
  "last_sync": "2025-01-15T10:30:00Z",
  "last_download": "2025-01-15T11:00:00Z"
}
```

### ユーティリティ (`utils/`)

**責任**: 共通ユーティリティ関数

- **id_reader.rs**: 様々なソースからPDB IDを読み込み（引数、ファイル、stdin）
- **format.rs**: 出力フォーマット（text, json, csv）

```rust
// IDリーダー
let ids = IdReader::from_args(pdb_ids)?;          // コマンドライン引数
let ids = IdReader::from_file(path)?;             // ファイル
let ids = IdReader::from_stdin()?;                // 標準入力

// フォーマット出力
formatter.write(&results, &mut stdout)?;
```

## エラー処理

```rust
// error.rs
#[derive(Error, Debug)]
pub enum PdbSyncError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Search API error: {0}")]
    SearchApi(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Invalid PDB ID: {0}")]
    InvalidPdbId(String),

    // ... 他のエラー型
}

pub type Result<T> = std::result::Result<T, PdbSyncError>;
```

## 非同期ランタイム

**tokio**を使用した非同期I/O:

- HTTPリクエスト並列実行
- rsyncプロセスの非同期実行
- バックグラウンドジョブの管理

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    run_command(args).await?;
    Ok(())
}
```

## ログ出力

**tracing**を使用した構造化ログ:

```rust
// ログ設定
tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::from_default_env())
    .init();

// ログ出力
tracing::debug!("RCSB Search query: {}", query_json);
tracing::info!("Found {} entries", count);
tracing::error!("Download failed: {}", e);
```

## 進捗表示

**indicatif**を使用したプログレスバー:

```rust
let pb = ProgressBar::new(total);
pb.set_style(ProgressStyle::default_bar()
    .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}"));

for file in files {
    pb.inc(1);
    process(file).await?;
}
pb.finish_with_message("Done");
```

## 依存クレート

| クレート | 用途 |
|---------|------|
| clap | CLI引数パース |
| tokio | 非同期ランタイム |
| reqwest | HTTPクライアント |
| serde/serde_json | シリアライゼーション |
| tracing | 構造化ログ |
| indicatif | プログレスバー |
| directories | システムディレクトリ取得 |
| chrono | 日時処理 |
| regex | 正規表現 |
| anyhow/thiserror | エラー処理 |
| async-compression | 非同期圧縮/解凍 |
| tempfile | テスト用一時ファイル |

## 環境変数

| 変数 | 説明 |
|------|------|
| `PDB_DIR` | PDBファイルのベースディレクトリ |
| `PDB_SYNC_CONFIG` | 設定ファイルのパス |
| `PDB_SYNC_MIRROR` | デフォルトミラー |
| `RUST_LOG` | ログレベル（debug, info, warn, error） |

## キャッシュと状態

### ディレクトリ構造

```
~/.config/pdb-sync/       # 設定
├── config.toml

~/.cache/pdb-sync/        # キャッシュ・状態
├── history.json          # 履歴
├── jobs/                 # バックグラウンドジョブ
│   ├── abc12345/
│   │   ├── meta.json     # ジョブメタデータ
│   │   ├── output.log    # 標準出力
│   │   └── error.log     # 標準エラー
│   └── ...
└── watch/                # watch状態
    ├── last_check.json   # 最終チェック時刻
    └── seen_entries.json # 既見エントリ
```

## テスト戦略

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdb_id_parse() {
        let id = PdbId::parse("4hhb").unwrap();
        assert_eq!(id.id(), "4HHB");
        assert_eq!(id.middle_chars(), "hh");
    }

    #[tokio::test]
    async fn test_search_api() {
        // ネットワークテストは通常無効
        #[ignore]
        async fn test_real_api() {
            let client = RcsbSearchClient::new().unwrap();
            let result = client.search_new_entries(...).await;
            assert!(result.is_ok());
        }
    }
}
```

## 拡張ポイント

### 新しいデータタイプを追加

1. `src/data_types.rs` に `DataType` 列挙子を追加
2. `src/files/paths.rs` にパス構築ロジックを追加
3. `src/cli/commands/sync/` に同期ロジックを追加
4. テストを追加

### 新しいミラーを追加

1. `src/mirrors/registry.rs` に `Mirror` 定義を追加
2. 自動選択の地域を更新
3. テストでURLを確認

### 新しいコマンドを追加

1. `src/cli/args.rs` にサブコマンド定義を追加
2. `src/cli/commands/` にハンドラーを実装
3. `src/cli/commands/mod.rs` にルーティングを追加
4. テストを追加
