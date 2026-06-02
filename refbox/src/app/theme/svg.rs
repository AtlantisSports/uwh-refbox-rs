use super::{black, disabled_color, white};
use iced::{
    Theme,
    widget::svg::{Status, Style},
};

pub fn white_svg(_theme: &Theme, _status: Status) -> Style {
    Style {
        color: Some(white()),
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
