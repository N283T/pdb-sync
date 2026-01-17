# PR Merge and Rebase Plan

## Goal
Merge ready PRs and rebase conflicting ones to get all v3 phase PRs merged.

## Current State
- **Mergeable**: #12 (Phase 11), #18 (Phase 10), #21 (Phase 9)
- **Conflicting**: #14 (Phase 1), #15 (Phase 4), #17 (Phase 12), #19 (Phase 5), #20 (Phase 6), #22 (Phase 8)

## Execution Plan

### Step 1: Merge Ready PRs (Group 1)
Merge in any order (independent new commands):
```bash
gh pr merge 12 --squash
gh pr merge 18 --squash
gh pr merge 21 --squash
git pull origin master
```

### Step 2: Rebase Conflicting PRs
For each conflicting PR, rebase onto updated master:

**Order (simple to complex):**
1. #17 (Phase 12 - Tree) - simple new command
2. #22 (Phase 8 - Stats) - new command
3. #15 (Phase 4 - Update) - new command
4. #19 (Phase 5 - Background) - new command + job system
5. #20 (Phase 6 - aria2c) - modifies download
6. #14 (Phase 1 - Sync) - foundational, most complex

**For each:**
```bash
git checkout feature/v3-phase<N>
git fetch origin
git rebase origin/master
# Resolve conflicts (keep both additions to enums)
cargo build && cargo test
git push --force-with-lease
```

### Step 3: Merge Rebased PRs
After each successful rebase + CI pass:
```bash
gh pr merge <number> --squash
git checkout master && git pull
```

## Verification
- `cargo build` passes after each merge
- `cargo test` passes
- All PRs merged to master

## Files Likely to Conflict
- `src/cli/args.rs` - Commands enum
- `src/cli/commands/mod.rs` - module declarations
- `Cargo.toml` - dependencies
