# Plan: sync --parallel

## Overview
Run multiple custom sync configs concurrently with a configurable limit.

## Goals
- `pdb-sync sync --parallel <N>` to cap concurrent rsync processes.
- Maintain per-config output separation.

## Steps
1. Add CLI flag and plumb to sync runner.
2. Use a semaphore to limit concurrency.
3. Stream each rsync output with prefix tags or log files.
4. Preserve exit codes and summary reporting.

## Tests
- Unit tests for concurrency limiter.
- Integration tests for error handling and exit code aggregation.

## Risks
- Interleaved stdout may be noisy; consider log files by default.
