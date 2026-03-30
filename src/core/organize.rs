use anyhow::Result;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use super::date::{from_exif_str, from_filename, MediaDate};
use super::dedup::hash_file;
use super::exif::read_exif;
use super::scan::scan_media_files;

pub struct OrganizeOptions<'a> {
    pub src: &'a Path,
    pub dest: &'a Path,
    pub recursive: bool,
    pub allowed_exts: &'a HashSet<String>,
    pub dry_run: bool,
    pub copy: bool,
}

pub struct OrganizeResult {
    pub moved: usize,
    pub skipped: usize,
    pub errors: usize,
}

/// Extract a MediaDate for a file: try EXIF first, then fall back to filename.
/// Public alias used by the TUI to build preview lists.
pub fn resolve_date_pub(path: &Path) -> Option<MediaDate> {
    resolve_date(path)
}

fn resolve_date(path: &Path) -> Option<MediaDate> {
    // 1. Try EXIF
    if let Ok(Some(info)) = read_exif(path) {
        if let Some(date_str) = info.date_taken.as_deref() {
            if let Some(d) = from_exif_str(date_str) {
                return Some(d);
            }
        }
    }

    // 2. Fall back to filename (strip all but the last extension to get the stem)
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        // strip a secondary extension like ".TS" in "PXL_20240209_174735042.TS.mp4"
        .map(|s| Path::new(s).file_stem().and_then(|s2| s2.to_str()).unwrap_or(s));

    stem.and_then(from_filename)
}

/// Build a destination path that doesn't collide with existing files.
/// Tries `dir/name.ext`, then `dir/name_2.ext`, `dir/name_3.ext`, …
fn unique_dest(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let name = if ext.is_empty() {
        stem.to_string()
    } else {
        format!("{stem}.{ext}")
    };
    let candidate = dir.join(&name);
    if !candidate.exists() {
        return candidate;
    }

    for n in 2u32.. {
        let name = if ext.is_empty() {
            format!("{stem}_{n}")
        } else {
            format!("{stem}_{n}.{ext}")
        };
        let candidate = dir.join(&name);
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!()
}

pub fn organize(opts: &OrganizeOptions) -> Result<OrganizeResult> {
    let files = scan_media_files(opts.src, opts.recursive, opts.allowed_exts)?;

    let mut result = OrganizeResult {
        moved: 0,
        skipped: 0,
        errors: 0,
    };

    // Track hashes of files already moved to dest, to skip source duplicates.
    // Pre-populate with any files already in dest so re-runs are idempotent.
    let mut seen_hashes: HashMap<String, PathBuf> = HashMap::new();
    if opts.dest.is_dir() {
        let dest_files =
            scan_media_files(opts.dest, true, opts.allowed_exts).unwrap_or_default();
        for p in dest_files {
            if let Ok(h) = hash_file(&p) {
                seen_hashes.insert(h, p);
            }
        }
    }

    for src_path in &files {
        // Use only the final extension (e.g. "mp4" from "file.TS.mp4")
        let ext = src_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        // Skip if an identical file is already in dest
        match hash_file(src_path) {
            Ok(hash) => {
                if let Some(existing) = seen_hashes.get(&hash) {
                    println!(
                        "skip (duplicate of {}): {}",
                        existing.display(),
                        src_path.display()
                    );
                    result.skipped += 1;
                    continue;
                }
                // Record now; insert into seen_hashes after a successful move below
                // (we store the hash temporarily — see insert after move)
                seen_hashes.insert(hash, src_path.clone());
            }
            Err(e) => eprintln!("  warn: could not hash {}: {e}", src_path.display()),
        }

        let (subdir, new_stem) = match resolve_date(src_path) {
            Some(date) => (opts.dest.join(date.subdir()), date.filename_stem()),
            None => {
                // No date found — keep original stem, put in unknown/
                let stem = src_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                (opts.dest.join("unknown"), stem)
            }
        };

        let dest_path = unique_dest(&subdir, &new_stem, &ext);

        println!("{} -> {}", src_path.display(), dest_path.display());

        if opts.dry_run {
            result.moved += 1;
            continue;
        }

        if let Err(e) = fs::create_dir_all(&subdir) {
            eprintln!("  error creating {}: {e}", subdir.display());
            result.errors += 1;
            continue;
        }

        if opts.copy {
            match fs::copy(src_path, &dest_path) {
                Ok(_) => result.moved += 1,
                Err(e) => {
                    eprintln!("  error copying: {e}");
                    result.errors += 1;
                }
            }
        } else {
            match fs::rename(src_path, &dest_path) {
                Ok(_) => result.moved += 1,
                Err(_) => {
                    // rename fails across filesystems — fall back to copy + delete
                    match fs::copy(src_path, &dest_path) {
                        Err(e) => {
                            eprintln!("  error moving (copy phase): {e}");
                            result.errors += 1;
                        }
                        Ok(_) => match fs::remove_file(src_path) {
                            Err(e) => {
                                eprintln!("  error moving (delete phase): {e}");
                                result.errors += 1;
                            }
                            Ok(_) => result.moved += 1,
                        },
                    }
                }
            }
        }
    }

    Ok(result)
}
