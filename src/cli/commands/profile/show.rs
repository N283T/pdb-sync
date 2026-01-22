//! Show details of a specific sync profile preset.

use crate::error::{PdbSyncError, Result};
use pdb_sync::profiles::SyncPreset;

/// Run the profile show command.
pub fn run_profile_show(name: String) -> Result<()> {
    let preset = SyncPreset::find(&name).ok_or_else(|| {
        PdbSyncError::InvalidInput(format!(
            "Preset '{}' not found. Available presets: {}",
            name,
            available_presets_list()
        ))
    })?;

    println!("Preset: {} ({})", preset.id, preset.name);
    println!("Description: {}", preset.description);
    println!("URL: {}", preset.url);
    println!("Destination: {}", preset.dest);
    println!("Options:");
    println!("  delete: {}", preset.delete);
    println!("  compress: {}", preset.compress);
    println!("  checksum: {}", preset.checksum);

    Ok(())
}

/// Get a formatted list of available preset IDs.
fn available_presets_list() -> String {
    let presets = SyncPreset::all();
    let ids: Vec<&str> = presets.iter().map(|p| p.id).collect();
    ids.join(", ")
}
