use anyhow::Result;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufReader, Read},
    path::{Path, PathBuf},
};

/// SHA-256 hex digest of a file's contents.
pub fn hash_file(path: &Path) -> Result<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Group a list of paths by content hash.
/// Returns only groups with more than one file (the actual duplicates).
pub fn find_duplicates(paths: &[PathBuf]) -> Result<Vec<Vec<PathBuf>>> {
    let mut by_hash: HashMap<String, Vec<PathBuf>> = HashMap::new();

    for path in paths {
        match hash_file(path) {
            Ok(hash) => by_hash.entry(hash).or_default().push(path.clone()),
            Err(e) => eprintln!("  skip (hash error) {}: {e}", path.display()),
        }
    }

    let mut groups: Vec<Vec<PathBuf>> = by_hash
        .into_values()
        .filter(|g| g.len() > 1)
        .collect();

    // Sort groups by their first path for deterministic output
    groups.sort_by(|a, b| a[0].cmp(&b[0]));
    for g in &mut groups {
        g.sort();
    }

    Ok(groups)
}

/// Within each duplicate group keep the first path (sorted), delete the rest.
/// Returns the number of files deleted.
pub fn delete_duplicates(groups: &[Vec<PathBuf>], dry_run: bool) -> io::Result<usize> {
    let mut deleted = 0;
    for group in groups {
        // group is already sorted; keep index 0, remove the rest
        for dup in group.iter().skip(1) {
            if dry_run {
                println!("  would delete: {}", dup.display());
            } else {
                std::fs::remove_file(dup)?;
                println!("  deleted: {}", dup.display());
            }
            deleted += 1;
        }
    }
    Ok(deleted)
}
