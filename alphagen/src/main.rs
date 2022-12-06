use clap::Parser;
use image::GenericImageView;
use rayon::iter::ParallelBridge;
use rayon::iter::ParallelIterator;
use std::{fs, path::PathBuf};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(long, short, required = true)]
    /// Directory within which to find the color images
    path: PathBuf,

    #[clap(long, short)]
    /// Output directory for the alpha images
    out: Option<PathBuf>,
}

fn main() {
    let args = Cli::parse();

    if let Some(ref out) = args.out {
        if out == &args.path {
            println!("Error: if the <out> arg is specified, it cannot match the <path> arg");
            return;
        }
        assert!(out.is_dir());
    }

    assert!(args.path.is_dir());

    let append_alpha = args.out.is_none();
    let out_dir = args.out.as_ref().unwrap_or(&args.path).to_path_buf();

    let input_paths = fs::read_dir(args.path).unwrap();

    input_paths.par_bridge().for_each(|path| {
        if let Ok(path) = path {
            let file_name: String = path.path().file_stem().unwrap().to_str().unwrap().into();
            let file = image::open(path.path()).unwrap();

            let mut imgbuf = image::GrayAlphaImage::new(file.width(), file.height());
            let mut pixs = imgbuf.pixels_mut();

            let mut out_path = out_dir.clone();
            out_path.push(if append_alpha {
                format!("{file_name}_alpha.png")
            } else {
                format!("{file_name}.png")
            });
            let fout = &mut fs::File::create(out_path).unwrap();

            for pixel in file.pixels() {
                let p = pixs.next().unwrap();
                p.0[0] = pixel.2[3];
                p.0[1] = pixel.2[3];
            }

            imgbuf.write_to(fout, image::ImageFormat::Png).unwrap();
            println!("Completed: {}", path.path().to_str().unwrap());
        }
    });
}
