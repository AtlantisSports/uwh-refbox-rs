use super::{
    BLACK, BORDER_COLOR, BORDER_RADIUS, BORDER_WIDTH, DISABLED_COLOR, GRAY, GRAY_PRESSED, GREEN,
    GREEN_PRESSED, LIGHT_GRAY, LIGHT_GRAY_PRESSED, ORANGE, ORANGE_PRESSED, RED, RED_PRESSED,
    WINDOW_BACKGROUND,
};
use iced::{
    Background, Border, Shadow, Theme,
    widget::button::{Status, Style},
};

pub fn gray_button(_theme: &Theme, status: Status) -> Style {
    let background = match status {
        Status::Disabled => Some(Background::Color(WINDOW_BACKGROUND)),
        Status::Pressed => Some(Background::Color(GRAY_PRESSED)),
        Status::Active | Status::Hovered => Some(Background::Color(GRAY)),
    };

    let text_color = if matches!(status, Status::Disabled) {
        DISABLED_COLOR
    } else {
        BLACK
    };

    let border_color = match status {
        Status::Disabled => DISABLED_COLOR,
        Status::Pressed | Status::Active | Status::Hovered => BORDER_COLOR,
    };

    let border_width = match status {
        Status::Disabled => BORDER_WIDTH,
        Status::Pressed | Status::Active | Status::Hovered => 0.0,
    };

    let border = Border {
        width: border_width,
        color: border_color,
        radius: BORDER_RADIUS,
    };

    Style {
        background,
        text_color,
        border,
        shadow: Shadow::default(),
    }
}

pub fn light_gray_button(theme: &Theme, status: Status) -> Style {
    let background = match status {
        Status::Disabled => Some(Background::Color(WINDOW_BACKGROUND)),
        Status::Pressed => Some(Background::Color(LIGHT_GRAY_PRESSED)),
        Status::Active | Status::Hovered => Some(Background::Color(LIGHT_GRAY)),
    };

    Style {
        background,
        ..gray_button(theme, status)
    }
}

pub fn red_button(theme: &Theme, status: Status) -> Style {
    let background = match status {
        Status::Disabled => Some(Background::Color(WINDOW_BACKGROUND)),
        Status::Pressed => Some(Background::Color(RED_PRESSED)),
        Status::Active | Status::Hovered => Some(Background::Color(RED)),
    };

    Style {
        background,
        ..gray_button(theme, status)
    }
}

pub fn orange_button(theme: &Theme, status: Status) -> Style {
    let background = match status {
        Status::Disabled => Some(Background::Color(WINDOW_BACKGROUND)),
        Status::Pressed => Some(Background::Color(ORANGE_PRESSED)),
        Status::Active | Status::Hovered => Some(Background::Color(ORANGE)),
    };

    Style {
        background,
        ..gray_button(theme, status)
    }
}

pub fn green_button(theme: &Theme, status: Status) -> Style {
    let background = match status {
        Status::Disabled => Some(Background::Color(WINDOW_BACKGROUND)),
        Status::Pressed => Some(Background::Color(GREEN_PRESSED)),
        Status::Active | Status::Hovered => Some(Background::Color(GREEN)),
    };

    Style {
        background,
        ..gray_button(theme, status)
    }
}
