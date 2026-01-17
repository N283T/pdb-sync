//! Progress display for rsync operations.

use crate::utils::human_bytes;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// Tracks sync progress by parsing rsync output.
pub struct SyncProgress {
    progress_bar: ProgressBar,
    files_transferred: u64,
    bytes_transferred: u64,
}

impl SyncProgress {
    /// Create a new progress tracker with the given description.
    pub fn new(description: &str) -> Self {
        let progress_bar = ProgressBar::new_spinner();
        progress_bar.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .expect("Invalid progress style"),
        );
        progress_bar.enable_steady_tick(Duration::from_millis(100));
        progress_bar.set_message(description.to_string());

        Self {
            progress_bar,
            files_transferred: 0,
            bytes_transferred: 0,
        }
    }

    /// Parse a line of rsync output and update progress.
    ///
    /// rsync output format varies:
    /// - File transfer: "          1,234 100%   12.34MB/s    0:00:00 (xfr#1, to-chk=99/100)"
    /// - Directory listing: "receiving incremental file list"
    /// - File name: "mmCIF/ab/1abc.cif.gz"
    pub fn parse_line(&mut self, line: &str) {
        // Check if this is a file transfer progress line
        if line.contains("xfr#") || line.contains("to-chk=") {
            // Parse transferred bytes from the line
            if let Some(bytes) = self.parse_bytes(line) {
                self.bytes_transferred += bytes;
            }
            self.files_transferred += 1;
            self.update_message();
        } else if line.ends_with(".gz") || line.ends_with(".cif") || line.ends_with(".pdb") {
            // This is likely a filename being transferred
            self.progress_bar.set_message(format!(
                "Syncing: {} ({} files, {})",
                line,
                self.files_transferred,
                human_bytes(self.bytes_transferred)
            ));
        }
    }

    /// Parse bytes from rsync progress line.
    fn parse_bytes(&self, line: &str) -> Option<u64> {
        // Format: "          1,234 100%   12.34MB/s ..."
        let trimmed = line.trim();
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if !parts.is_empty() {
            // First part should be the byte count with commas
            let bytes_str = parts[0].replace(',', "");
            return bytes_str.parse().ok();
        }
        None
    }

    fn update_message(&mut self) {
        self.progress_bar.set_message(format!(
            "{} files transferred ({})",
            self.files_transferred,
            human_bytes(self.bytes_transferred)
        ));
    }

    /// Finish progress display with a final message.
    pub fn finish(&self, description: &str) {
        self.progress_bar
            .finish_with_message(description.to_string());
    }

    /// Get current transfer statistics.
    pub fn stats(&self) -> (u64, u64) {
        (self.files_transferred, self.bytes_transferred)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bytes() {
        let progress = SyncProgress::new("test");
        assert_eq!(
            progress.parse_bytes("          1,234 100%   12.34MB/s"),
            Some(1234)
        );
        assert_eq!(progress.parse_bytes("1234567 100%"), Some(1234567));
    }
}
