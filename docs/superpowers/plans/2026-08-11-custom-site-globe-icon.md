# Custom Site Globe Icon Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show a wireframe globe in the banner's status tile when the committed game source is
`Custom`, in place of the UWH/UWR Portal logo.

**Architecture:** One new SVG asset, one `bool` added to the struct the tile already receives, one
branch in the tile builder, and one line where that struct is built. No signature changes anywhere.

**Tech Stack:** Rust 2024, MSRV 1.85, iced 0.13 (`Svg` widget + `svg::Handle::from_memory`),
`just check` (fmt + clippy `-D warnings` + tests + audit).

**Design spec:** `docs/superpowers/specs/2026-08-11-custom-site-globe-icon-design.md` — read it
first. It carries the rationale and the rejected alternatives.

## Global Constraints

- Branch: `feat/refbox/game-source-selection` (this rides with the source picker).
  **Do not push and do not open a PR** — the human's approval is required for both.
- Crate: `refbox` only. **No changes to `uwh-common`** or any other crate.
- No new dependencies. The `Svg` widget and `svg::Handle` are already used in this file.
- No new Fluent keys — this change adds no text.
- **No `iced` canvas.** Canvas is known to panic in this app when the settings pages render
  (`project_iced_canvas_settings_page_damage_panic`), and the banner draws on those pages.
- `refbox` is a binary crate: run `cargo test -p refbox` with **no** `--lib`, and clippy without
  `--all-targets`.
- Lean process (`.claude/rules/plan-execution.md`): no per-task deviation commits; record deviations
  in this file's Deviations section.
- The health tile is view code and this crate has no harness for view code. Verification is
  `just check` plus the on-screen checks in Step 5. Do not invent test scaffolding in production
  source for it.

---

### Task 1: The globe replaces the portal logo under CUSTOM

**Files:**
- Create: `refbox/resources/globe.svg`
- Modify: `refbox/src/portal_manager/mod.rs` (the `PortalIndicatorState` struct at ~234, its
  `Default` impl at ~245, and the one struct literal in `recompute_indicator` at ~504)
- Modify: `refbox/src/app/view_builders/shared_elements.rs` (`make_health_tile`, ~465-536)
- Modify: `refbox/src/app/mod.rs` (the `portal_indicator` field of `ViewData` in `RefBoxApp::view()`)

**Interfaces:**
- Consumes: `GameSource` and `RefBoxApp::source` (both exist).
- Produces: `blue_svg` in `crate::app::theme` — a new svg style beside `black_svg`/`white_svg`.
- Produces: `PortalIndicatorState { health, token_expired, site_is_custom }` — the third field is
  new and defaults to `false`.

- [x] **Step 1: Create the asset**

Create `refbox/resources/globe.svg` with exactly this content. The `24 24` viewBox and the
`#000000` paint match the existing icons in that directory; the colour is overridden at render time
by the widget style, so it only matters as a fallback.

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" id="vector">
    <g id="globe" fill="none" stroke="#000000" stroke-width="1.0" stroke-linecap="round">
        <circle cx="12" cy="12" r="9.5"/>
        <path d="M12 2.5v19"/>
        <ellipse cx="12" cy="12" rx="4.45" ry="9.5"/>
        <path d="M2.5 12h19"/>
        <path d="M3.92 7Q12 10.2 20.08 7"/>
        <path d="M3.92 17Q12 13.8 20.08 17"/>
    </g>
</svg>
```

Every number here was arrived at by comparing renders against the reference the human supplied, and
each one matters:

- **Stroke `1.0`** is 5.3% of the diameter. At 1.6 the lines close up the white space between them
  and the glyph reads as a blot at tile size.
- **The two outer latitudes are arcs, not straight lines**, and they sag *toward* the equator in the
  middle while their ends ride up to the rim — that is the near face of a sphere. Curving them the
  other way looks like a globe seen inside out.
- **The arc ends sit on the rim**: at `y = 7` the circle's half-width is `sqrt(9.5² - 5²) = 8.08`,
  hence `x = 3.92` and `20.08`. Recompute these if the radius or the latitude height changes.
- **Only the equator is straight**, because it is the one latitude seen edge-on.

- [x] **Step 2: Carry the flag on the indicator state**

In `refbox/src/portal_manager/mod.rs`, add the field to `PortalIndicatorState`:

```rust
    /// True when this indicator is reporting on a third-party site rather than
    /// the built-in portal. Set by the view layer from the committed game
    /// source; the manager itself has no notion of which site is configured.
    /// Drives which emblem the health tile draws, nothing else.
    pub site_is_custom: bool,
```

Add it to the `Default` impl:

```rust
            site_is_custom: false,
```

And to the one struct literal in `recompute_indicator`. It stays `false` here: `indicator_state()`
hands out a *copy*, and the view sets the flag on that copy, so the manager never needs to know.

```rust
        self.indicator_state = PortalIndicatorState {
            health,
            token_expired: self.token_known_problem,
            // Left false: the view sets this on the copy it takes, because
            // which site is configured is not something the manager knows.
            site_is_custom: false,
        };
```

- [x] **Step 3: Draw the globe in the tile**

In `make_health_tile` (`shared_elements.rs`), replace the logo block. The existing `mode` match
stays exactly as it is for the portal case; the globe is chosen ahead of it.

Current:

```rust
    let logo_bytes: &[u8] = match mode {
        Mode::Rugby => &include_bytes!("../../../resources/UWR_Compact_Logo.png")[..],
        Mode::Hockey6V6 | Mode::Hockey3V3 => {
            &include_bytes!("../../../resources/UWH_Portal_Compact_Logo.png")[..]
        }
        Mode::BeepTest => &include_bytes!("../../../resources/UWH_Portal_Compact_Logo.png")[..],
    };
    let logo = Image::new(image::Handle::from_bytes(logo_bytes))
        .width(Length::Fill)
        .height(Length::Fill);
```

Replace with:

```rust
    // A third-party site gets a generic globe: showing the official Portal's
    // emblem above a connection to somebody else's server would be a false
    // statement in the one place an operator looks to see what they are
    // connected to. Blue is taken from the palette, so it follows the display
    // mode like every other themed colour.
    let emblem: Element<'a, Message> = if state.site_is_custom {
        Svg::new(svg::Handle::from_memory(
            &include_bytes!("../../../resources/globe.svg")[..],
        ))
        .style(blue_svg)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    } else {
        let logo_bytes: &[u8] = match mode {
            Mode::Rugby => &include_bytes!("../../../resources/UWR_Compact_Logo.png")[..],
            Mode::Hockey6V6 | Mode::Hockey3V3 => {
                &include_bytes!("../../../resources/UWH_Portal_Compact_Logo.png")[..]
            }
            Mode::BeepTest => &include_bytes!("../../../resources/UWH_Portal_Compact_Logo.png")[..],
        };
        Image::new(image::Handle::from_bytes(logo_bytes))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    };
```

Then change the single use of `logo` in `tile_contents` to `emblem`:

```rust
        container(emblem)
```

Notes for the implementer:
- `Svg`, `svg`, `Image` and `image` are all already imported at the top of this file — check before
  adding anything.
- `blue_svg` does not exist yet: add it to `refbox/src/app/theme/svg.rs` beside `black_svg`,
  returning `Style { color: Some(blue()) }`, and add it to the `pub use svg::{...}` re-export in
  `theme/mod.rs` — the view builders import these through `theme`, not from the module directly.
- If the borrow checker objects to `Element<'a, Message>`, the cause is the lifetime on
  `make_health_tile<'a>`; both branches own their bytes via `include_bytes!` (`&'static [u8]`), so
  no borrow of a local escapes.

- [x] **Step 4: Set the flag from the committed source**

In `RefBoxApp::view()` (`refbox/src/app/mod.rs`), the `portal_indicator` field currently reads:

```rust
            portal_indicator: if self.uses_remote() {
                self.current_event_id
                    .as_ref()
                    .map(|_| self.portal_manager.indicator_state())
            } else {
                None
            },
```

Change the `map` closure so the flag comes from the committed source:

```rust
            portal_indicator: if self.uses_remote() {
                self.current_event_id.as_ref().map(|_| {
                    let mut state = self.portal_manager.indicator_state();
                    // The committed source, not the one staged in the editor:
                    // the tile reports the live connection, so choosing CUSTOM
                    // must not change the emblem until APPLY.
                    state.site_is_custom = self.source == GameSource::Custom;
                    state
                })
            } else {
                None
            },
```

- [ ] **Step 5: Verify**

Run: `just check` — expected: exit 0, all tests passing, no clippy warnings.

Then build the real binary (`just check` builds a *test* binary, which is not the app) and check on
screen against the stub:

```bash
cargo build -p refbox
WAYLAND_DISPLAY= UWH_PORTAL_URL_OVERRIDE=http://localhost:8099 ./target/debug/refbox --allow-http
```

Confirm, in order:
1. On launch with the portal source, the tile still shows the UWH Portal logo.
2. Selecting CUSTOM in settings *without* applying leaves the logo unchanged.
3. After applying a custom site, the tile shows the globe.
4. The globe is legible at tile size, sits where the logo sat, and the coloured dot below it is
   unmoved.

Check 3 needs a linked event, because the tile only renders when one exists. Before Task 7 of the
source-picker plan lands, reach it by starting from a session that already has a portal event
linked, then applying the custom site.

The spec also lists Rugby mode showing the UWR logo. That branch is the `match mode` arm moved
verbatim into the `else` above, so it is unchanged code; checking it on screen costs a mode switch
and a restart and is not required unless the mode match was edited.

- [ ] **Step 6: Commit**

```bash
git add refbox/resources/globe.svg refbox/src/ docs/superpowers/specs/ docs/superpowers/plans/
git commit -m "feat(refbox): show a globe for a custom site in the status tile"
```

---

## Deviations and outcome

_(Record deviations here as execution proceeds. Per `.claude/rules/plan-execution.md`, do not create
standalone deviation commits — fold notes into the code commit or record them here.)_

- **The globe is blue, not black — the human's decision at the visual check (2026-08-11).** The plan
  specified `black_svg`, matching the pause/play icons on the same banner. A new `blue_svg` was added
  to `refbox/src/app/theme/svg.rs` alongside them, taking `blue()` from the active palette so it
  still follows the display mode, and re-exported from `theme/mod.rs` with the other svg styles. The
  design spec's "How the icon is drawn" section was corrected to match rather than left stale.

- **The glyph took four rounds, and the first three were wrong in ways a screenshot barely shows.**
  What was wrong each time: (1) strokes far too heavy — 8% of the diameter against the reference's
  5%, which closed up the white space between the lines; (2) the outer latitudes drawn as straight
  lines rather than arcs; (3) the arcs curving toward the poles, when the near side of a sphere puts
  the *middle* of each latitude closest to the equator and its ends riding up to the rim. Final
  geometry: `r = 9.5`, stroke `1.0`, meridian ellipse `rx = 4.45`, latitude arcs meeting the rim at
  `y = 7` and `y = 17` with control points at `10.2` / `13.8`.

- **A scratch rasteriser made the iteration self-checking.** `resvg` — the same renderer iced draws
  SVGs through — was built as a throwaway binary in the session scratchpad, so each candidate could
  be rendered at both full size and the tile's 72px and inspected directly, instead of rebuilding
  refbox and asking the human for a screenshot every round. Worth repeating for any future icon
  work: it turns a several-minute round trip into seconds and catches "unreadable when small"
  before it reaches the screen.

## Out of scope

- The indicator's behaviour: the red/yellow/green dot and the OK/FAILED/CHECKING states (Task 8 of
  the source-picker plan).
- The portal logos themselves, and what the tile shows under `Manual` or `Portal`.
- The tile's tap target, size and position.
- The `TOKEN: OK` wording decision for custom sites.
