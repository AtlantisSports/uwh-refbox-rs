use iced::{Color, Theme, border::Radius, widget::text};
use iced_core::text::LineHeight;
use paste::paste;

pub const BORDER_RADIUS: Radius = Radius {
    top_left: 9.0,
    top_right: 9.0,
    bottom_right: 9.0,
    bottom_left: 9.0,
};
pub const BORDER_RADIUS_ZERO: Radius = Radius {
    top_left: 0.0,
    top_right: 0.0,
    bottom_right: 0.0,
    bottom_left: 0.0,
};
pub const BORDER_WIDTH: f32 = 6.0;
pub const SPACING: f32 = 8.0;
pub const PADDING: f32 = 8.0;
pub const MIN_BUTTON_SIZE: f32 = 89.0;

pub const SMALL_TEXT: f32 = 19.0;
pub const SMALL_PLUS_TEXT: f32 = 29.0;
pub const LARGE_TEXT: f32 = 66.0;
pub const MEDIUM_TEXT: f32 = 38.0;

pub const LINE_HEIGHT: LineHeight = LineHeight::Relative(1.15);

// See https://stackoverflow.com/a/727339 for color mixing math. For darkening colors with pure
// black, the math simplifies to new_r = orig_r * (1 - black_alpha), so we will multiply by the
// value of (1 - black_alpha)
macro_rules! make_color {
    ($name:ident, $r:literal, $g:literal, $b:literal) => {
        paste! {
            pub const $name: iced::Color = iced::Color::from_rgb($r, $g, $b);
            pub const [<$name _PRESSED>]: iced::Color = iced::Color::from_rgb(
                $r * 0.85,
                $g * 0.85,
                $b * 0.85);
        }
    };
}

make_color!(RED, 1.0, 0.0, 0.0);
make_color!(ORANGE, 1.0, 0.5, 0.0);
make_color!(GREEN, 0.0, 1.0, 0.0);
make_color!(GRAY, 0.5, 0.5, 0.5);
make_color!(LIGHT_GRAY, 0.7, 0.7, 0.7);

pub const BLACK: Color = Color::from_rgb(0.0, 0.0, 0.0);
pub const WHITE: Color = Color::from_rgb(1.0, 1.0, 1.0);
pub const YELLOW: Color = Color::from_rgb(1.0, 1.0, 0.0);

pub const BORDER_COLOR: Color = Color::from_rgb(0.3, 0.47, 1.0);

pub const DISABLED_COLOR: Color = GRAY;

pub const WINDOW_BACKGROUND: Color = Color::from_rgb(0.82, 0.82, 0.82);

pub mod button;
pub use button::{gray_button, green_button, light_gray_button, orange_button, red_button};

pub mod container;
pub use container::{light_gray_container, square_black_container, square_light_gray_container};

pub fn yellow_text(_theme: &Theme) -> text::Style {
    text::Style {
        color: Some(YELLOW),
    }
}
