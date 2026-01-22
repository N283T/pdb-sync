//! Sync command arguments.

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

    /// Maximum number of concurrent sync processes (default: 1 = sequential)
    #[arg(long, value_name = "N", default_value = "1")]
    pub parallel: usize,
}

impl SyncArgs {
    /// Convert args to rsync flags (for CLI override functionality)
    pub fn to_rsync_flags(&self) -> RsyncFlags {
        RsyncFlags::default()
    }

    /// Validate the parallel flag value.
    pub fn validate_parallel(&self) -> Result<()> {
        if self.parallel == 0 {
            return Err(crate::error::PdbSyncError::InvalidInput(
                "Parallel must be at least 1".to_string(),
            ));
        }
        if self.parallel > 100 {
            return Err(crate::error::PdbSyncError::InvalidInput(
                "Parallel cannot exceed 100".to_string(),
            ));
        }
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_default() {
        let args = SyncArgs {
            name: None,
            all: false,
            dest: None,
            parallel: 1,
        };
        assert_eq!(args.parallel, 1);
        assert!(args.validate_parallel().is_ok());
    }

    #[test]
    fn test_parallel_validation_zero() {
        let args = SyncArgs {
            name: None,
            all: false,
            dest: None,
            parallel: 0,
        };
        assert!(args.validate_parallel().is_err());
        let err = args.validate_parallel().unwrap_err();
        assert!(err.to_string().contains("at least 1"));
    }

    #[test]
    fn test_parallel_validation_too_large() {
        let args = SyncArgs {
            name: None,
            all: false,
            dest: None,
            parallel: 101,
        };
        assert!(args.validate_parallel().is_err());
        let err = args.validate_parallel().unwrap_err();
        assert!(err.to_string().contains("exceed 100"));
    }

    #[test]
    fn test_parallel_validation_valid() {
        for parallel in [1, 2, 10, 50, 100] {
            let args = SyncArgs {
                name: None,
                all: false,
                dest: None,
                parallel,
            };
            assert!(args.validate_parallel().is_ok());
        }
    }
}
