# Custom Site Globe Icon — Design

**Date:** 2026-08-11
**Crate:** `refbox` only
**Branch:** `feat/refbox/game-source-selection`
**Relates to:** `docs/superpowers/specs/2026-08-10-game-source-selection-design.md` (the source picker
this belongs to) and its plan `docs/superpowers/plans/2026-08-10-game-source-selection.md`.

## Problem

The status tile at the left of the time banner shows the UWH Portal logo (or the UWR logo in Rugby
mode) above a coloured health dot. The logo is chosen from the app `Mode` alone.

Once the operator can select a third-party site as the game source, that logo becomes a false
statement: the refbox displays the official Portal's emblem while it is talking to somebody else's
server. The tile is the one place an operator looks to answer "what am I connected to, and is it
healthy?", so it is the worst place for the answer to be wrong.

## Goal

When the game source is `Custom`, the tile shows a generic wireframe globe — a website, not a
particular organisation — instead of a portal emblem.

## Scope

In scope:

- The icon shown in the health tile when the committed source is `Custom`.
- One new SVG asset.

Out of scope:

- The indicator's *behaviour*: the red / yellow / green dot, and the OK / FAILED / CHECKING states.
  That belongs to Task 8 of the source-picker plan.
- The portal logos themselves, and what the tile shows under `Manual` or `Portal`.
- The tile's tap target (it still opens the portal detail page) and its size or position.
- The deferred `TOKEN: OK` wording decision for custom sites.

## What the operator sees

| Committed source | Tile icon |
|---|---|
| `Portal`, Hockey modes | UWH Portal compact logo (unchanged) |
| `Portal`, Rugby mode | UWR compact logo (unchanged) |
| `Custom` | Wireframe globe |
| `Manual` | No tile at all (unchanged — the tile only renders when a remote event is linked) |

The globe is the classic "website" glyph: a circle with one vertical meridian and two horizontal
parallels. It was chosen over a continents-style globe and a browser-window glyph because it reads
clearly at tile size and matches the flat line-art of the app's existing icons.

## Design

### Which source decides the icon

The **committed** source (`RefBoxApp::source`), not the source staged in the settings editor.

The tile reports on the live connection. Choosing CUSTOM in settings and not applying it leaves the
refbox connected to the portal, so the tile must keep showing the portal logo until APPLY. This is
the same rule the SITE row follows, for the same reason: no part of the UI may advertise a site the
refbox is not actually using.

### How the icon is drawn

A new `refbox/resources/globe.svg`, in the flat line-art style of the existing
`arrow_drop_up.svg`, `pause.svg` and `power.svg`.

It is rendered with the widget the app already uses for such icons, in blue:

```rust
Svg::new(svg::Handle::from_memory(&include_bytes!("../../../resources/globe.svg")[..]))
    .style(blue_svg)
```

`blue_svg` is a new style beside the existing `black_svg` / `white_svg`, taking `blue()` from the
active palette. Reading the colour from the palette rather than baking it into the asset is what
makes the globe follow the display mode like every other themed colour, so there is no separate
high-contrast case to get right or to forget.

Deliberately **not** an `iced` canvas drawing: canvas is known to panic in this app (see
`project_iced_canvas_settings_page_damage_panic`), and the banner renders on the settings pages.

### How the flag reaches the tile

`make_health_tile(state: PortalIndicatorState, tile_size: f32, mode: Mode)` currently picks the logo
from `mode`. It needs one more input: whether the site is a custom one.

That input travels on `PortalIndicatorState`, the struct the tile already receives, as a new
`site_is_custom: bool`. The struct is threaded to every page that draws a banner, so no signature
changes.

The rejected alternative was a new parameter on `make_game_time_button`, which would have meant
editing 29 call sites and about 13 intermediate signatures to place one icon — disproportionate, and
churn on that scale in shared view code carries more risk than the change itself.

The field is set where `ViewData::portal_indicator` is built in `RefBoxApp::view()`. That expression
already branches on the source (it yields `None` unless a remote source is in use), so it is the
existing meeting point of "which source" and "what the indicator shows"; knowledge of the source
spreads nowhere new.

### Units

| Unit | Purpose | Depends on |
|---|---|---|
| `resources/globe.svg` | The glyph | nothing |
| `PortalIndicatorState::site_is_custom` | Carries "this indicator reports on a custom site" to the tile | nothing |
| `make_health_tile` | Chooses emblem-or-globe and renders it | the field above, the asset |
| `RefBoxApp::view()` | Sets the field from the committed source | `RefBoxApp::source` |

Each is independently readable: the tile can be understood without knowing where the flag came from,
and the flag can be set without knowing how the tile draws.

## Error handling

There is none to add. A missing or malformed asset is a compile-time failure — `include_bytes!`
embeds the file, so the binary cannot ship without it. No runtime path can fail.

## Testing

The health tile is view code, and this crate has no harness for view code — the same position as the
rest of the source-picker feature. Verification is by eye, against these cases:

1. Source `Portal`, Hockey mode → UWH logo.
2. Source `Portal`, Rugby mode → UWR logo.
3. Source `Custom` → globe.
4. CUSTOM selected in settings but not applied → still the portal logo.

Case 3 needs a linked event, because the tile only renders when one exists; under `Custom` that
arrives with Task 7 of the source-picker plan. Until then it can be brought on screen by applying a
custom site while an event is still linked from a previous portal session.

## Consequences

- One more reason a reader of `PortalIndicatorState` must know it is a view-facing summary rather
  than pure portal-health state. The field is named and documented to say so.
- The tile's meaning becomes source-dependent, which is the point: an operator glancing at the
  banner can tell a third-party site from the official Portal without opening settings.
