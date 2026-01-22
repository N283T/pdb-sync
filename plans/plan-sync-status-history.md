# Plan: sync status / history

## Overview
Record sync runs (start/end, success/failure, bytes transferred, rsync exit code) and show them via a new command.

## Goals
- Persist per-run metadata in a stable location.
- Provide `pdb-sync sync status` and `pdb-sync sync history` outputs.
- Keep output script-friendly (optional JSON output).

## Steps
1. Define a run record schema (timestamp, config name, args, exit code, bytes, duration).
2. Add a storage layer (e.g., `~/.local/share/pdb-sync/history/` with one JSON per run).
3. Capture rsync output summary to populate bytes/transferred stats.
4. Implement CLI subcommands and table/JSON formatting.
5. Add retention policy (max entries) and pruning.

## Tests
- Unit tests for record serialization/deserialization.
- Integration-style tests for history listing order and filtering.

## Risks
- rsync output parsing may be locale-dependent.
- Need to ensure history writes don’t block sync performance.
