# Display Modes (Light / Dark / High Contrast) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add three switchable on-screen colour palettes — Light, Dark, High Contrast — to the refbox, cycled live from a `VIEW MODE` button on the User Options page, persisted per user, matching the uwh-portal web refbox.

**Architecture:** Approach B (app-wide active mode). A `DisplayMode` enum and three `Palette` constants live in `refbox/src/app/theme/`. One process-wide atomic holds the active mode; the theme's colour values become accessor functions that read the palette for the active mode. Existing styling functions keep their signatures and call sites; they just resolve colour through the accessors. The mode is stored on the top-level `Config` (outside the `EditableSettings`/DONE round-trip, so it commits immediately and is never clobbered), set into the atomic at startup and on each button press.

**Tech Stack:** Rust 2024, iced 0.13, confy/serde (TOML config), fluent (`fl!` translations). MSRV 1.85.

**Spec:** `docs/superpowers/specs/2026-06-02-display-modes-design.md`

**Process:** Lean (refbox UI feature). One code review at the end; `just check` green before PR. No per-task deviation commits — note any deviation in the Deviations section at the bottom.

**Scope guard — do NOT touch:** LED panel output, `overlay`, wire format, game logic, `uwh-common`. This is `refbox`-only.

---

## File Structure

| File | Responsibility | Change |
|------|----------------|--------|
| `refbox/src/app/theme/mod.rs` | Colour/style definitions | Add `DisplayMode`, `Palette`, three palettes, active-mode atomic, colour accessor fns |
| `refbox/src/app/theme/button.rs` | Button styles | Resolve colours via accessors |
| `refbox/src/app/theme/container.rs` | Container styles | Resolve colours via accessors |
| `refbox/src/app/theme/text.rs` | Text styles | Resolve colours via accessors |
| `refbox/src/app/theme/svg.rs` | SVG styles | Resolve colours via accessors |
| `refbox/src/app/view_builders/shared_elements.rs` | Shared widgets | Route the 3 raw health-dot colours (+1 test) via accessors |
| `refbox/src/config.rs` | Persisted config | Add `display_mode: DisplayMode` field (+ serde default, migrate, tests) |
| `refbox/src/app/mod.rs` | App state / update loop | Set active mode at startup; handle `CycleDisplayMode` |
| `refbox/src/app/message.rs` | Message enum | Add `CycleDisplayMode` |
| `refbox/src/app/view_builders/configuration.rs` | Settings pages | Add `VIEW MODE` button to User Options page |
| `refbox/translations/*/refbox.ftl` | UI text (15 locales) | Add `view-mode` + 3 mode-label keys |

---

## Task 1: Theme core — `DisplayMode`, `Palette`, palettes, active-mode holder, accessors

**Files:**
- Modify: `refbox/src/app/theme/mod.rs`

This task is purely additive: it introduces the new types and functions alongside the existing colour `const`s without changing any styling function yet. The existing colour `const`s (`WHITE`, `RED`, …) are reused as the Light palette's input values, so they stay alive and Light is guaranteed identical to today.

- [ ] **Step 1: Write failing unit tests** (append to a `#[cfg(test)] mod theme_tests` at the bottom of `mod.rs`)

```rust
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
    }

    #[test]
    fn dark_and_high_contrast_window_backgrounds_differ() {
        assert_eq!(DisplayMode::Dark.palette().window_background, rgb8(45, 45, 45));
        assert_eq!(
            DisplayMode::HighContrast.palette().window_background,
            rgb8(10, 10, 10)
        );
    }

    #[test]
    fn active_mode_round_trips_through_atomic() {
        set_display_mode(DisplayMode::Dark);
        assert_eq!(display_mode(), DisplayMode::Dark);
        assert_eq!(active_palette().window_background, rgb8(45, 45, 45));
        // restore default so other tests are not affected
        set_display_mode(DisplayMode::Light);
        assert_eq!(display_mode(), DisplayMode::Light);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail to compile**

Run: `cargo test -p refbox --lib display_mode_tests 2>&1 | head -30`
Expected: compile errors — `DisplayMode`, `Palette`, `rgb8`, `set_display_mode`, `display_mode`, `active_palette` not found.

- [ ] **Step 3: Add the serde import** at the top of `mod.rs` (after the existing `use` block)

```rust
use serde::{Deserialize, Serialize};
```

- [ ] **Step 4: Add `Palette`, helpers, the three palettes, `DisplayMode`, and the active-mode holder.**

Add this block to `mod.rs` (after the existing colour `const`s, e.g. just below `WINDOW_BACKGROUND`):

```rust
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
    WHITE, BLACK, RED, ORANGE, YELLOW, GREEN, BLUE, GRAY, LIGHT_GRAY, BORDER_COLOR,
    DISABLED_COLOR, WINDOW_BACKGROUND,
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

use core::sync::atomic::{AtomicU8, Ordering};

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
pub fn white() -> Color { active_palette().white }
pub fn white_pressed() -> Color { active_palette().white_pressed }
pub fn black() -> Color { active_palette().black }
pub fn black_pressed() -> Color { active_palette().black_pressed }
pub fn red() -> Color { active_palette().red }
pub fn red_pressed() -> Color { active_palette().red_pressed }
pub fn orange() -> Color { active_palette().orange }
pub fn orange_pressed() -> Color { active_palette().orange_pressed }
pub fn yellow() -> Color { active_palette().yellow }
pub fn yellow_pressed() -> Color { active_palette().yellow_pressed }
pub fn green() -> Color { active_palette().green }
pub fn green_pressed() -> Color { active_palette().green_pressed }
pub fn blue() -> Color { active_palette().blue }
pub fn blue_pressed() -> Color { active_palette().blue_pressed }
pub fn gray() -> Color { active_palette().gray }
pub fn gray_pressed() -> Color { active_palette().gray_pressed }
pub fn light_gray() -> Color { active_palette().light_gray }
pub fn light_gray_pressed() -> Color { active_palette().light_gray_pressed }
pub fn border_color() -> Color { active_palette().border_color }
pub fn disabled_color() -> Color { active_palette().disabled_color }
pub fn window_background() -> Color { active_palette().window_background }
```

> Note: `SCROLLBAR_COLOR` (semi-transparent black) stays a `const` — it is mode-independent and reads acceptably over any background.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p refbox --lib display_mode_tests 2>&1 | tail -20`
Expected: all 5 tests PASS. If `dim` fails to compile as `const fn`, confirm toolchain ≥ 1.85 (float arithmetic in `const fn` is stable since 1.82).

- [ ] **Step 6: Confirm whole crate still builds (consts are now used by both old style fns and the Light palette)**

Run: `cargo build -p refbox 2>&1 | tail -15`
Expected: builds with no errors and no new warnings.

- [ ] **Step 7: Commit**

```bash
git add refbox/src/app/theme/mod.rs
git commit -m "feat(refbox): add DisplayMode, palettes, and active-mode holder"
```

---

## Task 2: Route the theme styling functions through the accessors

**Files:**
- Modify: `refbox/src/app/theme/button.rs`
- Modify: `refbox/src/app/theme/container.rs`
- Modify: `refbox/src/app/theme/text.rs`
- Modify: `refbox/src/app/theme/svg.rs`

**Mechanical transformation rule (applies to all four files):**
1. In the `use super::{ … }` line, replace every *colour* name (`WHITE`, `WHITE_PRESSED`, `BLACK`, `BLACK_PRESSED`, `RED`, `RED_PRESSED`, `ORANGE`, `ORANGE_PRESSED`, `YELLOW`, `YELLOW_PRESSED`, `GREEN`, `GREEN_PRESSED`, `BLUE`, `BLUE_PRESSED`, `GRAY`, `GRAY_PRESSED`, `LIGHT_GRAY`, `LIGHT_GRAY_PRESSED`, `BORDER_COLOR`, `DISABLED_COLOR`, `WINDOW_BACKGROUND`) with its lowercase accessor (`white`, `white_pressed`, …, `window_background`).
2. Keep non-colour imports unchanged: `BORDER_RADIUS`, `BORDER_RADIUS_ZERO`, `BORDER_WIDTH`.
3. In each function body, append `()` to every use of those colour names (e.g. `WHITE` → `white()`, `BORDER_COLOR` → `border_color()`).

There is no behaviour change in Light mode (default), so the existing rendering is preserved; this task is verified by compilation.

- [ ] **Step 1: Convert `text.rs`** (full file after change shown — it is short)

```rust
use super::{black, green, orange, red, white, yellow};
use iced::{Theme, widget::text::Style};

pub fn black_text(_theme: &Theme) -> Style {
    Style { color: Some(black()) }
}

pub fn white_text(_theme: &Theme) -> Style {
    Style { color: Some(white()) }
}

pub fn green_text(_theme: &Theme) -> Style {
    Style { color: Some(green()) }
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
```

- [ ] **Step 2: Convert `svg.rs`** (full file after change)

```rust
use super::{black, disabled_color, white};
use iced::{
    Theme,
    widget::svg::{Status, Style},
};

pub fn white_svg(_theme: &Theme, _status: Status) -> Style {
    Style { color: Some(white()) }
}

pub fn black_svg(_theme: &Theme, _status: Status) -> Style {
    Style { color: Some(black()) }
}

pub fn disabled_svg(_theme: &Theme, _status: Status) -> Style {
    Style {
        color: Some(disabled_color()),
    }
}
```

- [ ] **Step 3: Convert `container.rs`** (apply the rule). New import line:

```rust
use super::{
    black, blue, blue_pressed, border_color, disabled_color, gray, green, light_gray, red,
    red_pressed, white, window_background, BORDER_RADIUS, BORDER_WIDTH,
};
```

Then in the bodies replace each colour use with its accessor call, e.g. `gray_container`:

```rust
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
```

Apply the same `()` change to every remaining function in the file (`light_gray_container`→`light_gray()`, `black_container`→`black()`/`white()`, `white_container`→`white()`, `blue_container`→`blue()`/`white()`, `green_container`→`green()`, `red_container`→`red()`, `red_pressed_container`→`red_pressed()`, `blue_pressed_container`→`blue_pressed()`/`white()`, `scroll_bar_container`→`window_background()`, `disabled_container`→`disabled_color()`).

- [ ] **Step 4: Convert `button.rs`** (apply the rule). New import line:

```rust
use super::{
    black, black_pressed, blue, blue_pressed, border_color, disabled_color, gray, gray_pressed,
    green, green_pressed, light_gray, light_gray_pressed, orange, orange_pressed, red, red_pressed,
    white, white_pressed, window_background, yellow, yellow_pressed, BORDER_RADIUS, BORDER_WIDTH,
};
```

Then append `()` to every colour use in every function body. Exemplar (`gray_button`):

```rust
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

    Style {
        background,
        text_color,
        border,
        shadow: Shadow::default(),
    }
}
```

> Note: where a local was previously a colour name shadowing nothing, prefer a distinct local name (e.g. `border_color_val`) to avoid colliding with the imported `border_color` function, as shown above.

Apply the same `()` conversion to the rest of the functions (`light_gray_button`, `black_button`, `black_selected_button`, `blue_button`, `blue_selected_button`, `blue_with_border_button`, `green_button`, `green_selected_button`, `red_button`, `red_selected_button`, `white_button`, `white_selected_button`, `yellow_button`, `yellow_selected_button`, `orange_button`, `orange_selected_button`, `light_gray_selected_button`).

- [ ] **Step 5: Build and run the full theme test set**

Run: `cargo build -p refbox 2>&1 | tail -20 && cargo test -p refbox --lib display_mode_tests 2>&1 | tail -5`
Expected: clean build, tests still PASS. Fix any "expected function, found …" or unused-import errors the compiler reports.

- [ ] **Step 6: Lint (zero warnings required)**

Run: `cargo clippy -p refbox --all-targets -- -D warnings 2>&1 | tail -20`
Expected: no warnings. (Watch for "function `WHITE` never used" style dead-code if any const was orphaned — all colour consts should still be referenced by `LIGHT_PALETTE`.)

- [ ] **Step 7: Commit**

```bash
git add refbox/src/app/theme/button.rs refbox/src/app/theme/container.rs refbox/src/app/theme/text.rs refbox/src/app/theme/svg.rs
git commit -m "refactor(refbox): resolve theme colours through active display mode"
```

---

## Task 3: Route the raw colour uses in `shared_elements.rs`

**Files:**
- Modify: `refbox/src/app/view_builders/shared_elements.rs`

The health-status dot picks a colour directly from the theme `const`s. Route it through the accessors so the dot follows the active mode. (`shared_elements.rs` imports the theme via `theme::*`, so the accessor functions are already in scope.)

- [ ] **Step 1: Convert the health-dot colour match** (around the `make_health_tile` function)

Change:

```rust
    let dot_color = match state.health {
        HealthState::Green => GREEN,
        HealthState::Yellow => YELLOW,
        HealthState::Red => RED,
    };
```

to:

```rust
    let dot_color = match state.health {
        HealthState::Green => green(),
        HealthState::Yellow => yellow(),
        HealthState::Red => red(),
    };
```

- [ ] **Step 2: Fix the test reference to `BLUE`** (around line 91, inside a `#[cfg(test)]` block)

Change `Some(Background::Color(BLUE))` to `Some(Background::Color(blue()))`.

- [ ] **Step 3: Build + clippy**

Run: `cargo build -p refbox 2>&1 | tail -10 && cargo clippy -p refbox --all-targets -- -D warnings 2>&1 | tail -10`
Expected: clean. If the compiler reports `GREEN`/`YELLOW`/`RED`/`BLUE` still imported-but-unused, remove them from the `use` list (they are now accessed as functions via `theme::*`).

- [ ] **Step 4: Commit**

```bash
git add refbox/src/app/view_builders/shared_elements.rs
git commit -m "refactor(refbox): route health-dot colours through display mode"
```

---

## Task 4: Add `display_mode` to `Config`

**Files:**
- Modify: `refbox/src/config.rs`

- [ ] **Step 1: Write failing tests** (add to the existing `#[cfg(test)] mod test` in `config.rs`)

```rust
    #[test]
    fn config_missing_display_mode_defaults_to_light() {
        // A config TOML written before this field existed must still load.
        let toml_without_field = toml::to_string(&Config::default())
            .unwrap()
            .lines()
            .filter(|l| !l.starts_with("display_mode"))
            .collect::<Vec<_>>()
            .join("\n");
        let parsed: Config = toml::from_str(&toml_without_field).unwrap();
        assert_eq!(
            parsed.display_mode,
            crate::app::theme::DisplayMode::Light
        );
    }

    #[test]
    fn config_display_mode_round_trips() {
        let mut config = Config::default();
        config.display_mode = crate::app::theme::DisplayMode::HighContrast;
        let serialized = toml::to_string(&config).unwrap();
        let deser: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(deser.display_mode, crate::app::theme::DisplayMode::HighContrast);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p refbox --lib config::test::config_ 2>&1 | head -20`
Expected: compile error — no field `display_mode` on `Config`.

- [ ] **Step 3: Add the field** to the `Config` struct (after `language`):

```rust
    pub language: Option<Language>,
    #[serde(default)]
    pub display_mode: crate::app::theme::DisplayMode,
```

- [ ] **Step 4: Thread the field through `Config::migrate`.** In the destructuring of `Default::default()` add `display_mode,` and in the returned `Self { … }` literal add `display_mode,`. (Old TOML never carried the field, so it stays `Light` via the default — no `get_*_value` read is needed.)

Destructure block becomes:

```rust
        let Self {
            mut mode,
            mut hide_time,
            mut collect_scorer_cap_num,
            mut track_fouls_and_warnings,
            confirm_score,
            mut game,
            mut beep_test,
            mut hardware,
            mut uwhportal,
            mut sound,
            language,
            display_mode,
        } = Default::default();
```

and the final constructor gains `display_mode,`. (Match the exact current field set in `migrate`; add `display_mode` alongside the others. If `beep_test` is not currently destructured in `migrate`, leave it as it is — only add `display_mode`.)

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p refbox --lib config::test 2>&1 | tail -20`
Expected: the two new tests PASS and the existing `test_ser_config` still PASSES.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/config.rs
git commit -m "feat(refbox): persist display_mode in Config"
```

---

## Task 5: Apply the saved mode at startup

**Files:**
- Modify: `refbox/src/app/mod.rs`

- [ ] **Step 1: Set the active mode from config in `RefBoxApp::new`.** Immediately after the `RefBoxAppFlags { … } = flags;` destructure (around line 1154), add:

```rust
        // Paint in the saved display mode from the first frame.
        crate::app::theme::set_display_mode(config.display_mode);
```

- [ ] **Step 2: Build**

Run: `cargo build -p refbox 2>&1 | tail -10`
Expected: clean build.

- [ ] **Step 3: Commit**

```bash
git add refbox/src/app/mod.rs
git commit -m "feat(refbox): apply saved display mode on startup"
```

---

## Task 6: `CycleDisplayMode` message + handler

**Files:**
- Modify: `refbox/src/app/message.rs`
- Modify: `refbox/src/app/mod.rs`

- [ ] **Step 1: Add the message variant.** In the `Message` enum (`message.rs`), add a unit variant near the other settings-related variants:

```rust
    /// Advance the on-screen display mode (Light → Dark → High Contrast → …).
    /// Commits immediately; not part of the DONE/Apply settings round-trip.
    CycleDisplayMode,
```

- [ ] **Step 2: Handle it in `update`.** In the big `match message` in `mod.rs`, add an arm (place it near `Message::ChangeConfigPage`):

```rust
            Message::CycleDisplayMode => {
                let next = self.config.display_mode.next();
                self.config.display_mode = next;
                crate::app::theme::set_display_mode(next);
                self.persist_config();
                Task::none()
            }
```

> `persist_config` is the existing helper (`mod.rs:1107`) that writes `self.config` via confy. Using it keeps persistence consistent with the Apply path. The repaint is automatic: iced re-runs `view()` after every `update`, and the styling functions now read the new mode.

- [ ] **Step 3: Build**

Run: `cargo build -p refbox 2>&1 | tail -10`
Expected: clean build. (If the `match` is non-exhaustive elsewhere — e.g. a second match on `Message` — add the arm there too as the compiler directs.)

- [ ] **Step 4: Commit**

```bash
git add refbox/src/app/message.rs refbox/src/app/mod.rs
git commit -m "feat(refbox): handle CycleDisplayMode message"
```

---

## Task 7: `VIEW MODE` button on the User Options page + translations

**Files:**
- Modify: `refbox/src/app/view_builders/configuration.rs`
- Modify: `refbox/translations/en-US/refbox.ftl` (authoritative)
- Modify: the other 14 locale `refbox.ftl` files

- [ ] **Step 1: Add the translation keys to `en-US`.** Append to `refbox/translations/en-US/refbox.ftl` (near the other `*-options` keys):

```ftl
view-mode = VIEW MODE
display-mode-light = LIGHT
display-mode-dark = DARK
display-mode-high-contrast = HIGH CONTRAST
```

- [ ] **Step 2: Add the same four keys to every other locale** as placeholders (English text, pending translation). Add the identical four lines above to each of:
`de-DE`, `es`, `fr`, `id-ID`, `it-IT`, `ja-JP`, `ko-KR`, `ms-MY`, `nl-NL`, `pt-PT`, `th-TH`, `tl-PH`, `tr-TR`, `zh-CN` (each at `refbox/translations/<locale>/refbox.ftl`).

> Rationale: the project convention is full key coverage across locales (English placeholder is acceptable until translated). This keeps the fluent loader from depending on fallback for these keys.

- [ ] **Step 3: Add the `VIEW MODE` button to `make_user_config_page`.** In `configuration.rs`, inside `make_user_config_page` (the file already imports `theme::*` and `message::*`), build the button before the `column!` and place it in the first spacer row.

Insert before the `column![` macro:

```rust
    let view_mode_label = match display_mode() {
        DisplayMode::Light => fl!("display-mode-light"),
        DisplayMode::Dark => fl!("display-mode-dark"),
        DisplayMode::HighContrast => fl!("display-mode-high-contrast"),
    };
    let view_mode_button = make_value_button(
        fl!("view-mode"),
        view_mode_label,
        (false, false),
        Some(Message::CycleDisplayMode),
    );
```

Then change the first spacer row in the `column!` from:

```rust
        tiles,
        row![horizontal_space()].height(Length::Fill),
        row![horizontal_space()].height(Length::Fill),
        row![horizontal_space()].height(Length::Fill),
```

to:

```rust
        tiles,
        row![view_mode_button].height(Length::Fill),
        row![horizontal_space()].height(Length::Fill),
        row![horizontal_space()].height(Length::Fill),
```

> `display_mode()` and `DisplayMode` resolve via the existing `theme::*` glob import. `make_value_button` and `Message` are already in scope via `shared_elements::*` and `message::*`.

- [ ] **Step 4: Build + clippy + tests**

Run: `cargo build -p refbox 2>&1 | tail -10 && cargo clippy -p refbox --all-targets -- -D warnings 2>&1 | tail -10 && cargo test -p refbox --lib 2>&1 | tail -10`
Expected: clean build, no warnings, all tests pass.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/app/view_builders/configuration.rs refbox/translations
git commit -m "feat(refbox): add VIEW MODE button to User Options page"
```

---

## Task 8: Full verification

**Files:** none (verification only)

- [ ] **Step 1: Run the full gate**

Run: `just check`
Expected: fmt, lint, tests, audit all clean.

- [ ] **Step 2: Manual walkthrough** (operator-observable acceptance criteria — see spec §"How you'll verify"). Launch the refbox (per project run conventions) and confirm:
  - User Options shows a `VIEW MODE` button reading `LIGHT`.
  - Tapping it cycles `LIGHT → DARK → HIGH CONTRAST → LIGHT`, repainting the whole UI each tap with no separate apply step.
  - Pressing `CANCEL`/`DONE` does not revert the mode.
  - Quitting and relaunching restores the last-chosen mode from the first frame.
  - The LED panel sim and overlay are visually unaffected by the mode.

- [ ] **Step 3: Code review** (lean process — once, here at the end): run `superpowers:requesting-code-review` before opening the PR.

---

## Deviations

(Record any execution deviations from this plan here, folded into the relevant code commit — no standalone deviation commits.)
