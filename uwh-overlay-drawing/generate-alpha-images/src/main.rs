use image::GenericImageView;
use rayon::iter::ParallelBridge;
use rayon::iter::ParallelIterator;
use std::fs;
fn main() {
    let paths = vec![
        "../../assets/color/1080/Penalty Shot Flag.png",
        "../../assets/color/1080/Black Timeout Flag.png",
        "../../assets/color/1080/White Timeout Flag.png",
        "../../assets/color/1080/Referee Timeout Flag.png",
    ];
    paths.iter().par_bridge().for_each(|path| {
        let file = image::open(path).unwrap();
        let mut imgbuf = image::GrayAlphaImage::new(file.width(), file.height());
        let mut pixs = imgbuf.pixels_mut();
        let fout = &mut fs::File::create(&std::path::Path::new(&format!(
            "../../assets/alpha/1080/{}",
            std::path::Path::new(path)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
        )))
        .unwrap();
        for pixel in file.pixels() {
            let p = pixs.next().unwrap();
            p.0[0] = pixel.2[3];
            p.0[1] = pixel.2[3];
        }
        imgbuf.write_to(fout, image::ImageFormat::Png).unwrap();
        println!("Completed: {}", path);
    });
}
