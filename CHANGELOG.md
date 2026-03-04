# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Rsync flag presets**: Four built-in presets (safe, fast, minimal, conservative) for common sync scenarios
- **Nested config format**: Cleaner `[sync.custom.options]` format without `rsync_` prefix
- **Preset + override support**: Start with a preset and override specific options
- **Config migration tool**: `pdb-sync config migrate` to convert old configs to new format
- **Config validation**: `pdb-sync config validate` to check config file syntax
- **Preset listing**: `pdb-sync config presets` to list available rsync flag presets
- **Sync target listing**: `pdb-sync config list` to display configured sync targets with URL, destination, and preset
- **Backward compatibility**: Old `rsync_*` format still works

### Changed
- **Environment variable rename**: `PDB_DIR` renamed to `PDB_SYNC_DIR` to align with `PDB_SYNC_*` naming convention. `PDB_DIR` still works with a deprecation warning.
- **Config priority**: Config file `pdb_dir` now takes precedence over environment variables. Priority order: CLI args > config.toml > env vars > defaults.
- Config format now supports three styles: preset-only, preset + override, and fully custom
- Priority order for config merging: options > preset > legacy fields

### Fixed
- None

## Previous Versions

See git history for changes before this CHANGELOG was added.
