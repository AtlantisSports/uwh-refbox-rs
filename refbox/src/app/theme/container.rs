use super::{
    BORDER_RADIUS, BORDER_RADIUS_ZERO, BORDER_WIDTH, DisplayMode, HC_DARK_GREY, black, blue,
    blue_pressed, border_color, disabled_color, display_mode, gray, green, light_gray, red,
    red_pressed, white, window_background, yellow,
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
        // Panels carry a dark fill in High Contrast, so any text that inherits
        // the container colour (e.g. the time-edit digits, keypad labels) must
        // be light to stay visible.
        style.text_color = Some(white());
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

// ── Game-info table cells ───────────────────────────────────────────────────
// The game-info table is composed of square (radius-zero) filled cells laid out
// on a dark `table_grid_container` backing. A 1px gap between cells (and 1px
// backing padding) lets the backing show through as thin gridlines, giving a
// spreadsheet-style grid without per-cell borders.

/// Outer frame of the game-info table: a dark backing shown through 1px of
/// padding around the cells, so the outer edge reads as the same 2px line as the
/// inner gridlines (which are two abutting 1px cell borders).
pub fn table_grid_container(_theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(black())),
        text_color: None,
        border: Border {
            width: 0.0,
            color: black(),
            radius: BORDER_RADIUS_ZERO,
        },
        shadow: Default::default(),
    }
}

/// A single square table cell with the given fill and text colour. The 1px black
/// border draws the gridlines: because borders don't consume layout space (unlike
/// inter-cell gaps), every row divides its full width by the same proportions, so
/// columns line up across rows regardless of how many cells a row has.
fn table_cell(fill: Color, text: Color) -> Style {
    Style {
        background: Some(Background::Color(fill)),
        text_color: Some(text),
        border: Border {
            width: 1.0,
            color: black(),
            radius: BORDER_RADIUS_ZERO,
        },
        shadow: Default::default(),
    }
}

/// Label cell — light-grey fill, dark text. (Same fill as value cells so the
/// whole settings grid is one uniform grey; cells are separated by gridlines.)
pub fn table_label_cell(_theme: &Theme) -> Style {
    table_cell(light_gray(), black())
}

/// Value cell — light-grey fill, dark text.
pub fn table_value_cell(_theme: &Theme) -> Style {
    table_cell(light_gray(), black())
}

/// White-team row cell — white fill, dark text.
pub fn table_white_cell(_theme: &Theme) -> Style {
    table_cell(white(), black())
}

/// Black-team row cell — black fill, light text.
pub fn table_black_cell(_theme: &Theme) -> Style {
    table_cell(black(), white())
}

/// Greyed label cell — light-grey fill, dimmed text (an inactive setting).
pub fn table_label_cell_grayed(_theme: &Theme) -> Style {
    table_cell(light_gray(), disabled_color())
}

/// Greyed value cell — light-grey fill, dimmed text.
pub fn table_value_cell_grayed(_theme: &Theme) -> Style {
    table_cell(light_gray(), disabled_color())
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
        BORDER_WIDTH, DisplayMode, HC_DARK_GREY, black, gray, red, set_display_mode, white,
        window_background, yellow,
    };
    use iced::{Background, Theme};

    #[test]
    fn high_contrast_red_container_is_outlined() {
        let _guard = crate::app::theme::DISPLAY_MODE_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        set_display_mode(DisplayMode::HighContrast);
        let s = red_container(&Theme::default());
        assert_eq!(s.background, Some(Background::Color(black())));
        assert_eq!(s.border.color, red());
        assert_eq!(s.border.width, BORDER_WIDTH);
        set_display_mode(DisplayMode::Light);
    }

    #[test]
    fn high_contrast_gray_panel_has_dark_fill_and_grey_border() {
        let _guard = crate::app::theme::DISPLAY_MODE_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        set_display_mode(DisplayMode::HighContrast);
        let s = gray_container(&Theme::default());
        assert_eq!(s.background, Some(Background::Color(window_background())));
        assert_eq!(s.border.color, HC_DARK_GREY);
        assert_eq!(s.border.width, BORDER_WIDTH);
        // Inherited panel text must be light so nested labels/digits stay visible.
        assert_eq!(s.text_color, Some(white()));
        set_display_mode(DisplayMode::Light);
    }

    #[test]
    fn high_contrast_transparent_container_stays_borderless() {
        let _guard = crate::app::theme::DISPLAY_MODE_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        set_display_mode(DisplayMode::HighContrast);
        let s = transparent_container(&Theme::default());
        assert_eq!(s.background, None);
        assert_eq!(s.border.width, 0.0);
        set_display_mode(DisplayMode::Light);
    }

    #[test]
    fn high_contrast_yellow_container_is_outlined() {
        let _guard = crate::app::theme::DISPLAY_MODE_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        set_display_mode(DisplayMode::HighContrast);
        let s = yellow_container(&Theme::default());
        assert_eq!(s.background, Some(Background::Color(black())));
        assert_eq!(s.border.color, yellow());
        assert_eq!(s.border.width, BORDER_WIDTH);
        set_display_mode(DisplayMode::Light);
    }

    #[test]
    fn light_mode_gray_container_unchanged() {
        let _guard = crate::app::theme::DISPLAY_MODE_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        set_display_mode(DisplayMode::Light);
        let s = gray_container(&Theme::default());
        assert_eq!(s.background, Some(Background::Color(gray())));
        assert_eq!(s.border.width, 0.0);
    }
}
