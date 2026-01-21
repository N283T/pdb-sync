# Refactor Phase 5: Flag Source Tracking

## Overview
Track where configuration values come from (CLI args vs env vars vs config file) to improve error messages and debugging.

**Effort**: Medium (2-3 days)

## Requirements
- Add FlagSource enum to track value origins
- Attach source information to config values
- Improve error messages showing flag sources
- Add debug command to show config sources

## Architecture Changes

### File: src/config/source.rs (NEW)

```rust
/// Where a configuration value came from
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FlagSource {
    /// Default value
    Default,
    /// Config file
    Config,
    /// Environment variable
    Env,
    /// Command-line argument
    CliArg,
}

impl std::fmt::Display for FlagSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlagSource::Default => write!(f, "default"),
            FlagSource::Config => write!(f, "config file"),
            FlagSource::Env => write!(f, "environment variable"),
            FlagSource::CliArg => write!(f, "command-line argument"),
        }
    }
}

/// A configuration value with its source
#[derive(Debug, Clone)]
pub struct SourcedValue<T> {
    pub value: T,
    pub source: FlagSource,
}

impl<T> SourcedValue<T> {
    pub fn new(value: T, source: FlagSource) -> Self {
        Self { value, source }
    }
}
```

### File: src/cli/commands/config.rs (MODIFY)

Add config sources command:

```rust
#[derive(Subcommand, Clone)]
pub enum ConfigAction {
    // ... existing actions
    /// Show where each configuration value is coming from
    Sources,
}
```

## Implementation Steps

### Step 1: Create FlagSource types
- [ ] Create `src/config/source.rs`
- [ ] Implement FlagSource enum
- [ ] Implement SourcedValue wrapper

### Step 2: Create merged config
- [ ] Create `src/config/merged.rs`
- [ ] Implement MergedConfig struct
- [ ] Implement merge logic with priority

### Step 3: Update context loading
- [ ] Update AppContext to use MergedConfig
- [ ] Track sources when loading config

### Step 4: Add sources command
- [ ] Add `ConfigAction::Sources`
- [ ] Implement handler to show sources

### Step 5: Update error messages
- [ ] Include source information in errors
- [ ] Show where conflicting values came from

## Verification

```bash
# Test sources command
cargo run -- config sources

# Test with different sources
PDB_DIR=/tmp cargo run -- download 1ABC
cargo run -- download --pdb-dir /tmp 1ABC

# Test error messages show sources
cargo run -- download 1ABC --mirror invalid_mirror
```

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Config merge logic complexity | Medium | Thorough testing |
| Breaking AppContext API | High | Keep backward compatible |
| Performance overhead | Low | Negligible |

## Success Criteria

- [ ] FlagSource tracking implemented
- [ ] MergedConfig works correctly
- [ ] `config sources` command shows output
- [ ] Error messages include source info
- [ ] All tests pass

---
- [x] **DONE** - Phase 5 complete (2025-01-21)
