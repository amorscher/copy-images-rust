

use clap::Parser;




#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Dir to check for images
    #[arg(short, long="sourceDir",value_name = "DIR", value_hint = clap::ValueHint::DirPath)]
    pub source_dir: String,
}
