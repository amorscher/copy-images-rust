use chrono::{DateTime, Local};
use core::fmt;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use std::{fs, path::PathBuf};
use walkdir::{DirEntry, WalkDir};

pub struct FileInfo {
    path: PathBuf,
    creation_date: Option<DateTime<Local>>,
}

impl fmt::Display for FileInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(date) = &self.creation_date {
            write!(
                f,
                "{} (created on {})",
                self.path.display(),
                date.format("%Y-%m-%d %H:%M:%S")
            )
        } else {
            write!(f, "{} (creation date not available)", self.path.display())
        }
    }
}

pub fn get_creation_date(path: &PathBuf) -> Option<DateTime<Local>> {
    match fs::metadata(path) {
        Ok(metadata) => match metadata.modified() {
            Ok(ctime) => Some(DateTime::<Local>::from(ctime)),
            Err(_) => None,
        },
        Err(_) => None,
    }
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.') && s != "." && !s.starts_with("./") && !s.starts_with(".."))
        .unwrap_or(false)
}

fn is_skipped_dir(entry: &DirEntry, skipped_dirs: &[&str]) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }
    let dir_path = entry.path().to_str().unwrap_or("");
    skipped_dirs.iter().any(|&skip| dir_path.contains(skip))
}
// Function to collect files with specific extensions, showing progress
pub fn collect_files_with_extension(
    dir: &str,
    extensions: &[&str],
    skipped_dirs: &[&str],
) -> Vec<FileInfo> {
    // Collect all top-level directories
    let top_level_dirs: Vec<_> = WalkDir::new(dir)
        .max_depth(1)
        .into_iter()
        .filter_entry(|e| !is_hidden(e) && !is_skipped_dir(e, skipped_dirs))
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
        .collect();

    let total_top_level_dirs = top_level_dirs.len() as u64;
    let top_level_pb = ProgressBar::new(total_top_level_dirs);
    top_level_pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )
            .unwrap()
            .progress_chars("#>-"),
    );

    let re = Regex::new(&format!(r"\.({})$", extensions.join("|"))).unwrap();
    let mut files = Vec::new();

    for top_level_dir in top_level_dirs {
        let walk_entries: Vec<_> = WalkDir::new(top_level_dir.path())
            .into_iter()
            .filter_entry(|e| !is_hidden(e) && !is_skipped_dir(e, skipped_dirs))
            .filter_map(|e| e.ok())
            .collect();

        for entry in walk_entries {
            if entry.file_type().is_file() {
                if let Some(path_str) = entry.path().to_str() {
                    if re.is_match(path_str) {
                        let creation_date = get_creation_date(&entry.path().to_path_buf());
                        files.push(FileInfo {
                            path: entry.path().to_path_buf(),
                            creation_date,
                        });
                    }
                }
            }
        }

        top_level_pb.inc(1);
    }

    top_level_pb.finish_and_clear();
    files
}
