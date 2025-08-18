use embedded_graphics::{
    pixelcolor::{Rgb888, RgbColor},
    prelude::*,
};
use iced::Color;
use skip_error::SkipError;
use std::ops::{Deref, DerefMut};

#[derive(Debug)]
pub struct DisplayBuffer<const WIDTH: usize, const HEIGHT: usize>([[Option<Color>; WIDTH]; HEIGHT]);

impl<const WIDTH: usize, const HEIGHT: usize> Default for DisplayBuffer<WIDTH, HEIGHT> {
    fn default() -> Self {
        Self([[None; WIDTH]; HEIGHT])
    }
}

impl<const WIDTH: usize, const HEIGHT: usize> Deref for DisplayBuffer<WIDTH, HEIGHT> {
    type Target = [[Option<Color>; WIDTH]; HEIGHT];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const WIDTH: usize, const HEIGHT: usize> DerefMut for DisplayBuffer<WIDTH, HEIGHT> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<const WIDTH: usize, const HEIGHT: usize> DisplayBuffer<WIDTH, HEIGHT> {
    pub fn clear_buffer(&mut self) {
        for pix in self.iter_mut().flatten() {
            *pix = None;
        }
    }
}

impl<const WIDTH: usize, const HEIGHT: usize> DrawTarget for DisplayBuffer<WIDTH, HEIGHT> {
    type Color = Rgb888;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for (x, y, color) in pixels
            .into_iter()
            .map(
                |Pixel(coord, color)| -> Result<(usize, usize, Self::Color), std::num::TryFromIntError> {
                    let x: usize = coord.x.try_into()?;
                    let y: usize = coord.y.try_into()?;
                    Ok((x, y, color))
                },
            )
            .skip_error()
        {
            // Check if the pixel coordinates are not out of bounds
            if (x < WIDTH) & (y < HEIGHT) {
                let color = Color::from_rgb8(color.r(), color.g(), color.b());
                self[y][x] = Some(color);
            }
        }

        Ok(())
    }
}

impl<const WIDTH: usize, const HEIGHT: usize> OriginDimensions for DisplayBuffer<WIDTH, HEIGHT> {
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}
