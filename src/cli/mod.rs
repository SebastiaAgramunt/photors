use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "photors")]
#[command(about = "Photo organizer: scan now, rename/move next")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Scan a folder and print the media files we would consider organizing
    Scan {
        /// Source directory
        src: PathBuf,

        /// Recurse into subfolders
        #[arg(short, long)]
        recursive: bool,

        /// Allowed extensions (comma-separated), e.g. jpg,jpeg,png,heic,mp4,mov
        #[arg(long, default_value = "jpg,jpeg,png,heic,mp4,mov,m4v,avi")]
        exts: String,
    },

    /// Find (and optionally delete) duplicate media files in a directory
    Dedup {
        /// Directory to scan for duplicates
        src: PathBuf,

        /// Recurse into subfolders
        #[arg(short, long)]
        recursive: bool,

        /// Allowed extensions (comma-separated)
        #[arg(long, default_value = "jpg,jpeg,png,heic,mp4,mov,m4v,avi")]
        exts: String,

        /// Delete duplicates, keeping the first (alphabetically) in each group
        #[arg(long)]
        delete: bool,

        /// Preview what would be deleted without removing any files (implies --delete output)
        #[arg(long)]
        dry_run: bool,
    },

    /// Launch the interactive TUI
    Ui,

    /// Organize media files into dest/YYYY/MM/ based on EXIF date
    Organize {
        /// Source directory
        src: PathBuf,

        /// Destination directory (will be created if it doesn't exist)
        dest: PathBuf,

        /// Recurse into subfolders
        #[arg(short, long)]
        recursive: bool,

        /// Allowed extensions (comma-separated)
        #[arg(long, default_value = "jpg,jpeg,png,heic,mp4,mov,m4v,avi")]
        exts: String,

        /// Preview what would happen without moving any files
        #[arg(long)]
        dry_run: bool,

        /// Copy files instead of moving them
        #[arg(long)]
        copy: bool,
    },
}