use iced::{
    Border, Color, Shadow, Theme,
    widget::{
        container::Style as ContainerStyle,
        scrollable::{self, Scroller},
    },
};
use iced_core::border::Radius;
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
pub const XS_BUTTON_SIZE: f32 = 86.0;

pub const SMALL_TEXT: f32 = 19.0;
pub const SMALL_PLUS_TEXT: f32 = 29.0;
pub const MEDIUM_TEXT: f32 = 38.0;
pub const LARGE_TEXT: f32 = 66.0;

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

make_color!(WHITE, 1.0, 1.0, 1.0);
make_color!(RED, 1.0, 0.0, 0.0);
make_color!(ORANGE, 1.0, 0.5, 0.0);
make_color!(YELLOW, 1.0, 1.0, 0.0);
make_color!(GREEN, 0.0, 1.0, 0.0);
make_color!(BLUE, 0.0, 0.0, 1.0);
make_color!(GRAY, 0.5, 0.5, 0.5);
make_color!(LIGHT_GRAY, 0.7, 0.7, 0.7);

pub const BLACK: Color = Color::from_rgb(0.0, 0.0, 0.0);
pub const BLACK_PRESSED: Color = Color::from_rgb(0.15, 0.15, 0.15);

pub const BORDER_COLOR: Color = Color::from_rgb(0.3, 0.47, 1.0);

pub const DISABLED_COLOR: Color = GRAY;

pub const WINDOW_BACKGROUND: Color = Color::from_rgb(0.82, 0.82, 0.82);

pub const SCROLLBAR_COLOR: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.7);

pub mod button;
pub use button::{
    black_button, black_selected_button, blue_button, blue_selected_button,
    blue_with_border_button, gray_button, green_button, green_selected_button, light_gray_button,
    light_gray_selected_button, orange_button, orange_selected_button, red_button,
    red_selected_button, white_button, white_selected_button, yellow_button,
    yellow_selected_button,
};

pub mod container;
pub use container::{
    black_container, blue_container, disabled_container, gray_container, green_container,
    light_gray_container, red_container, scroll_bar_container, transparent_container,
    white_container,
};

pub mod text;
pub use text::{black_text, green_text, orange_text, red_text, white_text, yellow_text};

pub mod svg;
pub use svg::{black_svg, disabled_svg, white_svg};

pub fn scrollable_style(_theme: &Theme, _status: scrollable::Status) -> scrollable::Style {
    let container = ContainerStyle {
        text_color: None,
        background: None,
        border: Border {
            width: 0.0,
            color: Color::TRANSPARENT,
            radius: BORDER_RADIUS_ZERO,
        },
        shadow: Shadow::default(),
    };

    let rail = scrollable::Rail {
        background: None,
        border: Border {
            width: 0.0,
            color: Color::TRANSPARENT,
            radius: BORDER_RADIUS_ZERO,
        },
        scroller: Scroller {
            color: SCROLLBAR_COLOR,
            border: Border {
                width: 0.0,
                color: Color::TRANSPARENT,
                radius: BORDER_RADIUS_ZERO,
            },
        },
    };

    scrollable::Style {
        container,
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
    }
}
