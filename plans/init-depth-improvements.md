# init command improvements

## Issue 1: Depth setting

Current depth setting (0-2) is difficult to understand. Users want to create directories up to the format level (just before individual entries).

## Current behavior
```
depth 0: data/
depth 1: data/structures/
depth 2: data/structures/divided/
```

## Proposed changes

### Add depth 3 for format level
```
depth 3: data/structures/divided/mmCIF/  (and PDB, etc.)
```

### Alternative: Named depth values
More user-friendly than numbers:
```
--depth base     // same as 0
--depth types    // same as 1
--depth layouts  // same as 2
--depth format   // same as 3 (NEW)
```

### Example usage
```bash
# Create up to format level
pdb-sync init --depth 3
# or
pdb-sync init --depth format

# Result:
# pdb/
#   data/
#     structures/
#       divided/
#         mmCIF/    ← files go here: mmCIF/4h/hb/4hhb.cif.gz
#         PDB/
#       all/
#         mmCIF/
#         PDB/
```

## Implementation notes
1. Add format subdirectories to `get_layout_subdirs()` or create new function
2. Update `build_directory_tree()` to handle depth 3
3. Consider supporting both numeric and named depth values
4. Update help text
5. Add tests for depth 3

---

## Issue 2: Tree display after init

After running `init`, users want to see the created directory structure.

### Proposed solutions

**Option 1: Auto-run tree command after init**
```bash
$ pdb-sync init --depth 2
Directory structure initialized at: /home/user/pdb

[Tree output here]
```

**Option 2: Add --show-tree flag**
```bash
pdb-sync init --depth 2 --show-tree
```

**Option 3: Just suggest running tree**
```bash
$ pdb-sync init --depth 2
Directory structure initialized at: /home/user/pdb

💡 Run 'pdb-sync tree' to see the directory structure
```

### Recommendation
Option 1 (auto-show) seems most user-friendly for init command since:
- Users just ran init and want to verify what was created
- Extra output is helpful, not intrusive
- Can be skipped with --quiet if needed

### Implementation
Call `tree` command module at the end of `run_init()` after directory creation is complete.
