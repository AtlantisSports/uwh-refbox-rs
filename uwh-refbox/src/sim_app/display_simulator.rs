use embedded_graphics::{
    pixelcolor::{PixelColor, Rgb888, RgbColor},
    prelude::*,
};
use iced::{
    pure::{
        column, container, row,
        widget::{
            container::{Style, StyleSheet},
            Column, Space,
        },
    },
    Background, Color, Length,
};
use skip_error::SkipError;
use std::convert::TryInto;

#[derive(Debug)]
pub struct DisplaySimulator<const WIDTH: usize, const HEIGHT: usize, P>
where
    P: PixelColor + Default + Copy + Into<Rgb888>,
{
    buffer: [[P; WIDTH]; HEIGHT],
    scale: u16,
    spacing: u16,
}

impl<const WIDTH: usize, const HEIGHT: usize, P: PixelColor + Default + Copy + Into<Rgb888>>
    DisplaySimulator<WIDTH, HEIGHT, P>
{
    pub fn new(scale: u16, spacing: u16) -> Self {
        Self {
            buffer: [[Default::default(); WIDTH]; HEIGHT],
            scale,
            spacing,
        }
    }
}

impl<
        'a,
        const WIDTH: usize,
        const HEIGHT: usize,
        P: 'a + PixelColor + Default + Copy + Into<Rgb888>,
    > DisplaySimulator<WIDTH, HEIGHT, P>
{
    pub fn clear_buffer(&mut self) {
        for pix in self.buffer.iter_mut().flatten() {
            *pix = Default::default();
        }
    }

    pub fn view<M: 'a>(&self) -> Column<'a, M> {
        let view = column()
            .spacing(self.spacing)
            .padding(self.spacing)
            .height(Length::Fill)
            .width(Length::Fill);

        self.buffer.iter().fold(view, |view, color_row| {
            let view_row = row()
                .spacing(self.spacing)
                .height(Length::Fill)
                .width(Length::Fill);

            view.push(color_row.iter().fold(view_row, |view_row, color| {
                view_row.push(
                    container(Space::new(
                        Length::Units(self.scale),
                        Length::Units(self.scale),
                    ))
                    .style(ColorStyle::new(*color))
                    .height(Length::Fill)
                    .width(Length::Fill),
                )
            }))
        })
    }
}

impl<const WIDTH: usize, const HEIGHT: usize, P: PixelColor + Default + Copy + Into<Rgb888>>
    DrawTarget for DisplaySimulator<WIDTH, HEIGHT, P>
{
    type Color = P;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for (x, y, color) in pixels
            .into_iter()
            .map(
                |Pixel(coord, color)| -> Result<(usize, usize, P), std::num::TryFromIntError> {
                    let x: usize = coord.x.try_into()?;
                    let y: usize = coord.y.try_into()?;
                    Ok((x, y, color))
                },
            )
            .skip_error()
        {
            // Check if the pixel coordinates are not out of bounds
            if (x < WIDTH) & (y < HEIGHT) {
                self.buffer[y][x] = color;
            }
        }

        Ok(())
    }

    fn fill_contiguous<I>(
        &mut self,
        area: &embedded_graphics::primitives::Rectangle,
        colors: I,
    ) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        self.draw_iter(
            area.points()
                .zip(colors)
                .map(|(pos, color)| Pixel(pos, color)),
        )
    }

    fn fill_solid(
        &mut self,
        area: &embedded_graphics::primitives::Rectangle,
        color: Self::Color,
    ) -> Result<(), Self::Error> {
        self.fill_contiguous(area, core::iter::repeat(color))
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        self.fill_solid(&self.bounding_box(), color)
    }
}

impl<const WIDTH: usize, const HEIGHT: usize, P: PixelColor + Default + Copy + Into<Rgb888>>
    OriginDimensions for DisplaySimulator<WIDTH, HEIGHT, P>
{
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}

#[derive(Clone, Copy, Debug)]
struct ColorStyle<P: PixelColor + Default + Copy + Into<Rgb888>> {
    color: P,
}

impl<P: PixelColor + Default + Copy + Into<Rgb888>> ColorStyle<P> {
    fn new(color: P) -> Self {
        Self { color }
    }
}

impl<P: PixelColor + Default + Copy + Into<Rgb888>> StyleSheet for ColorStyle<P> {
    fn style(&self) -> Style {
        let rgb: Rgb888 = self.color.into();
        let color = Color::from_rgb8(rgb.r(), rgb.g(), rgb.b());
        Style {
            text_color: None,
            background: Some(Background::Color(color)),
            border_radius: 0.0,
            border_width: 0.0,
            border_color: Default::default(),
        }
    }
}
