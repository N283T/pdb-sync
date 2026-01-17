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
        // Warn if using BinaryCIF with non-RCSB mirror (will fall back to mmCIF)
        if format.base_format() == FileFormat::Bcif && self.options.mirror != MirrorId::Rcsb {
            eprintln!(
                "Warning: BinaryCIF is only available from RCSB. Falling back to mmCIF for {}.",
                self.options.mirror
            );
        }

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

        // Handle compression based on user preference and what mirror provides
        let is_gzipped = self.is_gzipped(&temp_path).await;
        let wants_compressed = format.is_compressed();

        if is_gzipped && !wants_compressed && self.options.decompress {
            // Downloaded gzipped, user wants uncompressed → decompress
            self.decompress_file(&temp_path, &dest_file).await?;
            tokio::fs::remove_file(&temp_path).await?;
            println!("Saved (decompressed) to: {}", dest_file.display());
        } else if is_gzipped && !wants_compressed && !self.options.decompress {
            // Downloaded gzipped, user wants uncompressed but decompress=false
            // Save with corrected extension (.cif.gz instead of .cif)
            let corrected_path =
                dest.join(format!("{}.{}.gz", pdb_id.as_str(), format.extension()));
            tokio::fs::rename(&temp_path, &corrected_path).await?;
            println!(
                "Saved to: {} (compressed, use --decompress to extract)",
                corrected_path.display()
            );
        } else {
            // Normal case: downloaded format matches requested format
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
                // BinaryCIF is served from a different host (models.rcsb.org)
                FileFormat::Bcif => format!("https://models.rcsb.org/{}.bcif", id),
                _ => unreachable!(),
            },
            MirrorId::Pdbj => match base {
                FileFormat::Pdb => format!("{}?format=pdb&id={}", mirror.https_base, id),
                FileFormat::Mmcif => format!("{}?format=mmcif&id={}", mirror.https_base, id),
                // PDBj doesn't support BinaryCIF, fall back to mmCIF
                FileFormat::Bcif => format!("{}?format=mmcif&id={}", mirror.https_base, id),
                _ => unreachable!(),
            },
            MirrorId::Pdbe => match base {
                FileFormat::Pdb => format!("{}/pdb{}.ent", mirror.https_base, id),
                FileFormat::Mmcif => format!("{}/{}.cif", mirror.https_base, id),
                // PDBe doesn't support BinaryCIF, fall back to mmCIF
                FileFormat::Bcif => format!("{}/{}.cif", mirror.https_base, id),
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
                    // wwPDB doesn't support BinaryCIF, fall back to mmCIF
                    FileFormat::Bcif => {
                        format!(
                            "{}/divided/mmCIF/{}/{}.cif.gz",
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
        // BinaryCIF uses models.rcsb.org (different from files.rcsb.org)
        assert_eq!(
            downloader.build_url(&pdb_id, FileFormat::Bcif),
            "https://models.rcsb.org/1abc.bcif"
        );
    }

    #[test]
    fn test_build_url_wwpdb() {
        let downloader = HttpsDownloader::new(DownloadOptions {
            mirror: MirrorId::Wwpdb,
            ..Default::default()
        });
        let pdb_id = PdbId::new("1abc").unwrap();

        // wwPDB provides compressed files via HTTPS
        assert_eq!(
            downloader.build_url(&pdb_id, FileFormat::Mmcif),
            "https://files.wwpdb.org/pub/pdb/data/structures/divided/mmCIF/ab/1abc.cif.gz"
        );
        assert_eq!(
            downloader.build_url(&pdb_id, FileFormat::Pdb),
            "https://files.wwpdb.org/pub/pdb/data/structures/divided/pdb/ab/pdb1abc.ent.gz"
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
