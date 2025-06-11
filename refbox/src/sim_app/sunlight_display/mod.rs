use core::ops::Index;
use iced::{
    Color, Font, Point, Size,
    alignment::{Horizontal, Vertical},
};
use iced_core::text::{LineHeight, Shaping};
use iced_graphics::geometry::Text;
use led_panel_sim::DisplayState;

mod signal;
use signal::Signal;

mod leds;
use leds::LEDS;

const WHITE: Color = Color::from_rgb(1.0, 1.0, 1.0);
const TEXT_WHITE: Color = Color::from_rgb(0.75, 0.75, 0.75);
const GREEN: Color = Color::from_rgb(0.0, 1.0, 0.0);
const GRAY: Color = Color::from_rgb(0.125, 0.125, 0.125);
const BLUE: Color = Color::from_rgb(0.0, 0.0, 0.5);

// LED sizes
pub const HORIZONTAL: Size = Size::new(3.5, 2.8);
pub const VERTICAL: Size = Size::new(2.8, 3.5);

pub const PANEL_WIDTH: f32 = 637.0;
pub const PANEL_HEIGHT: f32 = 192.0;

pub fn calculate_scale(frame_width: f32, frame_height: f32) -> f32 {
    let scale_x = frame_width / PANEL_WIDTH;
    let scale_y = frame_height / PANEL_HEIGHT;

    scale_x.min(scale_y)
}

pub fn static_rectangles(scale: f32) -> impl Iterator<Item = (Point, Size, Option<f32>, Color)> {
    [(
        Point::new(0.0, 0.0),
        Size::new(PANEL_WIDTH * scale, PANEL_HEIGHT * scale),
        None,
        BLUE,
    )]
    .into_iter()
}

pub fn static_text(scale: f32) -> impl Iterator<Item = Text> {
    // This scale is determined by trial and error. It is not based on any
    // real unit conversion that I am aware of. It seems like it should be
    // based the pt to mm conversion factor, but that is 2.8346456675, which
    // is not the correct value
    const TEXT_SCALE: f32 = 1.4;

    [
        Text {
            content: "OT".to_string(),
            position: Point::new(350.775 * scale, 13.4 * scale),
            color: TEXT_WHITE,
            size: (18.0 * TEXT_SCALE * scale).into(),
            line_height: LineHeight::Relative(1.0),
            font: Font::with_name("Helvetica"),
            horizontal_alignment: Horizontal::Left,
            vertical_alignment: Vertical::Center,
            shaping: Shaping::Basic,
        },
        Text {
            content: "SD".to_string(),
            position: Point::new(350.775 * scale, 35.39 * scale),
            color: TEXT_WHITE,
            size: (18.0 * TEXT_SCALE * scale).into(),
            line_height: LineHeight::Relative(1.0),
            font: Font::with_name("Helvetica"),
            horizontal_alignment: Horizontal::Left,
            vertical_alignment: Vertical::Center,
            shaping: Shaping::Basic,
        },
    ]
    .into_iter()
}

pub fn led_panel_rectangles(
    state: &DisplayState,
    scale: f32,
) -> impl Iterator<Item = (Point, Size, Option<f32>, Color)> + '_ {
    LEDS.iter().map(move |(point, size, angle, signal, color)| {
        let point = Point::new(
            (point.x - (size.width / 2.0)) * scale,
            (point.y - (size.height / 2.0)) * scale,
        );
        let size = Size::new(size.width * scale, size.height * scale);
        let color = if state[*signal] { *color } else { GRAY };
        (point, size, *angle, color)
    })
}
