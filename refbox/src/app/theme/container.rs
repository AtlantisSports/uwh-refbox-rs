use super::{
    BORDER_RADIUS, BORDER_WIDTH, DisplayMode, HC_DARK_GREY, black, blue, blue_pressed,
    border_color, disabled_color, display_mode, gray, green, light_gray, red, red_pressed, white,
    window_background, yellow,
};
use iced::{Background, Border, Color, Theme, widget::container::Style};

/// In High Contrast only, restyle a panel/area as dark `fill` with a visible
/// `accent` border at `BORDER_WIDTH`. In Light/Dark the style is unchanged.
fn outline_container_in_high_contrast(
    mut style: Style,
    accent: Color,
    fill: Option<Background>,
) -> Style {
    if display_mode() == DisplayMode::HighContrast {
        style.background = fill;
        style.border.color = accent;
        style.border.width = BORDER_WIDTH;
    }
    style
}

/// Plain panel base (no High-Contrast transform). Used as the spread base for
/// the other container styles.
fn gray_container_base(_theme: &Theme) -> Style {
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

pub fn gray_container(theme: &Theme) -> Style {
    outline_container_in_high_contrast(
        gray_container_base(theme),
        HC_DARK_GREY,
        Some(Background::Color(window_background())),
    )
}

pub fn light_gray_container(theme: &Theme) -> Style {
    let style = Style {
        background: Some(Background::Color(light_gray())),
        ..gray_container_base(theme)
    };
    outline_container_in_high_contrast(
        style,
        HC_DARK_GREY,
        Some(Background::Color(window_background())),
    )
}

pub fn black_container(theme: &Theme) -> Style {
    let style = Style {
        background: Some(Background::Color(black())),
        text_color: Some(white()),
        ..gray_container_base(theme)
    };
    outline_container_in_high_contrast(style, white(), Some(Background::Color(black())))
}

pub fn white_container(theme: &Theme) -> Style {
    let style = Style {
        background: Some(Background::Color(white())),
        ..gray_container_base(theme)
    };
    outline_container_in_high_contrast(style, white(), Some(Background::Color(HC_DARK_GREY)))
}

pub fn blue_container(theme: &Theme) -> Style {
    let style = Style {
        background: Some(Background::Color(blue())),
        text_color: Some(white()),
        ..gray_container_base(theme)
    };
    outline_container_in_high_contrast(style, blue(), Some(Background::Color(black())))
}

pub fn green_container(theme: &Theme) -> Style {
    let style = Style {
        background: Some(Background::Color(green())),
        ..gray_container_base(theme)
    };
    outline_container_in_high_contrast(style, green(), Some(Background::Color(black())))
}

pub fn yellow_container(theme: &Theme) -> Style {
    let style = Style {
        background: Some(Background::Color(yellow())),
        ..gray_container_base(theme)
    };
    outline_container_in_high_contrast(style, yellow(), Some(Background::Color(black())))
}

pub fn red_container(theme: &Theme) -> Style {
    let style = Style {
        background: Some(Background::Color(red())),
        ..gray_container_base(theme)
    };
    outline_container_in_high_contrast(style, red(), Some(Background::Color(black())))
}

pub fn red_pressed_container(theme: &Theme) -> Style {
    let style = Style {
        background: Some(Background::Color(red_pressed())),
        ..gray_container_base(theme)
    };
    outline_container_in_high_contrast(style, red(), Some(Background::Color(black())))
}

pub fn blue_pressed_container(theme: &Theme) -> Style {
    let style = Style {
        background: Some(Background::Color(blue_pressed())),
        text_color: Some(white()),
        ..gray_container_base(theme)
    };
    outline_container_in_high_contrast(style, blue(), Some(Background::Color(black())))
}

pub fn scroll_bar_container(theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(window_background())),
        ..gray_container_base(theme)
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
        ..gray_container_base(theme)
    }
}

#[cfg(test)]
mod high_contrast_container_tests {
    use super::*;
    use crate::app::theme::{
        BORDER_WIDTH, DisplayMode, HC_DARK_GREY, black, gray, red, set_display_mode,
        window_background, yellow,
    };
    use iced::{Background, Theme};

    #[test]
    fn high_contrast_red_container_is_outlined() {
        set_display_mode(DisplayMode::HighContrast);
        let s = red_container(&Theme::default());
        assert_eq!(s.background, Some(Background::Color(black())));
        assert_eq!(s.border.color, red());
        assert_eq!(s.border.width, BORDER_WIDTH);
        set_display_mode(DisplayMode::Light);
    }

    #[test]
    fn high_contrast_gray_panel_has_dark_fill_and_grey_border() {
        set_display_mode(DisplayMode::HighContrast);
        let s = gray_container(&Theme::default());
        assert_eq!(s.background, Some(Background::Color(window_background())));
        assert_eq!(s.border.color, HC_DARK_GREY);
        assert_eq!(s.border.width, BORDER_WIDTH);
        set_display_mode(DisplayMode::Light);
    }

    #[test]
    fn high_contrast_transparent_container_stays_borderless() {
        set_display_mode(DisplayMode::HighContrast);
        let s = transparent_container(&Theme::default());
        assert_eq!(s.background, None);
        assert_eq!(s.border.width, 0.0);
        set_display_mode(DisplayMode::Light);
    }

    #[test]
    fn high_contrast_yellow_container_is_outlined() {
        set_display_mode(DisplayMode::HighContrast);
        let s = yellow_container(&Theme::default());
        assert_eq!(s.background, Some(Background::Color(black())));
        assert_eq!(s.border.color, yellow());
        assert_eq!(s.border.width, BORDER_WIDTH);
        set_display_mode(DisplayMode::Light);
    }

    #[test]
    fn light_mode_gray_container_unchanged() {
        set_display_mode(DisplayMode::Light);
        let s = gray_container(&Theme::default());
        assert_eq!(s.background, Some(Background::Color(gray())));
        assert_eq!(s.border.width, 0.0);
    }
}
