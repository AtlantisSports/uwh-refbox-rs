use super::{black, green, orange, red, white, yellow};
use iced::{Theme, widget::text::Style};

pub fn black_text(_theme: &Theme) -> Style {
    Style {
        color: Some(black()),
    }
}

pub fn white_text(_theme: &Theme) -> Style {
    Style {
        color: Some(white()),
    }
}

pub fn green_text(_theme: &Theme) -> Style {
    Style {
        color: Some(green()),
    }
}

pub fn yellow_text(_theme: &Theme) -> Style {
    Style {
        color: Some(yellow()),
    }
}

pub fn orange_text(_theme: &Theme) -> Style {
    Style {
        color: Some(orange()),
    }
}

pub fn red_text(_theme: &Theme) -> Style {
    Style { color: Some(red()) }
}
