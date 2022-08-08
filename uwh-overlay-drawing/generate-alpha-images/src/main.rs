use image::GenericImageView;
use rayon::iter::ParallelBridge;
use rayon::iter::ParallelIterator;
use std::fs;
fn main() {
    let paths = fs::read_dir("../assetst/color/1080").unwrap();

    paths.par_bridge().for_each(|path| {
        if let Ok(path) = path {
            let file = image::open(path.path()).unwrap();
            let mut imgbuf = image::GrayAlphaImage::new(file.width(), file.height());
            let mut pixs = imgbuf.pixels_mut();
            let fout = &mut fs::File::create(&std::path::Path::new(&format!(
                "../assetst/alpha/1080/{}",
                path.path().file_name().unwrap().to_str().unwrap()
            )))
            .unwrap();
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
