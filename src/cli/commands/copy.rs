use crate::cli::args::CopyArgs;
use crate::context::AppContext;
use crate::error::{PdbCliError, Result};
use crate::files::{paths::build_relative_path, FileFormat, PdbId};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};

pub async fn run_copy(args: CopyArgs, ctx: AppContext) -> Result<()> {
    // Collect PDB IDs from args and/or list file
    let mut pdb_ids = args.pdb_ids.clone();

    if let Some(list_path) = &args.list {
        let ids_from_file = read_id_list(list_path).await?;
        pdb_ids.extend(ids_from_file);
    }

    if pdb_ids.is_empty() {
        return Err(PdbCliError::InvalidInput(
            "No PDB IDs provided. Use positional arguments or --list".into(),
        ));
    }

    // Mirror directory is the source
    let mirror_dir = &ctx.pdb_dir;
    if !mirror_dir.exists() {
        return Err(PdbCliError::Path(format!(
            "Mirror directory does not exist: {}",
            mirror_dir.display()
        )));
    }

    // Create destination directory
    fs::create_dir_all(&args.dest).await?;

    let mut success_count = 0;
    let mut errors = Vec::new();

    for id_str in &pdb_ids {
        match PdbId::new(id_str) {
            Ok(pdb_id) => {
                match copy_pdb_file(
                    &pdb_id,
                    args.format,
                    mirror_dir,
                    &args.dest,
                    args.keep_structure,
                    args.symlink,
                )
                .await
                {
                    Ok(dest_path) => {
                        if args.symlink {
                            println!("Linked: {} -> {}", pdb_id, dest_path.display());
                        } else {
                            println!("Copied: {} -> {}", pdb_id, dest_path.display());
                        }
                        success_count += 1;
                    }
                    Err(e) => {
                        eprintln!("Error copying {}: {}", pdb_id, e);
                        errors.push((pdb_id.to_string(), e));
                    }
                }
            }
            Err(e) => {
                eprintln!("Invalid PDB ID '{}': {}", id_str, e);
                errors.push((id_str.clone(), e));
            }
        }
    }

    println!(
        "\nCopied {} of {} file(s) to {}",
        success_count,
        pdb_ids.len(),
        args.dest.display()
    );

    if !errors.is_empty() {
        eprintln!("{} error(s) occurred", errors.len());
    }

    Ok(())
}

async fn copy_pdb_file(
    pdb_id: &PdbId,
    format: FileFormat,
    mirror_dir: &Path,
    dest_dir: &Path,
    keep_structure: bool,
    symlink: bool,
) -> Result<PathBuf> {
    // Build source path in mirror
    let relative_path = build_relative_path(pdb_id, format);
    let source_path = mirror_dir.join(&relative_path);

    if !source_path.exists() {
        return Err(PdbCliError::Path(format!(
            "File not found in mirror: {}",
            source_path.display()
        )));
    }

    // Build destination path
    let dest_path = if keep_structure {
        let dest = dest_dir.join(&relative_path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).await?;
        }
        dest
    } else {
        // Flat: just the filename
        let filename = source_path
            .file_name()
            .ok_or_else(|| PdbCliError::Path("Invalid filename".into()))?;
        dest_dir.join(filename)
    };

    // Check if destination already exists
    if dest_path.exists() {
        return Err(PdbCliError::Path(format!(
            "Destination already exists: {}",
            dest_path.display()
        )));
    }

    if symlink {
        let source_abs = source_path.canonicalize()?;
        #[cfg(unix)]
        fs::symlink(&source_abs, &dest_path).await?;
        #[cfg(windows)]
        fs::symlink_file(&source_abs, &dest_path).await?;
    } else {
        fs::copy(&source_path, &dest_path).await?;
    }

    Ok(dest_path)
}

async fn read_id_list(path: &Path) -> Result<Vec<String>> {
    let file = fs::File::open(path).await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut ids = Vec::new();

    while let Some(line) = lines.next_line().await? {
        let trimmed = line.trim();
        // Skip empty lines and comments
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            ids.push(trimmed.to_string());
        }
    }

    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_id_list() {
        use tokio::io::AsyncWriteExt;

        let temp_dir = tempfile::tempdir().unwrap();
        let list_file = temp_dir.path().join("ids.txt");

        let mut file = fs::File::create(&list_file).await.unwrap();
        file.write_all(b"1abc\n2xyz\n# comment\n\n3def\n")
            .await
            .unwrap();
        file.flush().await.unwrap();

        let ids = read_id_list(&list_file).await.unwrap();
        assert_eq!(ids, vec!["1abc", "2xyz", "3def"]);
    }
}
