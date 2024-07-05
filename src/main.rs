use files::file::collect_files_with_extension;



mod files;

fn main() {
    let extensions = ["png", "jpeg", "jpg", "gif"];
    let dir = "/tmp";
    let skipped_dirs = ["Android/Data", ".thumbnails", "WhatsApp/.Shared", "WhatsApp/Media/.Statuses", "WhatsApp/.Thumbs"];

    let files = collect_files_with_extension(dir, &extensions, &skipped_dirs);
    
    for file in files {
        println!("{}", file);
    }
}