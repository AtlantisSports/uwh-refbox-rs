use super::{BLACK, DISABLED_COLOR, WHITE};
use iced::{
    Theme,
    widget::svg::{Status, Style},
};

pub fn white_svg(_theme: &Theme, _status: Status) -> Style {
    Style { color: Some(WHITE) }
}

pub fn black_svg(_theme: &Theme, _status: Status) -> Style {
    Style { color: Some(BLACK) }
}

pub fn disabled_svg(_theme: &Theme, _status: Status) -> Style {
    Style {
        color: Some(DISABLED_COLOR),
    }
}
