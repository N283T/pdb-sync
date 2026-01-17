use crate::api::RcsbClient;
use crate::cli::{InfoArgs, OutputFormat};
use crate::context::AppContext;
use crate::error::Result;
use crate::files::{build_full_path, FileFormat, PdbId};

/// Run the info command
pub async fn run_info(args: InfoArgs, ctx: AppContext) -> Result<()> {
    let pdb_id = PdbId::new(&args.pdb_id)?;

    if args.local {
        show_local_info(&pdb_id, &ctx, args.output)?;
    } else {
        let client = RcsbClient::new();
        let metadata = client.fetch_entry(&pdb_id).await?;
        print_metadata(&metadata, args.output, args.all)?;
    }

    Ok(())
}

/// Show local file information
fn show_local_info(pdb_id: &PdbId, ctx: &AppContext, output: OutputFormat) -> Result<()> {
    let formats = [FileFormat::CifGz, FileFormat::PdbGz, FileFormat::BcifGz];
    let mut found_files = Vec::new();

    for format in &formats {
        let path = build_full_path(&ctx.pdb_dir, pdb_id, *format);
        if path.exists() {
            let metadata = std::fs::metadata(&path)?;
            found_files.push(LocalFileInfo {
                format: *format,
                path: path.display().to_string(),
                size: metadata.len(),
            });
        }
    }

    match output {
        OutputFormat::Text => {
            println!("PDB ID: {}", pdb_id.as_str().to_uppercase());
            println!("Local directory: {}", ctx.pdb_dir.display());
            println!();

            if found_files.is_empty() {
                println!("No local files found.");
            } else {
                println!("Local files:");
                for file in &found_files {
                    println!(
                        "  {} ({}) - {}",
                        file.format,
                        format_size(file.size),
                        file.path
                    );
                }
            }
        }
        OutputFormat::Json => {
            let output = serde_json::json!({
                "pdb_id": pdb_id.as_str().to_uppercase(),
                "local_dir": ctx.pdb_dir.display().to_string(),
                "files": found_files.iter().map(|f| {
                    serde_json::json!({
                        "format": f.format.to_string(),
                        "path": f.path,
                        "size": f.size,
                    })
                }).collect::<Vec<_>>(),
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Csv => {
            println!("pdb_id,format,path,size");
            for file in &found_files {
                println!(
                    "{},{},{},{}",
                    pdb_id.as_str().to_uppercase(),
                    file.format,
                    csv_escape(&file.path),
                    file.size
                );
            }
        }
        OutputFormat::Ids => {
            // For single entry info, just print the ID
            println!("{}", pdb_id.as_str().to_uppercase());
        }
    }

    Ok(())
}

struct LocalFileInfo {
    format: FileFormat,
    path: String,
    size: u64,
}

fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;

    if size >= MB {
        format!("{:.1} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

/// Escape a string for CSV output (RFC 4180)
fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Print entry metadata in the specified format
fn print_metadata(
    metadata: &crate::api::rcsb::EntryMetadata,
    output: OutputFormat,
    show_all: bool,
) -> Result<()> {
    match output {
        OutputFormat::Text => print_metadata_text(metadata, show_all),
        OutputFormat::Json => print_metadata_json(metadata, show_all),
        OutputFormat::Csv => print_metadata_csv(metadata, show_all),
        OutputFormat::Ids => {
            // For single entry metadata, just print the ID
            println!("{}", metadata.rcsb_id);
            Ok(())
        }
    }
}

fn print_metadata_text(metadata: &crate::api::rcsb::EntryMetadata, show_all: bool) -> Result<()> {
    println!("PDB ID: {}", metadata.rcsb_id);

    if let Some(title) = metadata.title() {
        println!("Title:  {}", title);
    }

    if let Some(date) = metadata.deposit_date() {
        println!("Deposited: {}", date);
    }

    if let Some(date) = metadata.release_date() {
        println!("Released:  {}", date);
    }

    if let Some(method) = metadata.method() {
        println!("Method: {}", method);
    }

    if let Some(resolution) = metadata.resolution() {
        println!("Resolution: {:.2} A", resolution);
    }

    if let Some(count) = metadata.polymer_entity_count() {
        println!("Polymer entities: {}", count);
    }

    if let Some(count) = metadata.assembly_count() {
        println!("Assemblies: {}", count);
    }

    if show_all {
        if let Some(date) = metadata.revision_date() {
            println!("Revised:   {}", date);
        }

        if let Some(weight) = metadata.molecular_weight() {
            println!("Molecular weight: {:.1} Da", weight);
        }
    }

    Ok(())
}

fn print_metadata_json(metadata: &crate::api::rcsb::EntryMetadata, show_all: bool) -> Result<()> {
    let mut output = serde_json::json!({
        "pdb_id": metadata.rcsb_id,
        "title": metadata.title(),
        "deposited": metadata.deposit_date().map(|d| d.to_string()),
        "released": metadata.release_date().map(|d| d.to_string()),
        "method": metadata.method(),
        "resolution": metadata.resolution(),
        "polymer_entities": metadata.polymer_entity_count(),
        "assemblies": metadata.assembly_count(),
    });

    if show_all {
        if let Some(obj) = output.as_object_mut() {
            obj.insert(
                "revised".to_string(),
                serde_json::json!(metadata.revision_date().map(|d| d.to_string())),
            );
            obj.insert(
                "molecular_weight".to_string(),
                serde_json::json!(metadata.molecular_weight()),
            );
        }
    }

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn print_metadata_csv(metadata: &crate::api::rcsb::EntryMetadata, show_all: bool) -> Result<()> {
    // Header
    if show_all {
        println!(
            "pdb_id,title,deposited,released,method,resolution,polymer_entities,assemblies,revised,molecular_weight"
        );
    } else {
        println!("pdb_id,title,deposited,released,method,resolution,polymer_entities,assemblies");
    }

    // Data row - use csv_escape for fields that may contain special characters
    let title = metadata.title().map(csv_escape).unwrap_or_default();
    let deposited = metadata
        .deposit_date()
        .map(|d| d.to_string())
        .unwrap_or_default();
    let released = metadata
        .release_date()
        .map(|d| d.to_string())
        .unwrap_or_default();
    let method = metadata.method().map(csv_escape).unwrap_or_default();
    let resolution = metadata
        .resolution()
        .map(|r| format!("{:.2}", r))
        .unwrap_or_default();
    let polymer_entities = metadata
        .polymer_entity_count()
        .map(|c| c.to_string())
        .unwrap_or_default();
    let assemblies = metadata
        .assembly_count()
        .map(|c| c.to_string())
        .unwrap_or_default();

    if show_all {
        let revised = metadata
            .revision_date()
            .map(|d| d.to_string())
            .unwrap_or_default();
        let molecular_weight = metadata
            .molecular_weight()
            .map(|w| format!("{:.1}", w))
            .unwrap_or_default();

        println!(
            "{},{},{},{},{},{},{},{},{},{}",
            metadata.rcsb_id,
            title,
            deposited,
            released,
            method,
            resolution,
            polymer_entities,
            assemblies,
            revised,
            molecular_weight
        );
    } else {
        println!(
            "{},{},{},{},{},{},{},{}",
            metadata.rcsb_id,
            title,
            deposited,
            released,
            method,
            resolution,
            polymer_entities,
            assemblies
        );
    }

    Ok(())
}
