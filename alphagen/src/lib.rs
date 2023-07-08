use image::io::Reader;
use image::GenericImageView;
use image::ImageFormat;
use rayon::iter::ParallelBridge;
use rayon::iter::ParallelIterator;
use std::fs;
use std::io::BufWriter;
use std::io::Cursor;
use std::path::PathBuf;

/// Processes all files in `paths` and writes output images to `dir_output`
pub fn on_paths(paths: Vec<&PathBuf>, dir_output: PathBuf) {
    paths.iter().par_bridge().for_each(|path| {
        let file = image::open(path).unwrap();
        let mut output_image_buff = image::GrayAlphaImage::new(file.width(), file.height());
        let mut pixs = output_image_buff.pixels_mut();
        let mut file_out = fs::File::create(dir_output.join(path.file_name().unwrap()))
            .expect("Couldn't create output file");
        for (_, _, alpha_channel) in file.pixels() {
            let p = pixs.next().unwrap();
            p.0[0] = alpha_channel[3];
            p.0[1] = alpha_channel[3];
        }
        output_image_buff
            .write_to(&mut file_out, image::ImageFormat::Png)
            .expect(&format!(
                "Couldn't write to output directory {}/{:?}",
                dir_output.display(),
                path.file_name().unwrap_or_default()
            ));
    });
}

/// Process raw image data
pub fn on_raw(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let img_in = Reader::new(Cursor::new(input))
        .with_guessed_format()?
        .decode()?;
    let mut img_out = image::GrayAlphaImage::new(img_in.width(), img_in.height());
    let mut pixs = img_out.pixels_mut();
    for (_, _, alpha_channel) in img_in.pixels() {
        let p = pixs.next().unwrap();
        p.0[0] = alpha_channel[3];
        p.0[1] = alpha_channel[3];
    }
    let mut writer = BufWriter::new(Cursor::new(Vec::new()));
    img_out.write_to(&mut writer, ImageFormat::Png)?;
    Ok(writer.into_inner()?.into_inner())
}
