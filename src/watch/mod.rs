//! Watch command module - monitors RCSB for new PDB entries.
//!
//! This module provides functionality to:
//! - Monitor the RCSB PDB for new entries matching specified filters
//! - Automatically download new entries
//! - Execute hook scripts on new entries
//! - Send notifications (desktop or email)

pub mod hooks;
pub mod notify;
pub mod rcsb;
pub mod state;

use crate::cli::args::WatchArgs;
use crate::context::AppContext;
use crate::data_types::DataType;
use crate::download::DownloadResult;
use crate::download::{DownloadOptions, DownloadTask, HttpsDownloader};
use crate::error::{PdbCliError, Result};
use crate::files::{FileFormat, PdbId};
use chrono::NaiveDate;
use hooks::HookRunner;
use notify::{NotificationSender, NotifyConfig};
use rcsb::{RcsbSearchClient, SearchFilters};
use state::WatchState;
use std::path::PathBuf;
use std::time::Duration;

/// Watch configuration parsed from CLI args
pub struct WatchConfig {
    /// Check interval
    pub interval: Duration,
    /// Search filters
    pub filters: SearchFilters,
    /// Data types to download
    pub data_types: Vec<DataType>,
    /// File format
    pub format: FileFormat,
    /// Dry run mode
    pub dry_run: bool,
    /// Notification config
    pub notify_config: Option<NotifyConfig>,
    /// Hook script path
    pub hook_path: Option<PathBuf>,
    /// Destination directory
    pub dest: PathBuf,
    /// Mirror ID
    pub mirror: crate::mirrors::MirrorId,
    /// Run once and exit
    pub once: bool,
    /// Explicit start date
    pub since: Option<NaiveDate>,
}

impl WatchConfig {
    /// Create watch config from CLI args and context
    pub fn from_args(args: &WatchArgs, ctx: &AppContext) -> Result<Self> {
        // Parse interval
        let interval = parse_interval(&args.interval)?;

        // Build search filters
        let filters = SearchFilters {
            method: args.method,
            resolution: args.resolution,
            organism: args.organism.clone(),
        };

        // Parse since date if provided
        let since = if let Some(since_str) = &args.since {
            Some(
                NaiveDate::parse_from_str(since_str, "%Y-%m-%d").map_err(|e| {
                    PdbCliError::InvalidInput(format!("Invalid date format '{}': {}", since_str, e))
                })?,
            )
        } else {
            None
        };

        // Build notification config if requested
        let notify_config = if let Some(method) = args.notify {
            Some(NotifyConfig::new(method, args.email.clone())?)
        } else {
            None
        };

        // Determine data types (default to structures if empty)
        let data_types = if args.data_types.is_empty() {
            vec![DataType::Structures]
        } else {
            args.data_types.clone()
        };

        Ok(Self {
            interval,
            filters,
            data_types,
            format: args.format,
            dry_run: args.dry_run,
            notify_config,
            hook_path: args.on_new.clone(),
            dest: args.dest.clone().unwrap_or_else(|| ctx.pdb_dir.clone()),
            mirror: args.mirror.unwrap_or(ctx.mirror),
            once: args.once,
            since,
        })
    }
}

/// Parse interval string (e.g., "1h", "30m", "1d") to Duration
pub fn parse_interval(s: &str) -> Result<Duration> {
    humantime::parse_duration(s).map_err(|e| PdbCliError::InvalidInterval(format!("{}: {}", s, e)))
}

/// Main watcher struct
pub struct Watcher {
    config: WatchConfig,
    state: WatchState,
    search_client: RcsbSearchClient,
    downloader: HttpsDownloader,
    hook_runner: Option<HookRunner>,
    notifier: Option<NotificationSender>,
}

impl Watcher {
    /// Create a new watcher
    pub async fn new(config: WatchConfig) -> Result<Self> {
        // Load state
        let state = WatchState::load_or_init().await?;

        // Create search client
        let search_client = RcsbSearchClient::new();

        // Create downloader
        let download_options = DownloadOptions {
            mirror: config.mirror,
            parallel: 4,
            retry_count: 3,
            retry_delay: Duration::from_secs(1),
            decompress: false,
            overwrite: false,
        };
        let downloader = HttpsDownloader::new(download_options);

        // Create hook runner if configured
        let hook_runner = if let Some(path) = &config.hook_path {
            Some(HookRunner::new(path.clone())?)
        } else {
            None
        };

        // Create notifier if configured
        let notifier = config.notify_config.clone().map(NotificationSender::new);

        Ok(Self {
            config,
            state,
            search_client,
            downloader,
            hook_runner,
            notifier,
        })
    }

    /// Run the watch loop
    pub async fn run(&mut self) -> Result<()> {
        println!(
            "Starting watch (interval: {:?}, dry_run: {})",
            self.config.interval, self.config.dry_run
        );

        if let Some(method) = &self.config.filters.method {
            println!("  Filter: method = {:?}", method);
        }
        if let Some(resolution) = self.config.filters.resolution {
            println!("  Filter: resolution <= {} Å", resolution);
        }
        if let Some(organism) = &self.config.filters.organism {
            println!("  Filter: organism = {}", organism);
        }

        loop {
            // Run one check cycle
            match self.check_and_download().await {
                Ok(count) => {
                    if count > 0 {
                        println!("Processed {} new entries", count);
                    }
                }
                Err(e) => {
                    eprintln!("Watch cycle error: {}", e);
                }
            }

            // Save state after each cycle
            if let Err(e) = self.state.save().await {
                eprintln!("Failed to save state: {}", e);
            }

            // Exit if --once flag is set
            if self.config.once {
                println!("Single check complete, exiting");
                break;
            }

            // Wait for next interval
            println!(
                "Next check in {:?}... (Ctrl+C to stop)",
                self.config.interval
            );

            // Use tokio::select to handle graceful shutdown
            tokio::select! {
                _ = tokio::time::sleep(self.config.interval) => {
                    // Continue to next iteration
                }
                _ = tokio::signal::ctrl_c() => {
                    println!("\nReceived shutdown signal");
                    break;
                }
            }
        }

        // Save state on exit
        self.state.save().await?;
        println!("State saved");

        Ok(())
    }

    /// Run a single check cycle
    async fn check_and_download(&mut self) -> Result<usize> {
        let start_date = self.state.effective_start_date(self.config.since);
        let today = chrono::Utc::now().date_naive();

        println!(
            "Checking for new entries since {}...",
            start_date.format("%Y-%m-%d")
        );

        // Search for new entries
        let all_ids = self
            .search_client
            .search_new_entries(start_date, &self.config.filters)
            .await?;

        // Filter out already downloaded entries
        let new_ids: Vec<String> = all_ids
            .into_iter()
            .filter(|id| !self.state.is_downloaded(id))
            .collect();

        if new_ids.is_empty() {
            println!("No new entries found");
            self.state.update_last_check(today);
            return Ok(0);
        }

        println!("Found {} new entries", new_ids.len());

        // Dry run - just report
        if self.config.dry_run {
            for id in &new_ids {
                println!("  [dry-run] Would download: {}", id);
            }
            // Don't mark as downloaded or update last_check in dry run
            return Ok(new_ids.len());
        }

        // Download each entry
        let mut downloaded = Vec::new();

        for id_str in &new_ids {
            let pdb_id = match PdbId::new(id_str) {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("Invalid PDB ID '{}': {}", id_str, e);
                    continue;
                }
            };

            // Build download tasks for each data type
            let tasks: Vec<DownloadTask> = self
                .config
                .data_types
                .iter()
                .map(|dt| DownloadTask {
                    pdb_id: pdb_id.clone(),
                    data_type: *dt,
                    format: self.config.format,
                    assembly_number: None,
                })
                .collect();

            // Download
            let results = self
                .downloader
                .download_many(tasks, &self.config.dest)
                .await;

            // Check results
            let mut any_success = false;
            for result in &results {
                match result {
                    DownloadResult::Success { path, .. } => {
                        println!("  Downloaded: {}", path.display());
                        any_success = true;

                        // Run hook if configured
                        if let Some(runner) = &self.hook_runner {
                            if let Err(e) = runner.run(id_str, path).await {
                                eprintln!("  Hook failed for {}: {}", id_str, e);
                            }
                        }
                    }
                    DownloadResult::Failed { error, .. } => {
                        eprintln!("  Download failed for {}: {}", id_str, error);
                    }
                    DownloadResult::Skipped { reason, .. } => {
                        println!("  Skipped {}: {}", id_str, reason);
                        // Count skipped as success to avoid re-downloading
                        any_success = true;
                    }
                }
            }

            if any_success {
                self.state.mark_downloaded(id_str);
                downloaded.push(id_str.clone());
            }
        }

        // Send notification if configured and there are new downloads
        if !downloaded.is_empty() {
            if let Some(notifier) = &self.notifier {
                if let Err(e) = notifier.notify(&downloaded).await {
                    eprintln!("Notification failed: {}", e);
                }
            }
        }

        // Update last check date
        self.state.update_last_check(today);

        Ok(downloaded.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_interval_hours() {
        let result = parse_interval("1h");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Duration::from_secs(3600));
    }

    #[test]
    fn test_parse_interval_minutes() {
        let result = parse_interval("30m");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Duration::from_secs(1800));
    }

    #[test]
    fn test_parse_interval_days() {
        let result = parse_interval("1d");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Duration::from_secs(86400));
    }

    #[test]
    fn test_parse_interval_seconds() {
        let result = parse_interval("30s");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Duration::from_secs(30));
    }

    #[test]
    fn test_parse_interval_combined() {
        let result = parse_interval("1h 30m");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Duration::from_secs(5400));
    }

    #[test]
    fn test_parse_interval_invalid() {
        let result = parse_interval("invalid");
        assert!(result.is_err());
    }
}
