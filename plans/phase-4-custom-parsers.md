# Refactor Phase 4: Custom Value Parsers

## Overview
Add custom value parsers for PDB ID patterns and other structured inputs following uv's pattern.

**Effort**: Short (1 day)

## Requirements
- Create parsers for PDB ID patterns
- Add validation at parse time
- Provide clear error messages
- Add tests for parsers

## Architecture Changes

### File: src/cli/args/parsers.rs (NEW)

```rust
use clap::builder::TypedValueParser;
use clap::error::ErrorKind;
use clap::{Error, Parser};

/// PDB ID pattern (supports wildcards and ranges)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PdbIdPattern {
    Single(String),
    Wildcard(String),
    Range { start: String, end: String },
    List(Vec<String>),
}

impl PdbIdPattern {
    /// Parse a PDB ID or pattern
    pub fn parse(input: &str) -> Result<Self, String> {
        let input = input.trim();

        // Range pattern: 1ABC..1DEF
        if let Some((start, end)) = input.split_once("..") {
            return Ok(PdbIdPattern::Range {
                start: start.to_uppercase(),
                end: end.to_uppercase(),
            });
        }

        // Wildcard: 1*
        if input.contains('*') {
            return Ok(PdbIdPattern::Wildcard(input.to_uppercase()));
        }

        // Single PDB ID
        let normalized = input.to_uppercase();
        if Self::is_valid(&normalized) {
            Ok(PdbIdPattern::Single(normalized))
        } else {
            Err(format!("Invalid PDB ID pattern: {}", input))
        }
    }

    /// Check if a single PDB ID is valid
    pub fn is_valid(id: &str) -> bool {
        id.len() == 4 && id.chars().all(|c| c.is_alphanumeric())
    }
}

/// Custom clap parser for PDB ID patterns
pub struct PdbIdPatternParser;

impl TypedValueParser for PdbIdPatternParser {
    type Value = PdbIdPattern;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, Error> {
        let value_str = value
            .to_str()
            .ok_or_else(|| Error::new(ErrorKind::InvalidUtf8))?;

        PdbIdPattern::parse(value_str).map_err(|msg| {
            let mut err = Error::new(ErrorKind::ValueValidation);
            err.insert(cmd, arg, msg);
            err
        })
    }
}
```

### File: src/cli/args/enums.rs (MODIFY)

Move validation functions to parsers module:
- `validate_resolution()` → parsers.rs
- `validate_organism()` → parsers.rs
- Add `validate_interval()` function

## Implementation Steps

### Step 1: Create parsers module
- [ ] Create `src/cli/args/parsers.rs`
- [ ] Implement `PdbIdPattern` type
- [ ] Implement custom clap parser

### Step 2: Add validation functions
- [ ] Move `validate_resolution()` to parsers module
- [ ] Move `validate_organism()` to parsers module
- [ ] Add `validate_interval()` function
- [ ] Add unit tests for each validator

### Step 3: Update args to use custom parsers
- [ ] Use `PdbIdPatternParser` where applicable
- [ ] Add value_parser for interval args

### Step 4: Update handlers
- [ ] Update handlers to work with PdbIdPattern
- [ ] Add pattern expansion logic

## Verification

```bash
# Test parsers
cargo test parsers

# Test invalid inputs
cargo run -- download INVALID_ID
cargo run -- watch --resolution 150.0

# Test valid patterns
cargo run -- download "1ABC..1DEF"
cargo run -- download "{1ABC,1DEF,1XYZ}"
```

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Pattern parsing is too restrictive | Low | Allow flexible patterns |
| Breaking changes to input format | Medium | Document pattern syntax |
| Parser complexity | Low | Keep simple regex |

## Success Criteria

- [ ] Custom parsers module created
- [ ] At least 3 validators implemented
- [ ] PDB ID pattern parser works
- [ ] All tests pass
- [ ] Clear error messages

---
- [ ] **DONE** - Phase 4 complete
