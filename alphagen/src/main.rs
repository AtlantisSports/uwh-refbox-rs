use alphagen::on_paths;
use clap::Parser;
use log::warn;
use rayon::iter::ParallelBridge;
use rayon::iter::ParallelIterator;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[clap(help = "Input files", required = true)]
    input_location: Vec<PathBuf>,

    #[clap(help = "Output directory", required = true)]
    output_location: PathBuf,
}

fn main() {
    pretty_env_logger::init();
    let args = Args::parse();

    let dir_output = args.output_location;
    if !dir_output.is_dir() {
        std::fs::create_dir_all(&dir_output).expect("Could not create output directory!");
    }

    let paths = args
        .input_location
        .iter()
        .par_bridge()
        .filter(|p| {
            if p.is_file() {
                true
            } else {
                warn!("{} is not a valid file path. Skipping.", p.display());
                false
            }
        })
        .collect::<Vec<_>>();

    assert!(paths.is_empty(), "No valid input file paths!");
    on_paths(paths, dir_output);
}
