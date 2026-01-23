# config.toml リファレンス

pdb-sync の設定ファイル `config.toml` の完全なリファレンスドキュメントです。

## 目次

- [ファイルの場所](#ファイルの場所)
- [基本構造](#基本構造)
- [paths セクション](#paths-セクション)
- [sync セクション](#sync-セクション)
- [sync.custom.NAME セクション](#synccustomname-セクション)
- [sync.custom.NAME.options セクション](#synccustomnameoptions-セクション)
- [mirror_selection セクション](#mirror_selection-セクション)
- [プリセット一覧](#プリセット一覧)
- [優先順位](#優先順位)
- [設定例](#設定例)
- [環境変数](#環境変数)

---

## ファイルの場所

デフォルト: `~/.config/pdb-sync/config.toml`

環境変数 `PDB_SYNC_CONFIG` で上書き可能:
```bash
export PDB_SYNC_CONFIG=/path/to/custom/config.toml
pdb-sync sync
```

---

## 基本構造

```toml
[paths]
pdb_dir = "/data/pdb"

[sync]
mirror = "rcsb"
bwlimit = 5000
delete = false
layout = "divided"
data_types = ["structures", "assemblies"]

[sync.custom.structures]
url = "rsync.wwpdb.org::ftp_data/structures/divided/mmCIF/"
dest = "data/structures"
preset = "fast"

[mirror_selection]
auto_select = false
preferred_region = "us"
latency_cache_ttl = 3600
```

---

## paths セクション

PDB データの保存場所を設定します。

### `pdb_dir`

**型**: String (パス)
**デフォルト**: なし（必須）
**説明**: PDB データを保存するベースディレクトリ

```toml
[paths]
pdb_dir = "/mnt/data/pdb"
```

### `data_type_dirs`

**型**: HashMap<String, String>
**デフォルト**: `{}`
**説明**: データタイプごとに異なるディレクトリを指定

```toml
[paths.data_type_dirs]
structures = "/mnt/ssd/pdb/structures"
assemblies = "/mnt/hdd/pdb/assemblies"
```

---

## sync セクション

同期の全般設定を行います。

### `mirror`

**型**: String
**デフォルト**: `"rcsb"`
**選択肢**: `rcsb`, `pdbj`, `pdbe`, `wwpdb`
**説明**: 使用するミラーサーバー

エイリアス:
- `rcsb` / `us`
- `pdbj` / `jp`
- `pdbe` / `uk` / `eu` / `europe`
- `wwpdb` / `global`

```toml
[sync]
mirror = "pdbj"  # または "jp"
```

### `bwlimit`

**型**: Integer (KB/s)
**デフォルト**: `0` (制限なし)
**説明**: 帯域制限（キロバイト/秒）

```toml
[sync]
bwlimit = 5000  # 5 MB/s に制限
```

### `delete`

**型**: Boolean
**デフォルト**: `false`
**説明**: リモートに無いファイルをローカルから削除

```toml
[sync]
delete = true  # ミラーと完全一致させる
```

### `layout`

**型**: String
**デフォルト**: `"divided"`
**選択肢**: `divided`, `all`
**説明**: ファイルレイアウト形式

- `divided`: ハッシュベースのディレクトリ構造（例: `ab/1abc.cif.gz`）
- `all`: フラットなディレクトリ構造（例: `1abc.cif.gz`）

```toml
[sync]
layout = "divided"
```

### `data_types`

**型**: Array of String
**デフォルト**: `["structures"]`
**選択肢**: `structures`, `assemblies`, `structure-factors`, `chemical-components`, `validation-reports`
**説明**: 同期するデータタイプのリスト

```toml
[sync]
data_types = ["structures", "assemblies", "validation-reports"]
```

エイリアス:
- `structures` / `st` / `struct`
- `assemblies` / `asm` / `assembly`
- `structure-factors` / `sf` / `xray`
- `chemical-components` / `cc` / `chem`
- `validation-reports` / `val` / `validation`

---

## sync.custom.NAME セクション

カスタム rsync 設定を定義します。**HashMap 形式**を使用するため、`name` フィールドは不要です。

### 基本フィールド

#### `url`

**型**: String
**必須**: ✅
**説明**: rsync URL

対応形式:
- `rsync://` プロトコル: `rsync://rsync.ebi.ac.uk/pub/databases/msd/sifts/`
- `::` 形式: `data.pdbj.org::rsync/pub/emdb/`

```toml
[sync.custom.emdb]
url = "data.pdbj.org::rsync/pub/emdb/"
```

#### `dest`

**型**: String
**必須**: ✅
**説明**: `pdb_dir` からの相対パス

```toml
[sync.custom.emdb]
dest = "data/emdb"  # /data/pdb/data/emdb に保存
```

#### `description`

**型**: String
**デフォルト**: なし
**説明**: 設定の説明（`--list` で表示）

```toml
[sync.custom.emdb]
description = "EMDB (Electron Microscopy Data Bank)"
```

#### `preset`

**型**: String
**デフォルト**: なし
**選択肢**: `safe`, `fast`, `minimal`, `conservative`
**説明**: rsync フラグのプリセット（詳細は[プリセット一覧](#プリセット一覧)参照）

```toml
[sync.custom.structures]
url = "rsync.wwpdb.org::ftp_data/structures/"
dest = "data/structures"
preset = "fast"
```

---

## sync.custom.NAME.options セクション

個別の rsync オプションを設定します。プリセットよりも優先されます。

### Boolean フラグ

すべて **Optional** で、明示的に指定しない場合はプリセットまたはデフォルト値を使用します。

#### `delete`

**型**: Boolean
**デフォルト**: `false`
**説明**: リモートに無いファイルを削除

```toml
[sync.custom.structures.options]
delete = true
```

#### `compress`

**型**: Boolean
**デフォルト**: `false`
**説明**: 転送時にデータを圧縮

```toml
[sync.custom.structures.options]
compress = true
```

#### `checksum`

**型**: Boolean
**デフォルト**: `false`
**説明**: チェックサムでファイルを比較（タイムスタンプではなく）

```toml
[sync.custom.structures.options]
checksum = true
```

#### `size_only`

**型**: Boolean
**デフォルト**: `false`
**説明**: サイズのみでファイルを比較（タイムスタンプ無視）

```toml
[sync.custom.structures.options]
size_only = true
```

#### `ignore_times`

**型**: Boolean
**デフォルト**: `false`
**説明**: タイムスタンプを無視して常にファイルを転送

```toml
[sync.custom.structures.options]
ignore_times = true
```

#### `modify_window`

**型**: Integer（秒）
**デフォルト**: None
**説明**: 比較時のタイムスタンプ許容誤差（秒単位）

```toml
[sync.custom.structures.options]
modify_window = 2  # タイムスタンプの2秒差まで許容
```

#### `partial`

**型**: Boolean
**デフォルト**: `false`
**説明**: 部分転送ファイルを保持（再開可能）

```toml
[sync.custom.structures.options]
partial = true
```

#### `backup`

**型**: Boolean
**デフォルト**: `false`
**説明**: 上書き前にバックアップを作成

```toml
[sync.custom.structures.options]
backup = true
backup_dir = ".backup"
```

#### `verbose`

**型**: Boolean
**デフォルト**: `false`
**説明**: rsync の詳細出力

```toml
[sync.custom.structures.options]
verbose = true
```

#### `quiet`

**型**: Boolean
**デフォルト**: `false`
**説明**: rsync の簡易出力（`verbose` と排他）

```toml
[sync.custom.structures.options]
quiet = true
```

#### `itemize_changes`

**型**: Boolean
**デフォルト**: `false`
**説明**: 変更内容を一覧表示

```toml
[sync.custom.structures.options]
itemize_changes = true
```

### String / Integer オプション

#### `partial_dir`

**型**: String
**デフォルト**: なし
**説明**: 部分転送ファイルの保存先（`partial = true` が必要）

```toml
[sync.custom.structures.options]
partial = true
partial_dir = ".rsync-partial"
```

#### `backup_dir`

**型**: String
**デフォルト**: なし
**説明**: バックアップ先ディレクトリ（`backup = true` が必要）

```toml
[sync.custom.structures.options]
backup = true
backup_dir = ".backup"
```

#### `max_size`

**型**: String
**デフォルト**: なし
**説明**: 転送する最大ファイルサイズ

形式: `5GB`, `500MB`, `1024K`

```toml
[sync.custom.structures.options]
max_size = "5GB"
```

#### `min_size`

**型**: String
**デフォルト**: なし
**説明**: 転送する最小ファイルサイズ

```toml
[sync.custom.structures.options]
min_size = "1K"
```

#### `timeout`

**型**: Integer (秒)
**デフォルト**: なし
**説明**: I/O タイムアウト

```toml
[sync.custom.structures.options]
timeout = 300
```

#### `contimeout`

**型**: Integer (秒)
**デフォルト**: なし
**説明**: 接続タイムアウト

```toml
[sync.custom.structures.options]
contimeout = 30
```

#### `chmod`

**型**: String
**デフォルト**: なし
**説明**: パーミッション変更フラグ

```toml
[sync.custom.structures.options]
chmod = "644"
```

### 配列オプション

#### `exclude`

**型**: Array of String
**デフォルト**: `[]`
**説明**: 除外パターン（rsync グロブ形式）

```toml
[sync.custom.structures.options]
exclude = ["obsolete/", "*.tmp", "test/*"]
```

#### `include`

**型**: Array of String
**デフォルト**: `[]`
**説明**: 取り込みパターン

```toml
[sync.custom.structures.options]
include = ["*.cif.gz"]
exclude = ["*"]  # include 以外を除外
```

#### `exclude_from`

**型**: String
**デフォルト**: なし
**説明**: 除外パターンを記述したファイルパス

```toml
[sync.custom.structures.options]
exclude_from = "/path/to/exclude.txt"
```

#### `include_from`

**型**: String
**デフォルト**: なし
**説明**: 取り込みパターンを記述したファイルパス

```toml
[sync.custom.structures.options]
include_from = "/path/to/include.txt"
```

---

## mirror_selection セクション

ミラーの自動選択機能を設定します。

### `auto_select`

**型**: Boolean
**デフォルト**: `false`
**説明**: レイテンシに基づく自動ミラー選択を有効化

```toml
[mirror_selection]
auto_select = true
```

### `preferred_region`

**型**: String
**デフォルト**: なし
**選択肢**: `us`, `jp`, `europe`
**説明**: 優先地域（2倍のレイテンシ許容範囲内で優先）

```toml
[mirror_selection]
auto_select = true
preferred_region = "jp"
```

### `latency_cache_ttl`

**型**: Integer (秒)
**デフォルト**: `3600` (1時間)
**説明**: レイテンシキャッシュの有効期限

```toml
[mirror_selection]
latency_cache_ttl = 7200  # 2時間
```

---

## プリセット一覧

### `safe`（安全優先）

初回同期や慎重なユーザー向け。

| オプション | 値 |
|-----------|---|
| `delete` | ❌ `false` |
| `compress` | ✅ `true` |
| `checksum` | ✅ `true` |
| `partial` | ✅ `true` |
| `backup` | ❌ `false` |
| `verbose` | ✅ `true` |
| `quiet` | ❌ `false` |

**用途**: 誤削除を防ぎたい、確実に同期したい

```toml
[sync.custom.structures]
url = "rsync.wwpdb.org::ftp_data/structures/"
dest = "data/structures"
preset = "safe"
```

### `fast`（速度優先）

定期的な更新で速度を重視。

| オプション | 値 |
|-----------|---|
| `delete` | ✅ `true` |
| `compress` | ✅ `true` |
| `checksum` | ❌ `false` |
| `partial` | ✅ `true` |
| `backup` | ❌ `false` |
| `verbose` | ❌ `false` |
| `quiet` | ✅ `true` |

**用途**: 毎日の定期同期、完全ミラー維持

```toml
[sync.custom.structures]
preset = "fast"
```

### `minimal`（最小限）

完全制御が必要な場合の最小設定。

| オプション | 値 |
|-----------|---|
| `delete` | ❌ `false` |
| `compress` | ❌ `false` |
| `checksum` | ❌ `false` |
| `partial` | ❌ `false` |
| `backup` | ❌ `false` |
| `verbose` | ❌ `false` |
| `quiet` | ❌ `false` |

**用途**: カスタムオプションで細かく制御したい

```toml
[sync.custom.structures]
preset = "minimal"

[sync.custom.structures.options]
# 必要なオプションだけ追加
delete = true
timeout = 600
```

### `conservative`（保守的）

本番環境での最大限の安全性。

| オプション | 値 |
|-----------|---|
| `delete` | ❌ `false` |
| `compress` | ✅ `true` |
| `checksum` | ✅ `true` |
| `partial` | ✅ `true` |
| `backup` | ✅ `true` |
| `verbose` | ✅ `true` |
| `quiet` | ❌ `false` |

**用途**: 本番サーバー、データ損失を避けたい

```toml
[sync.custom.structures]
preset = "conservative"

[sync.custom.structures.options]
backup_dir = ".backup"
```

---

## 優先順位

複数の設定方法を併用した場合の優先順位:

**options > preset > legacy > デフォルト**

### 例: delete フラグの決定

```toml
[sync.custom.test]
url = "example.org::data"
dest = "data/test"

# Legacy: delete=false
rsync_delete = false

# Preset "fast": delete=true
preset = "fast"

# Options: delete=false (最優先)
[sync.custom.test.options]
delete = false
```

**結果**: `delete = false`（options から）

### 優先順位の詳細

1. **CLI 引数**（最優先）
   ```bash
   pdb-sync sync structures --delete
   ```

2. **options セクション**
   ```toml
   [sync.custom.structures.options]
   delete = true
   ```

3. **preset**
   ```toml
   [sync.custom.structures]
   preset = "fast"  # delete=true
   ```

4. **legacy フィールド**（後方互換）
   ```toml
   [sync.custom.structures]
   rsync_delete = true
   ```

5. **デフォルト値**
   - `delete = false`
   - `compress = false`
   - など

---

## 設定例

### 1. シンプル構成（プリセットのみ）

```toml
[paths]
pdb_dir = "/data/pdb"

[sync]
mirror = "rcsb"

[sync.custom.structures]
url = "rsync.wwpdb.org::ftp_data/structures/divided/mmCIF/"
dest = "data/structures"
preset = "fast"
```

### 2. 複数データソース + プリセット上書き

```toml
[paths]
pdb_dir = "/data/pdb"

[sync]
mirror = "pdbj"
bwlimit = 5000

# 構造データ（速度優先）
[sync.custom.structures]
url = "rsync.wwpdb.org::ftp_data/structures/divided/mmCIF/"
dest = "data/structures/mmCIF"
description = "PDB structures (mmCIF format)"
preset = "fast"

[sync.custom.structures.options]
max_size = "10GB"
exclude = ["obsolete/"]

# EMDB（安全優先 + サイズ制限）
[sync.custom.emdb]
url = "data.pdbj.org::rsync/pub/emdb/"
dest = "data/emdb"
description = "Electron Microscopy Data Bank"
preset = "safe"

[sync.custom.emdb.options]
max_size = "5GB"
timeout = 600

# SIFTS（カスタム設定）
[sync.custom.sifts]
url = "ftp.pdbj.org::pub/pdbj/data/sifts/"
dest = "pdbj/sifts"
description = "SIFTS data from PDBj"

[sync.custom.sifts.options]
delete = true
compress = true
checksum = true
exclude = ["*.tmp", "test/*"]
```

### 3. 本番環境設定（保守的）

```toml
[paths]
pdb_dir = "/mnt/storage/pdb"

[sync]
mirror = "rcsb"
bwlimit = 10000
delete = false

[mirror_selection]
auto_select = true
preferred_region = "us"
latency_cache_ttl = 7200

[sync.custom.structures]
url = "rsync.wwpdb.org::ftp_data/structures/divided/mmCIF/"
dest = "structures/mmCIF"
preset = "conservative"

[sync.custom.structures.options]
backup_dir = "/backup/pdb/structures"
timeout = 1800
partial = true
partial_dir = ".rsync-partial"
itemize_changes = true
```

### 4. 開発環境設定（最小限 + 詳細ログ）

```toml
[paths]
pdb_dir = "/home/user/dev/pdb-data"

[sync]
mirror = "pdbj"

[sync.custom.test-structures]
url = "rsync.wwpdb.org::ftp_data/structures/divided/mmCIF/"
dest = "structures"
preset = "minimal"

[sync.custom.test-structures.options]
max_size = "100MB"  # テスト用に小さいファイルのみ
verbose = true
itemize_changes = true
exclude = ["obsolete/"]
```

### 5. 並列実行最適化設定

```toml
[paths]
pdb_dir = "/data/pdb"

[sync]
mirror = "rcsb"

# 小ファイル向け（並列10推奨）
[sync.custom.structures]
url = "rsync.wwpdb.org::ftp_data/structures/divided/mmCIF/"
dest = "structures"
preset = "fast"

[sync.custom.structures.options]
timeout = 300

# 大ファイル向け（並列2-4推奨）
[sync.custom.emdb]
url = "data.pdbj.org::rsync/pub/emdb/"
dest = "emdb"
preset = "fast"

[sync.custom.emdb.options]
timeout = 3600
bwlimit = 5000  # 個別に帯域制限
```

実行:
```bash
pdb-sync sync --all --parallel 10
```

---

## 環境変数

設定ファイルの値は環境変数で上書き可能です。

### `PDB_DIR`

**説明**: PDB データディレクトリ
**優先順位**: CLI引数 > 環境変数 > config.toml

```bash
export PDB_DIR=/mnt/data/pdb
pdb-sync sync structures
```

### `PDB_SYNC_CONFIG`

**説明**: 設定ファイルのパス
**デフォルト**: `~/.config/pdb-sync/config.toml`

```bash
export PDB_SYNC_CONFIG=/etc/pdb-sync/config.toml
pdb-sync sync
```

### 環境変数の優先順位

```
CLI引数 > 環境変数 > config.toml > デフォルト値
```

例:
```bash
# config.toml に pdb_dir = "/data/pdb" があっても上書き
export PDB_DIR=/tmp/pdb
pdb-sync sync structures

# さらに CLI で上書き
pdb-sync sync structures --dest /override/path
```

---

## トラブルシューティング

### 設定ファイルのバリデーション

```bash
pdb-sync config validate
```

### 設定内容の確認

```bash
# カスタム設定の一覧
pdb-sync sync --list

# プリセット一覧
pdb-sync config presets
```

### よくあるエラー

#### エラー: "Config name cannot contain spaces"

**原因**: HashMap キー（設定名）にスペースが含まれている

**修正前**:
```toml
[sync.custom."my structures"]  # NG
```

**修正後**:
```toml
[sync.custom.my-structures]  # OK
```

#### エラー: "partial_dir is set but partial is false"

**原因**: `partial_dir` を指定しているが `partial = true` がない

**修正**:
```toml
[sync.custom.structures.options]
partial = true
partial_dir = ".rsync-partial"
```

#### エラー: "verbose and quiet are both true"

**原因**: 排他的なオプションを両方有効にしている

**修正**:
```toml
[sync.custom.structures.options]
verbose = true
# quiet = true  # 削除
```

---

## 関連コマンド

```bash
# 設定の検証
pdb-sync config validate

# プリセット一覧
pdb-sync config presets

# カスタム設定の一覧
pdb-sync sync --list

# ドライラン（実行せずに確認）
pdb-sync sync structures --dry-run

# プランモード（変更内容をプレビュー）
pdb-sync sync structures --plan
```

---

**最終更新**: 2026-01-23
**バージョン**: pdb-sync v0.1.0
