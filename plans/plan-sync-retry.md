# Plan: sync --retry

## Overview
Add automatic retry for rsync failures with backoff.

## Goals
- `pdb-sync sync --retry <N>` and optional `--retry-delay`.
- Retry only for retriable exit codes.

## Steps
1. Define retryable exit codes (reuse `is_retriable`).
2. Add CLI flags for retry count and delay strategy.
3. Implement retry loop around rsync execution.
4. Record retries in history/status output.

## Tests
- Unit tests for retry decision logic.
- Integration tests for retry counter behavior.

## Risks
- Retrying non-idempotent rsync options (e.g., delete) may surprise users.
