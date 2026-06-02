use super::{DisplayMode, black, display_mode, green, orange, red, white, yellow};
use iced::{Theme, widget::text::Style};

/// "Primary" text: dark on the light/dark palettes, but inverted to light in
/// High Contrast so it stays visible on the near-black background.
pub fn black_text(_theme: &Theme) -> Style {
    let color = if display_mode() == DisplayMode::HighContrast {
        white()
    } else {
        black()
    };
    Style { color: Some(color) }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::theme::{DISPLAY_MODE_TEST_LOCK, DisplayMode, set_display_mode};

    #[test]
    fn black_text_inverts_to_white_in_high_contrast() {
        let _guard = DISPLAY_MODE_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        set_display_mode(DisplayMode::HighContrast);
        assert_eq!(black_text(&Theme::default()).color, Some(white()));
        set_display_mode(DisplayMode::Light);
        assert_eq!(black_text(&Theme::default()).color, Some(black()));
    }
}
