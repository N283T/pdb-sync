# Phase 8: Extended PDB ID Support

## 目標

wwPDBが導入予定の拡張PDB ID (8文字以上) のサポート

## 背景

現在のPDB IDは4文字 (例: 1abc) だが、エントリ数増加に伴い、wwPDBは拡張形式を導入予定:
- Classic: `1abc` (4文字: 数字 + 英数字3文字)
- Extended: `pdb_00001abc` (12文字: "pdb_" + 8桁)

## 依存

なし (独立して実装可能)

## 実装内容

### 1. PdbId 更新: `src/files/pdb_id.rs`

```rust
use crate::error::{PdbCliError, Result};
use regex::Regex;
use std::sync::LazyLock;

// Classic 4-char PDB ID: 1abc
static CLASSIC_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[0-9][a-zA-Z0-9]{3}$").unwrap());

// Extended PDB ID: pdb_00001abc
static EXTENDED_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^pdb_[0-9a-zA-Z]{8}$").unwrap());

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PdbId {
    /// Classic 4-character PDB ID (e.g., 1abc)
    Classic(String),
    /// Extended PDB ID (e.g., pdb_00001abc)
    Extended(String),
}

impl PdbId {
    pub fn new(id: &str) -> Result<Self> {
        let normalized = id.trim().to_lowercase();

        if CLASSIC_REGEX.is_match(&normalized) {
            Ok(PdbId::Classic(normalized))
        } else if EXTENDED_REGEX.is_match(&normalized) {
            Ok(PdbId::Extended(normalized))
        } else {
            Err(PdbCliError::InvalidPdbId(format!(
                "Invalid PDB ID '{}': must be 4 characters (e.g., 1abc) or extended format (e.g., pdb_00001abc)",
                id
            )))
        }
    }

    /// Get the raw ID string
    pub fn as_str(&self) -> &str {
        match self {
            PdbId::Classic(s) => s,
            PdbId::Extended(s) => s,
        }
    }

    /// Get the 2-character hash for directory partitioning
    /// Classic: characters 1-2 (e.g., 1abc -> ab)
    /// Extended: characters 6-7 (e.g., pdb_00001abc -> 01)
    pub fn middle_chars(&self) -> &str {
        match self {
            PdbId::Classic(s) => &s[1..3],
            PdbId::Extended(s) => &s[6..8], // pdb_00 01 abc
        }
    }

    /// Check if this is a classic (4-char) PDB ID
    pub fn is_classic(&self) -> bool {
        matches!(self, PdbId::Classic(_))
    }

    /// Check if this is an extended PDB ID
    pub fn is_extended(&self) -> bool {
        matches!(self, PdbId::Extended(_))
    }

    /// Get the short form for display
    /// Classic: as-is (1abc)
    /// Extended: without prefix (00001abc)
    pub fn short_form(&self) -> &str {
        match self {
            PdbId::Classic(s) => s,
            PdbId::Extended(s) => &s[4..], // Skip "pdb_"
        }
    }
}

impl std::fmt::Display for PdbId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for PdbId {
    type Err = PdbCliError;

    fn from_str(s: &str) -> Result<Self> {
        PdbId::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classic_pdb_id() {
        let id = PdbId::new("1abc").unwrap();
        assert!(id.is_classic());
        assert_eq!(id.as_str(), "1abc");
        assert_eq!(id.middle_chars(), "ab");
    }

    #[test]
    fn test_extended_pdb_id() {
        let id = PdbId::new("pdb_00001abc").unwrap();
        assert!(id.is_extended());
        assert_eq!(id.as_str(), "pdb_00001abc");
        assert_eq!(id.middle_chars(), "01");
        assert_eq!(id.short_form(), "00001abc");
    }

    #[test]
    fn test_case_insensitive() {
        let id1 = PdbId::new("1ABC").unwrap();
        let id2 = PdbId::new("1abc").unwrap();
        assert_eq!(id1, id2);

        let id3 = PdbId::new("PDB_00001ABC").unwrap();
        let id4 = PdbId::new("pdb_00001abc").unwrap();
        assert_eq!(id3, id4);
    }

    #[test]
    fn test_invalid_ids() {
        assert!(PdbId::new("abc").is_err());      // Too short
        assert!(PdbId::new("12345").is_err());    // Wrong format
        assert!(PdbId::new("pdb_123").is_err());  // Too short extended
        assert!(PdbId::new("pdb_123456789").is_err()); // Too long extended
    }
}
```

### 2. paths.rs 更新

```rust
use crate::files::PdbId;

/// Build filename for a PDB ID
pub fn build_filename(pdb_id: &PdbId, format: FileFormat) -> String {
    match format {
        FileFormat::Pdb | FileFormat::PdbGz => {
            // Classic: pdb1abc.ent.gz
            // Extended: pdb_00001abc.ent.gz (use full ID)
            match pdb_id {
                PdbId::Classic(id) => format!("pdb{}.ent.gz", id),
                PdbId::Extended(id) => format!("{}.ent.gz", id),
            }
        }
        FileFormat::Mmcif | FileFormat::CifGz => {
            // Both: {id}.cif.gz
            format!("{}.cif.gz", pdb_id.as_str())
        }
        FileFormat::Bcif | FileFormat::BcifGz => {
            format!("{}.bcif.gz", pdb_id.as_str())
        }
    }
}

/// Build relative path for a PDB file
pub fn build_relative_path(pdb_id: &PdbId, format: FileFormat) -> PathBuf {
    let middle = pdb_id.middle_chars();
    let filename = build_filename(pdb_id, format);

    PathBuf::from(format!("{}/{}/{}", format.subdir(), middle, filename))
}
```

### 3. URL構築更新: `src/download/https.rs`

```rust
fn build_structure_url(&self, mirror: &Mirror, pdb_id: &PdbId, format: FileFormat) -> String {
    match self.options.mirror {
        MirrorId::Rcsb => {
            // RCSB accepts both classic and extended IDs directly
            match format.base_format() {
                FileFormat::Pdb => format!("{}/{}.pdb", mirror.https_base, pdb_id.as_str()),
                FileFormat::Mmcif => format!("{}/{}.cif", mirror.https_base, pdb_id.as_str()),
                FileFormat::Bcif => format!("https://models.rcsb.org/{}.bcif", pdb_id.as_str()),
                _ => unreachable!(),
            }
        }
        MirrorId::Wwpdb => {
            let middle = pdb_id.middle_chars();
            let id = pdb_id.as_str();
            match format.base_format() {
                FileFormat::Pdb => format!(
                    "{}/divided/pdb/{}/pdb{}.ent.gz",
                    mirror.https_base, middle, id
                ),
                FileFormat::Mmcif => format!(
                    "{}/divided/mmCIF/{}/{}.cif.gz",
                    mirror.https_base, middle, id
                ),
                _ => unreachable!(),
            }
        }
        // ... other mirrors
    }
}
```

## 使用例

```bash
# Classic ID
pdb-cli download 1abc

# Extended ID
pdb-cli download pdb_00001abc

# Mixed
pdb-cli download 1abc pdb_00001abc 2xyz

# List (auto-detect both formats)
pdb-cli list
```

## 完了条件

- [ ] PdbId enum (Classic, Extended)
- [ ] middle_chars() for both formats
- [ ] 大文字小文字正規化
- [ ] パス構築対応
- [ ] URL構築対応
- [ ] ユニットテスト
- [ ] cargo build 成功
- [ ] cargo test 成功

## 注意事項

- 現時点で拡張IDは未導入のため、実際のテストは限定的
- wwPDBの正式仕様が確定次第、調整が必要な可能性あり

## 工数

0.5日
