# Plan: config validate

## Overview
Add a command to validate config file correctness and report issues clearly.

## Goals
- `pdb-sync config validate` to validate URLs, paths, and rsync flags.
- Optional `--fix` for safe, automatic fixes (e.g., normalize paths).

## Steps
1. Implement validation helpers for URLs and dest subpaths.
2. Validate rsync flag consistency (partial_dir requires partial, etc.).
3. Add CLI command and output formatting (human + JSON).
4. Integrate with config loader for accurate error locations.

## Tests
- Unit tests for validation helpers.
- Integration tests for sample config files.

## Risks
- Overly strict validation may reject valid rsync URLs; keep warnings vs errors.
