use core::sync::atomic::{AtomicU8, Ordering};
use iced::{
    Border, Color, Shadow, Theme,
    widget::{
        container::Style as ContainerStyle,
        scrollable::{self, Scroller},
    },
};
use iced_core::border::Radius;
use paste::paste;
use serde::{Deserialize, Serialize};

pub const BORDER_RADIUS: Radius = Radius {
    top_left: 9.0,
    top_right: 9.0,
    bottom_right: 9.0,
    bottom_left: 9.0,
};
pub const BORDER_RADIUS_ZERO: Radius = Radius {
    top_left: 0.0,
    top_right: 0.0,
    bottom_right: 0.0,
    bottom_left: 0.0,
};
pub const BORDER_WIDTH: f32 = 6.0;
pub const SPACING: f32 = 8.0;
pub const PADDING: f32 = 8.0;
pub const MIN_BUTTON_SIZE: f32 = 89.0;
pub const XS_BUTTON_SIZE: f32 = 86.0;

/// Outer size of the portal health tile shown at the left of the time banner.
/// Matches `MIN_BUTTON_SIZE` so the tile fits the standard banner height;
/// the button-height formula in `make_game_time_button` depends on this.
pub const HEALTH_TILE_SIZE: f32 = MIN_BUTTON_SIZE;
/// Diameter of the coloured status dot inside the health tile.
pub const HEALTH_DOT_SIZE: f32 = 34.0;

pub const SMALL_TEXT: f32 = 19.0;
pub const SMALL_PLUS_TEXT: f32 = 29.0;
pub const MEDIUM_TEXT: f32 = 38.0;
pub const LARGE_TEXT: f32 = 66.0;

// See https://stackoverflow.com/a/727339 for color mixing math. For darkening colors with pure
// black, the math simplifies to new_r = orig_r * (1 - black_alpha), so we will multiply by the
// value of (1 - black_alpha)
macro_rules! make_color {
    ($name:ident, $r:literal, $g:literal, $b:literal) => {
        paste! {
            pub const $name: iced::Color = iced::Color::from_rgb($r, $g, $b);
            // _PRESSED kept for the Light palette test assertions in display_mode_tests.
            #[allow(dead_code)]
            pub const [<$name _PRESSED>]: iced::Color = iced::Color::from_rgb(
                $r * 0.85,
                $g * 0.85,
                $b * 0.85);
        }
    };
}

make_color!(WHITE, 1.0, 1.0, 1.0);
make_color!(RED, 1.0, 0.0, 0.0);
make_color!(ORANGE, 1.0, 0.5, 0.0);
make_color!(YELLOW, 1.0, 1.0, 0.0);
make_color!(GREEN, 0.0, 1.0, 0.0);
make_color!(BLUE, 0.0, 0.0, 1.0);
make_color!(GRAY, 0.5, 0.5, 0.5);
make_color!(LIGHT_GRAY, 0.7, 0.7, 0.7);

pub const BLACK: Color = Color::from_rgb(0.0, 0.0, 0.0);
pub const BLACK_PRESSED: Color = Color::from_rgb(0.15, 0.15, 0.15);

pub const BORDER_COLOR: Color = Color::from_rgb(0.3, 0.47, 1.0);

pub const DISABLED_COLOR: Color = GRAY;

pub const WINDOW_BACKGROUND: Color = Color::from_rgb(0.82, 0.82, 0.82);

pub const SCROLLBAR_COLOR: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.7);

/// Convert 8-bit sRGB channels (matching the web `refbox-theme.scss`) to an
/// iced `Color`. `const fn` so palettes can be `const`.
const fn rgb8(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}

/// Derive a "pressed" shade by darkening ×0.85, matching the `make_color!`
/// macro used for the Light palette. The web's Dark pressed values equal this
/// ×0.85 derivation; its High-Contrast pressed values differ by a few percent,
/// which is within the "match on screen" tolerance recorded in the spec.
const fn dim(c: Color) -> Color {
    Color {
        r: c.r * 0.85,
        g: c.g * 0.85,
        b: c.b * 0.85,
        a: c.a,
    }
}

/// All mode-dependent colour roles. Sizes/borders/radii are NOT here — they do
/// not change with display mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub white: Color,
    pub white_pressed: Color,
    pub black: Color,
    pub black_pressed: Color,
    pub red: Color,
    pub red_pressed: Color,
    pub orange: Color,
    pub orange_pressed: Color,
    pub yellow: Color,
    pub yellow_pressed: Color,
    pub green: Color,
    pub green_pressed: Color,
    pub blue: Color,
    pub blue_pressed: Color,
    pub gray: Color,
    pub gray_pressed: Color,
    pub light_gray: Color,
    pub light_gray_pressed: Color,
    pub border_color: Color,
    pub disabled_color: Color,
    pub window_background: Color,
}

/// Build a palette from its base colours. `*_pressed` shades derive from the
/// base via `dim`, except `black_pressed`, which is a fixed dark-grey so a
/// pressed black button is visible (matches today's `BLACK_PRESSED`).
// 12 arguments are needed because `Palette` has 12 independent base colours and
// `const fn` cannot yet take a struct literal that refers to other `const fn`s.
#[allow(clippy::too_many_arguments)]
const fn make_palette(
    white: Color,
    black: Color,
    red: Color,
    orange: Color,
    yellow: Color,
    green: Color,
    blue: Color,
    gray: Color,
    light_gray: Color,
    border_color: Color,
    disabled_color: Color,
    window_background: Color,
) -> Palette {
    Palette {
        white,
        white_pressed: dim(white),
        black,
        black_pressed: BLACK_PRESSED,
        red,
        red_pressed: dim(red),
        orange,
        orange_pressed: dim(orange),
        yellow,
        yellow_pressed: dim(yellow),
        green,
        green_pressed: dim(green),
        blue,
        blue_pressed: dim(blue),
        gray,
        gray_pressed: dim(gray),
        light_gray,
        light_gray_pressed: dim(light_gray),
        border_color,
        disabled_color,
        window_background,
    }
}

/// Light = today's palette exactly (built from the existing colour `const`s).
pub const LIGHT_PALETTE: Palette = make_palette(
    WHITE,
    BLACK,
    RED,
    ORANGE,
    YELLOW,
    GREEN,
    BLUE,
    GRAY,
    LIGHT_GRAY,
    BORDER_COLOR,
    DISABLED_COLOR,
    WINDOW_BACKGROUND,
);

/// Dark = muted palette. Values copied from web `refbox-theme.scss`
/// `[data-refbox-theme='dark']`.
pub const DARK_PALETTE: Palette = make_palette(
    rgb8(207, 207, 207), // white  (#cfcfcf)
    rgb8(0, 0, 0),       // black
    rgb8(200, 80, 80),   // red
    rgb8(200, 130, 70),  // orange
    rgb8(200, 180, 80),  // yellow
    rgb8(80, 200, 80),   // green
    rgb8(80, 120, 200),  // blue
    rgb8(128, 128, 128), // gray
    rgb8(100, 100, 100), // light_gray (button light-gray; see spec note on split role)
    rgb8(100, 140, 200), // border_color
    rgb8(150, 150, 150), // disabled_color
    rgb8(45, 45, 45),    // window_background
);

/// Dark grey used only in High Contrast: the white-team button/panel fill and
/// neutral panel borders (web `refbox-theme.scss` uses rgb(64,64,64) here).
pub const HC_DARK_GREY: Color = rgb8(64, 64, 64);

/// High Contrast = neon palette. Values copied from web `refbox-theme.scss`
/// `[data-refbox-theme='high-contrast']`.
pub const HIGH_CONTRAST_PALETTE: Palette = make_palette(
    rgb8(255, 255, 255), // white
    rgb8(0, 0, 0),       // black
    rgb8(255, 0, 102),   // red
    rgb8(255, 165, 0),   // orange
    rgb8(255, 255, 0),   // yellow
    rgb8(0, 255, 0),     // green
    rgb8(0, 170, 255),   // blue
    rgb8(128, 128, 128), // gray
    rgb8(180, 180, 180), // light_gray
    rgb8(255, 255, 0),   // border_color (neon yellow)
    rgb8(100, 100, 100), // disabled_color
    rgb8(10, 10, 10),    // window_background
);

/// The on-screen colour scheme. Persisted per user in `Config`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DisplayMode {
    #[default]
    Light,
    Dark,
    HighContrast,
}

impl DisplayMode {
    /// Cycle forward: Light → Dark → High Contrast → Light.
    pub const fn next(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::HighContrast,
            Self::HighContrast => Self::Light,
        }
    }

    pub const fn palette(self) -> Palette {
        match self {
            Self::Light => LIGHT_PALETTE,
            Self::Dark => DARK_PALETTE,
            Self::HighContrast => HIGH_CONTRAST_PALETTE,
        }
    }

    const fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Dark,
            2 => Self::HighContrast,
            _ => Self::Light,
        }
    }
}

/// Process-wide active display mode. Read on every style resolution; written at
/// startup and when the VIEW MODE button is pressed. A single GUI window with
/// change-only-on-tap semantics makes this ambient value safe.
static ACTIVE_DISPLAY_MODE: AtomicU8 = AtomicU8::new(DisplayMode::Light as u8);

pub fn set_display_mode(mode: DisplayMode) {
    ACTIVE_DISPLAY_MODE.store(mode as u8, Ordering::Relaxed);
}

pub fn display_mode() -> DisplayMode {
    DisplayMode::from_u8(ACTIVE_DISPLAY_MODE.load(Ordering::Relaxed))
}

pub fn active_palette() -> Palette {
    display_mode().palette()
}

// --- Mode-aware colour accessors. Styling functions call these instead of the
// raw colour `const`s so colour follows the active display mode. ---
pub fn white() -> Color {
    active_palette().white
}
pub fn white_pressed() -> Color {
    active_palette().white_pressed
}
pub fn black() -> Color {
    active_palette().black
}
pub fn black_pressed() -> Color {
    active_palette().black_pressed
}
pub fn red() -> Color {
    active_palette().red
}
pub fn red_pressed() -> Color {
    active_palette().red_pressed
}
pub fn orange() -> Color {
    active_palette().orange
}
pub fn orange_pressed() -> Color {
    active_palette().orange_pressed
}
pub fn yellow() -> Color {
    active_palette().yellow
}
pub fn yellow_pressed() -> Color {
    active_palette().yellow_pressed
}
pub fn green() -> Color {
    active_palette().green
}
pub fn green_pressed() -> Color {
    active_palette().green_pressed
}
pub fn blue() -> Color {
    active_palette().blue
}
pub fn blue_pressed() -> Color {
    active_palette().blue_pressed
}
pub fn gray() -> Color {
    active_palette().gray
}
pub fn gray_pressed() -> Color {
    active_palette().gray_pressed
}
pub fn light_gray() -> Color {
    active_palette().light_gray
}
pub fn light_gray_pressed() -> Color {
    active_palette().light_gray_pressed
}
pub fn border_color() -> Color {
    active_palette().border_color
}
pub fn disabled_color() -> Color {
    active_palette().disabled_color
}
pub fn window_background() -> Color {
    active_palette().window_background
}

pub mod button;
pub use button::{
    black_button, black_selected_button, blue_button, blue_selected_button,
    blue_with_border_button, gray_button, green_button, green_selected_button, light_gray_button,
    light_gray_selected_button, orange_button, orange_selected_button, red_button,
    red_selected_button, white_button, white_selected_button, yellow_button,
    yellow_selected_button,
};

pub mod container;
pub use container::{
    black_container, blue_container, blue_pressed_container, disabled_container, gray_container,
    green_container, light_gray_container, red_container, red_pressed_container,
    scroll_bar_container, transparent_container, white_container, yellow_container,
};

pub mod text;
pub use text::{black_text, green_text, orange_text, red_text, white_text, yellow_text};

pub mod svg;
pub use svg::{black_svg, disabled_svg, white_svg};

pub fn scrollable_style(_theme: &Theme, _status: scrollable::Status) -> scrollable::Style {
    let container = ContainerStyle {
        text_color: None,
        background: None,
        border: Border {
            width: 0.0,
            color: Color::TRANSPARENT,
            radius: BORDER_RADIUS_ZERO,
        },
        shadow: Shadow::default(),
    };

    let rail = scrollable::Rail {
        background: None,
        border: Border {
            width: 0.0,
            color: Color::TRANSPARENT,
            radius: BORDER_RADIUS_ZERO,
        },
        scroller: Scroller {
            color: SCROLLBAR_COLOR,
            border: Border {
                width: 0.0,
                color: Color::TRANSPARENT,
                radius: BORDER_RADIUS_ZERO,
            },
        },
    };

    scrollable::Style {
        container,
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
    }
}

/// Serializes tests that mutate the process-wide `ACTIVE_DISPLAY_MODE`
/// (the test harness runs tests in parallel). Acquire it at the top of any
/// test that calls `set_display_mode`.
#[cfg(test)]
pub(crate) static DISPLAY_MODE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod display_mode_tests {
    use super::*;

    #[test]
    fn cycle_wraps_light_dark_high_contrast() {
        assert_eq!(DisplayMode::Light.next(), DisplayMode::Dark);
        assert_eq!(DisplayMode::Dark.next(), DisplayMode::HighContrast);
        assert_eq!(DisplayMode::HighContrast.next(), DisplayMode::Light);
    }

    #[test]
    fn default_mode_is_light() {
        assert_eq!(DisplayMode::default(), DisplayMode::Light);
    }

    #[test]
    fn light_palette_matches_todays_constants() {
        let p = DisplayMode::Light.palette();
        assert_eq!(p.white, WHITE);
        assert_eq!(p.window_background, WINDOW_BACKGROUND);
        assert_eq!(p.border_color, BORDER_COLOR);
        assert_eq!(p.black_pressed, BLACK_PRESSED);
        assert_eq!(p.white_pressed, WHITE_PRESSED);
        assert_eq!(p.red, RED);
        assert_eq!(p.red_pressed, RED_PRESSED);
    }

    #[test]
    fn dark_and_high_contrast_window_backgrounds_differ() {
        assert_eq!(
            DisplayMode::Dark.palette().window_background,
            rgb8(45, 45, 45)
        );
        assert_eq!(
            DisplayMode::HighContrast.palette().window_background,
            rgb8(10, 10, 10)
        );
    }

    #[test]
    fn active_mode_round_trips_through_atomic() {
        let _guard = DISPLAY_MODE_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        set_display_mode(DisplayMode::Dark);
        assert_eq!(display_mode(), DisplayMode::Dark);
        assert_eq!(active_palette().window_background, rgb8(45, 45, 45));
        set_display_mode(DisplayMode::Light);
        assert_eq!(display_mode(), DisplayMode::Light);
    }
}
