# Refactor Phase 2: Structured Error Handling

## Overview
Improve error handling with structured context, source tracking, and helper methods following uv's error pattern.

**Effort**: Medium (2-3 days)

## Requirements
- Add structured error kinds with context
- Include source tracking (URL, pdb_id, etc.)
- Add helper methods like `is_retriable()`
- Maintain backward compatibility

## Architecture Changes

### File: src/error.rs (MODIFY)

#### New Error Pattern

```rust
#[derive(Error, Debug)]
pub enum PdbSyncError {
    #[error("Invalid PDB ID: {0}")]
    InvalidPdbId {
        input: String,
        #[source]
        source: Option<PdbIdError>,
    },

    #[error("Configuration error: {message}")]
    Config {
        message: String,
        key: Option<String>,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Network error: {url}")]
    Network {
        url: String,
        message: String,
        #[source]
        source: Option<reqwest::Error>,
        is_retriable: bool,
    },

    #[error("Download failed for {pdb_id}: {message}")]
    Download {
        pdb_id: String,
        url: String,
        message: String,
        is_retriable: bool,
    },

    #[error("Checksum mismatch for {pdb_id}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        pdb_id: String,
        expected: String,
        actual: String,
        file_path: PathBuf,
    },

    #[error("rsync failed: {command}")]
    Rsync {
        command: String,
        exit_code: Option<i32>,
        stderr: Option<String>,
    },

    #[error("Not found: {pdb_id}")]
    NotFound {
        pdb_id: String,
        mirror: Option<String>,
        searched_urls: Vec<String>,
    },
}
```

#### New Helper Methods

```rust
impl PdbSyncError {
    /// Check if this error is retriable (e.g., network timeout)
    pub fn is_retriable(&self) -> bool {
        match self {
            PdbSyncError::Network { is_retriable, .. } => *is_retriable,
            PdbSyncError::Download { is_retriable, .. } => *is_retriable,
            PdbSyncError::Rsync { exit_code, .. } => {
                matches!(exit_code, Some(5 | 10 | 30))
            }
            _ => false,
        }
    }

    /// Get the PDB ID associated with this error, if any
    pub fn pdb_id(&self) -> Option<&str> {
        match self {
            PdbSyncError::InvalidPdbId { input, .. } => Some(input),
            PdbSyncError::Download { pdb_id, .. } => Some(pdb_id),
            PdbSyncError::ChecksumMismatch { pdb_id, .. } => Some(pdb_id),
            PdbSyncError::NotFound { pdb_id, .. } => Some(pdb_id),
            _ => None,
        }
    }
}
```

## Implementation Steps

### Step 1: Add structured error variants
- [ ] Update InvalidPdbId with structured context
- [ ] Update Config with key information
- [ ] Update Network with URL and retriable flag

### Step 2: Add source tracking
- [ ] Add url field to download-related errors
- [ ] Add pdb_id field where applicable
- [ ] Add mirror field for remote-related errors

### Step 3: Add helper methods
- [ ] Implement `is_retriable()`
- [ ] Implement `pdb_id()`
- [ ] Add tests for helpers

### Step 4: Update error construction
- [ ] Update all error creation sites to use structured form
- [ ] Update download module errors
- [ ] Update sync module errors

### Step 5: Add tests
- [ ] Test is_retriable() for each error type
- [ ] Test pdb_id() extraction
- [ ] Test error display with context

## Verification

```bash
# Build and test
cargo build
cargo test

# Test error messages are clear
cargo run -- download 1INVALID

# Test error context is useful
cargo run -- download 1ABC --mirror invalid
```

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Breaking error Display format | Medium | Keep messages similar |
| Error construction becomes verbose | Low | Add helper constructors |
| Missing context in some errors | Low | Add Optional fields |

## Success Criteria

- [x] All errors have structured context where applicable
- [x] `is_retriable()` works correctly
- [x] Error messages include relevant details (URL, pdb_id)
- [x] All tests pass
- [x] No clippy warnings

---
- [x] **DONE** - Phase 2 complete (2025-01-21)
