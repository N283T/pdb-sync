# Plan: env doctor

## Overview
Add a diagnostics command to check tool dependencies and environment configuration.

## Goals
- `pdb-sync env doctor` to verify rsync, write permissions, and config location.
- Provide actionable recommendations.

## Steps
1. Check presence and version of rsync.
2. Validate config file path and readability.
3. Validate pdb_dir exists and is writable.
4. Summarize in a short report with pass/warn/fail.

## Tests
- Unit tests for report formatting and checks.
- Integration tests for missing rsync and missing config.

## Risks
- Platform-specific checks; keep OS-specific logic isolated.
