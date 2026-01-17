# pdb-cli

Protein Data Bank (PDB) ファイル管理用CLIツール。PDBミラーからのrsync同期、HTTPSダウンロード、ローカルファイル管理、検証、自動更新に対応。

## 機能

- **sync**: rsyncを使ってPDBミラーからファイルを同期
- **download**: HTTPSで並列ダウンロード
- **list**: ローカルPDBファイルの一覧表示・検索
- **find**: スクリプト用のパス出力でファイル検索
- **info**: RCSB APIからエントリ情報を取得
- **validate**: チェックサムでファイル整合性を検証、自動修復オプション付き
- **update**: ローカルファイルの更新確認・ダウンロード
- **watch**: 新規PDBエントリを監視して自動ダウンロード
- **stats**: ローカルコレクションの統計情報表示
- **tree**: ディレクトリ構造のツリー表示
- **convert**: フォーマット変換（圧縮/解凍/形式変換）
- **copy**: ローカルPDBファイルのコピー
- **config**: 設定管理、ミラー自動選択
- **jobs**: バックグラウンドジョブ管理
- **env**: 環境変数管理

## インストール

```bash
cargo install --path .
```

## クイックスタート

```bash
# 初期設定（対話式）
pdb-cli config init

# 構造ファイルをダウンロード
pdb-cli download 4hhb 1abc 2xyz

# ローカルファイル一覧
pdb-cli list

# エントリ情報を取得
pdb-cli info 4hhb

# ローカルファイルを検証
pdb-cli validate --progress

# 更新を確認
pdb-cli update --check

# 新規エントリを確認
pdb-cli watch --once --dry-run
```

## コマンド

### sync

rsyncを使ってPDBミラーからファイルを同期。サブコマンドで異なるデータソースに対応。

```bash
pdb-cli sync [OPTIONS] [COMMAND]

サブコマンド:
  wwpdb       wwPDB標準データ（構造、アセンブリなど）を同期
  structures  `wwpdb --type structures` のショートカット
  assemblies  `wwpdb --type assemblies` のショートカット
  pdbj        PDBj固有データ（EMDB、PDB-IHM、派生データ）を同期
  pdbe        PDBe固有データ（SIFTS、PDBeChem、Foldseek）を同期

オプション:
  -m, --mirror <MIRROR>     ミラー: rcsb, pdbj, pdbe, wwpdb
  -t, --type <DATA_TYPE>    データタイプ（複数指定可）
  -f, --format <FORMAT>     フォーマット: pdb, mmcif, both [デフォルト: mmcif]
  -l, --layout <LAYOUT>     レイアウト: divided, all [デフォルト: divided]
  -d, --dest <DIR>          出力先ディレクトリ
  --delete                  リモートにないファイルを削除
  --bwlimit <KBPS>          帯域制限 (KB/s)
  -n, --dry-run             変更なしでドライラン
  -P, --progress            詳細な進捗表示
  --bg                      バックグラウンドで実行
```

例:
```bash
# PDBjからmmCIF構造を同期（ドライラン）
pdb-cli sync --mirror pdbj --dry-run

# ショートカットで構造を同期
pdb-cli sync structures --mirror rcsb

# 複数のデータタイプを同期
pdb-cli sync wwpdb -t structures -t assemblies --mirror rcsb

# PDBj固有データ（EMDB）を同期
pdb-cli sync pdbj --type emdb

# PDBe Foldseekデータベースを同期
pdb-cli sync pdbe --type foldseek

# バックグラウンドで同期
pdb-cli sync --mirror wwpdb --bg
```

### download

HTTPSで個別ファイルを並列ダウンロード。

```bash
pdb-cli download [OPTIONS] <PDB_IDS>...

オプション:
  -t, --type <DATA_TYPE>    データタイプ [デフォルト: structures]
  -f, --format <FORMAT>     フォーマット: pdb, mmcif, bcif [デフォルト: mmcif]
  -a, --assembly <NUM>      アセンブリ番号（assembliesタイプ用）
  -d, --dest <DIR>          出力先ディレクトリ
  -m, --mirror <MIRROR>     使用ミラー
  -p, --parallel <NUM>      並列ダウンロード数 [デフォルト: 4]
  --retry <NUM>             リトライ回数 [デフォルト: 3]
  --decompress              ダウンロード後に解凍
  --overwrite               既存ファイルを上書き
  -l, --list <FILE>         ファイルからPDB IDを読み込み
  --bg                      バックグラウンドで実行
```

例:
```bash
# 複数の構造をダウンロード
pdb-cli download 4hhb 1abc 2xyz --dest ./structures

# PDBフォーマットで解凍してダウンロード
pdb-cli download 4hhb --format pdb --decompress

# 生物学的アセンブリをダウンロード
pdb-cli download 4hhb -t assemblies -a 1

# 構造因子をダウンロード
pdb-cli download 1abc -t structure-factors

# ファイルリストから8並列でダウンロード
pdb-cli download -l pdb_ids.txt -p 8

# バックグラウンドでダウンロード
pdb-cli download -l large_list.txt --bg
```

### list

ローカルPDBファイルの一覧表示・フィルタリング・統計。

```bash
pdb-cli list [OPTIONS] [PATTERN]

オプション:
  -f, --format <FORMAT>     表示するフォーマット
  -s, --size                ファイルサイズを表示
  --time                    更新日時を表示
  -o, --output <FORMAT>     出力形式: text, json, csv, ids [デフォルト: text]
  --stats                   統計のみ表示
  --sort <FIELD>            ソート: name, size, time [デフォルト: name]
  -r, --reverse             逆順ソート
```

例:
```bash
# 全ファイル一覧
pdb-cli list

# パターンにマッチするファイル
pdb-cli list "1ab*"

# 統計のみ表示
pdb-cli list --stats

# サイズ付きでサイズ降順ソート
pdb-cli list -s --sort size -r

# JSONでエクスポート
pdb-cli list -o json > files.json

# IDのみ取得（パイプ用）
pdb-cli list -o ids | head -10
```

### find

スクリプト用に最適化されたパス出力でファイル検索。

```bash
pdb-cli find [OPTIONS] [PATTERNS]...

オプション:
  -f, --format <FORMAT>     検索フォーマット
  --all-formats             各エントリの全フォーマットを表示
  --exists                  存在確認（終了コードのみ）
  --missing                 ローカルに存在しないエントリを表示
  -q, --quiet               静寂モード（出力なし、終了コードのみ）
  --count                   マッチ数のみ表示
  --stdin                   標準入力からパターンを読み込み
```

例:
```bash
# 特定エントリを検索
pdb-cli find 4hhb 1abc

# エントリの全フォーマットを表示
pdb-cli find 4hhb --all-formats

# ファイル存在確認（スクリプト用）
pdb-cli find 4hhb --exists && echo "Found"

# リストから存在しないエントリを検索
cat ids.txt | pdb-cli find --stdin --missing

# xargsと組み合わせ
pdb-cli find "1ab*" | xargs -I{} cp {} ./output/
```

### validate

チェックサムでローカルPDBファイルを検証。

```bash
pdb-cli validate [OPTIONS] [PDB_IDS]...

オプション:
  -f, --format <FORMAT>     検証するフォーマット
  -m, --mirror <MIRROR>     チェックサム取得元ミラー
  --fix                     破損ファイルを再ダウンロード
  -P, --progress            プログレスバー表示
  --errors-only             エラーのみ表示
  -o, --output <FORMAT>     出力形式: text, json, csv, ids [デフォルト: text]
```

例:
```bash
# 全ローカルファイルを検証
pdb-cli validate -P

# 特定IDを検証
pdb-cli validate 1abc 2xyz 3def

# 破損ファイルを検証・修復
pdb-cli validate --fix -P

# 無効なIDリストを取得（パイプ用）
pdb-cli validate -o ids | pdb-cli download -l -
```

### update

ローカルファイルの更新確認・ダウンロード。

```bash
pdb-cli update [OPTIONS] [PDB_IDS]...

オプション:
  -c, --check               確認のみ、ダウンロードしない
  -n, --dry-run             更新対象を表示（ダウンロードしない）
  --verify                  チェックサムで検証（遅いが正確）
  --force                   最新でも強制更新
  -f, --format <FORMAT>     確認するフォーマット
  -m, --mirror <MIRROR>     確認先ミラー
  -P, --progress            プログレスバー表示
  -o, --output <FORMAT>     出力形式: text, json, csv, ids [デフォルト: text]
  -j, --parallel <NUM>      並列チェック数 [デフォルト: 10]
```

例:
```bash
# 全ファイルの更新確認
pdb-cli update --check -P

# 特定ファイルを更新
pdb-cli update 4hhb 1abc

# 更新対象をドライラン確認
pdb-cli update --dry-run

# チェックサム検証で強制更新
pdb-cli update --force --verify

# 古いIDリストを取得
pdb-cli update --check -o ids
```

### watch

新規PDBエントリを監視して自動ダウンロード。

```bash
pdb-cli watch [OPTIONS]

オプション:
  -i, --interval <INTERVAL> チェック間隔 (例: "1h", "30m") [デフォルト: 1h]
  --method <METHOD>         フィルタ: xray, nmr, em, neutron
  --resolution <NUM>        最大分解能 (Å)
  --organism <NAME>         生物種でフィルタ
  -t, --type <DATA_TYPE>    ダウンロードするデータタイプ
  -f, --format <FORMAT>     フォーマット [デフォルト: mmcif]
  -n, --dry-run             ダウンロードせずマッチを表示
  --notify <TYPE>           通知: desktop, email
  --email <ADDR>            通知用メールアドレス
  --on-new <SCRIPT>         新規エントリごとに実行するスクリプト
  -m, --mirror <MIRROR>     ダウンロード元ミラー
  --once                    1回実行して終了
  --since <DATE>            開始日 (YYYY-MM-DD)
```

例:
```bash
# 新規エントリを監視（継続実行）
pdb-cli watch

# 高分解能X線構造を1回確認
pdb-cli watch --once --method xray --resolution 2.0

# デスクトップ通知付きで監視
pdb-cli watch --notify desktop

# 新規エントリでカスタムスクリプト実行
pdb-cli watch --on-new ./process_new.sh

# 最近のエントリをドライラン確認
pdb-cli watch --once --dry-run --since 2024-01-01
```

### stats

ローカルPDBコレクションの統計情報表示。

```bash
pdb-cli stats [OPTIONS]

オプション:
  --detailed                サイズ分布、最古/最新ファイルを表示
  --compare-remote          リモートPDBアーカイブと比較
  -f, --format <FORMAT>     フォーマットでフィルタ
  -t, --type <DATA_TYPE>    データタイプでフィルタ
  -o, --output <FORMAT>     出力形式: text, json, csv [デフォルト: text]
```

例:
```bash
# 基本統計を表示
pdb-cli stats

# 詳細統計を表示
pdb-cli stats --detailed

# リモートアーカイブと比較
pdb-cli stats --compare-remote

# 特定フォーマットの統計
pdb-cli stats -f cif-gz

# JSONでエクスポート
pdb-cli stats -o json
```

### tree

ローカルPDBミラーのディレクトリツリー表示。

```bash
pdb-cli tree [OPTIONS]

オプション:
  -d, --depth <NUM>         表示最大深度
  -f, --format <FORMAT>     フォーマットでフィルタ
  -s, --size                ファイルサイズを表示
  -c, --count               ファイル数を表示
  --no-summary              サマリー行を非表示
  --non-empty               空でないディレクトリのみ表示
  --top <NUM>               上位Nディレクトリを表示
  --sort-by <FIELD>         ソート: count, size [デフォルト: count]
  -o, --output <FORMAT>     出力形式: text, json, csv [デフォルト: text]
```

例:
```bash
# フルツリーを表示
pdb-cli tree

# 深度制限
pdb-cli tree --depth 2

# サイズ上位10ディレクトリ
pdb-cli tree --top 10 --sort-by size

# カウントとサイズ付き
pdb-cli tree -c -s

# JSONでエクスポート
pdb-cli tree -o json
```

### convert

ファイルフォーマット変換（圧縮、解凍、形式変換）。

```bash
pdb-cli convert [OPTIONS] [FILES]...

オプション:
  --decompress              .gzファイルを解凍
  --compress                .gzに圧縮
  --to <FORMAT>             変換先フォーマット（gemmi必要）
  --from <FORMAT>           ソースフォーマットフィルタ
  -d, --dest <DIR>          出力先ディレクトリ
  --in-place                元ファイルを置換
  --stdin                   標準入力からパスを読み込み
  -p, --parallel <NUM>      並列変換数 [デフォルト: 4]
```

例:
```bash
# ファイルを解凍
pdb-cli convert --decompress *.cif.gz

# ファイルを圧縮
pdb-cli convert --compress *.cif

# mmCIFをPDBフォーマットに変換（gemmi必要）
pdb-cli convert --to pdb --from cif-gz -d ./pdb_files/

# インプレース解凍
pdb-cli convert --decompress --in-place ./data/*.gz

# 標準入力からバッチ変換
find ./data -name "*.cif.gz" | pdb-cli convert --stdin --decompress
```

### jobs

バックグラウンドジョブ管理。

```bash
pdb-cli jobs [OPTIONS] [COMMAND]

サブコマンド:
  status <ID>     ジョブステータス表示
  log <ID>        ジョブログ表示
  cancel <ID>     実行中ジョブをキャンセル
  clean           古いジョブディレクトリを削除

オプション:
  -a, --all                 全ジョブ（古いものも含む）を表示
  --running                 実行中のジョブのみ表示
```

例:
```bash
# 全ジョブ一覧
pdb-cli jobs

# 実行中のジョブのみ
pdb-cli jobs --running

# ジョブステータス確認
pdb-cli jobs status abc123

# ジョブログ表示
pdb-cli jobs log abc123

# 実行中ジョブをキャンセル
pdb-cli jobs cancel abc123

# 古いジョブを削除
pdb-cli jobs clean
```

### config

設定管理。

```bash
pdb-cli config init              # 設定ファイル初期化
pdb-cli config show              # 現在の設定を表示
pdb-cli config get <KEY>         # 設定値を取得
pdb-cli config set <KEY> <VALUE> # 設定値を設定
pdb-cli config test-mirrors      # ミラー遅延をテスト
```

### env

環境変数管理。

```bash
pdb-cli env show                 # 環境変数を表示
pdb-cli env export               # シェルコマンドとしてエクスポート
pdb-cli env set <NAME> <VALUE>   # setコマンドを出力
```

## 設定

設定ファイル: `~/.config/pdb-cli/config.toml`

```toml
[paths]
pdb_dir = "/data/pdb"

[sync]
mirror = "rcsb"
bwlimit = 0
delete = false
layout = "divided"
data_types = ["structures"]

[download]
default_format = "mmcif"
auto_decompress = true
parallel = 4
retry_count = 3

[mirror_selection]
auto_select = false
preferred_region = "us"
latency_cache_ttl = 3600
```

## 環境変数

| 変数 | 説明 |
|------|------|
| `PDB_DIR` | PDBファイルのベースディレクトリ |
| `PDB_CLI_CONFIG` | 設定ファイルのパス |
| `PDB_CLI_MIRROR` | デフォルトミラー |

## 対応ミラー

| ID | 地域 | 説明 |
|----|------|------|
| rcsb | 米国 | RCSB PDB |
| pdbj | 日本 | PDBj (日本蛋白質構造データバンク) |
| pdbe | 欧州 | PDBe (欧州蛋白質構造データバンク) |
| wwpdb | グローバル | wwPDB (世界蛋白質構造データバンク) |

## データタイプ

| タイプ | 説明 |
|--------|------|
| structures | 座標ファイル (mmCIF/PDB形式) |
| assemblies | 生物学的アセンブリ |
| biounit | レガシーbiounit形式 |
| structure-factors | X線回折データ |
| nmr-chemical-shifts | NMR化学シフト |
| nmr-restraints | NMR拘束 |
| obsolete | 廃止エントリ |

## 拡張PDB IDサポート

クラシック（4文字）と拡張PDB ID形式の両方に対応:

- クラシック: `1abc`, `4hhb`
- 拡張: `pdb_00001abc` (将来の拡張用12文字形式)

## エイリアス

よく使うコマンドとオプションに短いエイリアスが利用可能。

### コマンドエイリアス

| フルコマンド | エイリアス |
|--------------|-----------|
| `download` | `dl` |
| `validate` | `val` |
| `config` | `cfg` |

### オプション値エイリアス

#### データタイプ (`--type` / `-t`)

| フル名 | エイリアス |
|--------|-----------|
| `structures` | `st`, `struct` |
| `assemblies` | `asm`, `assembly` |
| `structure-factors` | `sf`, `xray` |
| `nmr-chemical-shifts` | `nmr-cs`, `cs` |
| `nmr-restraints` | `nmr-r` |

#### フォーマット (`--format` / `-f`)

| フル名 | エイリアス |
|--------|-----------|
| `mmcif` | `cif` |

#### ミラー (`--mirror` / `-m`)

| フル名 | エイリアス |
|--------|-----------|
| `rcsb` | `us` |
| `pdbj` | `jp` |
| `pdbe` | `uk`, `eu` |
| `wwpdb` | `global` |

### 使用例

```bash
# フル名
pdb-cli download 4hhb --type structures --format mmcif

# エイリアス使用
pdb-cli dl 4hhb -t st -f cif

# validateショートハンド
pdb-cli val --fix -P

# configショートハンド
pdb-cli cfg show
```

## パイプとスクリプト

`-o ids`出力でパイプ対応:

```bash
# 古いファイルを見つけて更新
pdb-cli update --check -o ids | pdb-cli download -l -

# 破損ファイルを検証して再ダウンロード
pdb-cli validate -o ids | pdb-cli download -l - --overwrite

# 存在しないエントリを見つけてダウンロード
cat wanted.txt | pdb-cli find --stdin --missing | pdb-cli download -l -
```

## ライセンス

MIT
