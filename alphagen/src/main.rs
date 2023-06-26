use clap::Parser;
use image::GenericImageView;
use log::warn;
use rayon::iter::ParallelBridge;
use rayon::iter::ParallelIterator;
use std::fs;
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

    let out = args.output_location;
    if !out.is_dir() {
        std::fs::create_dir_all(&out).expect("Could not create output directory!")
    }

    let paths = args
        .input_location
        .iter()
        .filter(|p| {
            if p.is_file() {
                true
            } else {
                warn!("{} is not a valid file path. Skipping.", p.display());
                false
            }
        })
        .collect::<Vec<_>>();
    if paths.is_empty() {
        panic!("No valid input file paths!")
    }

    paths.iter().par_bridge().for_each(|path| {
        let file = image::open(path).unwrap();
        let mut output_image_buff = image::GrayAlphaImage::new(file.width(), file.height());
        let mut pixs = output_image_buff.pixels_mut();
        let fout = &mut fs::File::create({
            let mut p = std::path::PathBuf::from(&out);
            p.push(path.file_name().unwrap().to_str().unwrap());
            p
        })
        .expect("Couldn't create output file");
        for pixel in file.pixels() {
            let p = pixs.next().unwrap();
            p.0[0] = pixel.2[3];
            p.0[1] = pixel.2[3];
        }
        output_image_buff
            .write_to(fout, image::ImageFormat::Png)
            .unwrap();
        println!("Completed: {}", path.display());
    });
}
