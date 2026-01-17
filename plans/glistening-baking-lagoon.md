# 並行開発用プロンプトテンプレート

## 概要

v3フェーズを並行でworktree開発する際の汎用プロンプト。ブランチ名を指定するだけで対応するフェーズのプランに基づいて実装を行う。

---

## 汎用プロンプトテンプレート

```
ブランチ `feature/{BRANCH_NAME}` の機能を実装してください。

## 指示
1. `plans/{PLAN_FILE}` を読んで実装内容を把握
2. TDDで実装（テスト先行）
3. 実装後は `ruff format .` と `ruff check --fix .` と `ty check` を実行
4. テストを実行して全てパスすることを確認
5. 完了後、PRを作成してレビュー依頼

## 並行開発ルール
- masterから分岐して作業
- 他フェーズとの依存がある場合は、そのフェーズがマージされるまで待機
- PRはレビュー通過後にマージ
- 他フェーズが先にマージされた場合は `git rebase master` してから作業続行
```

---

## ブランチ名 → プランファイル マッピング

| ブランチ名 | プランファイル | 概要 |
|-----------|---------------|------|
| `feature/v3-sync-restructure` | `v3-phase1-sync-restructure.md` | sync コマンドのサブコマンド化 |
| `feature/v3-alias` | `v3-phase2-alias.md` | コマンド/オプションの短縮エイリアス |
| `feature/v3-update` | `v3-phase4-update.md` | update コマンド（ローカルファイル更新チェック） |
| `feature/v3-background` | `v3-phase5-background.md` | バックグラウンド実行 (--bg) とジョブ管理 |
| `feature/v3-aria2c` | `v3-phase6-aria2c.md` | aria2c ダウンロードエンジン対応 |
| `feature/v3-batch` | `v3-phase7-batch.md` | stdin/stdout パイプライン対応 |
| `feature/v3-stats` | `v3-phase8-stats.md` | stats コマンド（コレクション統計） |
| `feature/v3-watch` | `v3-phase9-watch.md` | watch コマンド（新エントリ監視） |
| `feature/v3-storage` | `v3-phase10-storage.md` | storage コマンド（圧縮/解凍管理） |
| `feature/v3-find` | `v3-phase11-find.md` | find コマンド（ローカルファイル検索） |
| `feature/v3-tree` | `v3-phase12-tree.md` | tree コマンド（ディレクトリ構造表示） |

---

## 依存関係

```
独立（並行可能）:
├── v3-alias         (単独で実装可能)
├── v3-update        (単独で実装可能)
├── v3-stats         (単独で実装可能)
├── v3-find          (単独で実装可能)
├── v3-tree          (単独で実装可能)
├── v3-storage       (単独で実装可能)
└── v3-batch         (単独で実装可能)

軽い依存:
├── v3-sync-restructure → v3-batch（sync でも --stdin 使いたい場合）
├── v3-background → v3-sync-restructure（sync --bg で使う）
├── v3-background → v3-update（update --bg で使う）
├── v3-aria2c → v3-batch（download --stdin との連携）
└── v3-watch → v3-stats（履歴機能を共有可能）

推奨実装順:
1. 独立フェーズを先に
2. v3-background は他がある程度揃ってから
```

---

## 使用例

### 例1: 単一フェーズ開始

```bash
# Worktree作成
git worktree add ../pdb-cli-v3-alias -b feature/v3-alias

# そのディレクトリでClaudeに指示
cd ../pdb-cli-v3-alias
claude "ブランチ feature/v3-alias の機能を実装してください。plans/v3-phase2-alias.md を参照。"
```

### 例2: 複数フェーズ並行（短縮版）

```bash
# 複数のworktreeを作成
git worktree add ../pdb-cli-v3-stats -b feature/v3-stats
git worktree add ../pdb-cli-v3-find -b feature/v3-find
git worktree add ../pdb-cli-v3-tree -b feature/v3-tree

# 各ディレクトリで並行作業
# Terminal 1:
cd ../pdb-cli-v3-stats && claude "feature/v3-stats を実装"

# Terminal 2:
cd ../pdb-cli-v3-find && claude "feature/v3-find を実装"

# Terminal 3:
cd ../pdb-cli-v3-tree && claude "feature/v3-tree を実装"
```

### 例3: 最小プロンプト

ブランチが `feature/v3-*` 形式であれば、以下の短いプロンプトで動作:

```
このブランチの機能を実装して。plans/ のプランファイルを参照。
```

---

## 各フェーズの1行サマリ

| フェーズ | 何をするか |
|---------|-----------|
| sync-restructure | `sync wwpdb/pdbj/pdbe` サブコマンド追加 |
| alias | `dl`, `val`, `cfg` などの短縮名追加 |
| update | ローカルファイルの更新チェック/ダウンロード |
| background | `--bg` でバックグラウンド実行、`jobs` コマンド追加 |
| aria2c | aria2c をダウンロードエンジンとして使用可能に |
| batch | `--stdin` と `-o ids` でパイプライン連携 |
| stats | ローカルコレクションの統計情報表示 |
| watch | 新しいPDBエントリを監視して自動ダウンロード |
| storage | ローカルファイルの圧縮/解凍/重複削除 |
| find | ローカルファイルのパス検索、存在チェック |
| tree | ディレクトリ構造のツリー表示 |

---

## Claudeが参照すべきファイル

各フェーズで実装時に参照するファイル:

```
plans/v3-phase{N}-{name}.md  # 詳細な実装計画
src/                          # 既存コード構造
Cargo.toml                    # 依存関係
tests/                        # 既存テスト構造
```

---

## PR作成時のテンプレート

```markdown
## Summary
- {フェーズ名}を実装
- {主な変更点1}
- {主な変更点2}

## Test plan
- [ ] `cargo test` 全パス
- [ ] `ruff format .` && `ruff check --fix .` 問題なし
- [ ] 手動テスト: {コマンド例}

## Related
- Plan: plans/v3-phase{N}-{name}.md
```
