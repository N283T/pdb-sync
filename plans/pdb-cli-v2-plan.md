# PDB-CLI v2 Implementation Plan

## Overview

調査結果に基づく pdb-cli の機能拡張計画。

## Current State (現状)

- CLI基本構造: sync, download, copy, config, env コマンド
- ミラーレジストリ: RCSB, PDBj, PDBe, wwPDB
- HTTPSダウンロード: 動作中
- rsync同期: 基本機能のみ
- copyコマンド: PDB ID ベースに再設計済み

## Archive Statistics (アーカイブ統計)

- 総エントリ数: ~248,000 structures
- 総ファイル数: ~3,000,000+
- 総サイズ: ~1 TB
- mmCIF推定: ~75-80 GB (divided layout)

---

## Phase 1: Data Type System (データタイプシステム)

**目標**: wwPDB標準に準拠した包括的なデータタイプシステム

### 新規 Enum

```rust
/// PDBアーカイブのデータタイプ
pub enum DataType {
    Structures,        // structures/divided/mmCIF or pdb
    Assemblies,        // assemblies/mmCIF/divided
    Biounit,           // biounit/coordinates/divided (legacy)
    StructureFactors,  // structures/divided/structure_factors
    NmrChemicalShifts, // nmr_chemical_shifts
    NmrRestraints,     // nmr_restraints
    Obsolete,          // obsolete entries
}

/// ディレクトリレイアウト
pub enum Layout {
    Divided,  // mmCIF/{hash}/{id}.cif.gz
    All,      // mmCIF/{id}.cif.gz (flat)
}
```

### ファイル変更
- `src/data_types.rs` (新規)
- `src/files/paths.rs` - パスビルダー追加
- `src/mirrors/registry.rs` - rsyncパス追加

**工数**: 中 (1-2日)

---

## Phase 2: Sync Command Enhancement (同期コマンド強化)

**目標**: レイアウト選択、データタイプ選択、プログレス表示

### 新規CLIオプション

```bash
pdb-cli sync [OPTIONS]
  -t, --data-types <TYPE>...  データタイプ (structures, assemblies, etc.)
  -l, --layout <LAYOUT>       レイアウト (divided, all)
  --incremental               増分同期
  --progress                  プログレス表示
```

### 新規ファイル
- `src/sync/progress.rs` - rsync出力のパース
- `src/sync/state.rs` - 同期状態追跡

### 使用例

```bash
# structures と assemblies を divided で同期
pdb-cli sync -t structures,assemblies -l divided

# structure factors のみ同期
pdb-cli sync -t structure-factors

# 増分同期
pdb-cli sync --incremental
```

**工数**: 大 (3-5日)

---

## Phase 3: Download Command Enhancement (ダウンロードコマンド強化)

**目標**: 並列ダウンロード、リトライ、アセンブリ/構造因子対応

### 新規機能
- 並列ダウンロード (Semaphore制御)
- リトライロジック
- アセンブリダウンロード
- 構造因子ダウンロード

### 新規CLIオプション

```bash
pdb-cli download [OPTIONS] <PDB_ID>...
  -t, --data-type <TYPE>   データタイプ
  -a, --assembly <N>       アセンブリ番号 (0=全て)
  -p, --parallel <N>       並列数 (default: 4)
  --retry <N>              リトライ回数 (default: 3)
```

### 新規ファイル
- `src/download/queue.rs` - ダウンロードキュー管理

### 使用例

```bash
# アセンブリをダウンロード
pdb-cli download 4hhb -t assemblies -a 0

# 構造因子をダウンロード
pdb-cli download 1abc -t structure-factors

# 並列ダウンロード
pdb-cli download 1abc 2xyz 3def -p 8 --retry 5
```

**工数**: 大 (3-4日)

---

## Phase 4: New `list` Command (listコマンド)

**目標**: ローカルファイル一覧、パターン検索、統計表示

### CLI定義

```bash
pdb-cli list [OPTIONS] [PATTERN]
  -t, --data-type <TYPE>   データタイプ
  -f, --format <FORMAT>    フォーマット
  -s, --size               サイズ表示
  -T, --time               更新日時表示
  --stats                  統計のみ表示
  --output <FORMAT>        出力形式 (text, json, csv)
```

### 使用例

```bash
# 全mmCIFファイル一覧
pdb-cli list --format cif-gz

# パターン検索
pdb-cli list "1ab*" --size

# 統計表示
pdb-cli list --stats
```

**工数**: 中 (1-2日)

---

## Phase 5: New `info` Command (infoコマンド)

**目標**: PDBエントリのメタデータ表示

### CLI定義

```bash
pdb-cli info <PDB_ID>
  --local           ローカル情報のみ
  --output <FMT>    出力形式
```

### RCSB API統合
- `https://data.rcsb.org/rest/v1/core/entry/{pdb_id}`

### 新規ファイル
- `src/api/rcsb.rs` - RCSB Data API クライアント
- `src/cli/commands/info.rs`

**工数**: 中 (2-3日)

---

## Phase 6: New `validate` Command (validateコマンド)

**目標**: チェックサム検証、破損ファイル検出

### CLI定義

```bash
pdb-cli validate [OPTIONS] [PDB_ID]...
  -t, --data-type <TYPE>   データタイプ
  --fix                    破損ファイルを再ダウンロード
  --progress               プログレス表示
```

### 新規ファイル
- `src/validation/checksum.rs` - MD5検証
- `src/cli/commands/validate.rs`

**工数**: 中 (2-3日)

---

## Phase 7: Configuration Improvements (設定改善)

**目標**: データタイプ別パス、レイアウト設定、自動ミラー選択

### 設定例

```toml
[paths]
pdb_dir = "/data/pdb"
structures_dir = "/data/pdb/structures"
assemblies_dir = "/data/pdb/assemblies"

[sync]
mirror = "pdbj"
layout = "divided"
data_types = ["structures", "assemblies"]

[download]
parallel = 8
retry_count = 3

[mirror_selection]
auto_select = true
preferred_region = "jp"
```

### 新規ファイル
- `src/mirrors/auto_select.rs` - 自動ミラー選択

**工数**: 中 (2日)

---

## Phase 8: Extended PDB ID Support (拡張PDB ID)

**目標**: 8文字拡張PDB IDのサポート

```
Classic:  1abc (4文字)
Extended: pdb_00001abc (12文字)
```

**工数**: 小 (0.5日)

---

## Implementation Priority (実装優先度)

| Phase | 優先度 | 工数 | 依存 |
|-------|--------|------|------|
| 1. Data Type System | 高 | 中 | なし |
| 2. Sync Enhancement | 高 | 大 | Phase 1 |
| 3. Download Enhancement | 高 | 大 | Phase 1 |
| 4. list Command | 中 | 中 | Phase 1 |
| 5. info Command | 中 | 中 | なし |
| 6. validate Command | 中 | 中 | Phase 1 |
| 7. Config Improvements | 中 | 中 | なし |
| 8. Extended PDB ID | 低 | 小 | なし |

**推奨順序**: 1 → 2 → 3 → 7 → 4 → 6 → 5 → 8

---

## New Dependencies

```toml
[dependencies]
md-5 = "0.10"                                    # Checksum
chrono = { version = "0.4", features = ["serde"] }  # Timestamps
glob = "0.3"                                     # Pattern matching
serde_json = "1.0"                               # JSON output
```

---

## Module Structure (完成後)

```
src/
├── main.rs
├── cli/
│   ├── args.rs              # CLI定義 (拡張)
│   └── commands/
│       ├── sync.rs          # 同期 (拡張)
│       ├── download.rs      # ダウンロード (拡張)
│       ├── copy.rs
│       ├── config.rs
│       ├── env.rs
│       ├── list.rs          # 新規
│       ├── info.rs          # 新規
│       └── validate.rs      # 新規
├── config/
│   └── schema.rs            # 設定 (拡張)
├── mirrors/
│   ├── registry.rs          # ミラー (拡張)
│   └── auto_select.rs       # 新規
├── sync/
│   ├── rsync.rs             # rsync (拡張)
│   ├── progress.rs          # 新規
│   └── state.rs             # 新規
├── download/
│   ├── https.rs             # ダウンロード (拡張)
│   └── queue.rs             # 新規
├── files/
│   ├── pdb_id.rs            # PDB ID (拡張)
│   └── paths.rs             # パス (拡張)
├── data_types.rs            # 新規
├── api/
│   └── rcsb.rs              # 新規
├── validation/
│   └── checksum.rs          # 新規
├── context.rs
└── error.rs
```

---

## 総工数見積もり

- **最小 (Phase 1-3)**: 7-11日
- **フル実装 (全Phase)**: 15-20日
