use crate::error::{PdbCliError, Result};
use crate::files::{FileFormat, PdbId};
use crate::mirrors::{Mirror, MirrorId};
use async_compression::tokio::bufread::GzipDecoder;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};

pub struct DownloadOptions {
    pub mirror: MirrorId,
    pub decompress: bool,
    pub overwrite: bool,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            mirror: MirrorId::Rcsb,
            decompress: false,
            overwrite: false,
        }
    }
}

pub struct HttpsDownloader {
    client: reqwest::Client,
    options: DownloadOptions,
}

impl HttpsDownloader {
    pub fn new(options: DownloadOptions) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("pdb-cli")
            .build()
            .expect("Failed to create HTTP client");

        Self { client, options }
    }

    pub async fn download(&self, pdb_id: &PdbId, format: FileFormat, dest: &Path) -> Result<()> {
        let url = self.build_url(pdb_id, format);
        let dest_file = self.build_dest_path(dest, pdb_id, format);

        if dest_file.exists() && !self.options.overwrite {
            println!("File already exists: {}", dest_file.display());
            return Ok(());
        }

        if let Some(parent) = dest_file.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        println!("Downloading {} ...", url);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(PdbCliError::Download(format!(
                "HTTP {} for {}",
                response.status(),
                url
            )));
        }

        let total_size = response.content_length();

        let pb = ProgressBar::new(total_size.unwrap_or(0));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );

        // Download to temporary file first
        let temp_path = dest_file.with_extension("tmp");
        let mut temp_file = File::create(&temp_path).await?;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            temp_file.write_all(&chunk).await?;
            pb.inc(chunk.len() as u64);
        }

        temp_file.flush().await?;
        drop(temp_file);

        pb.finish_with_message("done");

        // Decompress if requested and file is compressed
        let should_decompress = self.options.decompress && !format.is_compressed();
        if should_decompress && self.is_gzipped(&temp_path).await {
            let decompressed_path = &dest_file;
            self.decompress_file(&temp_path, decompressed_path).await?;
            tokio::fs::remove_file(&temp_path).await?;
            println!("Saved to: {}", decompressed_path.display());
        } else {
            tokio::fs::rename(&temp_path, &dest_file).await?;
            println!("Saved to: {}", dest_file.display());
        }

        Ok(())
    }

    async fn is_gzipped(&self, path: &Path) -> bool {
        if let Ok(mut file) = File::open(path).await {
            let mut magic = [0u8; 2];
            if file.read_exact(&mut magic).await.is_ok() {
                return magic == [0x1f, 0x8b];
            }
        }
        false
    }

    async fn decompress_file(&self, src: &Path, dest: &Path) -> Result<()> {
        let file = File::open(src).await?;
        let reader = BufReader::new(file);
        let mut decoder = GzipDecoder::new(reader);

        let mut output = File::create(dest).await?;
        let mut buffer = Vec::new();
        decoder.read_to_end(&mut buffer).await?;
        output.write_all(&buffer).await?;
        output.flush().await?;

        Ok(())
    }

    fn build_url(&self, pdb_id: &PdbId, format: FileFormat) -> String {
        let mirror = Mirror::get(self.options.mirror);
        let id = pdb_id.as_str();
        let base = format.base_format();

        match self.options.mirror {
            MirrorId::Rcsb => match base {
                FileFormat::Pdb => format!("{}/{}.pdb", mirror.https_base, id),
                FileFormat::Mmcif => format!("{}/{}.cif", mirror.https_base, id),
                FileFormat::Bcif => format!("{}/{}.bcif", mirror.https_base, id),
                _ => unreachable!(),
            },
            MirrorId::Pdbj => match base {
                FileFormat::Pdb => format!("{}?format=pdb&id={}", mirror.https_base, id),
                FileFormat::Mmcif => format!("{}?format=mmcif&id={}", mirror.https_base, id),
                FileFormat::Bcif => format!("{}?format=bcif&id={}", mirror.https_base, id),
                _ => unreachable!(),
            },
            MirrorId::Pdbe => match base {
                FileFormat::Pdb => format!("{}/pdb{}.ent", mirror.https_base, id),
                FileFormat::Mmcif => format!("{}/{}.cif", mirror.https_base, id),
                FileFormat::Bcif => format!("{}/{}.bcif", mirror.https_base, id),
                _ => unreachable!(),
            },
            MirrorId::Wwpdb => {
                let middle = pdb_id.middle_chars();
                match base {
                    FileFormat::Pdb => {
                        format!(
                            "{}/divided/pdb/{}/pdb{}.ent.gz",
                            mirror.https_base, middle, id
                        )
                    }
                    FileFormat::Mmcif => {
                        format!(
                            "{}/divided/mmCIF/{}/{}.cif.gz",
                            mirror.https_base, middle, id
                        )
                    }
                    FileFormat::Bcif => {
                        format!(
                            "{}/divided/bcif/{}/{}.bcif.gz",
                            mirror.https_base, middle, id
                        )
                    }
                    _ => unreachable!(),
                }
            }
        }
    }

    fn build_dest_path(
        &self,
        dest: &Path,
        pdb_id: &PdbId,
        format: FileFormat,
    ) -> std::path::PathBuf {
        let id = pdb_id.as_str();
        dest.join(format!("{}.{}", id, format.extension()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_url_rcsb() {
        let downloader = HttpsDownloader::new(DownloadOptions {
            mirror: MirrorId::Rcsb,
            ..Default::default()
        });
        let pdb_id = PdbId::new("1abc").unwrap();

        assert_eq!(
            downloader.build_url(&pdb_id, FileFormat::Pdb),
            "https://files.rcsb.org/download/1abc.pdb"
        );
        assert_eq!(
            downloader.build_url(&pdb_id, FileFormat::Mmcif),
            "https://files.rcsb.org/download/1abc.cif"
        );
        assert_eq!(
            downloader.build_url(&pdb_id, FileFormat::CifGz),
            "https://files.rcsb.org/download/1abc.cif"
        );
    }

    #[test]
    fn test_build_dest_path() {
        let downloader = HttpsDownloader::new(DownloadOptions::default());
        let pdb_id = PdbId::new("1abc").unwrap();

        let path = downloader.build_dest_path(Path::new("/tmp"), &pdb_id, FileFormat::Mmcif);
        assert_eq!(path, std::path::PathBuf::from("/tmp/1abc.cif"));

        let path = downloader.build_dest_path(Path::new("/tmp"), &pdb_id, FileFormat::CifGz);
        assert_eq!(path, std::path::PathBuf::from("/tmp/1abc.cif.gz"));
    }
}
