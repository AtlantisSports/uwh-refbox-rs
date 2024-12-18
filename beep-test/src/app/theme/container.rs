use super::{BLACK, BORDER_COLOR, BORDER_RADIUS, BORDER_RADIUS_ZERO, LIGHT_GRAY, WHITE};
use iced::{Background, Border, Theme, widget::container::Style};

pub fn light_gray_container(_theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(LIGHT_GRAY)),
        text_color: Some(BLACK),
        border: Border {
            width: 0.0,
            color: BORDER_COLOR,
            radius: BORDER_RADIUS,
        },
        shadow: Default::default(),
    }
}

pub fn square_black_container(_theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(BLACK)),
        text_color: Some(WHITE),
        border: Border {
            width: 0.0,
            color: BORDER_COLOR,
            radius: BORDER_RADIUS_ZERO,
        },
        shadow: Default::default(),
    }
}

pub fn square_light_gray_container(theme: &Theme) -> Style {
    let mut style = light_gray_container(theme);
    style.border.radius = BORDER_RADIUS_ZERO;
    style
}
