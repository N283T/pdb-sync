//! List all available sync profile presets.

use crate::error::Result;
use pdb_sync::profiles::SyncPreset;

/// Run the profile list command.
pub fn run_profile_list() -> Result<()> {
    let presets = SyncPreset::all();

    if presets.is_empty() {
        println!("No sync profile presets available.");
        return Ok(());
    }

    println!("Available sync profile presets:\n");

    for preset in presets {
        println!("  {} ({})", preset.id, preset.name);
        println!("    {}", preset.description);
        println!("    URL: {}", preset.url);
        println!();
    }

    println!("Use 'pdb-sync sync profile show <name>' for details on a specific preset.");
    println!("Use 'pdb-sync sync profile add <name>' to add a preset to your configuration.");

    Ok(())
}
