use alphagen::{gray_alpha, pre_multiply_on_paths, remove_alpha_on_paths, white_alpha};
use clap::Parser;
use log::warn;
use rayon::iter::ParallelBridge;
use rayon::iter::ParallelIterator;
use std::path::PathBuf;

#[derive(clap::ValueEnum, Default, Debug, Clone, Copy)]
enum Mode {
    GreyAlpha,
    #[default]
    WhiteAlpha,
    RemoveAlpha,
    PreMultiply,
}

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[clap(short, long, default_value = "white-alpha")]
    mode: Mode,

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

    assert!(!paths.is_empty(), "No valid input file paths!");
    match args.mode {
        Mode::GreyAlpha => gray_alpha(paths, dir_output),
        Mode::WhiteAlpha => white_alpha(paths, dir_output),
        Mode::RemoveAlpha => remove_alpha_on_paths(paths, dir_output),
        Mode::PreMultiply => pre_multiply_on_paths(paths, dir_output),
    }
}
