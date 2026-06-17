use super::{
    BORDER_RADIUS, BORDER_WIDTH, DisplayMode, HC_DARK_GREY, HC_WHITE_DISABLED, black,
    black_pressed, blue, blue_pressed, border_color, disabled_color, display_mode, gray,
    gray_pressed, green, green_pressed, light_gray, light_gray_pressed, orange, orange_pressed,
    red, red_pressed, white, white_pressed, window_background, yellow, yellow_pressed,
};
use iced::{
    Background, Border, Color, Shadow, Theme,
    widget::button::{Status, Style},
};

/// In High Contrast only, restyle a filled button as an outline: dark `hc_fill`,
/// the `accent` colour on the border + text, an always-visible `BORDER_WIDTH`
/// border, and no hover/pressed darkening. Disabled buttons are left untouched.
/// In Light/Dark the style is returned unchanged.
fn outline_in_high_contrast(
    mut style: Style,
    accent: Color,
    hc_fill: Color,
    status: Status,
) -> Style {
    if display_mode() == DisplayMode::HighContrast && !matches!(status, Status::Disabled) {
        style.background = Some(Background::Color(hc_fill));
        style.text_color = accent;
        style.border.color = accent;
        style.border.width = BORDER_WIDTH;
    }
    style
}

pub fn gray_button(_theme: &Theme, status: Status) -> Style {
    let background = match status {
        Status::Disabled => Some(Background::Color(window_background())),
        Status::Pressed => Some(Background::Color(gray_pressed())),
        Status::Active | Status::Hovered => Some(Background::Color(gray())),
    };

    let text_color = if matches!(status, Status::Disabled) {
        disabled_color()
    } else {
        black()
    };

    let border_color_val = match status {
        Status::Disabled => disabled_color(),
        Status::Pressed | Status::Active | Status::Hovered => border_color(),
    };

    let border_width = match status {
        Status::Disabled => BORDER_WIDTH,
        Status::Pressed | Status::Active | Status::Hovered => 0.0,
    };

    let border = Border {
        width: border_width,
        color: border_color_val,
        radius: BORDER_RADIUS,
    };

    let style = Style {
        background,
        text_color,
        border,
        shadow: Shadow::default(),
    };
    outline_in_high_contrast(style, white(), black(), status)
}

pub fn light_gray_button(theme: &Theme, status: Status) -> Style {
    let background = match status {
        Status::Disabled => Some(Background::Color(window_background())),
        Status::Pressed => Some(Background::Color(light_gray_pressed())),
        Status::Active | Status::Hovered => Some(Background::Color(light_gray())),
    };

    let style = Style {
        background,
        ..gray_button(theme, status)
    };
    outline_in_high_contrast(style, white(), black(), status)
}

pub fn light_gray_selected_button(theme: &Theme, status: Status) -> Style {
    let mut style = light_gray_button(theme, status);
    style.border.width = BORDER_WIDTH;
    style
}

pub fn white_button(theme: &Theme, status: Status) -> Style {
    let background = match status {
        Status::Disabled => Some(Background::Color(
            if display_mode() == DisplayMode::HighContrast {
                HC_WHITE_DISABLED
            } else {
                window_background()
            },
        )),
        Status::Pressed => Some(Background::Color(white_pressed())),
        Status::Active | Status::Hovered => Some(Background::Color(white())),
    };

    let style = Style {
        background,
        ..gray_button(theme, status)
    };
    outline_in_high_contrast(style, white(), HC_DARK_GREY, status)
}

pub fn white_selected_button(theme: &Theme, status: Status) -> Style {
    let mut style = white_button(theme, status);
    style.border.width = BORDER_WIDTH;
    style
}

pub fn black_button(theme: &Theme, status: Status) -> Style {
    let background = match status {
        Status::Disabled => Some(Background::Color(window_background())),
        Status::Pressed => Some(Background::Color(black_pressed())),
        Status::Active | Status::Hovered => Some(Background::Color(black())),
    };

    let text_color = if matches!(status, Status::Disabled) {
        disabled_color()
    } else {
        white()
    };

    let style = Style {
        background,
        text_color,
        ..gray_button(theme, status)
    };
    outline_in_high_contrast(style, white(), black(), status)
}

pub fn black_selected_button(theme: &Theme, status: Status) -> Style {
    let mut style = black_button(theme, status);
    style.border.width = BORDER_WIDTH;
    style
}

pub fn red_button(theme: &Theme, status: Status) -> Style {
    let background = match status {
        Status::Disabled => Some(Background::Color(window_background())),
        Status::Pressed => Some(Background::Color(red_pressed())),
        Status::Active | Status::Hovered => Some(Background::Color(red())),
    };

    let style = Style {
        background,
        ..gray_button(theme, status)
    };
    outline_in_high_contrast(style, red(), black(), status)
}

/// Like `red_button`, but always renders in the bright "active" colour even when
/// the button is non-interactive (no `on_press`). Colours a timeout button red
/// while it is held during the revive long-press, where the button is wrapped in
/// a `mouse_area` and has no `on_press` of its own.
pub fn red_button_armed(theme: &Theme, _status: Status) -> Style {
    red_button(theme, Status::Active)
}

pub fn red_selected_button(theme: &Theme, status: Status) -> Style {
    let mut style = red_button(theme, status);
    style.border.width = BORDER_WIDTH;
    style
}

pub fn orange_button(theme: &Theme, status: Status) -> Style {
    let background = match status {
        Status::Disabled => Some(Background::Color(window_background())),
        Status::Pressed => Some(Background::Color(orange_pressed())),
        Status::Active | Status::Hovered => Some(Background::Color(orange())),
    };

    let style = Style {
        background,
        ..gray_button(theme, status)
    };
    outline_in_high_contrast(style, orange(), black(), status)
}

pub fn orange_selected_button(theme: &Theme, status: Status) -> Style {
    let mut style = orange_button(theme, status);
    style.border.width = BORDER_WIDTH;
    style
}

pub fn yellow_button(theme: &Theme, status: Status) -> Style {
    let background = match status {
        Status::Disabled => Some(Background::Color(window_background())),
        Status::Pressed => Some(Background::Color(yellow_pressed())),
        Status::Active | Status::Hovered => Some(Background::Color(yellow())),
    };

    let style = Style {
        background,
        ..gray_button(theme, status)
    };
    outline_in_high_contrast(style, yellow(), black(), status)
}

/// Like `yellow_button`, but always renders in the bright "active" colour even
/// when the button is non-interactive (no `on_press`). Colours a timeout button
/// yellow during the post-revive RESTORED state, where the button is wrapped in
/// a `mouse_area` and has no `on_press` of its own.
pub fn yellow_button_armed(theme: &Theme, _status: Status) -> Style {
    yellow_button(theme, Status::Active)
}

pub fn yellow_selected_button(theme: &Theme, status: Status) -> Style {
    let mut style = yellow_button(theme, status);
    style.border.width = BORDER_WIDTH;
    style
}

pub fn green_button(theme: &Theme, status: Status) -> Style {
    let background = match status {
        Status::Disabled => Some(Background::Color(window_background())),
        Status::Pressed => Some(Background::Color(green_pressed())),
        Status::Active | Status::Hovered => Some(Background::Color(green())),
    };

    let style = Style {
        background,
        ..gray_button(theme, status)
    };
    outline_in_high_contrast(style, green(), black(), status)
}

pub fn green_selected_button(theme: &Theme, status: Status) -> Style {
    let mut style = green_button(theme, status);
    style.border.width = BORDER_WIDTH;
    style
}

pub fn blue_button(theme: &Theme, status: Status) -> Style {
    let background = match status {
        Status::Disabled => Some(Background::Color(window_background())),
        Status::Pressed => Some(Background::Color(blue_pressed())),
        Status::Active | Status::Hovered => Some(Background::Color(blue())),
    };

    let text_color = if matches!(status, Status::Disabled) {
        disabled_color()
    } else {
        white()
    };

    let style = Style {
        background,
        text_color,
        ..gray_button(theme, status)
    };
    outline_in_high_contrast(style, blue(), black(), status)
}

pub fn blue_selected_button(theme: &Theme, status: Status) -> Style {
    let mut style = blue_button(theme, status);
    style.border.width = BORDER_WIDTH;
    style
}

pub fn blue_with_border_button(theme: &Theme, status: Status) -> Style {
    let mut style = blue_button(theme, status);
    style.border.width = BORDER_WIDTH;
    style.border.color = gray();
    style
}

#[cfg(test)]
mod high_contrast_tests {
    use super::*;
    use crate::app::theme::{
        BORDER_WIDTH, DisplayMode, HC_DARK_GREY, HC_WHITE_DISABLED, black, light_gray_button, red,
        set_display_mode, white, window_background,
    };
    use iced::widget::button::Status;
    use iced::{Background, Theme};

    #[test]
    fn high_contrast_red_button_is_outlined() {
        let _guard = crate::app::theme::DISPLAY_MODE_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        set_display_mode(DisplayMode::HighContrast);
        let s = red_button(&Theme::default(), Status::Active);
        assert_eq!(s.background, Some(Background::Color(black())));
        assert_eq!(s.text_color, red());
        assert_eq!(s.border.color, red());
        assert_eq!(s.border.width, BORDER_WIDTH);
        set_display_mode(DisplayMode::Light);
    }

    #[test]
    fn high_contrast_white_button_uses_dark_grey_fill() {
        let _guard = crate::app::theme::DISPLAY_MODE_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        set_display_mode(DisplayMode::HighContrast);
        let s = white_button(&Theme::default(), Status::Active);
        assert_eq!(s.background, Some(Background::Color(HC_DARK_GREY)));
        assert_eq!(s.text_color, white());
        assert_eq!(s.border.color, white());
        set_display_mode(DisplayMode::Light);
    }

    #[test]
    fn high_contrast_disabled_button_is_not_outlined() {
        let _guard = crate::app::theme::DISPLAY_MODE_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        set_display_mode(DisplayMode::HighContrast);
        let s = blue_button(&Theme::default(), Status::Disabled);
        assert_eq!(s.background, Some(Background::Color(window_background())));
        set_display_mode(DisplayMode::Light);
    }

    #[test]
    fn light_mode_red_button_unchanged() {
        let _guard = crate::app::theme::DISPLAY_MODE_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        set_display_mode(DisplayMode::Light);
        let s = red_button(&Theme::default(), Status::Active);
        assert_eq!(s.background, Some(Background::Color(red())));
        assert_eq!(s.border.width, 0.0);
    }

    #[test]
    fn high_contrast_light_gray_button_uses_white_accent() {
        let _guard = crate::app::theme::DISPLAY_MODE_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        set_display_mode(DisplayMode::HighContrast);
        let s = light_gray_button(&Theme::default(), Status::Active);
        assert_eq!(s.background, Some(Background::Color(black())));
        assert_eq!(s.text_color, white());
        assert_eq!(s.border.color, white());
        set_display_mode(DisplayMode::Light);
    }

    #[test]
    fn high_contrast_disabled_white_button_has_distinct_fill() {
        let _guard = crate::app::theme::DISPLAY_MODE_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        set_display_mode(DisplayMode::HighContrast);
        let s = white_button(&Theme::default(), Status::Disabled);
        assert_eq!(s.background, Some(Background::Color(HC_WHITE_DISABLED)));
        set_display_mode(DisplayMode::Light);
    }
}
