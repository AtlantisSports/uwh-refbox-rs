use image::GenericImageView;
use rayon::iter::ParallelBridge;
use rayon::iter::ParallelIterator;
use std::fs;
fn main() {
    let paths = fs::read_dir("../8k").unwrap();

    paths.par_bridge().for_each(|path| {
        if let Ok(path) = path {
            let file = image::open(path.path()).unwrap();
            let mut imgbuf = image::GrayImage::new(file.width(), file.height());
            let mut pixs = imgbuf.pixels_mut();
            let fout = &mut fs::File::create(&std::path::Path::new(&format!(
                "../alpha/8k/{}",
                path.path().to_str().unwrap()
            )))
            .unwrap();
            for pixel in file.pixels() {
                pixs.next().unwrap().0[0] = pixel.2[3];
            }
            imgbuf.write_to(fout, image::ImageFormat::Png).unwrap();
            println!("Completed: {}", path.path().to_str().unwrap());
        }
    });
}
