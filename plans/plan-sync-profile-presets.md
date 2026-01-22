# Plan: sync profile (presets)

## Overview
Provide built-in presets (structures/assemblies/EMDB/SIFTS) and allow users to add them to config.

## Goals
- `pdb-sync sync profile list` to show presets.
- `pdb-sync sync profile add <name>` to add to config.
- Keep presets versioned in code for easy updates.

## Steps
1. Define preset registry (name, url, dest, description, default flags).
2. Add CLI commands for listing/adding.
3. Implement config update (preserve existing custom configs).
4. Add dry-run mode for config changes.

## Tests
- Unit tests for preset lookup and config merge.
- Integration tests for config file update.

## Risks
- Users may have conflicting names; need conflict handling.
