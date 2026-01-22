//! Sync command arguments.

use crate::cli::args::OutputFormat;
use crate::context::AppContext;
use crate::error::Result;
use crate::sync::RsyncFlags;
use clap::Parser;

/// Sync command arguments.
#[derive(Parser, Clone, Debug)]
pub struct SyncArgs {
    /// Name of the custom sync config to run (optional - runs all if not specified)
    #[arg(value_name = "NAME")]
    pub name: Option<String>,

    /// Run all custom sync configs
    #[arg(long)]
    pub all: bool,

    /// Override destination directory
    #[arg(short, long)]
    pub dest: Option<std::path::PathBuf>,

    /// Show what would change (plan mode)
    #[arg(long)]
    pub plan: bool,

    /// Output format for plan mode
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub output: OutputFormat,
}

impl SyncArgs {
    /// Convert args to rsync flags (for CLI override functionality)
    pub fn to_rsync_flags(&self) -> RsyncFlags {
        RsyncFlags::default()
    }

    /// Check if plan mode is enabled
    pub fn is_plan_mode(&self) -> bool {
        self.plan
    }
}

/// Run sync based on arguments.
pub async fn run_sync(args: SyncArgs, ctx: AppContext) -> Result<()> {
    use crate::cli::commands::sync::wwpdb::{run_custom, run_custom_all};

    if args.all {
        run_custom_all(args, ctx).await
    } else if let Some(ref name) = args.name {
        run_custom(name.clone(), args, ctx).await
    } else {
        // No name specified, run all
        run_custom_all(args, ctx).await
    }
}
