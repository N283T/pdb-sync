# Phase 4: List Command Implementation Plan

## Overview

ローカルミラーのPDBファイル一覧表示、パターン検索、統計表示機能を実装する。

## Implementation Steps

### Step 1: Add Dependencies (Cargo.toml)

```toml
[dependencies]
glob = "0.3"
chrono = { version = "0.4", features = ["serde"] }
serde_json = "1.0"  # dev-dependencies から移動
```

### Step 2: CLI Arguments (src/cli/args.rs)

1. インポート追加: `use pdb_cli::data_types::DataType;`
2. 新規enum定義:
   - `OutputFormat` (Text, Json, Csv)
   - `SortField` (Name, Size, Time)
3. `ListArgs` 構造体定義 (pattern, data_type, format, size, time, output, stats, sort, reverse)
4. `Commands` enum に `List(ListArgs)` variant 追加

### Step 3: Command Handler (src/cli/commands/list.rs) - 新規作成

主要コンポーネント:
- `LocalFile` 構造体 (pdb_id, path, size, modified, data_type, format)
- `run_list()` - メインエントリポイント
- `scan_local_files()` - ディレクトリスキャン (hash-divided構造対応)
- `extract_pdb_id()` - ファイル名からPDB ID抽出
- `sort_files()` - ソート処理
- 出力関数群 (text/json/csv)
- `print_statistics()` - 統計表示
- `human_bytes()` - バイト数のフォーマット

### Step 4: Module Export (src/cli/commands/mod.rs)

```rust
pub mod list;
pub use list::run_list;
```

### Step 5: Main Dispatch (src/main.rs)

```rust
Commands::List(args) => {
    cli::commands::run_list(args, ctx).await?;
}
```

## Files to Modify/Create

| File | Action |
|------|--------|
| `Cargo.toml` | 依存追加 (glob, chrono, serde_json移動) |
| `src/cli/args.rs` | ListArgs, OutputFormat, SortField定義 |
| `src/cli/commands/list.rs` | 新規作成 |
| `src/cli/commands/mod.rs` | モジュール追加 |
| `src/main.rs` | dispatch arm追加 |

## Test Strategy

### Unit Tests (list.rs内)
- `test_extract_pdb_id_cifgz` - mmCIF形式のID抽出
- `test_extract_pdb_id_pdbgz` - PDB形式のID抽出
- `test_human_bytes` - バイト数フォーマット
- `test_sort_files_by_name/size` - ソート機能

### Integration Tests
```bash
pdb-cli list
pdb-cli list "1ab*"
pdb-cli list --size --time
pdb-cli list --stats
pdb-cli list --output json
pdb-cli list --sort size --reverse
```

## Verification

1. `cargo build` - コンパイル成功
2. `cargo test` - テスト成功
3. `cargo clippy` - lint通過
4. `cargo fmt --check` - フォーマット確認
5. `pdb-cli list --help` - ヘルプ出力確認

## Usage Examples (完成後)

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

# サイズ順ソート
pdb-cli list --sort size --reverse
```

## PR Review Cycle

1. 実装完了後 `cargo build && cargo test`
2. `ruff format . && ruff check --fix .` (Python部分があれば)
3. コードレビュー (code-reviewer agent)
4. CI確認 `gh pr checks`
5. マージ
