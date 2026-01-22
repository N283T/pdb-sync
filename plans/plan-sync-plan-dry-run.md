# Plan: sync plan --dry-run

## Overview
Expose a planning mode that estimates what would change: counts and approximate sizes.

## Goals
- Provide `pdb-sync sync plan` and/or `pdb-sync sync --dry-run --plan` mode.
- Display file count deltas, total bytes, and deletions.

## Steps
1. Add CLI flags for plan mode and optional JSON output.
2. Execute rsync with `--dry-run --stats --itemize-changes`.
3. Parse stats from rsync output into a structured summary.
4. Print a concise summary and optionally save it to a file.

## Tests
- Unit tests for rsync stats parsing.
- Golden output tests for summary formatting.

## Risks
- rsync `--stats` output format can vary by version.
