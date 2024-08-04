use std::path::Path;

use clap::Parser;
use files::file::{collect_files_with_extension, CopyFile};

use crate::args::Args;
mod args;
mod files;

fn main() {
    let extensions = ["png", "jpeg", "jpg", "gif"];
    let args = Args::parse();
    let dir_str = args.source_dir;
    let dest_dir_str = args.target_dir;

    let skipped_dirs = [
        "Android/Data",
        ".thumbnails",
        "WhatsApp/.Shared",
        "WhatsApp/Media/.Statuses",
        "WhatsApp/.Thumbs",
    ];

    println!("Checking {}", dir_str);
    let files = collect_files_with_extension(&dir_str, &extensions, &skipped_dirs);

    let mut copy_file = CopyFile::new();
    copy_file
        .copy_files_with_progress(&files, &Path::new(&dest_dir_str))
        .unwrap();
    // for file in files {
    //     println!("{}", file);
    // }
}
