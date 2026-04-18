# 010 — Display Modes: Dark and High-Contrast

**Date:** 2026-04-19
**Status:** proposed

## Context

The uwh-portal web refbox supports three display modes — **Default**
(light), **Dark**, and **High Contrast** — switchable live from the
User Options page. Operators have reported that the existing light
palette washes out in poolside sun and glares at night; tournament
organizers have also asked for a neon-bright mode for low-light control
rooms and for referees with low-vision accommodations.

The project's back-porting rule applies: the web refbox is the
authoritative design source, and colours should match exactly unless an
explicit deviation is approved. (See
`memory/feedback_backport_web_is_standard.md`.)

Today's state in the Rust refbox:

- The UI is themed via `refbox/src/app/theme/`, which defines colours
  and button styles used by every view builder. There is **one** set
  of colours, matching today's light palette.
- No display-mode concept exists in `GameConfig` or in any persisted
  config. The main screen and the settings pages always render in the
  current (light) palette.
- Settings already has a `ConfigPage::Display` page, but that page is
  about **LED panel and visual output** settings — not about the
  refbox's own colour scheme.

Today's state in the web refbox (authoritative source):

- Three themes implemented as CSS custom-property palettes in
  `@underwater-web/styles/refbox-theme.scss` and switched by writing
  `document.documentElement.dataset.refboxTheme = theme`.
  (`@underwater-web/contexts/RefboxThemeContext.tsx`)
- The choice is persisted to `localStorage` under the key
  `refbox-theme`, and the palette is swapped **live** as soon as the
  cycle button is pressed — with no separate confirmation step.
- The cycle button shows the current mode's name: `DEFAULT`, `DARK`,
  or `HIGH CONTRAST`. Each press moves to the next mode in that
  order and wraps.

The Rust refbox's existing settings convention is that edits live in
`EditableSettings` and commit only when `DONE` is pressed. The web
refbox's View Mode cycle button violates that convention — it commits
immediately. This deviation has been explicitly discussed and accepted
because:

1. The web behaviour is the authoritative standard.
2. The visual change itself is the preview; a separate "apply" step
   would be redundant and confusing.
3. The choice is cheap to reverse — one tap cycles forward again.

## Decision

Introduce three display modes — **Default**, **Dark**, and
**High Contrast** — selectable live from a `VIEW MODE` cycle button
on the User Options page (see ADR 009).

### Palette definitions

Colours are copied verbatim from the web refbox's theme file. The
Rust theme layer gains a `DisplayMode` enum (`Default`, `Dark`,
`HighContrast`) and each style in `refbox/src/app/theme/` resolves
its colour through the active mode.

**Default (light).** Identical to the palette in use today. No
colour in the light palette changes as part of this ADR.

**Dark.** Muted palette for low-light viewing. Key values:

| Role | Value |
|------|-------|
| Window background | `rgb(45, 45, 45)` |
| Panels / light-gray backgrounds | `rgb(75, 75, 75)` |
| White-team elements (score, buttons) | `#cfcfcf` |
| Red (penalty, stopped timer) | `rgb(200, 80, 80)` |
| Orange | `rgb(200, 130, 70)` |
| Yellow | `rgb(200, 180, 80)` |
| Green (running timer) | `rgb(80, 200, 80)` |
| Blue | `rgb(80, 120, 200)` |
| Disabled text | `rgb(150, 150, 150)` |

Pure black and pure white are replaced with softer tones so the
screen is easier on the eyes in a dim room. Button behaviour,
placement, and text do not change.

**High Contrast.** Neon palette for maximum legibility. Key values:

| Role | Value |
|------|-------|
| Window background | `rgb(10, 10, 10)` (near black) |
| Border colour (all buttons and panels) | `#ffff00` neon yellow |
| Primary text | `#00ff00` neon green |
| Secondary text | `#00ffff` cyan |
| Muted text | `#ffff00` yellow |
| Red (penalty, stopped timer) | `rgb(255, 0, 102)` hot pink |
| Orange | `rgb(255, 165, 0)` |
| Yellow | `rgb(255, 255, 0)` |
| Green (running timer) | `rgb(0, 255, 0)` |
| Blue | `rgb(0, 170, 255)` bright cyan |

The full palette for all three modes lives in
`@underwater-web/styles/refbox-theme.scss`; that file is the
authoritative colour source and the Rust theme module copies from
it.

### Cycle-button behaviour

- Label shows the currently-active mode: `DEFAULT`, `DARK`, or
  `HIGH CONTRAST`.
- Each press cycles forward: Default → Dark → High Contrast →
  Default.
- The palette swap is **live and immediate** — pressing the button
  repaints the entire refbox in the new palette with no separate
  apply step.
- Pressing `CANCEL` on User Options or on any outer settings page
  does **not** revert the mode. The choice persists as soon as the
  button is pressed.

This is the one accepted deviation from the Rust refbox's usual
"edits live in `EditableSettings` until `DONE`" convention. The
reason and the alternative are recorded in this ADR so future work
does not rediscover the question.

### Persistence

The active display mode is saved to the refbox's existing
`confy`-managed config file (the same mechanism already used for
`GameConfig` and `AppConfig`). On startup the refbox reads the saved
mode and paints in that palette from the first frame; there is no
flash of the default palette.

- A `display_mode: DisplayMode` field is added to `AppConfig` (the
  app-level preference struct, **not** `GameConfig` — this is a
  user preference, not a game parameter).
- Existing config files without the field default to
  `DisplayMode::Default` on first read. Behaviour for users who do
  not change the mode is exactly as today.
- No migration is required beyond serde's default-on-missing handling.

### What is **not** changing

- LED panel output colours are not affected. The LED panel has its
  own palette chosen for hardware legibility; the on-screen display
  mode is a software-only preference.
- Stream overlay colours are not affected. The overlay is a separate
  crate with its own look and is out of scope.
- Tournament portal integration and wire format are unaffected.
- No game-logic code is touched.

## Open design questions (to resolve during implementation)

- **How many theme primitives the iced theme layer needs.** The web
  refbox uses ~40 CSS custom properties; the Rust refbox's theme
  layer uses a smaller set of named colour constants. Implementation
  must decide whether to mirror the full web token set or map the
  web palette onto the existing Rust abstractions. Either is
  acceptable as long as visible output matches.
- **Transition polish.** The web refbox repaints instantly. If iced
  would benefit from an animation pass it can be considered, but the
  default is "no animation, instant repaint," matching the web.
- **Simulator coverage.** `sim_app` should render each mode for
  screenshot comparison against the web refbox during QA.

## Consequences

**Becomes easier:**

- Referees working in bright sunshine, dim control rooms, or with
  low-vision accommodations can pick a palette that works for their
  environment.
- Operators who use both the web refbox and the native refbox see
  the same palette choices with the same names and the same
  behaviour.
- Future palette tweaks are one-file changes in `refbox/src/app/theme/`
  and one palette table in this ADR (or the web source file).

**Becomes harder / constrained:**

- Every view builder in `refbox/src/app/view_builders/` must resolve
  colours through the theme layer rather than using any remaining
  inline hex values. Inline colours, if found, become bugs to fix.
- Screenshots in documentation and test fixtures are palette-
  dependent. Any regression test that compares against a saved
  screenshot must pin a mode (presumably `Default`).
- The project now supports three palettes that must be kept in sync
  with the web refbox. When the web palette changes, a follow-up
  PR is required on the Rust side.
- The View Mode button breaks the refbox's usual save-on-DONE
  convention. This is documented here so it stays an isolated
  exception rather than growing into a pattern.

**Scope:**

- `refbox` — theme module extended with a `DisplayMode` enum and
  palette resolution; `AppConfig` gains a `display_mode` field; the
  User Options page gains the `VIEW MODE` cycle button; every view
  builder that has an inline colour is updated to route through the
  theme.
- `uwh-common` — no change required. `DisplayMode` is an app-level
  preference, not a core game type.
- `overlay`, `schedule-processor`, `wireless-remote`, LED panel
  crates — no change.

## References

- `memory/feedback_backport_web_is_standard.md` — back-porting rule
  that makes the web refbox the authoritative design source.
- ADR 009 — settings navigation rework that introduces the User
  Options page where the View Mode cycle button lives.
- `refbox/src/app/theme/` — the module that owns colour and style
  definitions; the place where `DisplayMode` and palette resolution
  are implemented.
- `refbox/src/app/message.rs` — `Message` enum where the new
  `CycleDisplayMode` message lives; `ConfigPage::User` (ADR 009)
  is the page that hosts the button.
- `@underwater-web/styles/refbox-theme.scss` — authoritative palette
  definitions for all three modes. Rust colour values are copied
  from here.
- `@underwater-web/contexts/RefboxThemeContext.tsx` — web's live-swap
  behaviour and `localStorage` persistence (the behaviour model the
  Rust cycle button matches, substituting `confy` for `localStorage`).
- `@underwater-web/components/refbox/pages/UserOptionsPage.tsx` —
  web's cycle-button labels (`DEFAULT` / `DARK` / `HIGH CONTRAST`).
