# Plan: Parallel Sync Execution with `--parallel` Flag

## Overview
Add parallel execution capability to the custom sync command with configurable concurrency limits, proper output management, and comprehensive error reporting.

## Design Decisions

### 1. Output Management: Prefix Tags
Use `[config-name]` prefix tags for stdout/stderr. Log files add unnecessary complexity and users can redirect output via shell if needed.

### 2. Concurrency Control: Tokio Semaphore
Use `tokio::sync::Semaphore` with permit-based limiting, following the existing pattern in `src/mirrors/auto_select.rs`.

### 3. Result Aggregation: Custom `SyncResult` Struct
Track per-config results with success/failure status, exit codes, and timing.

## Files to Modify

1. **`src/cli/args/sync.rs`** - Add `--parallel` flag and validation
2. **`src/cli/commands/sync/wwpdb.rs`** - Core parallel execution logic

## Implementation Steps

### Step 1: Add CLI Flag (`src/cli/args/sync.rs`)

Add to `SyncArgs` struct:
```rust
/// Maximum number of concurrent sync processes (default: 1 = sequential)
#[arg(long, value_name = "N", default_value = "1")]
pub parallel: usize,
```

Add validation:
- Must be >= 1 (reject 0)
- Must be <= 100 (prevent resource exhaustion)

### Step 2: Add `SyncResult` Struct (`src/cli/commands/sync/wwpdb.rs`)

```rust
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub name: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub error: Option<String>,
    pub duration: Duration,
}

impl SyncResult {
    pub fn success(name: String, duration: Duration) -> Self { ... }
    pub fn failed(name: String, exit_code: Option<i32>, error: String, duration: Duration) -> Self { ... }
    pub fn display(&self) -> String { ... }
}
```

### Step 3: Refactor `run_custom_all`

Change from sequential-only to dispatch based on `parallel` value:
```rust
pub async fn run_custom_all(args: SyncArgs, ctx: AppContext) -> Result<()> {
    // ... existing empty check ...

    if args.parallel == 1 {
        run_sequential(&custom_configs, args, ctx).await
    } else {
        run_parallel(&custom_configs, args, ctx, args.parallel).await
    }
}
```

### Step 4: Extract Sequential Path

Move existing loop logic into `run_sequential()`:
```rust
async fn run_sequential(
    configs: &[CustomRsyncConfig],
    args: SyncArgs,
    ctx: AppContext,
) -> Result<()> {
    // ... existing sequential loop with SyncResult tracking ...
}
```

### Step 5: Create Output Prefix Function

New function `run_sync_with_prefix()`:
- Captures stdout/stderr with `Stdio::piped()`
- Uses `BufReader` for line-by-line reading
- Prepends `[name]` to each line
- Handles both stdout and stderr concurrently

### Step 6: Create Parallel Executor

New function `run_parallel()`:
```rust
async fn run_parallel(
    configs: &[CustomRsyncConfig],
    args: SyncArgs,
    ctx: AppContext,
    parallel: usize,
) -> Result<()> {
    let semaphore = Arc::new(Semaphore::new(parallel));
    let mut tasks = Vec::new();

    for config in configs {
        let semaphore = semaphore.clone();
        let task = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            run_sync_with_prefix(name, args, ctx).await
        });
        tasks.push(task);
    }

    // Collect results and print summary
}
```

### Step 7: Add Summary Display

```rust
fn print_summary(results: &[SyncResult]) {
    // Show: Total | Success | Failed
    // List failed syncs with details
    // Show total/average duration
}
```

## Tests (Target: 80%+ Coverage)

### Unit Tests

**`src/cli/args/sync.rs`:**
- `test_parallel_default` - Verify default is 1
- `test_parallel_validation_zero` - Reject 0
- `test_parallel_validation_too_large` - Reject > 100
- `test_parallel_validation_valid` - Accept 1-100

**`src/cli/commands/sync/wwpdb.rs`:**
- `test_sync_result_success` - Success creation
- `test_sync_result_failed` - Failure creation
- `test_sync_result_display_success` - Success display format
- `test_sync_result_display_failed` - Failure display format

### Integration Tests

- `test_run_sequential_multiple_configs` - Sequential with multiple configs
- `test_run_parallel_concurrency_limit` - Verify semaphore limits max concurrent

## Verification

1. Run `cargo test` - all tests pass with >80% coverage
2. Run `cargo clippy -- -D warnings` - no warnings
3. Run `cargo fmt --all -- --check` - formatting verified
4. Manual test with `--parallel 2` - verify output prefixing works
5. Manual test with `--parallel 1` - verify sequential behavior preserved

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Interleaved output confusion | Clear `[name]` prefix tags; default to sequential |
| Resource exhaustion | Hard limit of 100; semaphore control |
| Task panic propagation | Wrap tasks, catch panics, convert to failures |
| Line buffering issues | Use `BufReader::lines()`; test with actual rsync |
| Backward compatibility | Default `parallel=1`; explicit flag only |

## Example Usage

```bash
# Sequential (default, backward compatible)
pdb-sync sync --all

# Parallel with 3 concurrent rsync processes
pdb-sync sync --all --parallel 3

# Single sync (parallel flag ignored when name specified)
pdb-sync sync my-config
```

## Example Output

```
Syncing 3 custom configs...

[config1] Starting sync...
[config2] Starting sync...
[config1] receiving file list... done
[config3] Starting sync...
[config2] receiving file list... done
[config1] sent 1,234 bytes  received 12,345 bytes
[config1] Completed
[config3] sent 2,345 bytes  received 23,456 bytes
[config3] Completed
[config2] sent 3,456 bytes  received 34,567 bytes
[config2] Completed

=== Sync Summary ===
Total: 3 | Success: 3 | Failed: 0
All syncs completed successfully.
Total time: 45.23s | Average: 15.08s
```
