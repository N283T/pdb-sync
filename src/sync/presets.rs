//! Built-in sync profile presets and rsync flag presets.

use super::RsyncFlags;
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

// === Rsync Flag Presets ===

/// Rsync flag preset for common sync scenarios.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RsyncPreset {
    /// Safe preset: No delete, compress, checksum, partial, verbose
    /// Use for first-time sync or cautious users
    Safe,
    /// Fast preset: Delete, compress, no checksum, partial, quiet
    /// Use for regular updates where speed is priority
    Fast,
    /// Minimal preset: Bare minimum flags, full control needed
    Minimal,
    /// Conservative preset: No delete, compress, checksum, partial, backup, verbose
    /// Use for production environments requiring maximum safety
    Conservative,
}

impl RsyncPreset {
    /// Convert preset name to enum variant.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "safe" => Some(RsyncPreset::Safe),
            "fast" => Some(RsyncPreset::Fast),
            "minimal" => Some(RsyncPreset::Minimal),
            "conservative" => Some(RsyncPreset::Conservative),
            _ => None,
        }
    }

    /// Convert preset to string name.
    pub fn as_str(&self) -> &'static str {
        match self {
            RsyncPreset::Safe => "safe",
            RsyncPreset::Fast => "fast",
            RsyncPreset::Minimal => "minimal",
            RsyncPreset::Conservative => "conservative",
        }
    }

    /// Get the description of this preset.
    pub fn description(&self) -> &'static str {
        match self {
            RsyncPreset::Safe => "First-time sync, cautious users. No delete, with compress, checksum, and partial.",
            RsyncPreset::Fast => "Regular updates, speed priority. Delete enabled, compress, no checksum, partial, quiet.",
            RsyncPreset::Minimal => "Bare minimum flags for full manual control.",
            RsyncPreset::Conservative => "Production, maximum safety. No delete, compress, checksum, partial, backup, verbose.",
        }
    }

    /// Convert this preset to RsyncFlags.
    pub fn to_flags(self) -> RsyncFlags {
        match self {
            RsyncPreset::Safe => RsyncFlags {
                delete: false,
                compress: true,
                checksum: true,
                partial: true,
                verbose: true,
                ..Default::default()
            },
            RsyncPreset::Fast => RsyncFlags {
                delete: true,
                compress: true,
                checksum: false,
                partial: true,
                quiet: true,
                ..Default::default()
            },
            RsyncPreset::Minimal => RsyncFlags::default(),
            RsyncPreset::Conservative => RsyncFlags {
                delete: false,
                compress: true,
                checksum: true,
                partial: true,
                backup: true,
                verbose: true,
                ..Default::default()
            },
        }
    }
}

/// Get rsync flags from a preset name.
pub fn get_rsync_preset(name: &str) -> Option<RsyncFlags> {
    RsyncPreset::from_str(name).map(|preset| preset.to_flags())
}

/// List all available rsync flag presets with their descriptions.
pub fn list_rsync_presets() {
    let presets = [
        RsyncPreset::Safe,
        RsyncPreset::Fast,
        RsyncPreset::Minimal,
        RsyncPreset::Conservative,
    ];

    println!("Available rsync flag presets:");
    println!();

    for preset in &presets {
        println!("Name: {}", preset.as_str());
        println!("  Description: {}", preset.description());
        let flags = preset.to_flags();
        println!("  Flags:");
        println!("    delete: {}", flags.delete);
        println!("    compress: {}", flags.compress);
        println!("    checksum: {}", flags.checksum);
        println!("    partial: {}", flags.partial);
        println!("    backup: {}", flags.backup);
        println!("    verbose: {}", flags.verbose);
        println!("    quiet: {}", flags.quiet);
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

    #[test]
    fn test_rsync_preset_from_str() {
        assert_eq!(RsyncPreset::from_str("safe"), Some(RsyncPreset::Safe));
        assert_eq!(RsyncPreset::from_str("fast"), Some(RsyncPreset::Fast));
        assert_eq!(RsyncPreset::from_str("minimal"), Some(RsyncPreset::Minimal));
        assert_eq!(
            RsyncPreset::from_str("conservative"),
            Some(RsyncPreset::Conservative)
        );
        assert_eq!(RsyncPreset::from_str("SAFE"), Some(RsyncPreset::Safe)); // case insensitive
        assert_eq!(RsyncPreset::from_str("nonexistent"), None);
    }

    #[test]
    fn test_rsync_preset_to_flags() {
        let safe_flags = RsyncPreset::Safe.to_flags();
        assert!(!safe_flags.delete);
        assert!(safe_flags.compress);
        assert!(safe_flags.checksum);
        assert!(safe_flags.partial);
        assert!(safe_flags.verbose);

        let fast_flags = RsyncPreset::Fast.to_flags();
        assert!(fast_flags.delete);
        assert!(fast_flags.compress);
        assert!(!fast_flags.checksum);
        assert!(fast_flags.partial);
        assert!(fast_flags.quiet);

        let minimal_flags = RsyncPreset::Minimal.to_flags();
        assert!(!minimal_flags.delete);
        assert!(!minimal_flags.compress);
        assert!(!minimal_flags.checksum);

        let conservative_flags = RsyncPreset::Conservative.to_flags();
        assert!(!conservative_flags.delete);
        assert!(conservative_flags.compress);
        assert!(conservative_flags.checksum);
        assert!(conservative_flags.partial);
        assert!(conservative_flags.backup);
        assert!(conservative_flags.verbose);
    }

    #[test]
    fn test_get_rsync_preset() {
        let flags = get_rsync_preset("safe");
        assert!(flags.is_some());
        let flags = flags.unwrap();
        assert!(!flags.delete);
        assert!(flags.compress);

        let flags = get_rsync_preset("nonexistent");
        assert!(flags.is_none());
    }
}
