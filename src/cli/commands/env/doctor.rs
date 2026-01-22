//! Environment diagnostics command.

use crate::context::AppContext;
use crate::error::{PdbSyncError, Result};
use colored::Colorize;
use std::process::Command;

/// Test file name for checking directory writability.
const WRITE_TEST_FILE: &str = ".pdb-sync-write-test";

/// Result of a diagnostic check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

impl CheckStatus {
    /// Returns the symbol for this status.
    pub fn symbol(&self) -> &str {
        match self {
            CheckStatus::Pass => "\u{2713}", // ✓
            CheckStatus::Warn => "\u{26a0}", // ⚠
            CheckStatus::Fail => "\u{2717}", // ✗
        }
    }

    /// Returns the color code for this status.
    pub fn color(&self) -> colored::Color {
        match self {
            CheckStatus::Pass => colored::Color::Green,
            CheckStatus::Warn => colored::Color::Yellow,
            CheckStatus::Fail => colored::Color::Red,
        }
    }
}

/// A single diagnostic check result.
#[derive(Clone, Debug)]
pub struct Check {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
}

/// Doctor check results summary.
pub struct DoctorReport {
    pub checks: Vec<Check>,
}

impl DoctorReport {
    /// Create a new report from checks.
    pub fn new(checks: Vec<Check>) -> Self {
        Self { checks }
    }

    /// Print the report to stdout.
    pub fn print(&self) {
        println!("Environment Diagnostics");
        println!("======================");
        println!();

        for check in &self.checks {
            let symbol = check.status.symbol();
            let colored_name = check.name.color(check.status.color());
            println!("{} {:<14}{}", symbol, colored_name, check.message);
        }

        println!();
        self.print_summary();
    }

    /// Print the summary message.
    fn print_summary(&self) {
        let fail_count = self
            .checks
            .iter()
            .filter(|c| c.status == CheckStatus::Fail)
            .count();
        let warn_count = self
            .checks
            .iter()
            .filter(|c| c.status == CheckStatus::Warn)
            .count();

        if fail_count > 0 {
            eprintln!(
                "{}",
                format!("{} check(s) failed. Fix the issues above.", fail_count).red()
            );
        } else if warn_count > 0 {
            println!(
                "{}",
                format!("All checks passed (with {} warning(s)).", warn_count).yellow()
            );
        } else {
            println!("{}", "All checks passed.".green());
        }
    }

    /// Returns the appropriate exit code for this report.
    pub fn exit_code(&self) -> i32 {
        let has_fail = self.checks.iter().any(|c| c.status == CheckStatus::Fail);
        let has_warn = self.checks.iter().any(|c| c.status == CheckStatus::Warn);

        if has_fail {
            1
        } else if has_warn {
            2
        } else {
            0
        }
    }
}

/// Check if rsync is available and get its version.
fn check_rsync() -> Check {
    match Command::new("rsync").arg("--version").output() {
        Ok(output) => {
            if output.status.success() {
                let version_line = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .next()
                    .unwrap_or("rsync")
                    .to_string();

                // Extract version more robustly - filter out "rsync" and "version" words
                // and take the next token, which should be the version number
                let version_str = version_line
                    .split_whitespace()
                    .find(|s| {
                        !s.eq_ignore_ascii_case("rsync") && !s.eq_ignore_ascii_case("version")
                    })
                    .unwrap_or("unknown")
                    .to_string();

                Check {
                    name: "rsync".to_string(),
                    status: CheckStatus::Pass,
                    message: format!("v{}", version_str),
                }
            } else {
                Check {
                    name: "rsync".to_string(),
                    status: CheckStatus::Fail,
                    message: "Command failed".to_string(),
                }
            }
        }
        Err(_) => Check {
            name: "rsync".to_string(),
            status: CheckStatus::Fail,
            message: "Not found in PATH".to_string(),
        },
    }
}

/// Check the configuration file.
fn check_config() -> Check {
    use crate::config::ConfigLoader;

    match ConfigLoader::config_path() {
        Some(path) => {
            if path.exists() {
                match std::fs::read_to_string(&path) {
                    Ok(_) => Check {
                        name: "config".to_string(),
                        status: CheckStatus::Pass,
                        message: format!("Found: {}", path.display()),
                    },
                    Err(e) => Check {
                        name: "config".to_string(),
                        status: CheckStatus::Warn,
                        message: format!("Not readable: {}", e),
                    },
                }
            } else {
                Check {
                    name: "config".to_string(),
                    status: CheckStatus::Warn,
                    message: format!("File not found: {}", path.display()),
                }
            }
        }
        None => Check {
            name: "config".to_string(),
            status: CheckStatus::Warn,
            message: "Unable to determine config location".to_string(),
        },
    }
}

/// Check the PDB directory exists and is writable.
fn check_pdb_dir(pdb_dir: &std::path::Path) -> Check {
    if pdb_dir.exists() {
        // Test writability by creating a temp file
        let test_file = pdb_dir.join(WRITE_TEST_FILE);
        match std::fs::write(&test_file, b"test") {
            Ok(_) => {
                let _ = std::fs::remove_file(&test_file);
                Check {
                    name: "pdb_dir".to_string(),
                    status: CheckStatus::Pass,
                    message: format!("Writable: {}", pdb_dir.display()),
                }
            }
            Err(e) => Check {
                name: "pdb_dir".to_string(),
                status: CheckStatus::Fail,
                message: format!("Not writable: {}", e),
            },
        }
    } else {
        Check {
            name: "pdb_dir".to_string(),
            status: CheckStatus::Fail,
            message: format!("Does not exist: {}", pdb_dir.display()),
        }
    }
}

/// Run the environment diagnostics command.
pub fn run_doctor(ctx: AppContext) -> Result<()> {
    let checks = vec![check_rsync(), check_config(), check_pdb_dir(&ctx.pdb_dir)];

    let report = DoctorReport::new(checks);
    report.print();

    let code = report.exit_code();
    if code != 0 {
        Err(PdbSyncError::DoctorFailed { exit_code: code })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_status_symbols() {
        assert_eq!(CheckStatus::Pass.symbol(), "\u{2713}");
        assert_eq!(CheckStatus::Warn.symbol(), "\u{26a0}");
        assert_eq!(CheckStatus::Fail.symbol(), "\u{2717}");
    }

    #[test]
    fn test_report_exit_code_all_pass() {
        let report = DoctorReport::new(vec![
            Check {
                name: "check1".to_string(),
                status: CheckStatus::Pass,
                message: "OK".to_string(),
            },
            Check {
                name: "check2".to_string(),
                status: CheckStatus::Pass,
                message: "OK".to_string(),
            },
        ]);
        assert_eq!(report.exit_code(), 0);
    }

    #[test]
    fn test_report_exit_code_with_fail() {
        let report = DoctorReport::new(vec![
            Check {
                name: "check1".to_string(),
                status: CheckStatus::Pass,
                message: "OK".to_string(),
            },
            Check {
                name: "check2".to_string(),
                status: CheckStatus::Fail,
                message: "Failed".to_string(),
            },
        ]);
        assert_eq!(report.exit_code(), 1);
    }

    #[test]
    fn test_report_exit_code_warn_only() {
        let report = DoctorReport::new(vec![Check {
            name: "check1".to_string(),
            status: CheckStatus::Warn,
            message: "Warning".to_string(),
        }]);
        assert_eq!(report.exit_code(), 2);
    }

    #[test]
    fn test_report_exit_code_fail_and_warn() {
        let report = DoctorReport::new(vec![
            Check {
                name: "check1".to_string(),
                status: CheckStatus::Warn,
                message: "Warning".to_string(),
            },
            Check {
                name: "check2".to_string(),
                status: CheckStatus::Fail,
                message: "Failed".to_string(),
            },
        ]);
        assert_eq!(report.exit_code(), 1);
    }

    #[test]
    fn test_pdb_dir_writable() {
        let temp = tempfile::TempDir::new().unwrap();
        let check = check_pdb_dir(temp.path());
        assert_eq!(check.status, CheckStatus::Pass);
        assert!(check.message.contains("Writable"));
    }

    #[test]
    fn test_pdb_dir_not_exists() {
        let check = check_pdb_dir(std::path::Path::new(
            "/nonexistent/path/that/does/not/exist",
        ));
        assert_eq!(check.status, CheckStatus::Fail);
        assert!(check.message.contains("Does not exist"));
    }
}
