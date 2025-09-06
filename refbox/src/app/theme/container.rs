use super::{
    BLACK, BLUE, BORDER_COLOR, BORDER_RADIUS, BORDER_WIDTH, DISABLED_COLOR, GRAY, GREEN,
    LIGHT_GRAY, RED, WHITE, WINDOW_BACKGROUND,
};
use iced::{Background, Border, Theme, widget::container::Style};

pub fn gray_container(_theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(GRAY)),
        text_color: Some(BLACK),
        border: Border {
            width: 0.0,
            color: BORDER_COLOR,
            radius: BORDER_RADIUS,
        },
        shadow: Default::default(),
    }
}

pub fn light_gray_container(theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(LIGHT_GRAY)),
        ..gray_container(theme)
    }
}

pub fn black_container(theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(BLACK)),
        text_color: Some(WHITE),
        ..gray_container(theme)
    }
}

pub fn white_container(theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(WHITE)),
        ..gray_container(theme)
    }
}

pub fn blue_container(theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(BLUE)),
        text_color: Some(WHITE),
        ..gray_container(theme)
    }
}

pub fn green_container(theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(GREEN)),
        ..gray_container(theme)
    }
}

pub fn red_container(theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(RED)),
        ..gray_container(theme)
    }
}

pub fn scroll_bar_container(theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(WINDOW_BACKGROUND)),
        ..gray_container(theme)
    }
}

pub fn disabled_container(_theme: &Theme) -> Style {
    Style {
        background: None,
        text_color: Some(DISABLED_COLOR),
        border: Border {
            width: BORDER_WIDTH,
            color: DISABLED_COLOR,
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

#[allow(dead_code)] // This function is used through container::rounded_box path
pub fn rounded_box(theme: &Theme) -> Style {
    light_gray_container(theme)
}

pub fn team_color_white_container_square(_theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(WHITE)),
        text_color: Some(BLACK),
        border: Border {
            width: 0.0,
            color: BORDER_COLOR,
            radius: super::BORDER_RADIUS_ZERO,
        },
        shadow: Default::default(),
    }
}

pub fn team_color_black_container_square(_theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(BLACK)),
        text_color: Some(WHITE),
        border: Border {
            width: 0.0,
            color: BORDER_COLOR,
            radius: super::BORDER_RADIUS_ZERO,
        },
        shadow: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Theme;

    #[test]
    fn test_rounded_box_is_light_gray() {
        let theme = Theme::default();
        let rounded_style = rounded_box(&theme);
        let light_gray_style = light_gray_container(&theme);

        // rounded_box should be equivalent to light_gray_container
        assert_eq!(rounded_style.background, light_gray_style.background);
        assert_eq!(rounded_style.text_color, light_gray_style.text_color);
        assert_eq!(rounded_style.border.radius, light_gray_style.border.radius);
    }
}
