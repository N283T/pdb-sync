//! Built-in sync profile presets for common PDB data sources.
//!
//! This module provides pre-configured rsync profiles for common data sources
//! like PDB structures, assemblies, EMDB, and SIFTS. Users can quickly add
//! these presets to their configuration without manually specifying URLs and options.

use crate::config::schema::CustomRsyncConfig;

/// Built-in sync profile preset.
#[derive(Debug, Clone)]
pub struct SyncPreset {
    /// Unique identifier for this preset
    pub id: &'static str,
    /// Human-readable name
    pub name: &'static str,
    /// Description of what this preset syncs
    pub description: &'static str,
    /// rsync URL for this preset
    pub url: &'static str,
    /// Destination path relative to pdb_dir
    pub dest: &'static str,
    /// Whether to delete files that don't exist on remote
    pub delete: bool,
    /// Whether to compress data during transfer
    pub compress: bool,
    /// Whether to use checksum for file comparison
    pub checksum: bool,
}

impl SyncPreset {
    /// Get all available presets.
    pub fn all() -> &'static [SyncPreset] {
        PRESETS
    }

    /// Find a preset by ID.
    pub fn find(id: &str) -> Option<&'static SyncPreset> {
        PRESETS.iter().find(|p| p.id == id)
    }

    /// Convert this preset to a CustomRsyncConfig for saving to config.
    pub fn to_custom_config(&self) -> CustomRsyncConfig {
        CustomRsyncConfig {
            name: self.id.to_string(),
            url: self.url.to_string(),
            dest: self.dest.to_string(),
            description: Some(self.description.to_string()),
            rsync_delete: self.delete,
            rsync_compress: self.compress,
            rsync_checksum: self.checksum,
            ..Default::default()
        }
    }
}

/// Built-in sync profile presets.
///
/// These are curated, versioned presets for common PDB data sources.
const PRESETS: &[SyncPreset] = &[
    SyncPreset {
        id: "structures",
        name: "PDB Structures",
        description: "Coordinate files (mmCIF format) from wwPDB",
        url: "rsync://rsync.wwpdb.org/ftp_data/structures/divided/mmCIF/",
        dest: "wwpdb/structures/mmCIF",
        delete: false,
        compress: true,
        checksum: false,
    },
    SyncPreset {
        id: "assemblies",
        name: "PDB Assemblies",
        description: "Biological assembly files from wwPDB",
        url: "rsync://rsync.wwpdb.org/ftp_data/assemblies/mmCIF/divided/",
        dest: "wwpdb/assemblies/mmCIF",
        delete: false,
        compress: true,
        checksum: false,
    },
    SyncPreset {
        id: "emdb",
        name: "EMDB Entries",
        description: "Electron Microscopy Data Bank entries from PDBj",
        url: "data.pdbj.org::rsync/pub/emdb/",
        dest: "pub/emdb",
        delete: false,
        compress: true,
        checksum: false,
    },
    SyncPreset {
        id: "sifts",
        name: "SIFTS Mapping",
        description: "Structure-to-function mappings from PDBe",
        url: "rsync://rsync.ebi.ac.uk/pub/databases/msd/sifts/",
        dest: "pdbe/sifts",
        delete: false,
        compress: true,
        checksum: false,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_find_valid() {
        assert!(SyncPreset::find("structures").is_some());
        assert!(SyncPreset::find("assemblies").is_some());
        assert!(SyncPreset::find("emdb").is_some());
        assert!(SyncPreset::find("sifts").is_some());
    }

    #[test]
    fn test_preset_find_invalid() {
        assert!(SyncPreset::find("invalid").is_none());
        assert!(SyncPreset::find("").is_none());
        assert!(SyncPreset::find("STRUCTURES").is_none()); // case-sensitive
    }

    #[test]
    fn test_all_presets_count() {
        assert_eq!(SyncPreset::all().len(), 4);
    }

    #[test]
    fn test_preset_to_custom_config() {
        let preset = SyncPreset::find("structures").unwrap();
        let config = preset.to_custom_config();

        assert_eq!(config.name, "structures");
        assert_eq!(
            config.url,
            "rsync://rsync.wwpdb.org/ftp_data/structures/divided/mmCIF/"
        );
        assert_eq!(config.dest, "wwpdb/structures/mmCIF");
        assert_eq!(
            config.description,
            Some("Coordinate files (mmCIF format) from wwPDB".to_string())
        );
        assert_eq!(config.rsync_delete, false);
        assert_eq!(config.rsync_compress, true);
        assert_eq!(config.rsync_checksum, false);
    }

    #[test]
    fn test_all_presets_have_valid_ids() {
        for preset in SyncPreset::all() {
            assert!(!preset.id.is_empty(), "Preset ID is empty");
            assert!(
                !preset.id.chars().any(|c| c.is_whitespace()),
                "Preset ID contains whitespace: {}",
                preset.id
            );
            assert!(
                preset.id.is_ascii(),
                "Preset ID is not ASCII: {}",
                preset.id
            );
        }
    }

    #[test]
    fn test_all_presets_have_valid_names() {
        for preset in SyncPreset::all() {
            assert!(!preset.name.is_empty(), "Preset name is empty");
        }
    }

    #[test]
    fn test_all_presets_have_valid_descriptions() {
        for preset in SyncPreset::all() {
            assert!(
                !preset.description.is_empty(),
                "Preset description is empty"
            );
        }
    }

    #[test]
    fn test_all_presets_have_valid_urls() {
        for preset in SyncPreset::all() {
            assert!(!preset.url.is_empty(), "Preset URL is empty");
            // Check for valid rsync URL patterns
            let is_standard_rsync = preset.url.contains("::");
            let is_url_rsync = preset.url.starts_with("rsync://");
            assert!(
                is_standard_rsync || is_url_rsync,
                "Preset URL has invalid format: {}",
                preset.url
            );
            // Check for dangerous characters
            let dangerous_chars = [';', '&', '|', '`', '$', '\n', '\r', '\t'];
            for ch in dangerous_chars {
                assert!(
                    !preset.url.contains(ch),
                    "Preset URL contains dangerous character '{}': {}",
                    ch,
                    preset.url
                );
            }
        }
    }

    #[test]
    fn test_all_presets_have_valid_dest() {
        for preset in SyncPreset::all() {
            assert!(!preset.dest.is_empty(), "Preset dest is empty");
            // Check for path traversal attempts
            assert!(
                !preset.dest.contains(".."),
                "Preset dest contains path traversal: {}",
                preset.dest
            );
            // Check for dangerous characters
            let dangerous_chars = [';', '&', '|', '`', '$', '\n', '\r', '\t'];
            for ch in dangerous_chars {
                assert!(
                    !preset.dest.contains(ch),
                    "Preset dest contains dangerous character '{}': {}",
                    ch,
                    preset.dest
                );
            }
        }
    }

    #[test]
    fn test_all_presets_have_unique_ids() {
        let ids: Vec<&str> = SyncPreset::all().iter().map(|p| p.id).collect();
        let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(ids.len(), unique_ids.len(), "Preset IDs are not unique");
    }

    #[test]
    fn test_all_presets_have_unique_names() {
        let names: Vec<&str> = SyncPreset::all().iter().map(|p| p.name).collect();
        let unique_names: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(
            names.len(),
            unique_names.len(),
            "Preset names are not unique"
        );
    }
}
