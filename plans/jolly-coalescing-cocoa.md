# Implementation Plan: env doctor Command

## Overview
Add `pdb-sync env doctor` to verify rsync, write permissions, and config location.

## Files to Modify

### 1. `src/cli/args/global.rs`
Add `Env` variant to `SyncCommand` enum:
```rust
#[derive(Subcommand)]
pub enum SyncCommand {
    /// Sync from a configured source (runs all if no name specified)
    Sync(SyncArgs),
    /// Environment diagnostics and validation
    Env(EnvArgs),
}
```
Add import: `use super::env::EnvArgs;`

### 2. `src/cli/args/mod.rs`
Add `pub mod env;` and re-export `EnvArgs`

### 3. `src/cli/args/env.rs` (NEW)
Create argument structure:
```rust
#[derive(Parser, Clone, Debug)]
pub struct EnvArgs {
    #[command(subcommand)]
    pub command: EnvCommand,
}

#[derive(Subcommand, Clone, Debug)]
pub enum EnvCommand {
    /// Run environment diagnostics
    Doctor,
}
```

### 4. `src/cli/commands/mod.rs`
Add `pub mod env;`

### 5. `src/cli/commands/env/mod.rs` (NEW)
```rust
pub mod doctor;

pub use doctor::run_doctor;
```

### 6. `src/cli/commands/env/doctor.rs` (NEW)
Main implementation file with:
- `CheckResult` enum (Pass, Warn, Fail)
- `Check` struct with name, status, message
- `run_doctor()` function with all checks
- Unit tests in `#[cfg(test)]` module

### 7. `src/main.rs`
Add dispatch case:
```rust
SyncCommand::Env(args) => {
    cli::args::env::run_env(args, ctx).await?;
}
```

## Implementation Steps

### Step 1: Create Check Result Types (`src/cli/commands/env/doctor.rs`)

```rust
use std::process::Command;

/// Result of a diagnostic check
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

/// A single diagnostic check result
#[derive(Clone, Debug)]
pub struct Check {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
}

/// Doctor check results summary
pub struct DoctorReport {
    pub checks: Vec<Check>,
}

impl DoctorReport {
    pub fn print(&self) {
        // Print report with status symbols
    }

    pub fn exit_code(&self) -> i32 {
        // 0 if all pass, 1 if any fail, 2 if any warn
    }
}
```

### Step 2: Implement Individual Checks

**Rsync Check:**
```rust
fn check_rsync() -> Check {
    match Command::new("rsync").arg("--version").output() {
        Ok(output) => {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout);
                Parse version from first line
                Check {
                    name: "rsync".to_string(),
                    status: CheckStatus::Pass,
                    message: format!("v{}", version_str),
                }
            } else {
                fail_check("rsync", "Command failed")
            }
        }
        Err(_) => fail_check("rsync", "Not found in PATH"),
    }
}
```

**Config File Check:**
```rust
fn check_config() -> Check {
    use crate::config::ConfigLoader;

    match ConfigLoader::config_path() {
        Some(path) => {
            if path.exists() {
                match std::fs::read_to_string(&path) {
                    Ok(_) => pass_check("config", format!("Found: {}", path.display())),
                    Err(e) => warn_check("config", format!("Not readable: {}", e)),
                }
            } else {
                warn_check("config", format!("File not found: {}", path.display()))
            }
        }
        None => warn_check("config", "Unable to determine config location"),
    }
}
```

**PDB Dir Check:**
```rust
fn check_pdb_dir(pdb_dir: &std::path::Path) -> Check {
    if pdb_dir.exists() {
        // Test writability by creating a temp file
        let test_file = pdb_dir.join(".pdb-sync-write-test");
        match std::fs::write(&test_file, b"test") {
            Ok(_) => {
                let _ = std::fs::remove_file(&test_file);
                pass_check("pdb_dir", format!("Writable: {}", pdb_dir.display()))
            }
            Err(e) => fail_check("pdb_dir", format!("Not writable: {}", e)),
        }
    } else {
        fail_check("pdb_dir", format!("Does not exist: {}", pdb_dir.display()))
    }
}
```

### Step 3: Main Command Implementation

```rust
pub async fn run_doctor(ctx: AppContext) -> Result<()> {
    let mut checks = vec![
        check_rsync(),
        check_config(),
        check_pdb_dir(&ctx.pdb_dir),
    ];

    let report = DoctorReport { checks };
    report.print();

    std::process::exit(report.exit_code());
}
```

### Step 4: Update CLI Structure

**`src/cli/args/env.rs` run function:**
```rust
pub async fn run_env(args: EnvArgs, ctx: AppContext) -> Result<()> {
    match args.command {
        EnvCommand::Doctor => {
            crate::cli::commands::env::run_doctor(ctx).await
        }
    }
}
```

### Step 5: Update `src/main.rs` dispatch

Add the import and dispatch case.

## Tests (TDD Approach)

### Unit Tests in `src/cli/commands/env/doctor.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_status_display() {
        // Test status symbols/formats
    }

    #[test]
    fn test_report_exit_code_all_pass() {
        // Exit code 0 when all pass
    }

    #[test]
    fn test_report_exit_code_with_fail() {
        // Exit code 1 when any fail
    }

    #[test]
    fn test_report_exit_code_warn_only() {
        // Exit code 2 when only warnings
    }

    #[test]
    fn test_pdb_dir_writable() {
        use tempfile::TempDir;
        let temp = TempDir::new().unwrap();
        let check = check_pdb_dir(temp.path());
        assert_eq!(check.status, CheckStatus::Pass);
    }

    #[test]
    fn test_pdb_dir_not_writable() {
        // Test with read-only directory
    }

    #[test]
    fn test_pdb_dir_not_exists() {
        let check = check_pdb_dir(Path::new("/nonexistent/path"));
        assert_eq!(check.status, CheckStatus::Fail);
    }
}
```

## Verification

### Manual Testing

```bash
# Build
cargo build

# Run doctor
cargo run -- env doctor

# Test with missing rsync (mock PATH)
PATH=/usr/bin:/bin cargo run -- env doctor

# Test with invalid pdb_dir
PDB_DIR=/nonexistent cargo run -- env doctor
```

### Expected Output

```
Environment Diagnostics
======================

✓ rsync          v3.2.7
✓ config         Found: /home/user/.config/pdb-sync/config.toml
✓ pdb_dir        Writable: /home/user/pdb

All checks passed.
```

### CI Verification

```bash
cargo fmt --all -- --check
cargo clippy -- -D warnings
cargo test
```

## Key Implementation Notes

1. **Exit Codes**: Use `std::process::exit()` instead of returning `Err` for failed checks - the command should always print a report, not error out.

2. **Status Symbols**: Use ✓ for pass, ⚠ for warn, ✗ for fail.

3. **Config Path**: Use existing `ConfigLoader::config_path()` - don't reimplement path logic.

4. **Writability Test**: Create a temp file and clean it up (use `.pdb-sync-write-test` filename).

5. **Rsync Version**: Parse first line of `rsync --version` output (format: "rsync  version X.X.X").

6. **No New Error Types**: Use existing `PdbSyncError` variants; doctor returns results, not errors.
