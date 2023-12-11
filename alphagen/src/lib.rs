use image::io::Reader;
use image::GenericImageView;
use image::ImageBuffer;
use image::ImageFormat;
use image::Rgba;
use rayon::iter::ParallelBridge;
use rayon::iter::ParallelIterator;
use std::fs;
use std::io::BufWriter;
use std::io::Cursor;
use std::path::PathBuf;

/// Processes all files in `paths` and writes output images to `dir_output`
pub fn on_paths(paths: Vec<&PathBuf>, dir_output: PathBuf) {
    paths.iter().par_bridge().for_each(|path| {
        let file =
            image::open(path).unwrap_or_else(|_| panic!("Couldn't open image at {:?}", path));
        let mut output_image_buff = image::GrayAlphaImage::new(file.width(), file.height());
        let mut pixs = output_image_buff.pixels_mut();
        let mut file_out = fs::File::create(dir_output.join(path.file_name().unwrap()))
            .expect("Couldn't create output file");
        for (_, _, pixel) in file.pixels() {
            let p = pixs.next().unwrap();
            p.0[0] = pixel[3];
            p.0[1] = pixel[3];
        }
        output_image_buff
            .write_to(&mut file_out, image::ImageFormat::Png)
            .unwrap_or_else(|_| {
                panic!(
                    "Couldn't write to output directory {}/{:?}",
                    dir_output.display(),
                    path.file_name().unwrap_or_default()
                )
            });
    });
}

/// Processes all files in `paths` and writes output images to `dir_output`.
/// Removes alpha channel from images.
pub fn remove_alpha_on_paths(paths: Vec<&PathBuf>, dir_output: PathBuf) {
    paths.iter().par_bridge().for_each(|path| {
        let file =
            image::open(path).unwrap_or_else(|_| panic!("Couldn't open image at {:?}", path));
        let mut output_image_buff = image::RgbaImage::new(file.width(), file.height());
        let mut pixs = output_image_buff.pixels_mut();
        let mut file_out = fs::File::create(dir_output.join(path.file_name().unwrap()))
            .expect("Couldn't create output file");
        for (_, _, pixel) in file.pixels() {
            let p = pixs.next().unwrap();
            p.0[0] = pixel[0];
            p.0[1] = pixel[1];
            p.0[2] = pixel[2];
            if pixel[3] == 0 {
                p.0[3] = 0;
            } else {
                p.0[3] = 255;
            }
        }
        output_image_buff
            .write_to(&mut file_out, image::ImageFormat::Png)
            .unwrap_or_else(|_| {
                panic!(
                    "Couldn't write to output directory {}/{:?}",
                    dir_output.display(),
                    path.file_name().unwrap_or_default()
                )
            });
    });
}

/// Process raw image data
pub fn on_raw(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let img_in = Reader::new(Cursor::new(input))
        .with_guessed_format()?
        .decode()?;
    let mut img_out = image::GrayAlphaImage::new(img_in.width(), img_in.height());
    let mut pixs = img_out.pixels_mut();
    for (_, _, pixel) in img_in.pixels() {
        let p = pixs.next().unwrap();
        p.0[0] = pixel[3];
        p.0[1] = pixel[3];
    }
    let mut writer = BufWriter::new(Cursor::new(Vec::new()));
    img_out.write_to(&mut writer, ImageFormat::Png)?;
    Ok(writer.into_inner()?.into_inner())
}

/// Process raw rgba8 image data
pub fn on_raw_rgba8(
    width: u32,
    height: u32,
    buff: Vec<u8>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let img_in: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_raw(width, height, buff).unwrap();
    let mut img_out = image::GrayAlphaImage::new(img_in.width(), img_in.height());
    let mut pixs = img_out.pixels_mut();
    for Rgba([_, _, _, alpha_channel]) in img_in.pixels() {
        let p = pixs.next().unwrap();
        p.0[0] = *alpha_channel;
        p.0[1] = *alpha_channel;
    }
    let mut writer = BufWriter::new(Cursor::new(Vec::new()));
    img_out.write_to(&mut writer, ImageFormat::Png)?;
    Ok(writer.into_inner()?.into_inner())
}
