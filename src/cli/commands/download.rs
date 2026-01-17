use crate::cli::args::DownloadArgs;
use crate::context::AppContext;
use crate::download::{DownloadOptions, HttpsDownloader};
use crate::error::Result;
use crate::files::PdbId;

pub async fn run_download(args: DownloadArgs, ctx: AppContext) -> Result<()> {
    let dest = args.dest.unwrap_or_else(|| ctx.pdb_dir.clone());
    let mirror = args.mirror.unwrap_or(ctx.mirror);

    let options = DownloadOptions {
        mirror,
        decompress: args.decompress || ctx.config.download.auto_decompress,
        overwrite: args.overwrite,
    };

    let downloader = HttpsDownloader::new(options);

    let mut errors = Vec::new();

    for id_str in &args.pdb_ids {
        match PdbId::new(id_str) {
            Ok(pdb_id) => {
                println!("Downloading {} ({})...", pdb_id, args.format);
                if let Err(e) = downloader.download(&pdb_id, args.format, &dest).await {
                    eprintln!("Error downloading {}: {}", pdb_id, e);
                    errors.push((pdb_id.to_string(), e));
                }
            }
            Err(e) => {
                eprintln!("Invalid PDB ID '{}': {}", id_str, e);
                errors.push((id_str.clone(), e));
            }
        }
    }

    if !errors.is_empty() {
        eprintln!("\n{} download(s) failed:", errors.len());
        for (id, err) in &errors {
            eprintln!("  {}: {}", id, err);
        }
    }

    let success_count = args.pdb_ids.len() - errors.len();
    println!(
        "\nDownloaded {} of {} file(s) to {}",
        success_count,
        args.pdb_ids.len(),
        dest.display()
    );

    Ok(())
}
