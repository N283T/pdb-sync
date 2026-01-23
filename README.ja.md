# pdb-sync

PDB (Protein Data Bank) データを rsync ミラーから同期するためのシンプルな CLI ツールです。

## 機能

- **カスタム rsync 設定**: 設定ファイルに複数の同期ソースを定義可能
- **一括同期**: すべての設定を一度に実行
- **並列実行**: `--parallel` で複数の同期操作を並列実行
- **自動リトライ**: 指数バックオフ付きで一時的な失敗を自動リトライ
- **プランモード**: `--plan` で実行前に変更内容をプレビュー
- **ビルトインプリセット**: よく使われる PDB ソースのクイックスタートプロファイル
- **柔軟な rsync オプション**: 設定ファイルのデフォルト値を CLI で上書き
- **進捗表示**: rsync の `--info=progress2` を常に有効化

## インストール

```bash
cargo install --path .
```

## クイックスタート

1. 設定ファイルを作成（`~/.config/pdb-sync/config.toml`）:

```toml
[sync]
mirror = "rcsb"  # デフォルトミラー（custom には直接使いません）

# カスタム rsync 設定
[[sync.custom]]
name = "structures"
url = "rsync.wwpdb.org::ftp_data/structures/divided/mmCIF/"
dest = "data/structures/divided/mmCIF"
description = "PDB structures (mmCIF format, divided layout)"

[[sync.custom]]
name = "emdb"
url = "data.pdbj.org::rsync/pub/emdb/"
dest = "data/emdb"
description = "EMDB (Electron Microscopy Data Bank)"
```

2. 同期を実行:

```bash
# すべての設定を同期
pdb-sync sync

# 特定の設定だけ同期
pdb-sync sync emdb

# すべて明示的に同期
pdb-sync sync --all

# 同期設定の一覧
pdb-sync sync --list
```

## 使い方

```
pdb-sync sync [NAME] [OPTIONS]

Arguments:
  [NAME]  同期するカスタム設定名（省略時は全て）

Options:
  --all                     すべてのカスタム設定を実行
  -d, --dest <DIR>          出力先ディレクトリを上書き
  --list                    カスタム設定一覧の表示
  --fail-fast               一つ失敗したら以降を中断
  -n, --dry-run             実行せずにコマンド表示のみ
  --plan                    プランモード - 変更内容をプレビュー（実行しない）
  --parallel <N>            最大並列実行数

  # ビルトインプロファイル
  --profile-list            利用可能なプロファイルプリセット一覧
  --profile-add <NAME>      プロファイルプリセットを設定に追加
  --profile-dry-run         プロファイル追加のドライラン（追加内容を確認）

  # 失敗時のリトライ
  --retry <COUNT>           リトライ回数（0 = リトライなし、デフォルト: 0）
  --retry-delay <SECONDS>   リトライ間隔（秒）（デフォルト: 指数バックオフ）

  # rsync オプション
  --delete                  リモートに無いファイルを削除
  --no-delete               削除を行わない（--delete を上書き）
  --bwlimit <KBPS>          帯域制限 (KB/s)
  -z, --compress            転送中に圧縮
  --no-compress             圧縮を行わない（-z/--compress を上書き）
  -c, --checksum            チェックサム比較を使用
  --no-checksum             チェックサムを使わない（-c/--checksum を上書き）
  --partial                 部分転送ファイルを保持
  --no-partial              部分転送ファイルを保持しない
  --partial-dir <DIR>       部分転送ファイルの保存先
  --max-size <SIZE>         転送するファイルの最大サイズ
  --min-size <SIZE>         転送するファイルの最小サイズ
  --timeout <SECONDS>       I/O タイムアウト（秒）
  --contimeout <SECONDS>    接続タイムアウト（秒）
  --backup                  バックアップを作成
  --no-backup               バックアップを作成しない
  --backup-dir <DIR>        バックアップ先
  --chmod <FLAGS>           パーミッション変更
  --exclude <PATTERN>       除外パターン（繰り返し指定可）
  --include <PATTERN>       取り込みパターン（繰り返し指定可）
  --exclude-from <FILE>     除外パターンファイル
  --include-from <FILE>     取り込みパターンファイル
  --rsync-verbose           rsync の詳細出力
  --no-rsync-verbose        詳細出力を有効にしない
  --rsync-quiet             rsync の簡易出力
  --no-rsync-quiet          簡易出力を有効にしない
  --itemize-changes         変更内容の一覧表示
  --no-itemize-changes      変更内容の一覧を表示しない

  -v, --verbose             pdb-sync の詳細ログ
  -h, --help                ヘルプ表示
```

### ビルトインプロファイルでクイックスタート

```bash
# プロファイルプリセット一覧
pdb-sync sync --profile-list

# プリセットを設定に追加（まずドライランで確認）
pdb-sync sync --profile-add structures --profile-dry-run

# プリセットを設定に追加
pdb-sync sync --profile-add structures
```

### 並列実行

```bash
# 最大 4 並列ですべての設定を同期
pdb-sync sync --all --parallel 4

# リトライと組み合わせて堅牢な同期
pdb-sync sync --all --parallel 4 --retry 3
```

### プランモード

```bash
# 変更内容をプレビュー
pdb-sync sync structures --plan

# すべての設定の変更内容をプレビュー
pdb-sync sync --all --plan
```

### 失敗時のリトライ

```bash
# 最大 3 回リトライ（指数バックオフ: 1秒, 2秒, 4秒）
pdb-sync sync structures --retry 3

# 固定間隔（5秒）でリトライ
pdb-sync sync structures --retry 3 --retry-delay 5
```

## 設定

設定ファイルの場所: `~/.config/pdb-sync/config.toml`

### カスタム rsync 設定

```toml
[[sync.custom]]
name = "my-sync"              # 必須: 一意の識別子
url = "host::module/path"      # 必須: rsync URL
dest = "local/path"            # 必須: 出力先（pdb_dir からの相対パス）
description = "Description"    # 任意

# 任意の rsync フラグ（設定のデフォルト値）
rsync_delete = true
rsync_compress = true
rsync_bwlimit = 1000           # KB/s
rsync_timeout = 600            # seconds
rsync_exclude = ["*.tmp", "test/*"]
```

### rsync オプション一覧

| Config Field | CLI Flag | 説明 |
|--------------|----------|------|
| `rsync_delete` | --delete / --no-delete | リモートに無いファイルを削除 |
| `rsync_compress` | -z, --compress / --no-compress | 転送時に圧縮 |
| `rsync_checksum` | -c, --checksum / --no-checksum | チェックサム比較 |
| `rsync_partial` | --partial / --no-partial | 部分転送ファイルを保持 |
| `rsync_partial_dir` | --partial-dir | 部分転送保存先 |
| `rsync_max_size` | --max-size | 最大サイズ |
| `rsync_min_size` | --min-size | 最小サイズ |
| `rsync_timeout` | --timeout | I/O タイムアウト（秒） |
| `rsync_contimeout` | --contimeout | 接続タイムアウト（秒） |
| `rsync_backup` | --backup / --no-backup | バックアップを作成 |
| `rsync_backup_dir` | --backup-dir | バックアップ先 |
| `rsync_chmod` | --chmod | 権限変更 |
| `rsync_exclude` | --exclude | 除外パターン（配列） |
| `rsync_include` | --include | 取り込みパターン（配列） |
| `rsync_exclude_from` | --exclude-from | 除外パターンファイル |
| `rsync_include_from` | --include-from | 取り込みパターンファイル |
| `rsync_verbose` | --rsync-verbose / --no-rsync-verbose | 詳細出力 |
| `rsync_quiet` | --rsync-quiet / --no-rsync-quiet | 簡易出力 |
| `rsync_itemize_changes` | --itemize-changes / --no-itemize-changes | 変更内容の一覧表示 |

## 例

```bash
# すべての設定を同期
pdb-sync sync

# 特定の設定のみ同期
pdb-sync sync structures

# 出力先を上書き
pdb-sync sync structures --dest /mnt/c/pdb

# すべてを明示的に同期
pdb-sync sync --all

# 詳細ログ
pdb-sync sync -v --all
```

## 環境変数

| 変数 | 説明 |
|------|------|
| `PDB_DIR` | PDB のベースディレクトリ |
| `PDB_SYNC_CONFIG` | 設定ファイルのパス |

