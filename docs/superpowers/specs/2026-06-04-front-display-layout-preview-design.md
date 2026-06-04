# Front-Display Layout Preview — Design

**Date:** 2026-06-04
**Status:** Draft (awaiting user review)
**Scope crate:** `refbox` only
**Process:** Lean (single crate; no shared-type / wire-format / hardware-format change)

---

## Goal

Bring back an in-app **preview** of the front-display layout on the Display Options page — the
feature that was designed as "Decision B" during the front-display-layouts work and then dropped
as "Decision D" because it was drawn as a *live canvas*, which crashes the Linux/Raspberry-Pi
renderer.

This design rebuilds the preview as a **pre-rendered static picture**, so the operator sees what
each layout looks like *before* applying it, with none of the crash risk.

See the prior record: `docs/superpowers/specs/2026-06-03-front-display-layouts-design.md`,
Decisions B / C / D.

## Why a picture, not a live preview

The live-canvas preview "looked great" but crashed: a constantly-redrawing canvas on the busy
Display Options page reliably trips a defect in the GUI library's live-display path
(`iced_graphics 0.13 damage::group`), which panics on the Linux/tiny-skia renderer the Pi uses.
The *rendering* was never the problem — being a *live widget on that page* was.

So instead of redrawing live, we **capture the real rendering once as a picture** and show that
picture with a plain image widget (no canvas, no damage path, no crash).

## How the pictures are made (the key decision)

The display-simulator window (`refbox/src/sim_app/`) already renders **every** layout correctly
and stably — it is a dedicated display window, not the busy settings page, so it never hits the
crash. iced 0.13 exposes `iced::window::screenshot(id) -> Task<Screenshot>`, which returns the
window's pixels as RGBA bytes (confirmed present in `iced_runtime-0.13.2`).

Therefore: a **maintenance command** drives the display-simulator window through each layout (with
fixed sample data), screenshots it, and writes the pictures to `refbox/resources/`. Those pictures
are **committed and baked into the app**, exactly like the existing logo PNGs. Every machine —
including the poolside Pi — only ever *displays* a ready-made picture; it never renders previews
itself.

This means **the layout-drawing code (`scoreboard::draw_*`) is not modified at all** — we only
screenshot its output. Zero risk to the real referee scoreboard rendering.

## Decisions (agreed during brainstorming)

- **D1 — Static picture, not live canvas.** Preview is a plain `iced::widget::Image`, never a
  canvas, on the Display Options page.
- **D2 — Pictures generated ahead of time, baked into the app.** Produced on a dev machine by a
  maintenance command, committed to `refbox/resources/`, shipped in the binary. End-user devices
  only display them. (Not generated on first launch, and not on demand.)
- **D3 — Side flip honoured; both variants for every layout.** The preview matches the staged
  *Starting Sides* (white-on-right) setting. To keep the capture tool and the display logic simple,
  **every layout gets both a white-left and a white-right picture — 5 layouts × 2 = 10 pictures.**
  (No case-by-case "does this layout depend on the side" logic. Layouts that happen not to differ by
  side — e.g. clock-only BigTime — just produce two identical pictures, which is harmless. Note the
  `Default`/panel mirror *does* depend on side, since the panel honours white-on-right.) The display
  picks the picture by `(front_display_layout, white_on_right)`.
- **D4 — Sample data.** Every picture uses representative data: **WHITE 5 – BLACK 3, First Half,
  8:42** (the same values the Decision-B preview used).
- **D5 — Default is previewed too.** The `Default` layout (the 256×64 LED-panel mirror) is captured
  as the simulator renders it, presented inside the same 16:9 preview box; any empty area is filled
  **black** (matching the real display background), not transparent. Like the others it gets both
  side variants (the panel honours white-on-right). Exact letterboxing/sizing of the wide strip in
  the 16:9 box is confirmed during the capture spike (see Risks).
- **D6 — Placement (Display Options page rearranged).** The page becomes:
  - Row 1: **Starting Sides** toggle (◀ BLACK … WHITE ▶), full width.
  - Row 2: **Hide Time for Last 15 Seconds** | **Display Layout** (two buttons side by side).
  - Row 3: **Open New Display** (left) — preview picture (right).
  - Row 4: **Player Display Brightness** (left) — preview picture (right).
  - Row 5: **Cancel** … (space) … **Apply**.

  The preview is a *single* picture occupying the right half across rows 3–4; the left half of those
  rows stacks Open New Display above Player Display Brightness. **No text caption** — just the
  picture. The picture scales with the window like the rest of the UI; pictures are rendered at a
  generous resolution so they stay crisp. (Note: *Player Display Brightness* is a physical-LED-panel
  setting with no on-screen effect, so the preview does not reflect it — expected, not a gap.)
- **D7 — Updates instantly on cycle.** Cycling the DISPLAY LAYOUT button swaps which committed
  picture is shown, before APPLY — reproducing the Decision-B behaviour, minus the crash. No
  rendering happens at runtime.
- **D8 — Automated "don't forget to refresh" guard (full drift check).** A pre-merge (CI) check
  re-creates the pictures from the current layout code and **fails the build if they differ** from
  the committed pictures, telling the contributor to re-run the refresh command and commit. This
  catches both a brand-new layout with no picture and a restyle of an existing layout. Requires the
  refresh command to generate pictures a consistent way (see Risks).

## Acceptance criteria (what the operator / reviewer can observe)

1. On the Display Options page, a preview picture of the **currently-staged** layout appears in the
   right half of the page (beside the Open New Display / Player Display Brightness controls), with no
   text caption, per the D6 layout.
2. Cycling the DISPLAY LAYOUT button updates the picture immediately to the newly-staged layout.
3. Toggling **Starting Sides** updates the picture to the matching white-left / white-right variant
   for layouts that have team sides.
4. The picture shows sample data (WHITE 5 – BLACK 3, First Half, 8:42) and looks like the real
   display for that layout.
5. Opening the Display Options page **never crashes** on the Linux/tiny-skia renderer (the failure
   mode from Decision D). This is the core regression guard.
6. Running the refresh command regenerates all preview pictures from the current layout code.
7. The pre-merge drift check fails if a layout is changed (or added) without refreshing its picture.

## Architectural sketch

Files expected to change / be added (all in `refbox`):

- **Capture / refresh command** — a new dev-facing entry point (e.g. a `refbox` subcommand or a
  small binary target) that:
  - instantiates the display-simulator rendering for a chosen `FrontDisplayLayout` + sample
    `TransmittedData` (reusing `sim_app` / `scoreboard::draw_*`, *without modifying them*),
  - renders at a fixed window size / scale factor,
  - calls `window::screenshot`, encodes the returned RGBA bytes → PNG (prefer `tiny_skia`, already
    present via the Linux renderer, to avoid a new dependency; a direct image-writing crate would
    need dependency approval per `rust.md`), and writes to `refbox/resources/`,
  - iterates over all 10 layout × side variants and exits.
- **Preview assets** — the committed PNGs in `refbox/resources/` (one per layout × side variant —
  10 files: 5 layouts × 2 sides).
- **Display Options page** — `refbox/src/app/view_builders/configuration.rs`
  (`make_display_config_page`): rearrange the page per D6, and load the committed picture matching
  the staged `front_display_layout` + `white_on_right`, shown via `Image` in the right half spanning
  the Open-New-Display / Player-Display-Brightness rows, with no text caption. Picture bytes embedded
  with `include_bytes!`, same pattern as existing logos in `shared_elements.rs`.
- **CI drift check** — a pre-merge job (and a `just` recipe) that runs the refresh command into a
  temporary location and compares against the committed PNGs, failing on mismatch (D8).

Reference points (from the merged front-display-layouts work):
- `FrontDisplayLayout` enum (Default / Classic / BigTime / Corners / ScoresOnly).
- `refbox/src/sim_app/` — the stable display-simulator window and its `Program` rendering.
- `refbox/src/sim_app/scoreboard.rs` — `draw_classic` / `draw_big_time` / `draw_corners` /
  `draw_scores_only` (read-only here; not modified).
- `matrix-drawing` `draw_panels` — the Default/panel rendering path.

## Out of scope

- **No change to `scoreboard::draw_*` or the panel-drawing code.** We screenshot their output.
- **No iced upgrade.** Stays on 0.13 (`.claude/rules/rust.md`).
- **No change to `uwh-common`, the wire/serial format, the TCP `SimFrame` selector, or hardware
  behaviour.**
- **No change to the staged APPLY/CANCEL picker behaviour** shipped in PR #949.
- **No live/runtime rendering of previews on any end-user device.**

## Risks & open implementation questions (for the plan)

1. **Headless screenshot in CI.** `window::screenshot` needs a rendered window. On a dev machine
   (WSLg/X11) this works; the CI drift check will need a virtual display (e.g. Xvfb) or an
   equivalent offscreen setup. This is the main implementation risk and should be proven early.
2. **Reproducibility of the drift check.** The committed pictures and the CI-regenerated pictures
   must match. Mitigate by generating only via the Linux/tiny-skia path with fixed window size and
   `scale_factor = 1`, with locked dependencies (Cargo.lock). If exact-pixel comparison proves
   flaky across machines, fall back to a small per-pixel tolerance — decide during implementation.
3. **Default-layout framing.** Confirm exactly how the simulator renders `Default` to a window
   (strip placement / scaling) so the captured picture matches D5 (16:9 box, black fill). Adjust
   capture window sizing accordingly.
4. **Branch base.** This is a separate feature; confirm the correct base (master vs. the
   front-display-layouts line) so the capture command sees the layout code, before implementation.
   Will need its own branch (e.g. `feat/refbox/display-layout-preview`).

## Process

Lean process: `refbox`-only, no shared/wire/hardware change. Code review once at the end. Nothing
committed or pushed without the user's go-ahead. Adding a CI workflow step touches shared
infrastructure — flag and confirm before wiring it.
