use iced::{button, container, Background, Color, Vector};
use paste::paste;

pub const BORDER_RADIUS: f32 = 9.0;
pub const BORDER_WIDTH: f32 = 6.0;
pub const SPACING: u16 = 12; // Must be a multiple of 4
pub const PADDING: u16 = 12;
pub const MIN_BUTTON_SIZE: u32 = 96;

pub const NORMAL_TEXT: u16 = 24;
pub const MEDIUM_TEXT: u16 = 48;
pub const LARGE_TEXT: u16 = 90;

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
            pub const [<$name _DISABLED>]: iced::Color = iced::Color::from_rgb(
                $r * 0.7,
                $g * 0.7,
                $b * 0.7);
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
pub const BLACK_DISABLED: Color = Color::from_rgb(0.3, 0.3, 0.3);

pub const BORDER_COLOR: Color = Color::from_rgb(0.3, 0.47, 1.0);

pub const WINDOW_BACKGORUND: Color = Color::from_rgb(0.82, 0.82, 0.82);

#[derive(Clone, Copy, Debug)]
pub enum Button {
    White,
    WhiteSelected,
    Black,
    BlackSelected,
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Gray,
    LightGray,
}

impl button::StyleSheet for Button {
    fn active(&self) -> button::Style {
        let (background_color, text_color) = match self {
            Self::White | Self::WhiteSelected => (WHITE, BLACK),
            Self::Black | Self::BlackSelected => (BLACK, WHITE),
            Self::Red => (RED, BLACK),
            Self::Orange => (ORANGE, BLACK),
            Self::Yellow => (YELLOW, BLACK),
            Self::Green => (GREEN, BLACK),
            Self::Blue => (BLUE, WHITE),
            Self::Gray => (GRAY, BLACK),
            Self::LightGray => (LIGHT_GRAY, BLACK),
        };

        let border_width = match self {
            Self::White
            | Self::Black
            | Self::Red
            | Self::Orange
            | Self::Yellow
            | Self::Green
            | Self::Blue
            | Self::Gray
            | Self::LightGray => 0.0,
            Self::WhiteSelected | Self::BlackSelected => BORDER_WIDTH,
        };

        let background = Some(Background::Color(background_color));

        button::Style {
            shadow_offset: Vector::default(),
            background,
            border_radius: BORDER_RADIUS,
            border_width,
            border_color: BORDER_COLOR,
            text_color,
        }
    }

    fn hovered(&self) -> button::Style {
        self.active()
    }

    fn pressed(&self) -> button::Style {
        let background_color = match self {
            Self::White | Self::WhiteSelected => WHITE_PRESSED,
            Self::Black | Self::BlackSelected => BLACK_PRESSED,
            Self::Red => RED_PRESSED,
            Self::Orange => ORANGE_PRESSED,
            Self::Yellow => YELLOW_PRESSED,
            Self::Green => GREEN_PRESSED,
            Self::Blue => BLUE_PRESSED,
            Self::Gray => GRAY_PRESSED,
            Self::LightGray => LIGHT_GRAY_PRESSED,
        };

        button::Style {
            background: Some(Background::Color(background_color)),
            ..self.active()
        }
    }

    fn disabled(&self) -> button::Style {
        let (background_color, text_color) = match self {
            Self::White | Self::WhiteSelected => (WHITE_DISABLED, BLACK),
            Self::Black | Self::BlackSelected => (BLACK_DISABLED, WHITE_DISABLED),
            Self::Red => (RED_DISABLED, BLACK),
            Self::Orange => (ORANGE_DISABLED, BLACK),
            Self::Yellow => (YELLOW_DISABLED, BLACK),
            Self::Green => (GREEN_DISABLED, BLACK),
            Self::Blue => (BLUE_DISABLED, WHITE_DISABLED),
            Self::Gray => (GRAY_DISABLED, BLACK),
            Self::LightGray => (LIGHT_GRAY_DISABLED, BLACK),
        };

        button::Style {
            background: Some(Background::Color(background_color)),
            text_color,
            ..self.active()
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Container {
    LightGray,
    Black,
    White,
}

impl container::StyleSheet for Container {
    fn style(&self) -> container::Style {
        match self {
            Self::LightGray => cont_style(LIGHT_GRAY, BLACK),
            Self::Black => cont_style(BLACK, WHITE),
            Self::White => cont_style(WHITE, BLACK),
        }
    }
}

fn cont_style(bkgnd: Color, text: Color) -> container::Style {
    container::Style {
        text_color: Some(text),
        background: Some(Background::Color(bkgnd)),
        border_radius: BORDER_RADIUS,
        border_width: 0.0,
        border_color: BORDER_COLOR,
    }
}
