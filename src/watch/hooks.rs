//! Hook script execution for the watch command.
//!
//! Allows users to run custom scripts when new PDB entries are downloaded.

use crate::error::{PdbSyncError, Result};
use std::path::{Path, PathBuf};
use tokio::process::Command;

/// Hook runner for executing user scripts on new entries
#[derive(Debug)]
pub struct HookRunner {
    script_path: PathBuf,
}

impl HookRunner {
    /// Create a new hook runner for the given script path
    pub fn new(script_path: PathBuf) -> Result<Self> {
        // Verify script exists
        if !script_path.exists() {
            return Err(PdbSyncError::HookExecution(format!(
                "Hook script not found: {}",
                script_path.display()
            )));
        }

        // On Unix, check if it's executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(&script_path).map_err(|e| {
                PdbSyncError::HookExecution(format!("Cannot read script permissions: {}", e))
            })?;
            let permissions = metadata.permissions();
            if permissions.mode() & 0o111 == 0 {
                return Err(PdbSyncError::HookExecution(format!(
                    "Hook script is not executable: {}",
                    script_path.display()
                )));
            }
        }

        Ok(Self { script_path })
    }

    /// Run the hook script for a downloaded PDB entry
    ///
    /// The script receives:
    /// - PDB_ID environment variable: the PDB ID (e.g., "1abc")
    /// - PDB_FILE environment variable: path to the downloaded file
    /// - First argument: PDB ID
    /// - Second argument: file path
    pub async fn run(&self, pdb_id: &str, file_path: &Path) -> Result<()> {
        tracing::debug!(
            "Running hook script {} for {} ({})",
            self.script_path.display(),
            pdb_id,
            file_path.display()
        );

        let output = Command::new(&self.script_path)
            .arg(pdb_id)
            .arg(file_path)
            .env("PDB_ID", pdb_id)
            .env("PDB_FILE", file_path)
            .output()
            .await
            .map_err(|e| {
                PdbSyncError::HookExecution(format!("Failed to execute hook script: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let exit_code = output
                .status
                .code()
                .map_or("unknown".to_string(), |c| c.to_string());

            return Err(PdbSyncError::HookExecution(format!(
                "Hook script failed (exit code {})\nstdout: {}\nstderr: {}",
                exit_code, stdout, stderr
            )));
        }

        // Log stdout if any
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.trim().is_empty() {
            tracing::info!("Hook output for {}: {}", pdb_id, stdout.trim());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_hook_runner_script_not_found() {
        let result = HookRunner::new(PathBuf::from("/nonexistent/script.sh"));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Hook script not found"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_hook_runner_success() {
        let temp_dir = tempdir().unwrap();
        let script_path = temp_dir.path().join("hook.sh");

        // Create a simple script that echoes the arguments
        std::fs::write(&script_path, "#!/bin/bash\necho \"PDB: $1, File: $2\"\n").unwrap();

        // Make it executable
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();

        let runner = HookRunner::new(script_path).unwrap();
        let result = runner.run("1abc", Path::new("/tmp/1abc.cif.gz")).await;

        assert!(result.is_ok());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_hook_runner_failure() {
        let temp_dir = tempdir().unwrap();
        let script_path = temp_dir.path().join("hook.sh");

        // Create a script that exits with error
        std::fs::write(&script_path, "#!/bin/bash\nexit 1\n").unwrap();

        // Make it executable
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();

        let runner = HookRunner::new(script_path).unwrap();
        let result = runner.run("1abc", Path::new("/tmp/1abc.cif.gz")).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exit code 1"));
    }

    #[cfg(unix)]
    #[test]
    fn test_hook_runner_not_executable() {
        let temp_dir = tempdir().unwrap();
        let script_path = temp_dir.path().join("hook.sh");

        // Create a script but don't make it executable
        std::fs::write(&script_path, "#!/bin/bash\necho hello\n").unwrap();

        let result = HookRunner::new(script_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not executable"));
    }
}
