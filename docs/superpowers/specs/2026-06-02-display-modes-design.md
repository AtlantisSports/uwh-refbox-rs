# Design Spec — Refbox Display Modes (Light / Dark / High Contrast)

**Date:** 2026-06-02
**Status:** draft (awaiting user review)
**Crate:** `refbox` only
**Supersedes process-wise:** formalises ADR `docs/decisions/010-display-modes.md`
(status *proposed*) into a spec ready for an implementation plan. The ADR's
user-facing decisions were re-verified against the live web source on
2026-06-02 and still hold; this spec carries them forward with two recorded
adjustments (see **Decisions carried in** and **Approved deviations**).

---

## Goal

Give the refbox operator three on-screen colour palettes — **Light**, **Dark**,
and **High Contrast** — switchable live from the User Options page, matching the
uwh-portal web refbox. The web refbox is the authoritative design source
(back-porting rule); colours are copied from it.

The problem this solves: the single light palette washes out in poolside sun,
glares at night, and offers nothing for low-vision accommodation. Dark is for
dim control rooms; High Contrast is a neon palette for sunlight and low vision.

## Scope boundary

**In scope — `refbox` crate only:**
- A `DisplayMode` enum and palette resolution in `refbox/src/app/theme/`.
- A `display_mode` field on `AppConfig` (user preference) with serde
  default-on-missing.
- A `VIEW MODE` cycle button on the User Options page (`ConfigPage::User`).
- A `CycleDisplayMode` message and its handler.
- Routing the handful of raw colour-constant uses in `view_builders/` through
  the theme layer.

**Explicitly out of scope (not touched):**
- **LED panel** output colours — own hardware palette, software-only preference.
- **Stream overlay** (`overlay` crate) — separate look, separate crate.
- **Wire format / portal integration / game logic.**
- **`uwh-common`** — `DisplayMode` is an app-level preference, not a core game
  type. No shared-type or wire-format change.
- The existing `ConfigPage::Display` page — that is LED/visual-output settings,
  a different concept from the refbox's own colour scheme.

## Acceptance criteria (operator-observable)

1. User Options shows a `VIEW MODE` button whose label reads `LIGHT`, `DARK`, or
   `HIGH CONTRAST` for the currently-active mode.
2. Each tap cycles forward: Light → Dark → High Contrast → Light, and the entire
   refbox repaints **immediately** in the new palette — no separate apply step.
3. Pressing `CANCEL` or `DONE` on User Options (or any outer settings page) does
   **not** revert the mode; the change persists from the moment of the tap.
4. Closing and reopening the refbox restores the last-chosen mode, painted from
   the first frame (no flash of the light palette on startup).
5. An operator who never touches the button sees exactly today's appearance.
6. The LED panel and stream overlay are visually unaffected by the mode choice.

## Decisions carried in (from ADR 010, re-verified 2026-06-02)

| # | Criterion | Decision |
|---|-----------|----------|
| 1 | Modes | Three: Light, Dark, High Contrast |
| 2 | Location | `VIEW MODE` button on the User Options page |
| 3 | Control | Single cycle button; label shows active mode; wraps |
| 4 | Commit | Live & immediate; CANCEL/DONE do not revert (accepted exception to save-on-DONE) |
| 5 | Persistence | `AppConfig.display_mode` via confy; painted from first frame; defaults to Light |
| 6 | Palette source | Copied from web `refbox-theme.scss`, mapped onto existing Rust named colours |
| 7 | Colour fidelity | Map onto the refbox's existing named colours; skip web-only roles with no on-screen equivalent (e.g. statistics-page colours) |

## Approved deviations

- **Label `LIGHT` (not `DEFAULT`).** The web User Options page labels the first
  mode `DEFAULT`; this spec uses **`LIGHT`** per explicit user instruction
  (2026-06-02). All other labels (`DARK`, `HIGH CONTRAST`) match the web.
- **Immediate commit.** The `VIEW MODE` button commits on tap rather than on
  DONE — the one accepted exception to the refbox's save-on-DONE convention,
  because the repaint itself is the preview and the change is cheap to reverse
  (tap again). Recorded so it stays an isolated exception, not a pattern.

## Architecture (approach B — app-wide active mode)

Today the theme is built from module-level `const` `Color`s
(`refbox/src/app/theme/mod.rs`), and styling functions such as `white_button`
are passed to iced as `fn(&Theme, Status) -> Style` pointers that **ignore** the
`&Theme` argument and read the consts directly. The app supplies no custom iced
theme object; `iced::application(...)` sets only an application-level style.

Approach B keeps that structure and confines the change to the theme module:

- Add `enum DisplayMode { Light, Dark, HighContrast }` (in `theme/`).
- Hold **one app-wide active mode** that the theme's colour lookups consult.
  Each former `const` colour becomes a small mode-aware lookup returning the
  Light/Dark/High-Contrast value for the active mode. Styling functions
  (`white_button`, `red_container`, `black_text`, …) keep their existing
  signatures and call sites; they resolve colour through the active mode instead
  of a fixed const.
- The active mode is set at startup from `AppConfig.display_mode` and updated
  when the `VIEW MODE` button is tapped.

Why B over the alternatives (both rejected during brainstorming):
- **A — carry palette in the iced `Theme`:** iced 0.13's theme palette carries
  only ~5 colours; the refbox has ~10 named roles. Poor fit, higher risk.
- **C — thread palette through every screen:** touches every `.style(...)` call
  site across all view builders — largest, most error-prone change, no visible
  benefit.

B's blast radius is essentially the `theme/` module plus the **4** raw
colour-constant uses in `view_builders/` (one each of `RED`, `GREEN`, `BLUE`,
`YELLOW`), which get routed through the theme. All other `.style(...)` call
sites are untouched.

> Implementation note (not a blocker): the exact mechanism for the "app-wide
> active mode" (e.g. an atomic/lock-guarded value in the theme module set from
> the app) is left to the implementation plan. The single-window, change-only-
> on-tap nature of the app makes an ambient value acceptable; the executor picks
> the concrete representation.

## Palette mapping

**Authoritative source:** `js/@underwater-web/styles/refbox-theme.scss` in the
uwh-portal repo (`/home/estraily/projects/uwh-portal/`). The Rust values are
copied from there. **Light** equals today's Rust palette exactly (the scss
comments confirm the round-trip), so **no Light value changes**.

Concrete mapping for the unambiguous button/background/border roles (RGB 0–255):

| Rust named colour | Light (= today) | Dark | High Contrast |
|-------------------|-----------------|------|---------------|
| `WHITE` | 255,255,255 | 207,207,207 (`#cfcfcf`) | 255,255,255 |
| `BLACK` | 0,0,0 | 0,0,0 | 0,0,0 |
| `RED` | 255,0,0 | 200,80,80 | 255,0,102 |
| `ORANGE` | 255,128,0 | 200,130,70 | 255,165,0 |
| `YELLOW` | 255,255,0 | 200,180,80 | 255,255,0 |
| `GREEN` | 0,255,0 | 80,200,80 | 0,255,0 |
| `BLUE` | 0,0,255 | 80,120,200 | 0,170,255 |
| `GRAY` | 128,128,128 | 128,128,128 | 128,128,128 |
| `LIGHT_GRAY` | 179,179,179 | 100,100,100 | 180,180,180 |
| `BORDER_COLOR` | 77,120,255 | 100,140,200 | 255,255,0 (neon yellow) |
| `WINDOW_BACKGROUND` | 209,209,209 | 45,45,45 | 10,10,10 |
| `DISABLED_COLOR` (text) | 128,128,128 | 150,150,150 | 100,100,100 |

`*_PRESSED` variants: the existing `make_color!` macro derives pressed shades by
×0.85. The web defines explicit pressed values that are close to but not exactly
×0.85. Implementation keeps the ×0.85 derivation per mode unless a specific
pressed value reads wrong against the web on visual comparison — fidelity target
is "matches on screen," not byte-identical hex.

**Mappings the implementation resolves by visual comparison** (the web splits a
role the Rust app keeps single, or assigns a role the Rust app lacks):
- **Split light-gray role.** The web separates panel/background light-gray
  (Dark: `rgb(75,75,75)`) from button light-gray (Dark: `rgb(100,100,100)`).
  The Rust app has a single `LIGHT_GRAY`. The table above uses the button value;
  if panels read wrong in Dark, the executor introduces a distinct
  panel-background value rather than forcing one const to serve both.
- **High-Contrast text colours.** The web uses neon-green primary text on dark
  backgrounds. The Rust text styles (`white_text`, `black_text`, …) map to
  specific colours; the executor chooses High-Contrast text colours that match
  the web's on-screen result, prioritising legibility.

These are deliberately left to implementation under the "map onto existing
colours, match on screen" decision (criterion 7) rather than over-specified here.

## Components / surfaces touched

| File / area | Change |
|-------------|--------|
| `refbox/src/app/theme/mod.rs` | `DisplayMode` enum; active-mode holder; consts → mode-aware lookups |
| `refbox/src/app/theme/{button,container,text,svg}.rs` | Read colour via active mode (signatures unchanged) |
| `refbox/src/config.rs` (`AppConfig`) | New `display_mode: DisplayMode` field, serde default = Light |
| `refbox/src/app/message.rs` | New `Message::CycleDisplayMode`; button lives under `ConfigPage::User` |
| `refbox/src/app/mod.rs` | Handle `CycleDisplayMode`: advance mode, set active mode, persist `AppConfig`; set active mode from config at startup |
| `refbox/src/app/view_builders/configuration.rs` (User Options) | Add `VIEW MODE` cycle button using existing value-button helper |
| `refbox/src/app/view_builders/` (4 raw-const sites) | Route `RED`/`GREEN`/`BLUE`/`YELLOW` through the theme |
| `refbox/translations/` | New translation key(s) for the `VIEW MODE` label and mode names |

## Testing & verification

- **Unit:** mode cycling wraps correctly (Light → Dark → High Contrast → Light);
  `AppConfig` with no `display_mode` deserialises to `Light`; a saved mode
  round-trips through confy.
- **Manual (operator-observable):** the acceptance criteria above, exercised by
  running the refbox, cycling the button, and restarting to confirm persistence.
- **Regression caution:** any screenshot-based fixture must pin `Light`.
- Process: `refbox` UI feature → lean process (`docs/.../plan-execution.md`):
  one code review at the end, `just check` green before PR.

## Open items (resolved during implementation, not blockers)

- Concrete representation of the app-wide active mode.
- Exact `*_PRESSED`, split light-gray, and High-Contrast text mappings, settled
  by visual comparison against the web refbox.
- Whether `sim_app` should render each mode for screenshot QA (nice-to-have).

## References

- `docs/decisions/010-display-modes.md` — the originating ADR (proposed).
- `js/@underwater-web/styles/refbox-theme.scss` — authoritative palette.
- `js/@underwater-web/contexts/RefboxThemeContext.tsx` — web live-swap +
  persistence model.
- `js/@underwater-web/components/refbox/pages/UserOptionsPage.tsx` — web
  `VIEW MODE` cycle button and labels.
- `refbox/src/app/theme/` — where `DisplayMode` and palette resolution live.

---

## Addendum — 2026-06-02 — High-Contrast outline styling

**Status:** approved (follow-on increment to the above)

### Why

Manual walkthrough of the shipped feature revealed that **High Contrast**
renders buttons and panels as **solid neon fills**, which is wrong. In the web
refbox, Light and Dark use solid fills (Dark merely mutes the colours — these
two modes are correct), but **High Contrast is an outline style**: each control
is a dark fill with a *coloured border and coloured text*, with an always-visible
border and no hover/pressed darkening. The original Approach B (palette swap
only) changes colour *values* but not the **fill-vs-outline structure**, so the
High-Contrast structure was missing. Authoritative source:
`js/@underwater-web/components/refbox/game/RefboxButton.module.scss` and the
per-component `[data-refbox-theme='high-contrast']` rules.

### Decision

The refbox button/container styles already carry both a fill (`background`) and
an outline (`border` with `width`+`color`) plus `text_color`; in Light/Dark the
active-button border width is `0`, so only the fill shows. High Contrast is
achieved by **setting those existing fields differently when High Contrast is
active** — not by restructuring. Chosen approach: **a centralized transform
helper** (Approach A), matching the existing `*_selected_button`
mutate-and-return idiom. The High-Contrast rule lives in one place.

**Buttons** — `outline_in_high_contrast(style, accent, hc_fill, status)` in
`button.rs`, applied at the end of each colour button function. When the active
mode is `HighContrast` and `status != Disabled`, it rewrites the style to:
`background = hc_fill`, `text_color = accent`, `border.color = accent`,
`border.width = BORDER_WIDTH`, and flattens hover/pressed so the fill does not
darken. In Light/Dark it returns the style unchanged.

Per-button accent + fill (copied from the web scss):

| Button | accent (border + text) | hc_fill |
|--------|------------------------|---------|
| red / orange / yellow / green / blue / light-gray | its own colour | black |
| black, gray | white | black |
| white | white | dark grey `rgb(64, 64, 64)` |

The dark-grey white-team fill is the one new value, added as a High-Contrast-only
constant in the theme module (web: `rgb(64,64,64)` for the white timeout button).

**Panels / areas** — a sibling transform in `container.rs` applies the same idea
to the generic container styles: in High Contrast, fill → the near-black window
background with a **visible border** — coloured for colour-coded containers (e.g.
the red penalty highlight → dark fill + red border), and a neutral grey border
for the plain panel backgrounds (warnings panel, tables) so their edges read
against the near-black screen. Light/Dark unchanged.

### Edge cases (deliberate)

- **Disabled buttons:** the transform is skipped when `status == Disabled`, so
  disabled controls keep their existing distinct dark-fill + grey treatment and
  do not masquerade as active outlined buttons.
- **`*_selected_button`:** already force a `BORDER_WIDTH` border; in High
  Contrast the base is already outlined at that width, so selection still reads
  correctly.
- **`blue_with_border_button`:** keeps its existing grey-border behaviour (a
  visible grey outline in High Contrast) — acceptable, recorded.

### Scope

- `refbox` only: `refbox/src/app/theme/button.rs` and
  `refbox/src/app/theme/container.rs` (+ the one new High-Contrast fill constant
  in `theme/mod.rs`).
- **Not** changing: Light or Dark appearance; the `VIEW MODE` button, persistence,
  or palettes; LED panel; overlay; wire format; game logic; `uwh-common`.

### Acceptance (operator-observable)

In High Contrast: every button shows a dark fill with a coloured outline and
matching coloured text (no solid neon fills); panels/tables show dark fills with
visible borders; Light and Dark look exactly as before. `just check` green.

### Refinements (post-walkthrough, 2026-06-02)

Manual High-Contrast review surfaced three appearance fixes:

1. **Disabled white vs disabled black were indistinguishable.** The outline
   transform skips disabled buttons, so both fell back to the same near-black
   disabled fill. Fix: in High Contrast the **white** button uses a distinct
   mid-grey disabled fill, `HC_WHITE_DISABLED = rgb8(40, 40, 40)` (between the
   white button's `HC_DARK_GREY` fill and the near-black disabled fill), so
   disabled-white reads differently from disabled-black.

2. **Default/body text was invisible in High Contrast.** The app's
   application-level default text colour was `black()`, which is pure black in
   High Contrast — so any text relying on the default colour (the game-info
   config block, the time-edit digits) rendered black on the near-black
   background. Fix: `RefBoxApp::application_style` resolves its `text_color` to
   **`white()` in High Contrast** and `black()` otherwise. Light/Dark unchanged.

3. **The light-gray buttons looked disabled.** Their High-Contrast accent was
   `light_gray()` (dim grey). Fix: give the light-gray button the **same
   `white()` outline + text as the black button** in High Contrast. This is a
   deliberate, user-approved deviation from the web (which keeps `lightGray`
   dim), because these are the primary menu/option tiles and must read as
   active.

These touch `theme/mod.rs` (one new `HC_WHITE_DISABLED` const), `theme/button.rs`
(white-disabled fill + light-gray accent), and `app/mod.rs` (`application_style`
text colour). Light and Dark remain byte-identical.

4. **Container-inherited and explicit dark text were still invisible.** Setting
   the app *default* text colour was not enough: text that inherits a
   **container's** `text_color` (the time-edit "GAME TIME" digits, the keypad
   "PLAYER NUMBER:" label and entered value — all inside `light_gray_container`)
   and text using the explicit **`black_text`** style (timeout/penalty labels)
   stayed `black()`. Fix, both inverting dark→light only in High Contrast:
   - `outline_container_in_high_contrast` also sets `text_color = Some(white())`,
     so any text inheriting a transformed panel's colour is visible.
   - `black_text()` resolves to `white()` in High Contrast (`black()` otherwise).
   Touches `theme/container.rs` and `theme/text.rs`. Light/Dark unchanged.

5. **The Unknown infraction "?" icon stayed black.** The foul/penalty grid's
   first tile (`Infraction::Unknown`) uses a black "?" PNG (`get_image()` in
   `uwh-common` — out of scope, and a no_std core type that should not know
   about display modes), which is invisible on the black High-Contrast tile.
   Fix, refbox-only in `make_penalty_dropdown` (`shared_elements.rs`): in High
   Contrast only, render a themed white "?" (`text("?")` with `white_text`)
   centered in the tile instead of the image; Light and Dark keep the original
   PNG, so they are unchanged.
