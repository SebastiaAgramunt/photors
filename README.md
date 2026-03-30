# photors

A fast, terminal-based photo organizer written in Rust.

Scan your photo library, organize files into date-based folders, find duplicates — all from the CLI or an interactive TUI.

## Features

- **Scan** — list media files in a directory, with EXIF metadata (date, camera, GPS)
- **Organize** — move or copy files into `YYYY/MM/` folders based on EXIF date, with filename fallback for videos
- **Dedup** — find and remove duplicate files using SHA-256 content hashing
- **Interactive TUI** — keyboard-driven interface built with [ratatui](https://github.com/ratatui-org/ratatui)

## Installation

```
cargo build --release
```

The binary will be at `target/release/photors`.

## Usage

### Interactive TUI

```
photors ui
```

Navigate with arrow keys, `Tab` to switch fields, `Enter` to run, `q` to go back.

---

### Scan

List media files and their EXIF metadata.

```
photors scan ~/Photos
photors scan ~/Photos --recursive
photors scan ~/Photos --recursive --exts jpg,jpeg,heic
```

---

### Organize

Move (or copy) files into `dest/YYYY/MM/YYYY.MM.DD_HHMMSS.ext`.

```
# Preview what would happen
photors organize ~/Photos/inbox ~/Photos/sorted --dry-run

# Move files
photors organize ~/Photos/inbox ~/Photos/sorted --recursive

# Copy instead of move
photors organize ~/Photos/inbox ~/Photos/sorted --recursive --copy
```

Files without a readable date (no EXIF, no date in filename) go into `dest/unknown/`.
Re-running is safe — files already present at the destination are skipped by content hash.

---

### Dedup

Find duplicate files by content hash.

```
# Report duplicates
photors dedup ~/Photos --recursive

# Preview which files would be deleted
photors dedup ~/Photos --recursive --dry-run

# Delete duplicates (keeps the first alphabetically in each group)
photors dedup ~/Photos --recursive --delete
```

---

### Options

| Flag | Description |
|------|-------------|
| `-r`, `--recursive` | Recurse into subfolders |
| `--exts` | Comma-separated extensions (default: `jpg,jpeg,png,heic,mp4,mov,m4v,avi`) |
| `--dry-run` | Preview actions without modifying any files |
| `--copy` | Copy files instead of moving them (organize only) |
| `--delete` | Delete duplicates after confirmation (dedup only) |

## Project structure

```
src/
├── main.rs
├── cli/        # Argument parsing (clap)
└── core/
    ├── scan.rs      # Directory traversal and extension filtering
    ├── exif.rs      # EXIF metadata extraction
    ├── date.rs      # Date parsing from EXIF and filenames
    ├── organize.rs  # File move/copy logic
    └── dedup.rs     # SHA-256 deduplication
└── tui/
    ├── mod.rs       # Event loop and keyboard handling
    ├── app.rs       # Application state
    └── ui.rs        # Ratatui rendering
```

## License

MIT
