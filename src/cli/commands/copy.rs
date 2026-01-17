use crate::cli::args::CopyArgs;
use crate::error::{PdbCliError, Result};
use std::path::Path;

pub async fn run_copy(args: CopyArgs, _ctx: crate::context::AppContext) -> Result<()> {
    let source = &args.source;
    let dest = &args.dest;

    if !source.exists() {
        return Err(PdbCliError::Path(format!(
            "Source does not exist: {}",
            source.display()
        )));
    }

    tokio::fs::create_dir_all(dest).await?;

    if source.is_file() {
        copy_file(source, dest, args.flatten, args.symlink).await?;
    } else if source.is_dir() {
        copy_dir(source, dest, args.flatten, args.symlink).await?;
    }

    println!("Copy complete: {} -> {}", source.display(), dest.display());
    Ok(())
}

async fn copy_file(source: &Path, dest_dir: &Path, _flatten: bool, symlink: bool) -> Result<()> {
    let file_name = source
        .file_name()
        .ok_or_else(|| PdbCliError::Path("Invalid source file name".into()))?;
    let dest_path = dest_dir.join(file_name);

    if symlink {
        let source_abs = source.canonicalize()?;
        #[cfg(unix)]
        tokio::fs::symlink(&source_abs, &dest_path).await?;
        #[cfg(windows)]
        tokio::fs::symlink_file(&source_abs, &dest_path).await?;
        println!(
            "Created symlink: {} -> {}",
            dest_path.display(),
            source_abs.display()
        );
    } else {
        tokio::fs::copy(source, &dest_path).await?;
        println!("Copied: {} -> {}", source.display(), dest_path.display());
    }

    Ok(())
}

async fn copy_dir(source: &Path, dest: &Path, flatten: bool, symlink: bool) -> Result<()> {
    let mut entries = tokio::fs::read_dir(source).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        if path.is_file() {
            if flatten {
                copy_file(&path, dest, flatten, symlink).await?;
            } else {
                let relative = path.strip_prefix(source).unwrap_or(&path);
                let dest_file = dest.join(relative);
                if let Some(parent) = dest_file.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                if symlink {
                    let source_abs = path.canonicalize()?;
                    #[cfg(unix)]
                    tokio::fs::symlink(&source_abs, &dest_file).await?;
                    #[cfg(windows)]
                    tokio::fs::symlink_file(&source_abs, &dest_file).await?;
                } else {
                    tokio::fs::copy(&path, &dest_file).await?;
                }
            }
        } else if path.is_dir() {
            let new_dest = if flatten {
                dest.to_path_buf()
            } else {
                dest.join(path.file_name().unwrap())
            };
            tokio::fs::create_dir_all(&new_dest).await?;
            Box::pin(copy_dir(&path, &new_dest, flatten, symlink)).await?;
        }
    }

    Ok(())
}
