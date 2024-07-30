use chrono::{DateTime, Datelike, Local};
use core::fmt;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    time::Instant,
};
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
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_entry(|e| !is_hidden(e) && !is_skipped_dir(e, skipped_dirs))
        .filter_map(|e| e.ok())
        .filter(|e| {
            let file_name = format!(
                "{}",
                e.file_name().to_os_string().to_str().unwrap_or_else(|| "")
            );
            println!("{}", file_name);
            e.file_type().is_dir()
        })
        .collect();
    println!(
        "{:?}",
        top_level_dirs
            .iter()
            .map(|entry| entry
                .file_name()
                .to_os_string()
                .to_str()
                .unwrap_or_else(|| "")
                .to_string())
            .collect::<Vec<_>>()
    );
    println!("Start scan");
    let total_top_level_dirs = top_level_dirs.len() as u64;
    let top_level_pb = ProgressBar::new(total_top_level_dirs);
    top_level_pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}",
            )
            .unwrap()
            .progress_chars("#>-"),
    );

    let re = Regex::new(&format!(r"\.({})$", extensions.join("|"))).unwrap();
    let mut files = Vec::new();

    for top_level_dir in top_level_dirs {
        top_level_pb.set_message(format!(
            "Scanning {}",
            top_level_dir.path().to_string_lossy()
        ));
        let walk_entries: Vec<_> = WalkDir::new(top_level_dir.path())
            .into_iter()
            .filter_entry(|e: &DirEntry| {
                top_level_pb.tick();
                top_level_pb.set_message(format!("Scanning {}", e.path().to_string_lossy()));
                if e.file_type().is_file() {
                    if let Some(path_str) = e.path().to_str() {
                        if re.is_match(path_str) {
                            return true;
                        } else {
                            return false;
                        }
                    }
                }
                !is_hidden(e) && !is_skipped_dir(e, skipped_dirs)
            })
            .filter_map(|e| e.ok())
            .collect();

        for entry in walk_entries {
            if entry.file_type().is_file() {
                let creation_date = get_creation_date(&entry.path().to_path_buf());
                // entry.path().file_name().map(|file_name| {
                //     top_level_pb.set_message(file_name.to_string_lossy().into_owned())
                // });

                top_level_pb.tick();
                files.push(FileInfo {
                    path: entry.path().to_path_buf(),
                    creation_date,
                });
            }
        }

        top_level_pb.inc(1);
    }

    top_level_pb.finish_and_clear();
    files
}

/// Copies multiple files to a specific directory and reports the transfer rate.
///
/// # Arguments
///
/// * `files` - A vector of source file paths.
/// * `destination_dir` - The directory to copy the files to.
///
/// # Returns
///
/// * `Result<(), io::Error>` - Returns `Ok(())` on success, or an `io::Error` on failure.
pub fn copy_files_with_progress(files: &Vec<FileInfo>, destination_dir: &Path) -> io::Result<()> {
    // Ensure the destination directory exists
    if !destination_dir.exists() {
        fs::create_dir_all(destination_dir)?;
    }

    let total_files = files.len() as u64;
    let pb = ProgressBar::new(total_files);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{bar:40.cyan/blue} {pos}/{len} files ({percent}%) ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    let mut total_bytes_copied = 0;
    let mut last_report_time = Instant::now();
    let mut last_bytes_copied = 0;

    for file_info in files{

        let creation_year = file_info.creation_date.unwrap_or_default().year();
        let creation_month = file_info.creation_date.unwrap_or_default().format("%B").to_string();
        

        // Open the source file
        let mut source_file = File::open(file_info.path.clone())?;

        // Create the destination path by joining the destination directory and file name
        let file_name = file_info.path.file_name().unwrap_or_default();
        let image_destination_dir = destination_dir.join(format!("{}",creation_year)).join(creation_month);
        if !image_destination_dir.exists() {
            fs::create_dir_all(&image_destination_dir)?;
        }

         let destination_path = image_destination_dir.join(file_name);
        // Open the destination file
        let mut destination_file = File::create(destination_path)?;

        // Buffer to read chunks of the file
        let mut buffer = [0; 8192]; // Read in chunks of 8192 bytes
     

        loop {
            let bytes_read = source_file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            destination_file.write_all(&buffer[..bytes_read])?;

            // Update total bytes copied
            total_bytes_copied += bytes_read as u64;

            // Periodically update the progress bar and transfer rate
            let now = Instant::now();
            let elapsed_time = now.duration_since(last_report_time).as_secs_f64();
            if elapsed_time >= 1.0 {
                let bytes_since_last = total_bytes_copied - last_bytes_copied;
                let transfer_rate_mbps =
                    (bytes_since_last as f64 * 8.0) / (1024.0 * 1024.0) / elapsed_time;
                last_bytes_copied = total_bytes_copied;
                last_report_time = now;

                pb.set_message(format!("Transfer rate: {:.2} Mbps", transfer_rate_mbps));
            }
        }

        pb.inc(1); // Update the progress bar for each file
    }

    pb.finish_with_message("Files copied.");
    Ok(())
}
