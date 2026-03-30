use anyhow::Result;
use std::{
    collections::HashSet,
    ffi::OsStr,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

pub fn parse_exts(exts: &str) -> HashSet<String> {
    exts.split(',')
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn scan_media_files(
    src: &Path,
    recursive: bool,
    allowed_exts: &HashSet<String>,
) -> Result<Vec<PathBuf>> {
    let walker = if recursive {
        WalkDir::new(src)
    } else {
        WalkDir::new(src).max_depth(1)
    };

    let mut out = Vec::new();
    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        if path.is_dir() {
            continue;
        }

        let ext = match path.extension().and_then(OsStr::to_str) {
            Some(e) => e.to_ascii_lowercase(),
            None => continue,
        };

        if allowed_exts.contains(&ext) {
            out.push(path.to_path_buf());
        }
    }

    Ok(out)
}