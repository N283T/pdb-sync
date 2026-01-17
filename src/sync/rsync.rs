use crate::error::{PdbCliError, Result};
use crate::files::FileFormat;
use crate::mirrors::{Mirror, MirrorId};
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

pub struct RsyncOptions {
    pub mirror: MirrorId,
    pub formats: Vec<FileFormat>,
    pub delete: bool,
    pub bwlimit: Option<u32>,
    pub dry_run: bool,
    pub filters: Vec<String>,
}

impl Default for RsyncOptions {
    fn default() -> Self {
        Self {
            mirror: MirrorId::Rcsb,
            formats: vec![FileFormat::Mmcif],
            delete: false,
            bwlimit: None,
            dry_run: false,
            filters: Vec::new(),
        }
    }
}

pub struct RsyncRunner {
    options: RsyncOptions,
}

impl RsyncRunner {
    pub fn new(options: RsyncOptions) -> Self {
        Self { options }
    }

    pub async fn run(&self, dest: &Path) -> Result<()> {
        let mirror = Mirror::get(self.options.mirror);

        for format in &self.options.formats {
            self.sync_format(mirror, *format, dest).await?;
        }

        Ok(())
    }

    async fn sync_format(&self, mirror: &Mirror, format: FileFormat, dest: &Path) -> Result<()> {
        let mut cmd = Command::new("rsync");

        // Base options
        cmd.arg("-avz").arg("--progress");

        // Port specification for RCSB
        if let Some(port) = mirror.rsync_port {
            cmd.arg(format!("--port={}", port));
        }

        // Delete option
        if self.options.delete {
            cmd.arg("--delete");
        }

        // Bandwidth limit
        if let Some(limit) = self.options.bwlimit {
            if limit > 0 {
                cmd.arg(format!("--bwlimit={}", limit));
            }
        }

        // Dry run
        if self.options.dry_run {
            cmd.arg("--dry-run");
        }

        // Include filters
        for filter in &self.options.filters {
            cmd.arg(format!("--include=**/{}*", filter));
        }

        // Source URL
        let source = format!(
            "{}/ftp/data/structures/divided/{}/",
            mirror.rsync_url,
            format.subdir()
        );
        cmd.arg(&source);

        // Destination
        let dest_path = dest.join(format.subdir());
        std::fs::create_dir_all(&dest_path)?;
        cmd.arg(dest_path.to_string_lossy().as_ref());

        tracing::info!("Running rsync from {} to {}", source, dest_path.display());
        tracing::debug!("Command: {:?}", cmd);

        // Execute and stream output
        let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Some(line) = lines.next_line().await? {
                println!("{}", line);
            }
        }

        let status = child.wait().await?;

        if !status.success() {
            return Err(PdbCliError::Rsync(format!(
                "rsync exited with status {}",
                status
            )));
        }

        Ok(())
    }

    /// Build the rsync command for display purposes (dry-run preview)
    pub fn build_command_string(&self, dest: &Path) -> Vec<String> {
        let mirror = Mirror::get(self.options.mirror);
        let mut args = vec![
            "rsync".to_string(),
            "-avz".to_string(),
            "--progress".to_string(),
        ];

        if let Some(port) = mirror.rsync_port {
            args.push(format!("--port={}", port));
        }

        if self.options.delete {
            args.push("--delete".to_string());
        }

        if let Some(limit) = self.options.bwlimit {
            if limit > 0 {
                args.push(format!("--bwlimit={}", limit));
            }
        }

        if self.options.dry_run {
            args.push("--dry-run".to_string());
        }

        for format in &self.options.formats {
            let source = format!(
                "{}/ftp/data/structures/divided/{}/",
                mirror.rsync_url,
                format.subdir()
            );
            args.push(source);
        }

        args.push(dest.to_string_lossy().to_string());

        args
    }
}
