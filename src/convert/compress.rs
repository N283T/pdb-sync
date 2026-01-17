//! Compression and decompression utilities for PDB files.

use crate::error::Result;
use async_compression::tokio::bufread::GzipDecoder;
use async_compression::tokio::write::GzipEncoder;
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};

/// Check if a file starts with gzip magic bytes (0x1f 0x8b).
pub async fn is_gzipped(path: &Path) -> std::io::Result<bool> {
    let mut file = File::open(path).await?;
    let mut magic = [0u8; 2];
    if file.read_exact(&mut magic).await.is_ok() {
        Ok(magic == [0x1f, 0x8b])
    } else {
        Ok(false)
    }
}

/// Decompress a gzip file to the destination path.
pub async fn decompress_file(src: &Path, dest: &Path) -> Result<()> {
    let file = File::open(src).await?;
    let reader = BufReader::new(file);
    let mut decoder = GzipDecoder::new(reader);
    let mut output = File::create(dest).await?;
    tokio::io::copy(&mut decoder, &mut output).await?;
    output.flush().await?;
    Ok(())
}

/// Compress a file to gzip format at the destination path.
pub async fn compress_file(src: &Path, dest: &Path) -> Result<()> {
    let mut input = File::open(src).await?;
    let output = File::create(dest).await?;
    let mut encoder = GzipEncoder::new(output);
    tokio::io::copy(&mut input, &mut encoder).await?;
    encoder.shutdown().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_is_gzipped_true() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.gz");

        // Write gzip magic bytes followed by some data
        let mut file = File::create(&path).await.unwrap();
        file.write_all(&[0x1f, 0x8b, 0x08, 0x00]).await.unwrap();
        file.flush().await.unwrap();
        drop(file);

        assert!(is_gzipped(&path).await.unwrap());
    }

    #[tokio::test]
    async fn test_is_gzipped_false() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");

        let mut file = File::create(&path).await.unwrap();
        file.write_all(b"Hello, world!").await.unwrap();
        file.flush().await.unwrap();
        drop(file);

        assert!(!is_gzipped(&path).await.unwrap());
    }

    #[tokio::test]
    async fn test_compress_and_decompress_file() {
        let dir = tempdir().unwrap();
        let original = dir.path().join("original.txt");
        let compressed = dir.path().join("compressed.gz");
        let decompressed = dir.path().join("decompressed.txt");

        // Create original file
        let content = b"Hello, this is a test file for compression!";
        let mut file = File::create(&original).await.unwrap();
        file.write_all(content).await.unwrap();
        file.flush().await.unwrap();
        drop(file);

        // Compress
        compress_file(&original, &compressed).await.unwrap();

        // Verify compressed file has gzip magic bytes
        assert!(is_gzipped(&compressed).await.unwrap());

        // Decompress
        decompress_file(&compressed, &decompressed).await.unwrap();

        // Verify content matches
        let mut decompressed_content = Vec::new();
        let mut file = File::open(&decompressed).await.unwrap();
        file.read_to_end(&mut decompressed_content).await.unwrap();

        assert_eq!(decompressed_content, content);
    }

    #[tokio::test]
    async fn test_is_gzipped_empty_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("empty.txt");

        File::create(&path).await.unwrap();

        // Empty file should not be detected as gzipped
        assert!(!is_gzipped(&path).await.unwrap());
    }

    #[tokio::test]
    async fn test_is_gzipped_single_byte_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("single.txt");

        let mut file = File::create(&path).await.unwrap();
        file.write_all(&[0x1f]).await.unwrap();
        file.flush().await.unwrap();
        drop(file);

        // Single byte (even if it's the first magic byte) should not be gzipped
        assert!(!is_gzipped(&path).await.unwrap());
    }
}
