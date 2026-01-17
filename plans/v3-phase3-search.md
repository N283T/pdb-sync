# Phase 3: Search Command

## Parallel Development Note
This phase is being developed in parallel with other v3 phases.
- Create PR and request review
- Wait for review approval before merging
- Rebase on master if other phases are merged first

## Goal
Add `search` command to query RCSB API and return PDB IDs matching criteria.

## Usage Examples

```bash
# Search by resolution
pdb-cli search --resolution "<2.0"
pdb-cli search --resolution "1.5-2.5"

# Search by experimental method
pdb-cli search --method xray
pdb-cli search --method nmr
pdb-cli search --method em

# Search by organism
pdb-cli search --organism "Homo sapiens"
pdb-cli search --organism-id 9606

# Search by release date
pdb-cli search --released-after 2024-01-01
pdb-cli search --released-before 2023-12-31

# Search by text (title, abstract, etc.)
pdb-cli search --text "kinase inhibitor"

# Search by sequence (BLAST-like)
pdb-cli search --sequence "MVLSPADKTNVKAAWGKVGAHAGEYGAEAL..."
pdb-cli search --sequence-file protein.fasta --identity 90

# Search by structure similarity
pdb-cli search --similar-to 4hhb --rmsd 2.0

# Combine criteria (AND)
pdb-cli search --method xray --resolution "<1.5" --organism "Homo sapiens"

# Output options
pdb-cli search --resolution "<2.0" --output json
pdb-cli search --resolution "<2.0" --output ids    # Just PDB IDs, one per line
pdb-cli search --resolution "<2.0" --count         # Just count

# Limit results
pdb-cli search --text "covid" --limit 100

# Pipe to download
pdb-cli search --resolution "<1.5" --limit 10 -o ids | xargs pdb-cli download
```

## RCSB Search API

Base URL: `https://search.rcsb.org/rcsbsearch/v2/query`

### Query Types
- `TextQuery` - Full text search
- `AttributeQuery` - Search by specific attributes
- `SeqMotifQuery` - Sequence motif search
- `SequenceQuery` - BLAST sequence search
- `StructureQuery` - 3D structure similarity
- `StructMotifQuery` - Structural motif search

### Common Attributes
- `rcsb_entry_info.resolution_combined` - Resolution
- `exptl.method` - Experimental method
- `rcsb_entity_source_organism.scientific_name` - Organism
- `rcsb_accession_info.initial_release_date` - Release date

## Implementation Tasks

### 1. Create search module

```rust
// src/api/search.rs
pub struct SearchQuery {
    pub text: Option<String>,
    pub resolution: Option<ResolutionRange>,
    pub method: Option<ExperimentalMethod>,
    pub organism: Option<String>,
    pub released_after: Option<NaiveDate>,
    pub released_before: Option<NaiveDate>,
    pub limit: Option<u32>,
}

pub async fn search(query: SearchQuery) -> Result<SearchResults> {
    // Build RCSB search API query
    // Execute request
    // Parse and return results
}
```

### 2. Build RCSB query JSON

```rust
fn build_query_json(query: &SearchQuery) -> serde_json::Value {
    // Construct the nested query structure
    // Handle AND/OR combinations
}
```

### 3. Add CLI args

```rust
// src/cli/args.rs
#[derive(Args)]
pub struct SearchArgs {
    #[arg(long)]
    pub text: Option<String>,

    #[arg(long)]
    pub resolution: Option<String>,  // Parse "<2.0" or "1.5-2.5"

    #[arg(long)]
    pub method: Option<ExperimentalMethod>,

    #[arg(long)]
    pub organism: Option<String>,

    #[arg(long)]
    pub released_after: Option<NaiveDate>,

    #[arg(long)]
    pub released_before: Option<NaiveDate>,

    #[arg(long, default_value = "1000")]
    pub limit: u32,

    #[arg(short, long, default_value = "text")]
    pub output: OutputFormat,

    #[arg(long)]
    pub count: bool,
}
```

### 4. Implement command handler

```rust
// src/cli/commands/search.rs
pub async fn run_search(args: SearchArgs, ctx: AppContext) -> Result<()> {
    let query = SearchQuery::from(args);
    let results = search(query).await?;

    match args.output {
        OutputFormat::Ids => { /* print one ID per line */ }
        OutputFormat::Json => { /* print JSON */ }
        OutputFormat::Text => { /* print formatted table */ }
    }
}
```

## Files to Create/Modify

- `src/api/search.rs` - New: RCSB search API client
- `src/api/mod.rs` - Export search module
- `src/cli/args.rs` - Add SearchArgs
- `src/cli/commands/search.rs` - New: Search command handler
- `src/cli/commands/mod.rs` - Export search
- `src/main.rs` - Add search command

## Testing

- Unit tests for query building
- Integration tests with RCSB API (mock or real)
- Test various search criteria combinations
- Test output formats
- Test result limiting
