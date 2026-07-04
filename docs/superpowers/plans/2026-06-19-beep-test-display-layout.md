# Beep Test Display Layout — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Default the beep-test player display to the Default layout (white-on-left, black side blank), and add a Display Layout picker + beep-test preview to the Beep Test Settings page in the operator's requested grid.

**Architecture:** A session-only in-memory layout (`beep_test_display_layout`) starts at `Default` each boot, is changed live by a new `BeepTestCycleDisplayLayout` message, and is never persisted (the game-mode `config.front_display_layout` is untouched). The picker/preview/pushed-layout all agree via a shared `effective_beep_layout` helper. Beep-test previews are real captures (5 new white-on-left PNGs) from the existing preview-capture tool.

**Tech Stack:** Rust 2024, `iced` 0.13, the existing `refbox` view-builder + `update()` patterns, `matrix-drawing`/`sim_frame` types.

## Global Constraints

- Crate: **`refbox` only**. No `uwh-common`, wire-format, state-machine, or firmware changes.
- Rust edition 2024, **MSRV 1.85**. No new dependencies. No `#[allow(...)]` to silence warnings.
- Lint gate mirrors CI for this bin crate: `cargo clippy -p refbox -- -D warnings` (NOT `--all-targets`). Tests: `cargo test -p refbox` (no `--lib`). Full gate: `just check`.
- **No new translation keys** — reuse `front-display-layout`, `layout-default`, `layout-classic`, `layout-big-time`, `layout-corners`, `layout-scores-only`.
- Beep-test display is **white-on-left only**; no switch-sides control.
- **No version bump** here (0.4.2 → 0.4.3 happens when cutting the release, separately).
- Branch creation, commits, and pushes require the **user's explicit approval** (per project rules). Commit steps below are where approval is sought.
- `-D warnings` means new symbols (helper fn, struct field, message variant, preview helper) must land **together with their real callers** in one task — see Task 3.

---

## Branch / worktree setup (pre-Task 1, with user approval)

```bash
git fetch origin master
git switch -c feat/refbox/beep-test-display-layout origin/master
```
(Or an isolated worktree off `origin/master` via the using-git-worktrees skill. Do NOT branch from the current buzzer branch.)

---

## Task 1: Default + white-on-left for the beep-test display

Makes beep test always boot on the Default layout with the lap count on the left, independent of the game-mode layout/sides settings. No new symbols — edits existing code only.

**Files:**
- Modify: `refbox/src/app/mod.rs` (startup layout init `~1448-1452`; beep-test tick send `~3973`)

**Interfaces:**
- Produces: no new public symbols. Behavior: in beep-test mode the player display is forced to `FrontDisplayLayout::Default` and `white_on_right = false`.

- [ ] **Step 1: Force Default layout on beep-test boot**

In `refbox/src/app/mod.rs`, the `UpdateSender::new(...)` call's layout argument (currently):

```rust
            if has_led_panel {
                crate::sim_frame::FrontDisplayLayout::Default
            } else {
                config.front_display_layout
            },
```

becomes:

```rust
            if has_led_panel || config.mode == Mode::BeepTest {
                crate::sim_frame::FrontDisplayLayout::Default
            } else {
                config.front_display_layout
            },
```

- [ ] **Step 2: Pin white-on-left in the beep-test tick send**

In the `Message::BeepTestTick` arm, the `send_snapshot` call (currently):

```rust
                    if let Err(e) = self.update_sender.send_snapshot(
                        game_snap,
                        self.config.hardware.white_on_right,
                        self.config.hardware.brightness,
                    ) {
```

becomes (only the middle argument changes):

```rust
                    if let Err(e) = self.update_sender.send_snapshot(
                        game_snap,
                        // Beep test has no sides control: lap count always on the left.
                        false,
                        self.config.hardware.brightness,
                    ) {
```

- [ ] **Step 3: Build and lint**

Run: `cargo build -p refbox` → Expected: builds.
Run: `cargo clippy -p refbox -- -D warnings` → Expected: clean.

> No unit test: both edits are inline in `App::new` / the tick handler, which the existing test suite does not construct. Verified by the final walkthrough in Task 3 (beep test boots Default, lap on the left). This is the project's lean process for UI/wiring changes.

- [ ] **Step 4: Commit (with user approval)**

```bash
git add refbox/src/app/mod.rs
git commit -m "feat(refbox): default beep-test display to Default layout, white-on-left"
```

---

## Task 2: Beep-test preview images

Extends the preview-capture dev tool to also render five white-on-left beep-test previews, then generates and commits them. No app-facing code uses them yet (the helper that embeds them lands in Task 3, alongside its caller, to avoid a dead-code lint).

**Files:**
- Modify: `refbox/src/sim_app/capture.rs`
- Create (generated): `refbox/resources/layout-previews/beep-default.png`, `beep-classic.png`, `beep-big-time.png`, `beep-corners.png`, `beep-scores-only.png`
- Test: `refbox/src/sim_app/capture.rs` `#[cfg(test)] mod tests`

**Interfaces:**
- Produces: `beep_sample_data() -> TransmittedData`, `layout_stem(FrontDisplayLayout) -> &'static str`, and an internal `Variant` enum driving the capture loop. Five committed `beep-*.png` files.

- [ ] **Step 0: Confirm local env reproduces the committed previews**

Run: `just check-previews`
Expected: `Layout previews are up to date.` (If this FAILS on the clean tree, the local render env differs from the committed PNGs' env — stop and resolve before generating, or generate under CI's xvfb, otherwise the new PNGs won't match CI's `check-previews`.)

- [ ] **Step 1: Extract `layout_stem` and refactor `file_stem`**

Replace the existing `file_stem` function in `capture.rs` with:

```rust
/// The layout's filename token, e.g. `big-time`.
fn layout_stem(layout: FrontDisplayLayout) -> &'static str {
    match layout {
        FrontDisplayLayout::Default => "default",
        FrontDisplayLayout::Classic => "classic",
        FrontDisplayLayout::BigTime => "big-time",
        FrontDisplayLayout::Corners => "corners",
        FrontDisplayLayout::ScoresOnly => "scores-only",
    }
}

/// Stable filename (no extension) for a game layout/side pair, e.g. `classic-white-right`.
pub(crate) fn file_stem(layout: FrontDisplayLayout, white_on_right: bool) -> String {
    let side = if white_on_right {
        "white-right"
    } else {
        "white-left"
    };
    format!("{}-{}", layout_stem(layout), side)
}
```

- [ ] **Step 2: Add the beep-test sample**

Add next to `sample_data` in `capture.rs`:

```rust
/// Beep-test sample shown in the beep-test previews: lap count 12 on the white
/// side, black side 0, white on the left. Mirrors the
/// `BeepTestSnapshot -> GameSnapshotNoHeap` conversion (BetweenGames,
/// lap_count as the white score). Default blanks the black side; the other
/// layouts render the 0 — which is exactly what the preview should show.
pub(crate) fn beep_sample_data() -> TransmittedData {
    let snapshot = uwh_common::game_snapshot::GameSnapshotNoHeap {
        current_period: GamePeriod::BetweenGames,
        secs_in_period: 45,
        scores: BlackWhiteBundle { black: 0, white: 12 },
        ..Default::default()
    };

    TransmittedData {
        white_on_right: false,
        flash: false,
        beep_test: true,
        brightness: Brightness::Low,
        snapshot,
    }
}
```

- [ ] **Step 3: Introduce the `Variant` enum and rewrite `variants()`**

Replace the existing `variants()` function with the enum + updated builder:

```rust
/// One preview to render.
#[derive(Clone, Copy)]
enum Variant {
    /// Game-state preview, both starting-side orientations.
    Game {
        layout: FrontDisplayLayout,
        white_on_right: bool,
    },
    /// Beep-test preview, white-on-left only (beep test has no sides control).
    Beep { layout: FrontDisplayLayout },
}

impl Variant {
    fn layout(self) -> FrontDisplayLayout {
        match self {
            Variant::Game { layout, .. } | Variant::Beep { layout } => layout,
        }
    }

    fn sample(self) -> TransmittedData {
        match self {
            Variant::Game { white_on_right, .. } => sample_data(white_on_right),
            Variant::Beep { .. } => beep_sample_data(),
        }
    }

    fn file_stem(self) -> String {
        match self {
            Variant::Game {
                layout,
                white_on_right,
            } => file_stem(layout, white_on_right),
            Variant::Beep { layout } => format!("beep-{}", layout_stem(layout)),
        }
    }
}

/// Every preview: 10 game (5 layouts x 2 sides) + 5 beep (5 layouts, white-on-left).
pub(crate) fn variants() -> Vec<Variant> {
    let layouts = [
        FrontDisplayLayout::Default,
        FrontDisplayLayout::Classic,
        FrontDisplayLayout::BigTime,
        FrontDisplayLayout::Corners,
        FrontDisplayLayout::ScoresOnly,
    ];
    let mut out = Vec::with_capacity(15);
    for layout in layouts {
        for white_on_right in [false, true] {
            out.push(Variant::Game {
                layout,
                white_on_right,
            });
        }
    }
    for layout in layouts {
        out.push(Variant::Beep { layout });
    }
    out
}
```

- [ ] **Step 4: Update `push_variant`, `save_png`, and the `CaptureApp` loop to take `Variant`**

Replace `push_variant` and `save_png`:

```rust
fn push_variant(sim: &mut SimRefBoxApp, variant: Variant) {
    // `update` returns Task::none() for NewSnapshot; nothing to schedule.
    let _ = sim.update(SimMessage::NewSnapshot(SimFrame {
        layout: variant.layout(),
        data: variant.sample(),
    }));
}

fn save_png(dir: &Path, variant: Variant, shot: &Screenshot) {
    let buf = image::RgbaImage::from_raw(shot.size.width, shot.size.height, shot.bytes.to_vec())
        .expect("screenshot byte length matches its reported size");
    let path = dir.join(format!("{}.png", variant.file_stem()));
    buf.save(&path).expect("write preview png");
    println!("wrote {}", path.display());
}
```

Change the `CaptureApp` field type from `variants: Vec<(FrontDisplayLayout, bool)>` to:

```rust
    variants: Vec<Variant>,
```

In `CaptureApp::update`, the `Message::Captured(shot)` arm becomes:

```rust
            Message::Captured(shot) => {
                self.awaiting_shot = false;
                let variant = self.variants[self.index];
                save_png(&self.out_dir, variant, &shot);

                self.index += 1;
                if self.index >= self.variants.len() {
                    iced::exit()
                } else {
                    push_variant(&mut self.sim, self.variants[self.index]);
                    self.settle = SETTLE_FRAMES;
                    Task::none()
                }
            }
```

(`CaptureApp::new`'s `push_variant(&mut sim, variants[0]);` now passes a `Variant` — no change to that line's text.)

- [ ] **Step 5: Update/add tests**

Replace the existing `there_are_ten_variants_with_unique_filenames` test function with:

```rust
    #[test]
    fn variants_cover_game_and_beep_with_unique_filenames() {
        let v = variants();
        // 5 layouts x 2 sides (game) + 5 layouts white-on-left (beep) = 15.
        assert_eq!(v.len(), 15);
        let beep = v
            .iter()
            .filter(|x| matches!(x, Variant::Beep { .. }))
            .count();
        assert_eq!(beep, 5);
        let stems: std::collections::HashSet<String> = v.iter().map(|x| x.file_stem()).collect();
        assert_eq!(stems.len(), 15, "all preview filenames must be unique");
        assert!(stems.contains("beep-default"));
        assert!(stems.contains("beep-scores-only"));
        assert!(stems.contains("default-white-left"));
    }

    #[test]
    fn beep_sample_has_lap_count_on_white_and_blank_black() {
        let d = beep_sample_data();
        assert!(d.beep_test);
        assert!(!d.white_on_right);
        assert_eq!(d.snapshot.scores.white, 12);
        assert_eq!(d.snapshot.scores.black, 0);
    }
```

- [ ] **Step 6: Run the capture-tool tests**

Run: `cargo test -p refbox capture`
Expected: PASS (`variants_cover_game_and_beep_with_unique_filenames`, `beep_sample_has_lap_count_on_white_and_blank_black`, plus the existing `sample_has_expected_score_and_clock`).
Run: `cargo clippy -p refbox -- -D warnings`
Expected: clean (every new symbol is used within `capture.rs`/its tests).

- [ ] **Step 7: Generate the preview PNGs**

Run (needs a display; the recipe sets `WAYLAND_DISPLAY=` to force X11 on WSLg):
`just capture-previews`
Expected: a capture window opens, prints `wrote …` for all 15 variants, then exits.

Then verify:
Run: `just check-previews` → Expected: `Layout previews are up to date.`
Run: `git status --short refbox/resources/layout-previews`
Expected: exactly **five new** files (`beep-*.png`) and **no modified** game PNGs. If any `*-white-left.png` / `*-white-right.png` game file shows as modified, the render env diverged from the committed assets — `git checkout` those game files and investigate before committing (only the five beep files should change).

- [ ] **Step 8: Commit (with user approval)**

```bash
git add refbox/src/sim_app/capture.rs refbox/resources/layout-previews/beep-*.png
git commit -m "feat(refbox): add beep-test layout preview images"
```

---

## Task 3: Display Layout picker + preview wiring

Adds the session layout state, the cycle message + handler, the shared `effective_beep_layout` helper, the preview-embedding helper, and the rearranged landing grid — all together, because `-D warnings` rejects any of these symbols without its real caller. Steps are bite-sized; intermediate states compile under plain `cargo build` (dead-code is a warning), and the strict lint runs once at the end.

**Files:**
- Modify: `refbox/src/sim_frame.rs` (`effective_beep_layout` + test)
- Modify: `refbox/src/app/message.rs` (new `BeepTestCycleDisplayLayout` variant)
- Modify: `refbox/src/app/mod.rs` (field + init + handler + call site)
- Modify: `refbox/src/app/view_builders/beep_test_settings.rs` (imports, new params, button, preview, embed helper, grid)

**Interfaces:**
- Consumes: the five `beep-*.png` from Task 2; `FrontDisplayLayout::next()`; `UpdateSender::set_layout`.
- Produces:
  - `crate::sim_frame::effective_beep_layout(has_led_panel: bool, session_layout: FrontDisplayLayout) -> FrontDisplayLayout`
  - `Message::BeepTestCycleDisplayLayout`
  - `RefBoxApp.beep_test_display_layout: FrontDisplayLayout`
  - new signature `build_beep_test_settings_landing(config: &Config, staged_mode: Mode, has_run: bool, beep_test_layout: FrontDisplayLayout, has_led_panel: bool)`

- [ ] **Step 1: Write the failing test for `effective_beep_layout`**

In `refbox/src/sim_frame.rs`, inside `#[cfg(test)] mod tests { ... }`, add:

```rust
    #[test]
    fn effective_beep_layout_forces_default_with_panel() {
        // With a panel, any session choice collapses to Default.
        assert_eq!(
            effective_beep_layout(true, FrontDisplayLayout::Corners),
            FrontDisplayLayout::Default,
        );
        // Without a panel, the session choice is respected.
        assert_eq!(
            effective_beep_layout(false, FrontDisplayLayout::Corners),
            FrontDisplayLayout::Corners,
        );
        assert_eq!(
            effective_beep_layout(false, FrontDisplayLayout::Default),
            FrontDisplayLayout::Default,
        );
    }
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `cargo test -p refbox effective_beep_layout`
Expected: FAIL to compile — `cannot find function effective_beep_layout`.

- [ ] **Step 3: Implement `effective_beep_layout`**

In `refbox/src/sim_frame.rs`, after the `impl FrontDisplayLayout { ... }` block (before the `SimFrame` struct), add:

```rust
/// The layout actually shown during beep test: forced to `Default` whenever a
/// real LED panel is connected (the panel only renders Default), else the
/// operator's in-memory session choice. Mirrors the game-mode Display page's
/// `effective_layout` rule so the picker label, the preview, and the layout
/// pushed to the display always agree.
pub fn effective_beep_layout(
    has_led_panel: bool,
    session_layout: FrontDisplayLayout,
) -> FrontDisplayLayout {
    if has_led_panel {
        FrontDisplayLayout::Default
    } else {
        session_layout
    }
}
```

- [ ] **Step 4: Run the test to confirm it passes**

Run: `cargo test -p refbox effective_beep_layout`
Expected: PASS.

- [ ] **Step 5: Add the message variant**

In `refbox/src/app/message.rs`, add a variant alongside the other `BeepTest*` variants in the `Message` enum:

```rust
    /// Cycle the in-memory BeepTest display layout (session-only, live-apply).
    BeepTestCycleDisplayLayout,
```

- [ ] **Step 6: Add the session-layout field + initializer**

In `refbox/src/app/mod.rs`, in the `RefBoxApp` struct, after the `beep_test_snapshot` field, add:

```rust
    /// In-memory display layout for BeepTest mode, shown on the player-facing
    /// display. Starts at `Default` every boot and is never persisted (the
    /// game-mode `config.front_display_layout` is untouched). Changed only by
    /// the BeepTest Settings "DISPLAY LAYOUT" picker via
    /// `Message::BeepTestCycleDisplayLayout`.
    beep_test_display_layout: crate::sim_frame::FrontDisplayLayout,
```

In the `Self { ... }` struct literal in `App::new`, after the `beep_test_snapshot: BeepTestSnapshot::default(),` line, add:

```rust
            beep_test_display_layout: crate::sim_frame::FrontDisplayLayout::Default,
```

- [ ] **Step 7: Add the handler arm**

In `refbox/src/app/mod.rs`, in `update()`, add a new arm immediately after the `Message::BeepTestTick => { ... }` arm:

```rust
            Message::BeepTestCycleDisplayLayout => {
                // Session-only: advance the in-memory beep-test layout and push
                // it to the display. Never written to config (resets to Default
                // on the next boot).
                self.beep_test_display_layout = self.beep_test_display_layout.next();
                let effective = crate::sim_frame::effective_beep_layout(
                    self.has_led_panel,
                    self.beep_test_display_layout,
                );
                if let Err(e) = self.update_sender.set_layout(effective) {
                    warn!("Failed to push beep-test display layout: {e:?}");
                }
                Task::none()
            }
```

- [ ] **Step 8: Update the imports in `beep_test_settings.rs`**

In `refbox/src/app/view_builders/beep_test_settings.rs`, add the layout imports under the existing `use crate::config::{Config, Level};`:

```rust
use crate::sim_frame::{FrontDisplayLayout, effective_beep_layout};
```

and add `Image` and `image` to the `iced::widget` import list so it reads:

```rust
    widget::{Column, Row, Space, button, column, container, horizontal_space, image, row, text, Image},
```

- [ ] **Step 9: Add the preview-embedding helper**

In `refbox/src/app/view_builders/beep_test_settings.rs`, add (near the bottom, beside `make_beep_test_cancel_apply_footer`):

```rust
/// The embedded beep-test preview picture for a layout (white-on-left only —
/// beep test has no sides control). Exhaustive match, mirroring
/// `layout_preview_handle` in `configuration.rs`; adding a `FrontDisplayLayout`
/// variant won't compile until its `beep-*.png` is added here and generated via
/// `just capture-previews`.
fn beep_test_layout_preview_handle(layout: FrontDisplayLayout) -> image::Handle {
    macro_rules! preview {
        ($stem:literal) => {
            &include_bytes!(concat!(
                "../../../resources/layout-previews/",
                $stem,
                ".png"
            ))[..]
        };
    }
    let bytes: &'static [u8] = match layout {
        FrontDisplayLayout::Default => preview!("beep-default"),
        FrontDisplayLayout::Classic => preview!("beep-classic"),
        FrontDisplayLayout::BigTime => preview!("beep-big-time"),
        FrontDisplayLayout::Corners => preview!("beep-corners"),
        FrontDisplayLayout::ScoresOnly => preview!("beep-scores-only"),
    };
    image::Handle::from_bytes(bytes)
}
```

- [ ] **Step 10: Rewrite `build_beep_test_settings_landing`**

Replace the entire `build_beep_test_settings_landing` function (and its doc comment) with:

```rust
/// Landing page for the BeepTest Settings hierarchy.
///
/// Grid (top to bottom):
/// - Row 1: [APP MODE = <staged>] [EDIT LEVELS]
/// - Row 2: [SOUND SETTINGS]      [DISPLAY LAYOUT]
/// - Rows 3-4: left column [LANGUAGE] over a blank cell; right column a
///   beep-test PREVIEW spanning both rows.
/// - Bottom row: [BACK]   [horizontal_space]   [RESTART TO APPLY (when staged
///   mode != live mode and no test has run)] — unchanged.
///
/// DISPLAY LAYOUT cycles the in-memory beep-test layout live (no Apply). It is
/// grayed and forced to Default when a real LED panel is connected (the panel
/// only renders Default) or once a beep test has run. APP MODE, EDIT LEVELS,
/// and LANGUAGE are gated on `!has_run`; SOUND SETTINGS stays live.
pub(in super::super) fn build_beep_test_settings_landing<'a>(
    config: &Config,
    staged_mode: Mode,
    has_run: bool,
    beep_test_layout: FrontDisplayLayout,
    has_led_panel: bool,
) -> Element<'a, Message> {
    let sound_button = make_button(fl!("sound-settings"))
        .style(light_gray_button)
        .on_press(Message::BeepTestEditOpenSound);

    let edit_levels_button = if has_run {
        make_button(fl!("beep-test-edit-levels")).style(gray_button)
    } else {
        make_button(fl!("beep-test-edit-levels"))
            .style(light_gray_button)
            .on_press(Message::BeepTestEditOpenLevels)
    };

    let app_mode_button = make_value_button(
        fl!("app-mode"),
        staged_mode.to_string(),
        (false, true),
        if has_run {
            None
        } else {
            Some(Message::CycleParameter(CyclingParameter::Mode))
        },
    );

    let language_button = if has_run {
        make_button(fl!("language")).style(gray_button)
    } else {
        make_button(fl!("language"))
            .style(light_gray_button)
            .on_press(Message::BeepTestEditOpenLanguage)
    };

    // DISPLAY LAYOUT — cycles the in-memory beep-test layout (live-apply, not
    // persisted). Grayed + forced to Default with a panel connected or after a run.
    let effective_layout = effective_beep_layout(has_led_panel, beep_test_layout);
    let layout_label = match effective_layout {
        FrontDisplayLayout::Default => fl!("layout-default"),
        FrontDisplayLayout::Classic => fl!("layout-classic"),
        FrontDisplayLayout::BigTime => fl!("layout-big-time"),
        FrontDisplayLayout::Corners => fl!("layout-corners"),
        FrontDisplayLayout::ScoresOnly => fl!("layout-scores-only"),
    };
    let display_layout_button = make_value_button(
        fl!("front-display-layout"),
        layout_label,
        (false, true),
        if has_led_panel || has_run {
            None
        } else {
            Some(Message::BeepTestCycleDisplayLayout)
        },
    );

    // Static preview of the effective layout's beep-test appearance (white-on-left).
    let preview = container(
        Image::new(beep_test_layout_preview_handle(effective_layout))
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill);

    let row1 = row![app_mode_button, edit_levels_button]
        .spacing(SPACING)
        .height(Length::Fill);
    let row2 = row![sound_button, display_layout_button]
        .spacing(SPACING)
        .height(Length::Fill);

    // Rows 3-4: LANGUAGE over a blank cell on the left; preview on the right
    // spanning both rows. FillPortion(2) gives this band the height of two tile
    // rows, so the preview reads as a 2-row-tall cell.
    let lower_left = column![
        row![language_button].spacing(SPACING).height(Length::Fill),
        row![horizontal_space()].height(Length::Fill),
    ]
    .spacing(SPACING)
    .width(Length::Fill);

    let rows_34 = row![lower_left, preview]
        .spacing(SPACING)
        .height(Length::FillPortion(2));

    let back_button = make_button(fl!("back"))
        .style(red_button)
        .on_press(Message::BeepTestCloseSettings);

    // Bottom row unchanged: BACK on the left, and a blue RESTART TO APPLY at the
    // right when the staged App Mode differs from the live mode and no test has
    // run yet; otherwise a filler keeps BACK from shifting.
    let bottom_row: Element<'a, Message> = if staged_mode != config.mode && !has_run {
        let restart_button = make_button(fl!("restart-to-apply"))
            .style(blue_button)
            .on_press(Message::BeepTestRestartToApply);
        row![back_button, horizontal_space(), restart_button]
            .spacing(SPACING)
            .into()
    } else {
        row![back_button, horizontal_space(), horizontal_space()]
            .spacing(SPACING)
            .into()
    };

    column![row1, row2, rows_34, bottom_row]
        .spacing(SPACING)
        .height(Length::Fill)
        .into()
}
```

> If clippy later flags `Space` as unused in this file, remove it from the import list (the rewritten landing uses `horizontal_space()` for the blank cell). Leave it if other functions in the file still use it.

- [ ] **Step 11: Update the landing call site**

In `refbox/src/app/mod.rs` (~line 4581), the `build_beep_test_settings_landing(...)` call becomes:

```rust
                    build_beep_test_settings_landing(
                        &self.config,
                        staged_mode,
                        self.beep_test_has_run,
                        self.beep_test_display_layout,
                        self.has_led_panel,
                    )
```

- [ ] **Step 12: Build, lint, test**

Run: `cargo build -p refbox` → Expected: builds.
Run: `cargo clippy -p refbox -- -D warnings` → Expected: clean.
Run: `cargo test -p refbox` → Expected: all pass (incl. the new `effective_beep_layout_forces_default_with_panel`).

- [ ] **Step 13: Full gate**

Run: `just check` → Expected: fmt-check, lint, test, audit all pass.

- [ ] **Step 14: Simulator walkthrough (no panel)**

Rebuild first (clippy/test build a different binary): `cargo build -p refbox`, then launch the sim (`WAYLAND_DISPLAY= cargo run -p refbox` with sandbox disabled, beep-test mode in config).
- Boot beep test → player display shows **Default**: lap count on the **left**, black side **blank**.
- Open Settings → grid matches: `[App Mode][Edit Levels]` / `[Sound Settings][Display Layout]` / `[Language][Preview]` (preview spans the two lower rows; bottom-left blank); BACK / RESTART-TO-APPLY footer unchanged.
- Tap DISPLAY LAYOUT through all five layouts → the preview image updates each time; the player display follows live.
- Exit beep test settings; confirm the game-mode Display layout setting is unchanged (open a game-mode config if convenient, or inspect the config file — `front_display_layout` untouched).
- Set DISPLAY LAYOUT to a non-Default value, restart the app in beep-test mode → it boots back on **Default**.

- [ ] **Step 15: Commit (with user approval)**

```bash
git add refbox/src/sim_frame.rs refbox/src/app/message.rs refbox/src/app/mod.rs refbox/src/app/view_builders/beep_test_settings.rs
git commit -m "feat(refbox): add beep-test display layout picker and preview"
```

---

## Self-Review

**Spec coverage:**
- Default-on-boot → Task 1 Step 1 + Task 3 field init. ✓
- White-on-left, no sides toggle → Task 1 Step 2 (tick pin) + Task 2 (white-on-left previews only) + Task 3 (no sides control in the grid). ✓
- DISPLAY LAYOUT picker, session-only, live-apply, resets to Default → Task 3 Steps 5/7/10 + walkthrough Step 14. ✓
- Beep-test PREVIEW (accurate) → Task 2 (5 PNGs) + Task 3 Step 9/10. ✓
- `has_led_panel` ⇒ grayed/forced Default → `effective_beep_layout` (Task 3 Step 3) used by label, preview, and handler; button `on_press` gated. ✓
- Grid rearrangement, footer unchanged → Task 3 Step 10. ✓
- No new translations → reuses existing keys (Task 3 Step 10). ✓
- Game-mode layout untouched → dedicated field + dedicated message, never writes `config.front_display_layout` (Task 3). ✓

**Placeholder scan:** none — every code step shows complete code; every command shows expected output.

**Type consistency:** `effective_beep_layout(bool, FrontDisplayLayout) -> FrontDisplayLayout` is defined in Task 3 Step 3 and called identically in Step 7 (handler) and Step 10 (view). `beep_test_layout_preview_handle(FrontDisplayLayout) -> image::Handle` defined Step 9, called Step 10. `Message::BeepTestCycleDisplayLayout` added Step 5, matched Step 7, constructed Step 10. `build_beep_test_settings_landing(..., FrontDisplayLayout, bool)` new signature Step 10 matches the call site Step 11. `Variant` (Task 2) defined and used only within `capture.rs`. Consistent.
