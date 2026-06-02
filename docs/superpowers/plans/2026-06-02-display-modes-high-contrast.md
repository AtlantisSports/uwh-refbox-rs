# High-Contrast Outline Styling Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** In High Contrast mode, render buttons and panels as dark fill + coloured outline + coloured text (not solid neon fills), matching the web refbox; Light and Dark are unchanged.

**Architecture:** Add a small "outline-in-high-contrast" transform helper in `button.rs` and a sibling in `container.rs` (Approach A), each applied at the end of the styling functions — mirroring the existing `*_selected_button` mutate-and-return idiom. The transform only fires when the active `DisplayMode` is `HighContrast`; in Light/Dark it returns the style unchanged. One new dark-grey constant (`HC_DARK_GREY = rgb8(64,64,64)`) serves the white-team fill and neutral panel borders.

**Tech Stack:** Rust 2024, iced 0.13. MSRV 1.85.

**Spec:** `docs/superpowers/specs/2026-06-02-display-modes-design.md` (Addendum — High-Contrast outline styling).

**Process:** Lean (refbox UI). `just check` is the authoritative gate. `refbox` is a **binary crate**: use `cargo test -p refbox` (NOT `--lib`) and `cargo clippy -p refbox -- -D warnings` (NOT `--all-targets` — that surfaces ~90 pre-existing local-toolchain test-code lints in untouched files; ignore them).

**Scope guard — do NOT touch:** Light/Dark appearance; the VIEW MODE button, persistence, or palette values; LED panel; overlay; wire format; game logic; `uwh-common`.

---

## File Structure

| File | Change |
|------|--------|
| `refbox/src/app/theme/mod.rs` | Add `pub const HC_DARK_GREY: Color = rgb8(64, 64, 64);` |
| `refbox/src/app/theme/button.rs` | Add `outline_in_high_contrast(...)` helper; apply it at the end of each public colour button fn; add a `#[cfg(test)]` test |
| `refbox/src/app/theme/container.rs` | Add `outline_container_in_high_contrast(...)` helper; split out a private `gray_container_base`; apply the transform to the panel/colour containers; add a `#[cfg(test)]` test |

---

## Task 1: High-Contrast outline for buttons

**Files:**
- Modify: `refbox/src/app/theme/mod.rs` (one new const)
- Modify: `refbox/src/app/theme/button.rs`

### Background for the implementer
- `refbox/src/app/theme/mod.rs` has a private `const fn rgb8(r,g,b: u8) -> Color`, the colour accessor fns (`white()`, `red()`, `black()`, …), `pub fn display_mode() -> DisplayMode`, and `pub enum DisplayMode { Light, Dark, HighContrast }`. `BORDER_WIDTH: f32 = 6.0` is a const there.
- In `button.rs`, every public button fn returns an `iced::widget::button::Style { background: Option<Background>, text_color: Color, border: Border, shadow }`. Colours come from the accessors. In Light/Dark the active-button border width is `0`, so only the fill shows. The existing idiom for "tweak then return" is e.g. `light_gray_selected_button`: `let mut style = light_gray_button(theme, status); style.border.width = BORDER_WIDTH; style`.

- [ ] **Step 1: Add the `HC_DARK_GREY` constant** to `refbox/src/app/theme/mod.rs`, just below the palette constants (after `HIGH_CONTRAST_PALETTE`):

```rust
/// Dark grey used only in High Contrast: the white-team button/panel fill and
/// neutral panel borders (web `refbox-theme.scss` uses rgb(64,64,64) here).
pub const HC_DARK_GREY: Color = rgb8(64, 64, 64);
```

- [ ] **Step 2: Write the failing test** — append to `button.rs` a test module. (Add it at the very end of the file, BEFORE the pre-existing commented-out `impl button::Catalog` block if present, or at end of file.)

```rust
#[cfg(test)]
mod high_contrast_tests {
    use super::*;
    use crate::app::theme::{
        black, red, set_display_mode, white, window_background, DisplayMode, BORDER_WIDTH,
        HC_DARK_GREY,
    };
    use iced::widget::button::Status;
    use iced::{Background, Theme};

    #[test]
    fn high_contrast_red_button_is_outlined() {
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
        set_display_mode(DisplayMode::HighContrast);
        let s = white_button(&Theme::default(), Status::Active);
        assert_eq!(s.background, Some(Background::Color(HC_DARK_GREY)));
        assert_eq!(s.text_color, white());
        assert_eq!(s.border.color, white());
        set_display_mode(DisplayMode::Light);
    }

    #[test]
    fn high_contrast_disabled_button_is_not_outlined() {
        set_display_mode(DisplayMode::HighContrast);
        let s = blue_button(&Theme::default(), Status::Disabled);
        // Disabled keeps the base treatment: background is the window background,
        // NOT the dark outline fill, and text is the disabled colour.
        assert_eq!(s.background, Some(Background::Color(window_background())));
        set_display_mode(DisplayMode::Light);
    }

    #[test]
    fn light_mode_red_button_unchanged() {
        set_display_mode(DisplayMode::Light);
        let s = red_button(&Theme::default(), Status::Active);
        // Light: solid red fill, no outline (border width 0).
        assert_eq!(s.background, Some(Background::Color(red())));
        assert_eq!(s.border.width, 0.0);
    }
}
```

- [ ] **Step 3: Run the test to confirm it fails to compile** — `outline_in_high_contrast` and the new behaviour don't exist yet.

Run: `cargo test -p refbox high_contrast_tests 2>&1 | head -20`
Expected: compile error / assertion failures (the outline behaviour isn't implemented).

- [ ] **Step 4: Add the transform helper** near the top of `button.rs` (after the imports, before `gray_button`). First extend the imports: add `display_mode`, `DisplayMode`, and `HC_DARK_GREY` to the `use super::{…}` list, and add `Color` to the `use iced::{…}` list. Then:

```rust
/// In High Contrast only, restyle a filled button as an outline: dark `hc_fill`,
/// the `accent` colour on the border + text, an always-visible `BORDER_WIDTH`
/// border, and no hover/pressed darkening. Disabled buttons are left untouched
/// (they keep their distinct dark-fill + grey treatment). In Light/Dark the
/// style is returned unchanged.
fn outline_in_high_contrast(mut style: Style, accent: Color, hc_fill: Color, status: Status) -> Style {
    if display_mode() == DisplayMode::HighContrast && !matches!(status, Status::Disabled) {
        style.background = Some(Background::Color(hc_fill));
        style.text_color = accent;
        style.border.color = accent;
        style.border.width = BORDER_WIDTH;
    }
    style
}
```

- [ ] **Step 5: Apply the helper at the end of every public colour button fn.** Each fn currently ends by returning a `Style { … }`; bind that to `let style = …;` and return `outline_in_high_contrast(style, <accent>, <hc_fill>, status)`. Use these arguments:

| Function | accent | hc_fill |
|----------|--------|---------|
| `gray_button` | `white()` | `black()` |
| `light_gray_button` | `light_gray()` | `black()` |
| `white_button` | `white()` | `HC_DARK_GREY` |
| `black_button` | `white()` | `black()` |
| `red_button` | `red()` | `black()` |
| `orange_button` | `orange()` | `black()` |
| `yellow_button` | `yellow()` | `black()` |
| `green_button` | `green()` | `black()` |
| `blue_button` | `blue()` | `black()` |

Exemplar — `gray_button` (the base) becomes:

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
    let style = Style {
        background,
        text_color,
        border,
        shadow: Shadow::default(),
    };
    outline_in_high_contrast(style, white(), black(), status)
}
```

Exemplar — `white_button` (note the `HC_DARK_GREY` fill):

```rust
pub fn white_button(theme: &Theme, status: Status) -> Style {
    let background = match status {
        Status::Disabled => Some(Background::Color(window_background())),
        Status::Pressed => Some(Background::Color(white_pressed())),
        Status::Active | Status::Hovered => Some(Background::Color(white())),
    };
    let style = Style {
        background,
        ..gray_button(theme, status)
    };
    outline_in_high_contrast(style, white(), HC_DARK_GREY, status)
}
```

Exemplar — `red_button`:

```rust
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
```

Apply the identical pattern to `light_gray_button`, `black_button`, `orange_button`, `yellow_button`, `green_button`, `blue_button` using the accent/hc_fill from the table. **Do not change** the `*_selected_button` variants or `blue_with_border_button` — they spread from the now-transformed colour buttons and remain correct (the selected variants re-set `border.width = BORDER_WIDTH`, which in High Contrast is already the case).

> Why this works for the spread idiom: each spreader (e.g. `red_button`) takes `..gray_button(theme, status)` — which in High Contrast returns a white-accent outline — then applies its OWN `outline_in_high_contrast` last, overwriting text/border/background with its own accent. Last-applied wins, so each button gets its correct colour. In Light/Dark the helper is a no-op, so behaviour is byte-identical to today.

- [ ] **Step 6: Run the tests** — `cargo test -p refbox high_contrast_tests 2>&1 | tail -10`
Expected: all 4 tests PASS.

- [ ] **Step 7: Build + clippy + the existing theme tests**

Run: `cargo build -p refbox 2>&1 | tail -5 && cargo clippy -p refbox -- -D warnings 2>&1 | grep -cE "^error" && cargo test -p refbox display_mode_tests 2>&1 | tail -3`
Expected: clean build, clippy `0`, the 5 existing display-mode tests still PASS.

- [ ] **Step 8: Commit** (stage only these two files)

```bash
git add refbox/src/app/theme/mod.rs refbox/src/app/theme/button.rs
git commit -m "fix(refbox): outline buttons in High Contrast mode"
```

---

## Task 2: High-Contrast outline for panels / areas (containers)

**Files:**
- Modify: `refbox/src/app/theme/container.rs`

### Background for the implementer
- `container.rs` styling fns return `iced::widget::container::Style { background: Option<Background>, text_color, border, shadow }`. `gray_container` is the base; most other containers spread `..gray_container(theme)`. `transparent_container`, `scroll_bar_container` also spread from it but must NOT gain a visible border. `disabled_container` does NOT spread from `gray_container` (it has its own border) — leave it alone. Container fns take only `&Theme` (no `Status`).

- [ ] **Step 1: Write the failing test** — append to `container.rs`:

```rust
#[cfg(test)]
mod high_contrast_container_tests {
    use super::*;
    use crate::app::theme::{
        black, gray, red, set_display_mode, window_background, DisplayMode, BORDER_WIDTH,
        HC_DARK_GREY,
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
    fn light_mode_gray_container_unchanged() {
        set_display_mode(DisplayMode::Light);
        let s = gray_container(&Theme::default());
        assert_eq!(s.background, Some(Background::Color(gray())));
        assert_eq!(s.border.width, 0.0);
    }
}
```

- [ ] **Step 2: Run the test to confirm it fails** — `cargo test -p refbox high_contrast_container_tests 2>&1 | head -20`
Expected: compile/assertion failures.

- [ ] **Step 3: Extend imports + add the container helper.** Add `display_mode`, `DisplayMode`, `HC_DARK_GREY`, `window_background` to the `use super::{…}` import (keep existing imports), and ensure `Color` is imported from iced. Add the helper near the top of `container.rs`:

```rust
/// In High Contrast only, restyle a panel/area as dark `fill` with a visible
/// `accent` border at `BORDER_WIDTH`. In Light/Dark the style is unchanged.
fn outline_container_in_high_contrast(mut style: Style, accent: Color, fill: Option<Background>) -> Style {
    if display_mode() == DisplayMode::HighContrast {
        style.background = fill;
        style.border.color = accent;
        style.border.width = BORDER_WIDTH;
    }
    style
}
```

- [ ] **Step 4: Split a plain private base out of `gray_container`** so the borderless containers can spread from it without inheriting the outline. Replace the current `gray_container` with:

```rust
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
```

- [ ] **Step 5: Repoint every spreader from `gray_container` to `gray_container_base`, and apply the transform to the panel/colour containers.** For each container below, change `..gray_container(theme)` to `..gray_container_base(theme)` and wrap the returned `Style` in the transform with the given args. Containers and their `(accent, fill)`:

| Function | accent | fill |
|----------|--------|------|
| `light_gray_container` | `HC_DARK_GREY` | `Some(Background::Color(window_background()))` |
| `black_container` | `white()` | `Some(Background::Color(black()))` |
| `white_container` | `white()` | `Some(Background::Color(HC_DARK_GREY))` |
| `blue_container` | `blue()` | `Some(Background::Color(black()))` |
| `blue_pressed_container` | `blue()` | `Some(Background::Color(black()))` |
| `green_container` | `green()` | `Some(Background::Color(black()))` |
| `yellow_container` | `yellow()` | `Some(Background::Color(black()))` |
| `red_container` | `red()` | `Some(Background::Color(black()))` |
| `red_pressed_container` | `red()` | `Some(Background::Color(black()))` |

For `transparent_container` and `scroll_bar_container`: change their `..gray_container(theme)` to `..gray_container_base(theme)` but DO NOT apply the transform (they must stay borderless / unchanged). Leave `disabled_container` exactly as-is.

Exemplar — `red_container`:

```rust
pub fn red_container(theme: &Theme) -> Style {
    let style = Style {
        background: Some(Background::Color(red())),
        ..gray_container_base(theme)
    };
    outline_container_in_high_contrast(style, red(), Some(Background::Color(black())))
}
```

Exemplar — `white_container`:

```rust
pub fn white_container(theme: &Theme) -> Style {
    let style = Style {
        background: Some(Background::Color(white())),
        ..gray_container_base(theme)
    };
    outline_container_in_high_contrast(style, white(), Some(Background::Color(HC_DARK_GREY)))
}
```

Exemplar — `transparent_container` (repointed, NOT transformed):

```rust
pub fn transparent_container(theme: &Theme) -> Style {
    Style {
        background: None,
        text_color: None,
        ..gray_container_base(theme)
    }
}
```

Apply the same pattern to `light_gray_container`, `black_container`, `blue_container`, `green_container`, `red_pressed_container`, `blue_pressed_container` using their `(accent, fill)` from the table, and repoint `scroll_bar_container`'s spread to `gray_container_base`.

- [ ] **Step 6: Run the container tests** — `cargo test -p refbox high_contrast_container_tests 2>&1 | tail -10`
Expected: all 4 tests PASS.

- [ ] **Step 7: Build + clippy + full refbox tests**

Run: `cargo build -p refbox 2>&1 | tail -5 && cargo clippy -p refbox -- -D warnings 2>&1 | grep -cE "^error" && cargo test -p refbox 2>&1 | tail -5`
Expected: clean build, clippy `0`, all tests PASS.

- [ ] **Step 8: Commit** (stage only this file)

```bash
git add refbox/src/app/theme/container.rs
git commit -m "fix(refbox): outline panels in High Contrast mode"
```

---

## Task 3: Full verification

**Files:** none (verification only)

- [ ] **Step 1: Run the authoritative gate** — `just check`
Expected: fmt-check, lint, test, audit all clean.

- [ ] **Step 2: Manual eyeball (controller will launch the app for the user).** In High Contrast: every button shows a dark fill with a coloured outline + matching coloured text (no solid neon fills); the white-team button is dark-grey with a white outline; the black-team button is black with a white outline; panels/tables show dark fills with visible borders. Switch to Light and Dark and confirm they look exactly as before.

---

## Deviations

(Record any execution deviations here, folded into the relevant code commit — no standalone deviation commits.)
