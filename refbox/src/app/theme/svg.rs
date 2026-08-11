use super::{black, blue, disabled_color, white};
use iced::{
    Theme,
    widget::svg::{Status, Style},
};

pub fn white_svg(_theme: &Theme, _status: Status) -> Style {
    Style {
        color: Some(white()),
    }
}

/// Blue line art, taken from the palette so it follows the display mode.
/// Used for the globe that marks a third-party game source.
pub fn blue_svg(_theme: &Theme, _status: Status) -> Style {
    Style {
        color: Some(blue()),
    }
}

pub fn black_svg(_theme: &Theme, _status: Status) -> Style {
    Style {
        color: Some(black()),
    }
}

pub fn disabled_svg(_theme: &Theme, _status: Status) -> Style {
    Style {
        color: Some(disabled_color()),
    }
}
