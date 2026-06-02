use super::{
    BORDER_RADIUS, BORDER_WIDTH, black, blue, blue_pressed, border_color, disabled_color, gray,
    green, light_gray, red, red_pressed, white, window_background, yellow,
};
use iced::{Background, Border, Theme, widget::container::Style};

pub fn gray_container(_theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(gray())),
        text_color: Some(black()),
        border: Border {
            width: 0.0,
            color: border_color(),
            radius: BORDER_RADIUS,
        },
        shadow: Default::default(),
    }
}

pub fn light_gray_container(theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(light_gray())),
        ..gray_container(theme)
    }
}

pub fn black_container(theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(black())),
        text_color: Some(white()),
        ..gray_container(theme)
    }
}

pub fn white_container(theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(white())),
        ..gray_container(theme)
    }
}

pub fn blue_container(theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(blue())),
        text_color: Some(white()),
        ..gray_container(theme)
    }
}

pub fn green_container(theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(green())),
        ..gray_container(theme)
    }
}

pub fn yellow_container(theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(yellow())),
        ..gray_container(theme)
    }
}

pub fn red_container(theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(red())),
        ..gray_container(theme)
    }
}

pub fn red_pressed_container(theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(red_pressed())),
        ..gray_container(theme)
    }
}

pub fn blue_pressed_container(theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(blue_pressed())),
        text_color: Some(white()),
        ..gray_container(theme)
    }
}

pub fn scroll_bar_container(theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(window_background())),
        ..gray_container(theme)
    }
}

pub fn disabled_container(_theme: &Theme) -> Style {
    Style {
        background: None,
        text_color: Some(disabled_color()),
        border: Border {
            width: BORDER_WIDTH,
            color: disabled_color(),
            radius: BORDER_RADIUS,
        },
        shadow: Default::default(),
    }
}

pub fn transparent_container(theme: &Theme) -> Style {
    Style {
        background: None,
        text_color: None,
        ..gray_container(theme)
    }
}
