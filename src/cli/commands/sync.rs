use crate::cli::args::SyncArgs;
use crate::context::AppContext;
use crate::error::Result;
use crate::sync::{RsyncOptions, RsyncRunner};

pub async fn run_sync(args: SyncArgs, ctx: AppContext) -> Result<()> {
    let dest = args.dest.unwrap_or_else(|| ctx.pdb_dir.clone());
    let mirror = args.mirror.unwrap_or(ctx.mirror);

    let options = RsyncOptions {
        mirror,
        formats: args.format.to_file_formats(),
        delete: args.delete,
        bwlimit: args.bwlimit,
        dry_run: args.dry_run,
        filters: args.filters,
    };

    let runner = RsyncRunner::new(options);

    if args.dry_run {
        println!("Dry run - would execute:");
        for arg in runner.build_command_string(&dest) {
            print!("{} ", arg);
        }
        println!();
    }

    runner.run(&dest).await?;

    Ok(())
}
