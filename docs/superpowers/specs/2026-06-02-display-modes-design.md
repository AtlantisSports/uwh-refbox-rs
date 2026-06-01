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
