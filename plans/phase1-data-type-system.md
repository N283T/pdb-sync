# Phase 1: Data Type System

## 目標

wwPDB標準に準拠した包括的なデータタイプシステムの実装

## 背景

現状の実装は `structures/divided/` のみ対応。実際のPDBアーカイブには以下が存在:
- structures (mmCIF, PDB)
- assemblies (生物学的アセンブリ)
- biounit (レガシー形式)
- structure_factors (構造因子)
- nmr_* (NMRデータ)
- obsolete (廃止エントリ)

## 実装内容

### 1. 新規ファイル: `src/data_types.rs`

```rust
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// PDBアーカイブのデータタイプ
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
pub enum DataType {
    /// 座標ファイル (structures/divided/mmCIF or pdb)
    Structures,
    /// 生物学的アセンブリ (assemblies/mmCIF/divided)
    Assemblies,
    /// レガシーbiounit (biounit/coordinates/divided)
    Biounit,
    /// 構造因子 (structures/divided/structure_factors)
    StructureFactors,
    /// NMR化学シフト
    NmrChemicalShifts,
    /// NMR拘束条件
    NmrRestraints,
    /// 廃止エントリ
    Obsolete,
}

/// ディレクトリレイアウト
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum, Serialize, Deserialize)]
pub enum Layout {
    /// ハッシュディレクトリ構造 (mmCIF/{hash}/{id}.cif.gz)
    #[default]
    Divided,
    /// フラット構造 (mmCIF/{id}.cif.gz)
    All,
}

impl DataType {
    /// rsync サブパスを取得
    pub fn rsync_subpath(&self, layout: Layout) -> &'static str {
        match (self, layout) {
            (DataType::Structures, Layout::Divided) => "structures/divided",
            (DataType::Structures, Layout::All) => "structures/all",
            (DataType::Assemblies, Layout::Divided) => "assemblies/mmCIF/divided",
            (DataType::Assemblies, Layout::All) => "assemblies/mmCIF/all",
            (DataType::Biounit, Layout::Divided) => "biounit/coordinates/divided",
            (DataType::Biounit, Layout::All) => "biounit/coordinates/all",
            (DataType::StructureFactors, Layout::Divided) => "structures/divided/structure_factors",
            (DataType::StructureFactors, Layout::All) => "structures/all/structure_factors",
            (DataType::NmrChemicalShifts, Layout::Divided) => "structures/divided/nmr_chemical_shifts",
            (DataType::NmrChemicalShifts, Layout::All) => "structures/all/nmr_chemical_shifts",
            (DataType::NmrRestraints, Layout::Divided) => "structures/divided/nmr_restraints",
            (DataType::NmrRestraints, Layout::All) => "structures/all/nmr_restraints",
            (DataType::Obsolete, Layout::Divided) => "structures/obsolete",
            (DataType::Obsolete, Layout::All) => "structures/obsolete",
        }
    }

    /// ファイル名パターンを取得
    pub fn filename_pattern(&self, pdb_id: &str) -> String {
        match self {
            DataType::Structures => format!("{}.cif.gz", pdb_id),
            DataType::Assemblies => format!("{}-assembly*.cif.gz", pdb_id),
            DataType::Biounit => format!("{}.pdb*.gz", pdb_id),
            DataType::StructureFactors => format!("r{}sf.ent.gz", pdb_id),
            DataType::NmrChemicalShifts => format!("{}_cs.str.gz", pdb_id),
            DataType::NmrRestraints => format!("{}_mr.str.gz", pdb_id),
            DataType::Obsolete => format!("{}.cif.gz", pdb_id),
        }
    }
}

impl std::fmt::Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataType::Structures => write!(f, "structures"),
            DataType::Assemblies => write!(f, "assemblies"),
            DataType::Biounit => write!(f, "biounit"),
            DataType::StructureFactors => write!(f, "structure-factors"),
            DataType::NmrChemicalShifts => write!(f, "nmr-chemical-shifts"),
            DataType::NmrRestraints => write!(f, "nmr-restraints"),
            DataType::Obsolete => write!(f, "obsolete"),
        }
    }
}

impl std::fmt::Display for Layout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Layout::Divided => write!(f, "divided"),
            Layout::All => write!(f, "all"),
        }
    }
}
```

### 2. `src/files/paths.rs` 更新

- `build_relative_path` に DataType, Layout パラメータ追加
- 各データタイプ用のパスビルダー追加

### 3. `src/main.rs` 更新

- `mod data_types;` 追加
- `pub use data_types::{DataType, Layout};` 追加

### 4. `src/lib.rs` または `src/files/mod.rs` 更新

- DataType, Layout のエクスポート

## テスト

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rsync_subpath() {
        assert_eq!(
            DataType::Structures.rsync_subpath(Layout::Divided),
            "structures/divided"
        );
        assert_eq!(
            DataType::Assemblies.rsync_subpath(Layout::All),
            "assemblies/mmCIF/all"
        );
    }

    #[test]
    fn test_filename_pattern() {
        assert_eq!(
            DataType::StructureFactors.filename_pattern("1abc"),
            "r1abcsf.ent.gz"
        );
    }
}
```

## 完了条件

- [ ] `src/data_types.rs` 作成
- [ ] DataType enum (7種類)
- [ ] Layout enum (2種類)
- [ ] rsync_subpath() メソッド
- [ ] filename_pattern() メソッド
- [ ] Display trait 実装
- [ ] ユニットテスト
- [ ] cargo build 成功
- [ ] cargo test 成功

## 工数

1-2日
