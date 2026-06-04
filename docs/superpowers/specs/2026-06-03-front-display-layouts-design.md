# Front Display Layouts — Design Spec

**Date:** 2026-06-03
**Status:** Implemented (live version, PR #949); **redesign approved** (staged + preview), pending plan
**Crate scope:** `refbox` only

---

## Goal

Give the refbox a choice of full-screen, player/spectator-facing "front display" layouts —
ported in spirit from the web refbox's layout carousel — built natively in the Rust refbox using
existing conventions. The layouts are only relevant when the custom LED panel is **not** connected;
when it is, the display stays locked to today's behaviour.

The web reference lives on `origin/sandbox-cha-refbox-clean` in the `uwh-portal` repo
(`DisplayOptionsPage.tsx`, `ScoreboardDisplayPage.tsx`), which offers four layouts:
`classic`, `bigTime`, `corners`, `scoresOnly`. We reproduce those four plus the existing panel.

## Scope boundary

**In scope (all in the `refbox` crate):**
- Four new full-screen layouts drawn in the display window (`refbox/src/sim_app/`).
- A `Default` layout = today's 256×64 panel mirror, unchanged.
- A "Display Layout" cycling value-button on the Display Options config page.
- Persisting the choice in the refbox config file.
- Carrying the choice to the display window over the existing display feed (live updates).
- Hardware gating: lock to `Default` and disable the button when a real LED panel is connected.

**Explicitly out of scope:**
- No change to the physical 256×64 LED panel rendering or to its data/wire format
  (`matrix-drawing::transmitted_data::TransmittedData` as sent over **serial** stays byte-for-byte
  identical). Hardware and the wireless remote are untouched.
- No real team names or custom team colours — teams remain **WHITE** / **BLACK**. (Adding names
  would require extending the shared `uwh-common` snapshot / wire format — deliberately avoided.)
- No changes to the stream `overlay` crate.
- The new layouts do **not** show penalties, fouls, warnings, or timeouts-used (matching the web
  layouts). They show scores, clock, period, and timeout state only.

## Acceptance criteria (observable)

1. With no LED panel connected, the Display Options page shows a **Display Layout** button that
   cycles `Default → Classic → Big Time → Corners → Scores Only → Default`.
2. Selecting a layout changes every open display window **live** (within a fraction of a second).
3. Each new layout renders correctly, updates live with score/clock/period changes, and places
   WHITE/BLACK on the correct sides per the existing "White is on Left/Right" setting.
4. Starting a timeout shows the timeout countdown + type label on Classic / Corners / Big Time;
   Scores Only is unaffected.
5. With a real LED panel connected (serial port supplied), the button shows **Default** and is
   greyed out; the panel renders exactly as it does today.
6. The selected layout persists across an app restart and defaults to `Default`.
7. `Default` is pixel-identical to today's panel-mirror behaviour.
8. `cargo test -p refbox` and `just check` pass.

## Architecture sketch

### Rendering (in `refbox/src/sim_app/`)
- `sim_app` already draws to an `iced` `Canvas` that scales to the window
  (`SimRefBoxApp` / `Program::draw` in `refbox/src/sim_app/mod.rs`), and already renders both the
  256×64 `Matrix` buffer and the `Sunlight` display, plus text via `frame.fill_text`.
- Add a layout enum (working name `FrontDisplayLayout`) with variants
  `Default, Classic, BigTime, Corners, ScoresOnly`.
- Add one draw routine per new layout. Each composes filled rectangles (score boxes, period pill)
  and text (labels, scores, clock) positioned as **fractions of the canvas bounds**, so they scale
  to any window size / fullscreen (mirrors the web's responsive scaling; no fixed 1920×1080).
- `Default` keeps calling `draw_panels` exactly as today.
- The existing `DisplaySim` enum (`Matrix` / `Sunlight`) is extended or wrapped so the active
  layout selects the draw path. Sunlight mode is orthogonal and unchanged.

### Carrying the choice to the window (Approach A — live)
- The display window receives `TransmittedData` frames over a **TCP** connection
  (`snapshot_listener` in `sim_app/mod.rs`, fed by the binary worker in
  `refbox/src/app/update_sender.rs`).
- Introduce a display-feed frame that wraps the existing payload with a one-byte layout selector,
  used **only on the TCP path to the display window**. The **serial** path to the physical panel
  continues to send the unchanged `TransmittedData` encoding — the hardware/wireless format is not
  modified. (Implementation detail for the plan: either a thin `SimFrame { layout, data }` wrapper
  for the TCP worker, or an equivalent that keeps the serial encoder untouched.)
- The display window decodes the layout byte each frame and draws accordingly. Because frames
  arrive many times per second, cycling the button produces a near-instant switch with no relaunch
  and no window reposition.

### Setting + persistence
- Add a `front_display_layout` field to the refbox config (`refbox/src/config.rs`, same file that
  stores brightness and white-on-right), defaulting to `Default`.

### UI button + gating
- Add the button on the Display config page (`ConfigPage::Display` in
  `refbox/src/app/view_builders/configuration.rs`), using the existing `make_value_button` helper
  so it matches the display-mode and brightness buttons.
- Add a `Message` variant (working name `CycleFrontDisplayLayout`) handled in
  `refbox/src/app/mod.rs`, advancing the layout, saving config, and pushing the new value into the
  display feed.
- Reuse the existing "real panel connected?" signal (`has_led_panel`, derived from
  `!serial_ports.is_empty()`, see `refbox/src/app/mod.rs:1222` and the `open-new-display` gating in
  `view_data.rs`) to force `Default` and disable the button when hardware is present.

### Text rendering
- Large digits/clock need a scalable font on the canvas. `sim_app` already renders text via
  `frame.fill_text`. The plan will confirm which bundled font to use for the large clock (likely a
  monospace face to match the web look). No new dependency expected.

## The layouts

Background is black on all four. WHITE = white box / black number; BLACK = black box with a thin
outline / white number. Labels "WHITE"/"BLACK" above the boxes. Clock = bright yellow monospace
`MM:SS`. Period pill colour: **green** during play, **yellow** for breaks/pre-game, **red** for
sudden death. Side placement follows the existing white-on-right setting.

1. **Classic** — three columns: WHITE label + large score box (left), period pill above large
   centred clock (centre), BLACK label + large score box (right).
2. **Big Time** — the game clock only, filling the screen, centred.
3. **Corners + time** — small WHITE/BLACK score boxes in the top corners, period pill + very large
   clock centred below.
4. **Scores only** — the two team labels + large score boxes, left and right; no clock, no period.

### Timeout behaviour
- On **Classic / Corners / Big Time**, while a timeout runs the clock area shows the **timeout
  countdown** and the period pill shows the timeout type: `WHITE T/O`, `BLACK T/O`, `REF T/O`, or
  `PENALTY SHOT`. Big Time (no pill normally) shows a small timeout-type label above the countdown.
- **Scores only** has no clock and is unchanged during timeouts.

## Testing

- **Automated (`cargo test -p refbox`):**
  - Round-trip encode/decode of the display-feed frame carrying the layout byte.
  - Proof the serial/physical-panel encoding is unchanged (existing `TransmittedData` tests stay
    green; the layout byte never appears on the serial path).
  - Config save/load of `front_display_layout`, default `Default`.
  - Layout cycle order and hardware-gating logic.
- **Manual:** run the refbox with the simulator, eyeball each layout (scores/clock/period/timeout,
  side flipping, live switching, restart persistence, hardware-gating). Final visual confirmation
  against the web versions via the browser companion, with the user's permission.

## Rough task list (for the plan)

1. Add `FrontDisplayLayout` enum (cycle order, display names, translations) and config field.
2. Display-feed frame: wrap payload + layout byte on the TCP path; leave serial encoder untouched;
   round-trip tests.
3. `sim_app`: decode layout per frame; dispatch to draw routines; keep `Default` → `draw_panels`.
4. Implement the four draw routines (fraction-based positioning; shared colour/label helpers).
5. Timeout handling in the clock-bearing layouts.
6. Display Options button (`make_value_button`) + `Message` + `update()` handling + config save +
   push to feed.
7. Hardware gating (force `Default`, disable button).
8. Translations for new strings (en-US plus existing locales).
9. `cargo test -p refbox`, `just check`, manual walkthrough, browser visual confirmation.

## Redesign (2026-06-03) — supersedes the *trigger* in Approach A

After PR #949 was opened, the user revised two decisions during the Display-Options visual review.
The live version above shipped on the branch; this redesign folds into the **same PR #949**.

### Decision A — the layout becomes a STAGED setting (APPLY / CANCEL), not live

Picking a layout no longer changes any open display window instantly. Instead it behaves exactly
like the other settings on the Display Options page (brightness, white-on-right, hide-time):

- Cycling the **DISPLAY LAYOUT** button changes only the *staged* (uncommitted) choice.
- **APPLY** commits the choice: it is saved to config and pushed to the display window.
- **CANCEL** reverts the staged choice; no display window ever changed, so nothing to undo.

This *supersedes* the old "Back instead of Cancel" backlog idea: we keep **CANCEL / APPLY** and gain
consistency by staging the layout rather than renaming the button.

The display-feed transport from Approach A is unchanged — the layout still travels to the window via
the one-byte `SimFrame` selector on the TCP path, and the serial/hardware format is still untouched.
Only the *moment* the layout is pushed changes: on APPLY instead of on every button press.

Revised acceptance criterion (replaces criterion 2): *Selecting a layout updates the open display
window only after **APPLY**; **CANCEL** discards the selection and the window is unaffected.*

### Decision B — an in-UI layout preview on the Display Options page

The Display Options page shows a small preview (roughly 16:9) of the **currently-staged** layout,
drawn with the real layout-rendering code and representative sample data
(**WHITE 5 – BLACK 3, First Half, 8:42**), honouring the staged white-on-right setting. The preview
updates as the user cycles the button, before APPLY. The `Default` choice previews as a miniature
256×64 LED-panel mirror (the same `draw_panels` rendering the physical panel uses).

New acceptance criterion: *The Display Options page shows a live preview of the staged layout that
updates as the button is cycled, including a mini panel render for `Default`.*

### Decision C (2026-06-03) — preview stability via a cached canvas + the rearranged row

Live testing of Decision B revealed a crash/blanking bug. The preview canvas, as first built,
re-rendered on **every** `view()` — including every game-clock tick. That constant per-frame
canvas redraw repeatedly tripped a known `iced_graphics 0.13` defect in `damage::group` (a
sort whose comparator isn't a total order), which on Rust 1.85 **panics** (`'user-provided
comparison function does not correctly implement a total order'`). Symptoms: the app crashed,
or the DISPLAY LAYOUT button + preview vanished (the unstable preview collapsed their shared
row), and the WHITE score box failed to composite. The real full-screen display never hits this
because it renders through a persistent `canvas::Cache` and only redraws on new data.

**Fix — cache the preview, the same way the live display does:**
- A `canvas::Cache` lives on `RefBoxApp` (`display_preview_cache`). It is handed to the preview
  via a parameter threaded `build_game_config_edit_page → make_display_config_page →
  layout_preview` (NOT through `ViewData`, which derives `Eq` that `Cache` doesn't implement).
- `LayoutPreview` draws through `self.cache.draw(...)` instead of a fresh `Frame` each call, so
  it reuses cached geometry until explicitly cleared.
- The cache is cleared (→ preview re-renders) on exactly the actions that change the preview:
  cycling the layout (`CycleParameter`), flipping starting-sides (`ToggleBoolParameter`),
  entering the Display page (`ChangeConfigPage`), and Cancel (`CancelConfigPage`). Clock ticks
  do **not** clear it, so they produce no preview damage.
- The preview shows fixed sample data and does not tick with the live clock — this is intended
  and is what makes caching valid.

**Rearranged row (folded in):** the Display page's layout row becomes a left column with
DISPLAY LAYOUT stacked above OPEN NEW DISPLAY, and the cached preview filling the right half.
The earlier disappearing-button behaviour was a symptom of the same redraw bug; with the preview
cached and stable, the shared row no longer collapses.

Scope guard: still `refbox`-only. No `iced` upgrade, no change to the shared `scoreboard::draw_*`
routines or the sim display window, no serial/hardware change.

### Decision D (2026-06-04) — the in-UI preview is DROPPED

The caching fix from Decision C did **not** stop the crash. Live testing showed the
`iced_graphics::damage::group` total-order panic fires the instant the Display Options page is
opened (deterministic on navigation, not time-based) — caching reduces redraws but the canvas is
still drawn when the page first composites, which is enough to trip the bug on the Linux/tiny-skia
renderer (which is also what the Raspberry-Pi deployment uses). The preview shows fixed sample
data, but it is *drawn live on a canvas* (reusing the real `scoreboard::draw_*` code), and that
canvas is the trigger.

Decision: **remove the in-UI preview entirely** (revert Decision B + the Decision C caching/
rearrangement commits). What ships:
- The **staged APPLY/CANCEL layout picker** (Decision A) — solid, no canvas, unaffected.
- **OPEN NEW DISPLAY**, which already lets the operator see the real chosen layout in its own
  (stable) window.

The three reverted commits were `162c8a0b` (preview), `f62a3004` (cache attempt), `f4dd7f2e`
(rearrangement). The `scoreboard::draw_*` routines and the simulator display window are untouched
— only the Display-Options *preview tile* is removed.

A genuine **static-image** preview (pre-rendered picture per layout, shown via a plain `Image`
widget — no canvas, no damage bug) remains a possible *future, separate* task if the preview is
wanted back. It was not pursued here because it is its own piece of work (generating/bundling
images, side-flip variants, keeping them in sync with layout changes).

## Process

Lean process (single crate, no shared/wire/hardware format change). Code review once at the end;
no per-task deviation commits. Nothing committed or pushed without the user's go-ahead.

## Deviations

_(none yet)_
