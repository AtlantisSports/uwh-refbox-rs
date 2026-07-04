# Beep Test: Display Layout picker, preview, and Default-on-load — Design

**Date:** 2026-06-19
**Crate:** `refbox` only
**Target release:** v0.4.3
**Process:** Lean (refbox UI; no `uwh-common`, no wire format, no state machine, no firmware)

---

## Goal

When beep test runs, the player-facing display should default to the **Default** layout —
the only layout that blanks the black side and shows just the lap count on the white side.
Today beep test has no way to choose a layout, and it inherits whatever layout (and sides
setting) the game mode last used. This change:

1. Forces the player display to **Default** every time beep test boots.
2. Adds a **DISPLAY LAYOUT** picker and a beep-test **PREVIEW** to the Beep Test Settings page.
3. Rearranges the Beep Test Settings landing into the operator's requested 2-column grid.
4. Pins the beep-test display to **white-on-left** (lap count on the left), with no
   switch-sides control.

## Background (current behavior, verified)

- The Beep Test Settings landing is built by `build_beep_test_settings_landing`
  ([refbox/src/app/view_builders/beep_test_settings.rs:41](../../../refbox/src/app/view_builders/beep_test_settings.rs#L41)).
  Today it is a 2×2 of `[Sound Settings][Edit Levels]` / `[App Mode][Language]` plus a
  `[BACK] … [RESTART TO APPLY]` footer.
- Display layout is one shared setting, `config.front_display_layout`
  (`Default / Classic / BigTime / Corners / ScoresOnly`,
  [refbox/src/sim_frame.rs:9](../../../refbox/src/sim_frame.rs#L9)).
- At startup the player display is set to `Default` when a real LED panel is connected,
  else to `config.front_display_layout`
  ([refbox/src/app/mod.rs:1447](../../../refbox/src/app/mod.rs#L1447)). So in the
  sim / secondary-screen case, beep test currently inherits the game layout.
- Only the **Default** layout renders through `draw_panels`, which skips the black score
  during beep test. The other four are full-screen scoreboard layouts that render the
  black score as `0`.
- The beep-test tick sends its snapshot with `config.hardware.white_on_right`
  ([refbox/src/app/mod.rs:3973](../../../refbox/src/app/mod.rs#L3973), default `false`).
- The game-mode Display page (`make_display_config_page`,
  [refbox/src/app/view_builders/configuration.rs:1049](../../../refbox/src/app/view_builders/configuration.rs#L1049))
  already shows the pattern to mirror: a `front-display-layout` value button, an
  `effective_layout` that is forced to `Default` when `has_led_panel`, and a static-image
  preview via `layout_preview_handle`. Preview PNGs are produced by
  [refbox/src/sim_app/capture.rs](../../../refbox/src/sim_app/capture.rs) from a **game**
  sample (`scores: black 3, white 5`, `beep_test: false`).

## Behavior decisions (approved)

1. **Reset to Default each boot.** The beep-test layout is held in memory only and starts
   at `Default` every time the app boots in beep-test mode. It is **never persisted** and
   **never touches** `config.front_display_layout` (the game-mode setting).

2. **DISPLAY LAYOUT picker (live-apply).** A value button on the landing cycles
   `Default → Classic → BigTime → Corners → ScoresOnly`. Pressing it changes the in-memory
   layout and immediately pushes it to the player display (no Apply step). It reuses the
   existing labels — `front-display-layout`, `layout-default`, `layout-classic`,
   `layout-big-time`, `layout-corners`, `layout-scores-only` — so **no new translations**.

3. **Beep-test PREVIEW.** A static image (bottom-right, spanning the two lower rows) showing
   the real beep-test appearance of the *effective* layout. Default = lap count on white,
   black blank; the others = lap count on white, `0` on black (honestly showing why Default
   is the right pick). White-on-left only.

4. **Grid rearranged** to exactly:

   ```
   [App Mode]        [Edit Levels]
   [Sound Settings]  [Display Layout]
   [Language]        [ Preview            ]
   [ (blank) ]       [ (preview continues) ]
   ```

   The `[BACK] … [RESTART TO APPLY]` footer is unchanged.

5. **Real LED panel connected ⇒ locked to Default.** When `has_led_panel` is true the
   DISPLAY LAYOUT button is grayed (no `on_press`), its label shows `Default`, and the
   preview shows the Default beep-test image — identical to the game-mode Display page. The
   picker only does something when the player display runs on an ordinary screen.

6. **Ready-state gating.** DISPLAY LAYOUT is interactive only before a beep test has run
   (`!beep_test_has_run`), matching App Mode / Edit Levels / Language. (SOUND SETTINGS stays
   live mid-run; DISPLAY LAYOUT does not.)

7. **White-on-left, no sides toggle.** The beep-test display is pinned to
   `white_on_right = false` so the lap count is always on the left, independent of the
   game-mode sides setting. No switch-sides control is added. The preview therefore needs
   only the white-on-left variant.

## Architecture / changes

All changes are in `refbox`.

### State (`refbox/src/app/mod.rs`)
- Add field `beep_test_display_layout: FrontDisplayLayout` to the app struct. Initialize to
  `FrontDisplayLayout::Default` in `App::new` (regardless of mode; only read in beep-test
  mode).
- Startup layout init (~[mod.rs:1447](../../../refbox/src/app/mod.rs#L1447)): send `Default`
  when `has_led_panel` **or** `config.mode == Mode::BeepTest`; else `config.front_display_layout`.
- Beep-test tick send (~[mod.rs:3973](../../../refbox/src/app/mod.rs#L3973)): pass `false`
  for `white_on_right` (pin white-on-left).
- Landing view call (~[mod.rs:4581](../../../refbox/src/app/mod.rs#L4581)): also pass
  `self.beep_test_display_layout` and `self.has_led_panel`.

### Message (`refbox/src/app/message.rs`)
- Add `Message::BeepTestCycleDisplayLayout`. Handler: set
  `beep_test_display_layout = beep_test_display_layout.next()`, then
  `update_sender.set_layout(effective)` where
  `effective = if has_led_panel { Default } else { beep_test_display_layout }`.
  (A dedicated message — not `CycleParameter(FrontDisplayLayout)` — so the persisted
  game-mode layout path is never touched.)

### View (`refbox/src/app/view_builders/beep_test_settings.rs`)
- `build_beep_test_settings_landing` gains two params: `beep_test_layout: FrontDisplayLayout`
  and `has_led_panel: bool`.
- Compute `effective_layout = if has_led_panel { Default } else { beep_test_layout }`; map to
  the existing `layout-*` label keys.
- DISPLAY LAYOUT = `make_value_button(fl!("front-display-layout"), label, (false, true), …)`
  with `on_press = Some(Message::BeepTestCycleDisplayLayout)` only when
  `!has_led_panel && !has_run`; otherwise `None` (grayed).
- PREVIEW = `Image::new(beep_test_layout_preview_handle(effective_layout))` in a container,
  occupying the right column across the two lower rows. Build the grid with the preview as a
  fixed-height element spanning two row-heights (mirror the height math the existing landing
  uses; see the `iced` "no table widget" note — compose rows/columns, no rowspan widget).
- Reorder tiles to the approved grid; bottom-left cell is a `horizontal_space()`. Keep the
  existing BACK / RESTART-TO-APPLY footer exactly as-is.

### Preview images (`refbox/src/sim_app/capture.rs` + `refbox/resources/layout-previews/`)
- Add a beep-test sample (`beep_test: true`, `white_on_right: false`,
  `scores: { black: 0, white: 12 }` — `12` as a representative two-digit lap count) and emit
  the **five** white-on-left layouts.
- File stems: `beep-default`, `beep-classic`, `beep-big-time`, `beep-corners`,
  `beep-scores-only` (no side suffix). Existing game previews are left unchanged.
- `run_capture` produces both the existing game set and the new beep-test set in one run.
- Add `beep_test_layout_preview_handle(layout) -> image::Handle` **in
  `beep_test_settings.rs`** (keeps beep-test code self-contained and avoids widening the
  visibility of `configuration.rs`'s private helper). It mirrors `layout_preview_handle`'s
  `include_bytes!` pattern but is layout-only and references the five new PNGs. The
  `include_bytes!` relative path is the same `../../../resources/layout-previews/` because
  both files live in `view_builders/`. Exhaustive match over all five layouts.
- Commit the five generated PNGs.

## Acceptance criteria (operator-observable)

- Booting beep test shows the **Default** layout: lap count on the **left**, black side blank
  (verified in the panel simulator; on the physical panel this still depends on the panel
  firmware — see the separate firmware backlog item).
- The Beep Test Settings landing shows the new grid with a working **DISPLAY LAYOUT** tile
  and a **PREVIEW** that updates as the layout cycles.
- Cycling DISPLAY LAYOUT (no panel connected) changes the player display live and resets to
  Default the next time beep test boots; the game-mode layout setting is unchanged.
- With a real LED panel connected, DISPLAY LAYOUT is grayed at `Default` and the preview
  shows the Default beep-test image.
- No new strings appear untranslated (all reuse existing keys).

## Out of scope / non-goals

- Game mode, its Display page, and `config.front_display_layout` (untouched).
- The scoreboard `led-panel` firmware and how the physical panel renders (separate backlog:
  `docs/backlog/beep-test-panel-firmware/`).
- Making the non-Default layouts blank the black side in beep test.
- Beep-test timing, levels, sound, and lap-count logic.
- Any switch-sides / white-on-right control for beep test.

## Testing

- `just check` (fmt, clippy `-D warnings`, tests, audit).
- Walkthrough in the simulator (no panel): boot beep test → confirm Default + lap-on-left +
  blank black; open Settings → confirm grid + cycle through all five layouts watching the
  preview; confirm game-mode Display setting is unchanged after exiting beep test.
- Confirm preview PNGs render at the right size and the Default beep-test preview shows a
  blank black side.
- Unit-test coverage where cheap: `beep_test_display_layout` initializes to `Default`; the
  cycle handler advances it and does not mutate `config.front_display_layout`.
