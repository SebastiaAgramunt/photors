use anyhow::{bail, Result};
use clap::Parser;

mod cli;
mod core;
mod tui;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    match cli.command {
        cli::Commands::Scan {
            src,
            recursive,
            exts,
        } => {
            if !src.is_dir() {
                bail!("Source is not a directory: {}", src.display());
            }

            let allowed = crate::core::scan::parse_exts(&exts);
            let files = crate::core::scan::scan_media_files(&src, recursive, &allowed)?;

            println!("Found {} candidate media files:", files.len());
            for p in &files {
                println!("{}", p.display());
                match crate::core::exif::read_exif(p) {
                    Ok(Some(info)) => {
                        if let Some(d) = &info.date_taken {
                            println!("  date:   {d}");
                        }
                        if let Some(make) = &info.make {
                            let model = info.model.as_deref().unwrap_or("");
                            println!("  camera: {make} {model}");
                        }
                        if let (Some(lat), Some(lon)) = (info.gps_lat, info.gps_lon) {
                            println!("  gps:    {lat:.6}, {lon:.6}");
                        }
                    }
                    Ok(None) => println!("  (no EXIF)"),
                    Err(e) => println!("  (EXIF error: {e})"),
                }
            }
        }

        cli::Commands::Ui => {
            crate::tui::run()?;
        }

        cli::Commands::Dedup {
            src,
            recursive,
            exts,
            delete,
            dry_run,
        } => {
            if !src.is_dir() {
                bail!("Source is not a directory: {}", src.display());
            }

            let allowed = crate::core::scan::parse_exts(&exts);
            let files = crate::core::scan::scan_media_files(&src, recursive, &allowed)?;
            println!("Hashing {} files…", files.len());

            let groups = crate::core::dedup::find_duplicates(&files)?;

            if groups.is_empty() {
                println!("No duplicates found.");
            } else {
                let total_dups: usize = groups.iter().map(|g| g.len() - 1).sum();
                println!("Found {} duplicate groups ({} extra files):", groups.len(), total_dups);
                for group in &groups {
                    println!("  keep:  {}", group[0].display());
                    for dup in group.iter().skip(1) {
                        println!("  dup:   {}", dup.display());
                    }
                }

                if delete || dry_run {
                    if dry_run {
                        println!("\nDry run — no files will be deleted.");
                    }
                    let n = crate::core::dedup::delete_duplicates(&groups, dry_run)?;
                    let verb = if dry_run { "would delete" } else { "deleted" };
                    println!("{verb} {n} files.");
                } else {
                    println!("\nRun with --delete to remove duplicates (or --dry-run to preview).");
                }
            }
        }

        cli::Commands::Organize {
            src,
            dest,
            recursive,
            exts,
            dry_run,
            copy,
        } => {
            if !src.is_dir() {
                bail!("Source is not a directory: {}", src.display());
            }

            let allowed = crate::core::scan::parse_exts(&exts);
            let opts = crate::core::organize::OrganizeOptions {
                src: &src,
                dest: &dest,
                recursive,
                allowed_exts: &allowed,
                dry_run,
                copy,
            };

            if dry_run {
                println!("Dry run — no files will be moved.");
            }

            let result = crate::core::organize::organize(&opts)?;
            let verb = if dry_run {
                "would move"
            } else if copy {
                "copied"
            } else {
                "moved"
            };
            println!(
                "\nDone: {} files {verb}, {} errors.",
                result.moved, result.errors
            );
        }
    }

    Ok(())
}