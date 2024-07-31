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

    for file_info in files {
        let creation_year = file_info.creation_date.unwrap_or_default().year();
        let creation_month = file_info
            .creation_date
            .unwrap_or_default()
            .format("%B")
            .to_string();

        // Open the source file
        let mut source_file = File::open(file_info.path.clone())?;

        // Create the destination path by joining the destination directory and file name
        let file_name = file_info.path.file_name().unwrap_or_default();
        let image_destination_dir = destination_dir
            .join(format!("{}", creation_year))
            .join(creation_month);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_get_creation_date() {
        // GIVEN
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_file.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Test content").unwrap();

        //WHEN
        let creation_date = get_creation_date(&file_path);
        
        //THEN
        assert!(creation_date.is_some());
    }

    #[test]
    fn test_is_hidden() {
        // GIVEN
        let dir = tempdir().unwrap();
        let hidden_file_path = dir.path().join(".hidden_file");
        let normal_file_path = dir.path().join("normal_file");

        // Create files
        File::create(&hidden_file_path).unwrap();
        File::create(&normal_file_path).unwrap();

        // Get DirEntry for the created files
        let hidden_file = WalkDir::new(&hidden_file_path)
            .into_iter()
            .next()
            .unwrap()
            .unwrap();
        let normal_file = WalkDir::new(&normal_file_path)
            .into_iter()
            .next()
            .unwrap()
            .unwrap();

        // WHEN THEN
        assert!(is_hidden(&hidden_file));
        assert!(!is_hidden(&normal_file));

    }

    #[test]
    fn test_is_skipped_dir() {
        //GIVEN 
        let dir = tempdir().unwrap();
        let skip_dir_path = dir.path().join("temp");
        let normal_dir_path = dir.path().join("documents");

        // Create directories
        fs::create_dir(&skip_dir_path).unwrap();
        fs::create_dir(&normal_dir_path).unwrap();

        // Get DirEntry for the created directories
        let skip_dir = WalkDir::new(&skip_dir_path)
            .into_iter()
            .next()
            .unwrap()
            .unwrap();
        let normal_dir = WalkDir::new(&normal_dir_path)
            .into_iter()
            .next()
            .unwrap()
            .unwrap();

        // Define directories to skip
        let skipped_dirs = vec!["temp"];

        //WHEN --> THEN
        assert!(is_skipped_dir(&skip_dir, &skipped_dirs));
        assert!(!is_skipped_dir(&normal_dir, &skipped_dirs));

    }

    #[test]
    fn test_collect_files_with_extension() {
        //GIVEN
        // Create a temporary directory
        let dir = tempdir().unwrap();
        let subdir_path = dir.path().join("subdir");
        fs::create_dir(&subdir_path).unwrap();

        // Create some test files
        let txt_file = dir.path().join("test_file.txt");
        let jpg_file = subdir_path.join("image.jpg");
        let skip_file = dir.path().join(".hidden.jpg");
        File::create(&txt_file).unwrap();
        File::create(&jpg_file).unwrap();
        File::create(&skip_file).unwrap();

        // Define file extensions to search for and directories to skip
        let extensions = vec!["jpg"];
        let skipped_dirs = vec!["hidden_dir"];

        //WHEN
        let files =
            collect_files_with_extension(dir.path().to_str().unwrap(), &extensions, &skipped_dirs);

        //THEN --> Check that only the jpg file is collected
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, jpg_file);
    }

    #[test]
    fn test_copy_files_with_progress() -> io::Result<()> {
        // GIVEN
        let src_dir = tempdir().unwrap();
        let dest_dir = tempdir().unwrap();

        // Create some test files in the source directory
        let file1_path = src_dir.path().join("file1.txt");
        let file2_path = src_dir.path().join("file2.txt");

        let mut file1 = File::create(&file1_path)?;
        writeln!(file1, "This is file 1")?;
        let mut file2 = File::create(&file2_path)?;
        writeln!(file2, "This is file 2")?;

        // Set up FileInfo structures for these files with mocked creation dates
        let creation_date = DateTime::parse_from_rfc3339("2023-07-31T12:34:56+00:00")
            .unwrap()
            .with_timezone(&Local);
        let files = vec![
            FileInfo {
                path: file1_path.clone(),
                creation_date: Some(creation_date),
            },
            FileInfo {
                path: file2_path.clone(),
                creation_date: Some(creation_date),
            },
        ];

        // WHEN
        copy_files_with_progress(&files, dest_dir.path())?;

        //THEN --> Verify the files were copied correctly
        let year_dir = dest_dir.path().join("2023");
        let month_dir = year_dir.join("July");

        let copied_file1_path = month_dir.join("file1.txt");
        let copied_file2_path = month_dir.join("file2.txt");

        assert!(copied_file1_path.exists());
        assert!(copied_file2_path.exists());

        // Clean up
        src_dir.close().unwrap();
        dest_dir.close().unwrap();

        Ok(())
    }
}
