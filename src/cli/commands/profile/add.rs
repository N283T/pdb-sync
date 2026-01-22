//! Add a sync profile preset to configuration.

use crate::config::schema::CustomRsyncConfig;
use crate::config::ConfigLoader;
use crate::context::AppContext;
use crate::error::{PdbSyncError, Result};
use pdb_sync::profiles::SyncPreset;

/// Run the profile add command.
pub async fn run_profile_add(
    name: String,
    force: bool,
    dry_run: bool,
    ctx: AppContext,
) -> Result<()> {
    // Find the preset
    let preset = SyncPreset::find(&name).ok_or_else(|| {
        PdbSyncError::InvalidInput(format!(
            "Preset '{}' not found. Available presets: {}",
            name,
            available_presets_list()
        ))
    })?;

    // Load current config
    let mut config = ctx.config;

    // Check for existing custom config with the same name
    let existing_index = config.sync.custom.iter().position(|c| c.name == name);

    if let Some(index) = existing_index {
        if !force {
            return Err(PdbSyncError::InvalidInput(format!(
                "Custom config '{}' already exists. Use --force to overwrite.",
                name
            )));
        }
        // Show what will be replaced
        let existing = &config.sync.custom[index];
        println!("Replacing existing config '{}':", name);
        println!("  Old URL: {}", existing.url);
        println!("  New URL: {}", preset.url);
    } else {
        println!("Adding preset '{}' to configuration:", name);
    }

    println!("  URL: {}", preset.url);
    println!("  Destination: {}", preset.dest);

    // Convert preset to custom config using binary's types
    let new_config = CustomRsyncConfig {
        name: preset.id.to_string(),
        url: preset.url.to_string(),
        dest: preset.dest.to_string(),
        description: Some(preset.description.to_string()),
        rsync_delete: preset.delete,
        rsync_compress: preset.compress,
        rsync_checksum: preset.checksum,
        ..Default::default()
    };

    if dry_run {
        println!("\nDry run - no changes made.");
        println!("Would add/replace config '{}':", name);
        println!("  name: {}", new_config.name);
        println!("  url: {}", new_config.url);
        println!("  dest: {}", new_config.dest);
        if let Some(ref desc) = new_config.description {
            println!("  description: {}", desc);
        }
        return Ok(());
    }

    // Update config
    if let Some(index) = existing_index {
        config.sync.custom[index] = new_config;
    } else {
        config.sync.custom.push(new_config);
        // Sort by name for consistency
        config.sync.custom.sort_by(|a, b| a.name.cmp(&b.name));
    }

    // Save config
    ConfigLoader::save(&config)?;

    println!("\nAdded preset '{}' to configuration.", name);
    println!("You can now run: pdb-sync sync {}", name);

    Ok(())
}

/// Get a formatted list of available preset IDs.
fn available_presets_list() -> String {
    let presets = SyncPreset::all();
    let ids: Vec<&str> = presets.iter().map(|p| p.id).collect();
    ids.join(", ")
}
