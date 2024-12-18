use super::{BLACK, GREEN, ORANGE, RED, WHITE, YELLOW};
use iced::{Theme, widget::text::Style};

pub fn black_text(_theme: &Theme) -> Style {
    Style { color: Some(BLACK) }
}

pub fn white_text(_theme: &Theme) -> Style {
    Style { color: Some(WHITE) }
}

pub fn green_text(_theme: &Theme) -> Style {
    Style { color: Some(GREEN) }
}

pub fn yellow_text(_theme: &Theme) -> Style {
    Style {
        color: Some(YELLOW),
    }
}

pub fn orange_text(_theme: &Theme) -> Style {
    Style {
        color: Some(ORANGE),
    }
}

pub fn red_text(_theme: &Theme) -> Style {
    Style { color: Some(RED) }
}
