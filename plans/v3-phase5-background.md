# Phase 5: Background Execution

## Parallel Development Note
This phase is being developed in parallel with other v3 phases.
- Create PR and request review
- Wait for review approval before merging
- Rebase on master if other phases are merged first

## Goal
Allow long-running operations (sync, download, update) to run in the background with job management.

## Usage Examples

```bash
# Start sync in background
pdb-cli sync --type structures --mirror pdbj --bg
# Output: Job started: job_abc123
#         Log: ~/.cache/pdb-cli/jobs/job_abc123.log
#         Use 'pdb-cli jobs' to check status

# Start download in background
pdb-cli download -l large_list.txt --bg

# List running/completed jobs
pdb-cli jobs
pdb-cli jobs --all        # Include completed
pdb-cli jobs --running    # Only running

# Check specific job status
pdb-cli jobs status job_abc123
pdb-cli jobs log job_abc123
pdb-cli jobs log job_abc123 --follow  # Like tail -f

# Cancel a running job
pdb-cli jobs cancel job_abc123

# Clean up old job logs
pdb-cli jobs clean
pdb-cli jobs clean --older-than 7d
```

## Job Management

### Job Storage
```
~/.cache/pdb-cli/jobs/
├── job_abc123/
│   ├── meta.json      # Job metadata
│   ├── stdout.log     # Standard output
│   ├── stderr.log     # Standard error
│   └── pid            # Process ID (if running)
├── job_def456/
│   └── ...
└── index.json         # Job index for quick listing
```

### Job Metadata
```json
{
  "id": "job_abc123",
  "command": "sync --type structures --mirror pdbj",
  "started_at": "2024-01-15T10:30:00Z",
  "finished_at": null,
  "status": "running",
  "pid": 12345,
  "exit_code": null
}
```

## Implementation Tasks

### 1. Create job management module

```rust
// src/jobs/mod.rs
pub struct Job {
    pub id: String,
    pub command: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub status: JobStatus,
    pub pid: Option<u32>,
    pub exit_code: Option<i32>,
}

pub enum JobStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

pub struct JobManager {
    jobs_dir: PathBuf,
}

impl JobManager {
    pub fn new() -> Self { ... }
    pub fn create_job(&self, command: &str) -> Result<Job> { ... }
    pub fn list_jobs(&self, filter: JobFilter) -> Result<Vec<Job>> { ... }
    pub fn get_job(&self, id: &str) -> Result<Job> { ... }
    pub fn cancel_job(&self, id: &str) -> Result<()> { ... }
    pub fn clean_old_jobs(&self, older_than: Duration) -> Result<u32> { ... }
}
```

### 2. Implement background spawning

```rust
// src/jobs/spawn.rs
pub fn spawn_background(args: &[String]) -> Result<Job> {
    // 1. Create job directory
    // 2. Fork/spawn new process
    // 3. Redirect stdout/stderr to log files
    // 4. Write PID file
    // 5. Return job info

    // On Unix: use fork() or Command with proper detachment
    // Ensure process survives terminal close
}
```

### 3. Add --bg flag to commands

```rust
// src/cli/args.rs
#[derive(Args)]
pub struct SyncArgs {
    // ... existing args ...

    /// Run in background
    #[arg(long)]
    pub bg: bool,
}

// Similarly for DownloadArgs, UpdateArgs
```

### 4. Add jobs command

```rust
// src/cli/args.rs
#[derive(Subcommand)]
pub enum Commands {
    // ... existing commands ...

    /// Manage background jobs
    Jobs(JobsArgs),
}

#[derive(Args)]
pub struct JobsArgs {
    #[command(subcommand)]
    pub command: Option<JobsCommand>,
}

#[derive(Subcommand)]
pub enum JobsCommand {
    /// Show job status
    Status { job_id: String },
    /// Show job log
    Log {
        job_id: String,
        #[arg(short, long)]
        follow: bool,
    },
    /// Cancel a running job
    Cancel { job_id: String },
    /// Clean old job logs
    Clean {
        #[arg(long)]
        older_than: Option<String>,
    },
}
```

### 5. Handle --bg in main

```rust
// src/main.rs
async fn main() {
    let args = Cli::parse();

    // Check if --bg is set
    if should_run_background(&args) {
        let job = spawn_background(&args)?;
        println!("Job started: {}", job.id);
        println!("Log: {}", job.log_path());
        return Ok(());
    }

    // Normal execution
    // ...
}
```

## Platform Considerations

### Unix
- Use `fork()` via `nix` crate or `daemonize`
- Double-fork to fully detach
- Create new session with `setsid()`
- Redirect stdin/stdout/stderr

### Windows
- Use `CREATE_NO_WINDOW` flag
- Detach from console
- May need different approach

## Files to Create/Modify

- `src/jobs/mod.rs` - New: Job management
- `src/jobs/spawn.rs` - New: Background spawning
- `src/jobs/manager.rs` - New: Job listing/cleanup
- `src/lib.rs` - Export jobs module
- `src/cli/args.rs` - Add --bg flag, JobsArgs
- `src/cli/commands/jobs.rs` - New: Jobs command handler
- `src/main.rs` - Handle --bg flag

## Dependencies

```toml
# Cargo.toml
[target.'cfg(unix)'.dependencies]
nix = { version = "0.27", features = ["process", "signal"] }
daemonize = "0.5"
```

## Testing

- Test job creation and metadata
- Test background process spawning
- Test job listing and filtering
- Test log following
- Test job cancellation
- Test cleanup of old jobs
