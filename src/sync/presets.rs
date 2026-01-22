//! Built-in sync profile presets.

use serde::{Deserialize, Serialize};

/// A built-in sync profile preset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilePreset {
    pub name: String,
    pub url: String,
    pub dest: String,
    pub description: String,
}

/// Get all built-in profile presets.
pub fn get_all_presets() -> Vec<ProfilePreset> {
    vec![
        ProfilePreset {
            name: "structures".to_string(),
            url: "rsync.wwpdb.org::ftp_data/structures/divided/".to_string(),
            dest: "data/structures".to_string(),
            description: "PDB structure files (divided layout by 2-char prefix)".to_string(),
        },
        ProfilePreset {
            name: "assemblies".to_string(),
            url: "rsync.wwpdb.org::ftp_data/structures/divided/assembly/".to_string(),
            dest: "data/assemblies".to_string(),
            description: "PDB biological assembly files".to_string(),
        },
        ProfilePreset {
            name: "emdb".to_string(),
            url: "rsync.ebi.ac.uk::pdbe/data/emdb/structures/".to_string(),
            dest: "data/emdb".to_string(),
            description: "EMDB (Electron Microscopy Data Bank) structure maps".to_string(),
        },
        ProfilePreset {
            name: "sifts".to_string(),
            url: "rsync.wwpdb.org::ftp_data/sifts/".to_string(),
            dest: "data/sifts".to_string(),
            description: "SIFTS (Structure Integration with Function, Taxonomy and Sequence)"
                .to_string(),
        },
        ProfilePreset {
            name: "all-structures".to_string(),
            url: "rsync.wwpdb.org::ftp_data/structures/all/".to_string(),
            dest: "data/structures-all".to_string(),
            description: "All PDB structure files in a single directory (flat layout)".to_string(),
        },
    ]
}

/// Get a preset by name.
pub fn get_preset(name: &str) -> Option<ProfilePreset> {
    get_all_presets().into_iter().find(|p| p.name == name)
}

/// List all available presets.
pub fn list_presets() {
    let presets = get_all_presets();

    println!("Available profile presets ({}):", presets.len());
    println!();

    for preset in presets {
        println!("Name: {}", preset.name);
        println!("  Description: {}", preset.description);
        println!("  URL: {}", preset.url);
        println!("  Destination: {}", preset.dest);
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_presets() {
        let presets = get_all_presets();
        assert!(!presets.is_empty());
        assert!(presets.iter().any(|p| p.name == "structures"));
        assert!(presets.iter().any(|p| p.name == "assemblies"));
        assert!(presets.iter().any(|p| p.name == "emdb"));
        assert!(presets.iter().any(|p| p.name == "sifts"));
    }

    #[test]
    fn test_get_preset() {
        let preset = get_preset("structures");
        assert!(preset.is_some());
        let preset = preset.unwrap();
        assert_eq!(preset.name, "structures");
        assert_eq!(preset.dest, "data/structures");
    }

    #[test]
    fn test_get_preset_not_found() {
        let preset = get_preset("nonexistent");
        assert!(preset.is_none());
    }
}
