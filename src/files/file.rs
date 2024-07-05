use chrono::{DateTime, Local};
use walkdir::{WalkDir, DirEntry};
use regex::Regex;
use core::fmt;
use std::{fs, path::PathBuf};


pub struct  FileInfo{
    path: PathBuf,
    creation_date: Option<DateTime<Local>> 
}

impl fmt::Display for FileInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(date) = &self.creation_date {
            write!(f, "{} (created on {})", self.path.display(), date.format("%Y-%m-%d %H:%M:%S"))
        } else {
            write!(f, "{} (creation date not available)", self.path.display())
        }
    }
}

pub fn get_creation_date(path: &PathBuf) -> Option<DateTime<Local>> {
    match fs::metadata(path) {
        Ok(metadata) => {
            match metadata.created() {
                Ok(ctime) => Some(DateTime::<Local>::from(ctime)),
                Err(_) => None,
            }
        },
        Err(_) => None,
    }
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

fn is_skipped_dir(entry: &DirEntry, skipped_dirs: &[&str]) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }
    let dir_path = entry.path().to_str().unwrap_or("");
    skipped_dirs.iter().any(|&skip| dir_path.contains(skip))
   
}

pub fn collect_files_with_extension(dir: &str, extensions: &[&str], skipped_dirs: &[&str]) -> Vec<FileInfo> {
    let mut files: Vec<FileInfo> = Vec::new();
    let re = Regex::new(&format!(r"\.({})$", extensions.join("|"))).unwrap();

    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| !is_hidden(e) && !is_skipped_dir(e, skipped_dirs))
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Some(path_str) = entry.path().to_str() {
                if re.is_match(path_str) {
                    let creation_date = get_creation_date(&entry.path().to_path_buf());
                    files.push(FileInfo {path:entry.path().to_path_buf(),creation_date});
                }
            }
        }
    }

    files
}