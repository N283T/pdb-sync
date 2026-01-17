//! Watch command implementation.

use crate::cli::args::WatchArgs;
use crate::context::AppContext;
use crate::error::Result;
use crate::watch::{WatchConfig, Watcher};

/// Run the watch command
pub async fn run_watch(args: WatchArgs, ctx: AppContext) -> Result<()> {
    // Build configuration
    let config = WatchConfig::from_args(&args, &ctx)?;

    // Create and run watcher
    let mut watcher = Watcher::new(config).await?;
    watcher.run().await
}
