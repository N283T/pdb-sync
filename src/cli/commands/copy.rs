use crate::cli::args::CopyArgs;
use crate::context::AppContext;
use crate::error::{PdbCliError, Result};
use crate::files::{paths::build_relative_path, FileFormat, PdbId};
use crate::utils::IdSource;
use std::path::{Path, PathBuf};
use tokio::fs;

pub async fn run_copy(args: CopyArgs, ctx: AppContext) -> Result<()> {
    // Collect PDB IDs from args, list file, and/or stdin
    let id_source =
        IdSource::collect(args.pdb_ids.clone(), args.list.as_deref(), args.stdin).await?;

    if id_source.is_empty() {
        return Err(PdbCliError::InvalidInput(
            "No PDB IDs provided. Use positional arguments, --list, or --stdin".into(),
        ));
    }

    let pdb_ids = id_source.ids;

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
