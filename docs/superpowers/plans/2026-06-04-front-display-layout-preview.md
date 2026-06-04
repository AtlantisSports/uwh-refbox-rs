# Front-Display Layout Preview Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show a static picture of the staged front-display layout on the Display Options page, generated ahead of time by a maintenance command that screenshots the (stable) display-simulator window — no live canvas on the settings page, so no crash.

**Architecture:** A new `refbox` capture mode drives the existing standalone simulator `iced::application` to render each layout with fixed sample data, captures it with `iced::window::screenshot`, and writes 10 PNGs (5 layouts × 2 sides) into `refbox/resources/`. The PNGs are committed and embedded with `include_bytes!`. The Display Options page is rearranged and shows the committed picture for the staged `(layout, white_on_right)` via a plain `Image` widget. A pre-merge drift check regenerates the PNGs and fails if they differ from what's committed.

**Tech Stack:** Rust 2024 / MSRV 1.85, `iced` 0.13 (features `canvas,image,svg,tiny-skia,tokio` on Linux), `tiny_skia` (PNG encode), `clap` (CLI flags), `just` (task runner), the merged front-display-layouts code (`FrontDisplayLayout`, `sim_app`, `SimFrame`, `TransmittedData`).

---

## Spec

Design spec: `docs/superpowers/specs/2026-06-04-front-display-layout-preview-design.md`. Decisions D1–D8 there are authoritative; this plan implements them.

## Critical context (verified 2026-06-04)

- The layout code is on **`origin/master`** (PR #949, merge `c96f3440`). **Local `master` is stale** — fetch first. The current branch `feat/refbox/display-modes` is a *different* feature and does **not** contain the layout code.
- **Base this work on a fresh branch off `origin/master`.**
- The simulator is launched as its **own** `iced::application(...)` in `refbox/src/main.rs` when the hidden `--is-simulator` flag is set (see `main.rs:317-346`). It has a real `update → Task<Message>` flow, which is what lets us issue a `window::screenshot` Task. This is **not** a canvas embedded in the main window.
- `iced::window::screenshot(id: Id) -> Task<Screenshot>` exists (`iced_runtime-0.13.2/src/window.rs:416`); `Screenshot { bytes: Bytes /* RGBA, sRGB */, size: Size<u32>, scale_factor: f64 }`.
- `image` is **not** a direct dependency; it comes transitively through iced's `image` feature (gives the `Image` widget + `image::Handle::from_bytes`). For *writing* PNGs use `tiny_skia` (already present on Linux) to avoid a new dependency.

## Process

Lean process (single crate, `refbox`). One code review at the end. **Approval gates** (ask the human first, per `.claude/rules/communication.md`): creating the branch, every commit, opening the PR, and **adding the CI workflow step** (shared infrastructure — Task 8). Nothing pushed without go-ahead.

## File structure

| File | Responsibility | Action |
|------|----------------|--------|
| `refbox/src/preview_capture.rs` | New module: sample data, layout/side iteration, the capture `iced::application` that screenshots each variant and writes PNGs | Create |
| `refbox/src/main.rs` | Add `--capture-previews <DIR>` flag; dispatch to the capture app (mirrors the `--is-simulator` branch) | Modify (`:140-346`) |
| `refbox/resources/layout-previews/*.png` | The 10 committed preview pictures | Create (generated) |
| `refbox/src/app/view_builders/configuration.rs` | Rearrange `make_display_config_page` per D6; add the preview `Image`; map `(layout, white_on_right)` → embedded PNG | Modify (`:911-1038`) |
| `justfile` | `capture-previews` recipe + `check-previews` drift recipe | Modify |
| `.github/workflows/*.yml` | Drift check step (Xvfb + `check-previews`) | Modify (Task 8, infra approval) |

Naming convention for the PNGs: `layout-previews/<layout>-<side>.png` where `<layout>` ∈ `default|classic|big-time|corners|scores-only` and `<side>` ∈ `white-left|white-right`. Example: `classic-white-right.png`.

---

## Task 0: Branch setup + commit the spec (APPROVAL GATE)

**Files:** none (git only)

- [ ] **Step 1: Fetch and confirm base.** Run: `git fetch origin && git log -1 --oneline origin/master`. Expected: shows `c96f3440` in history (`git merge-base --is-ancestor c96f3440 origin/master && echo OK`).
- [ ] **Step 2: Ask the human for approval to create the branch** `feat/refbox/display-layout-preview` off `origin/master`. Do not proceed without a yes.
- [ ] **Step 3: Create the branch (worktree recommended to avoid disturbing the current `feat/refbox/display-modes` checkout).**

```bash
git worktree add .worktrees/display-layout-preview -b feat/refbox/display-layout-preview origin/master
```

- [ ] **Step 4: Copy the spec + this plan into the new branch** (they currently live as untracked files in the `feat/refbox/display-modes` worktree).

```bash
cp docs/superpowers/specs/2026-06-04-front-display-layout-preview-design.md \
   .worktrees/display-layout-preview/docs/superpowers/specs/
cp docs/superpowers/plans/2026-06-04-front-display-layout-preview.md \
   .worktrees/display-layout-preview/docs/superpowers/plans/
```

- [ ] **Step 5: Commit the spec + plan** (ask the human first, per approval gate).

```bash
cd .worktrees/display-layout-preview
git add docs/superpowers/specs/2026-06-04-front-display-layout-preview-design.md docs/superpowers/plans/2026-06-04-front-display-layout-preview.md
git commit -m "docs(refbox): design + plan for front-display layout preview"
```

All remaining tasks run inside `.worktrees/display-layout-preview/`.

---

## Task 1: SPIKE — capture ONE layout to a PNG

**Goal of the spike:** prove the end-to-end capture path (drive sim → render a chosen layout → `window::screenshot` → encode PNG → file on disk) before generalizing. This de-risks the two spec risks (headless screenshot, encoding). **It is expected that the exact Task-chaining API is confirmed here and the skeleton adjusted to match what compiles.**

**Files:**
- Create: `refbox/src/preview_capture.rs`
- Modify: `refbox/src/main.rs`

- [ ] **Step 1: Add the module + CLI flag.** In `main.rs`, add to the args struct (near `is_simulator`, `:161`):

```rust
/// Capture front-display layout preview PNGs into the given directory, then exit.
#[arg(long, value_name = "DIR")]
capture_previews: Option<std::path::PathBuf>,
```

Add `mod preview_capture;` with the other module declarations.

- [ ] **Step 2: Construct the sample data** in `preview_capture.rs` (values from the spec; types verified against the codebase):

```rust
use matrix_drawing::transmitted_data::{Brightness, TransmittedData};
use uwh_common::game_snapshot::{GamePeriod, GameSnapshotNoHeap};
use uwh_common::bundles::BlackWhiteBundle; // confirm exact path during spike
use arrayvec::ArrayVec;

/// WHITE 5 – BLACK 3, First Half, 8:42 (522 s). `white_on_right` set by the caller.
pub(crate) fn sample_data(white_on_right: bool) -> TransmittedData {
    TransmittedData {
        white_on_right,
        flash: false,
        beep_test: false,
        brightness: Brightness::Low,
        snapshot: GameSnapshotNoHeap {
            current_period: GamePeriod::FirstHalf,
            secs_in_period: 522,
            timeout: None,
            scores: BlackWhiteBundle { black: 3, white: 5 },
            penalties: Default::default(),
            is_old_game: false,
        },
    }
}
```

- [ ] **Step 3: Write a minimal capture `iced::application`.** Model it on the `--is-simulator` branch (`main.rs:317-346`), reusing `sim_app::SimRefBoxApp`. On startup it should: push one `SimFrame { layout: FrontDisplayLayout::Classic, data: sample_data(false) }` into the app, then (after the frame renders) issue `iced::window::screenshot(id)`. Get the window id via `iced::window::get_latest()` (returns `Task<Option<Id>>`). Deliver the `Screenshot` back through a new capture `Message` variant, then write the file and `exit()`.

Skeleton (confirm exact API names compile; adjust the chaining as needed — this is the spike's job):

```rust
// In the capture app's update():
//   on Init        -> Task::batch([ push NewSnapshot(frame), window::get_latest().map(Msg::GotId) ])
//   on GotId(Some) -> window::screenshot(id).map(Msg::Shot)
//   on Shot(s)     -> write_png(&out_dir.join("classic-white-left.png"), &s); iced::exit()
```

- [ ] **Step 4: Encode the PNG with tiny_skia.** RGBA bytes → PNG:

```rust
fn write_png(path: &std::path::Path, shot: &iced::window::Screenshot) {
    let pixmap = tiny_skia::Pixmap::from_vec(
        shot.bytes.to_vec(),
        tiny_skia::IntSize::from_wh(shot.size.width, shot.size.height).expect("nonzero size"),
    ).expect("RGBA buffer matches size");
    pixmap.save_png(path).expect("write png");
}
```

If `tiny_skia` is not exposed as a usable dependency of `refbox`, STOP and ask the human about adding it (or the `image` crate) as a dependency — do not add a dependency silently (`.claude/rules/rust.md`).

- [ ] **Step 5: Build and run the spike.**

Run: `cd .worktrees/display-layout-preview && WAYLAND_DISPLAY= cargo run -p refbox -- --capture-previews /tmp/previews`
(The `WAYLAND_DISPLAY=` prefix forces X11 on WSLg, per project setup.)
Expected: process exits cleanly; `/tmp/previews/classic-white-left.png` exists.

- [ ] **Step 6: Eyeball the PNG.** Open `/tmp/previews/classic-white-left.png`. Expected: the Classic layout with WHITE 5, BLACK 3, a period pill, and clock `8:42`. Confirm it is not blank/black-only.

- [ ] **Step 7: Record spike findings** at the bottom of this plan under "Deviations" — the exact API used (`get_latest`/`screenshot`/message wiring), the window size/scale chosen, and whether timing (waiting for a rendered frame before screenshotting) needed handling. **Commit** (ask first): `git commit -am "spike(refbox): capture one layout preview PNG via window::screenshot"`.

---

## Task 2: Capture all 10 variants + sample-data test

**Files:**
- Modify: `refbox/src/preview_capture.rs`
- Test: `refbox/src/preview_capture.rs` (`#[cfg(test)]`)

- [ ] **Step 1: Write the failing test for sample data** (no rendering needed):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sample_has_expected_score_and_clock() {
        let d = sample_data(false);
        assert_eq!(d.snapshot.scores.white, 5);
        assert_eq!(d.snapshot.scores.black, 3);
        assert_eq!(d.snapshot.secs_in_period, 522);
        assert_eq!(d.snapshot.current_period, uwh_common::game_snapshot::GamePeriod::FirstHalf);
        assert!(!d.white_on_right);
        assert!(sample_data(true).white_on_right);
    }
}
```

- [ ] **Step 2: Run it.** Run: `cargo test -p preview_capture sample_has_expected -- --exact` (or `cargo test -p refbox preview_capture`). Expected: PASS (it's plain data).
- [ ] **Step 3: Define the variant list + filenames.**

```rust
use refbox_sim_frame::FrontDisplayLayout; // use the real path: crate::sim_frame::FrontDisplayLayout

pub(crate) const LAYOUTS: [FrontDisplayLayout; 5] = [
    FrontDisplayLayout::Default, FrontDisplayLayout::Classic, FrontDisplayLayout::BigTime,
    FrontDisplayLayout::Corners, FrontDisplayLayout::ScoresOnly,
];

pub(crate) fn file_stem(layout: FrontDisplayLayout, white_on_right: bool) -> String {
    let l = match layout {
        FrontDisplayLayout::Default => "default",
        FrontDisplayLayout::Classic => "classic",
        FrontDisplayLayout::BigTime => "big-time",
        FrontDisplayLayout::Corners => "corners",
        FrontDisplayLayout::ScoresOnly => "scores-only",
    };
    let s = if white_on_right { "white-right" } else { "white-left" };
    format!("{l}-{s}")
}
```

- [ ] **Step 4: Generalize the capture loop** to iterate all 10 `(layout, side)` pairs: push the frame, screenshot, write `<stem>.png`, advance to the next pair; `exit()` after the last. Use the message/Task chaining confirmed in the spike (one screenshot fully completes before the next frame is pushed — do not overlap).
- [ ] **Step 5: Run the full capture.** Run: `WAYLAND_DISPLAY= cargo run -p refbox -- --capture-previews refbox/resources/layout-previews`. Expected: 10 PNGs written.
- [ ] **Step 6: Verify count + non-emptiness.** Run: `ls refbox/resources/layout-previews/*.png | wc -l` → `10`; and none are zero-byte (`find refbox/resources/layout-previews -name '*.png' -size 0`→ empty).
- [ ] **Step 7: Eyeball 2–3 PNGs** (one full-screen layout + `default-white-left.png` + a `white-right` variant) to confirm correctness and that `white-right` actually mirrors. Fix the capture if not.
- [ ] **Step 8: Commit** (ask first): `git add refbox/src/preview_capture.rs refbox/resources/layout-previews && git commit -m "feat(refbox): capture all front-display layout preview PNGs"`.

---

## Task 3: Default-layout letterboxing in the 16:9 box

**Goal:** the `Default` (256×64, ~4:1) capture must sit top-justified in the 16:9 preview box with **black** fill below (spec D5), so it matches the other previews' shape.

**Files:** Modify: `refbox/src/preview_capture.rs`

- [ ] **Step 1: Inspect** `default-white-left.png` from Task 2. If the simulator already renders Default acceptably inside the chosen 16:9 capture window, no code is needed — note that and skip to Step 4.
- [ ] **Step 2: If Default is stretched/centered wrong,** set the capture window to 16:9 and compose the panel strip top-justified on a black background before screenshotting (or post-process the captured RGBA: place the strip rows at the top, fill the remainder with opaque black). Keep this logic in `preview_capture.rs` only — do **not** modify `sim_app`/`scoreboard` drawing code (spec scope guard).
- [ ] **Step 3: Re-run capture** (Task 2 Step 5) and re-inspect `default-white-left.png`: wide strip at top, black below.
- [ ] **Step 4: Commit** (ask first): `git commit -am "feat(refbox): letterbox Default layout preview to 16:9 with black fill"`.

---

## Task 4: Rearrange the Display Options page + show the preview

**Files:** Modify: `refbox/src/app/view_builders/configuration.rs` (`make_display_config_page`, `:911-1038`)

- [ ] **Step 1: Add the layout→PNG mapping** (embeds all 10 at compile time). Place near the top of the function or as a private fn:

```rust
fn layout_preview_handle(layout: FrontDisplayLayout, white_on_right: bool) -> image::Handle {
    macro_rules! png { ($n:literal) => { &include_bytes!(concat!("../../../resources/layout-previews/", $n, ".png"))[..] }; }
    let bytes: &[u8] = match (layout, white_on_right) {
        (FrontDisplayLayout::Default, false)    => png!("default-white-left"),
        (FrontDisplayLayout::Default, true)     => png!("default-white-right"),
        (FrontDisplayLayout::Classic, false)    => png!("classic-white-left"),
        (FrontDisplayLayout::Classic, true)     => png!("classic-white-right"),
        (FrontDisplayLayout::BigTime, false)    => png!("big-time-white-left"),
        (FrontDisplayLayout::BigTime, true)     => png!("big-time-white-right"),
        (FrontDisplayLayout::Corners, false)    => png!("corners-white-left"),
        (FrontDisplayLayout::Corners, true)     => png!("corners-white-right"),
        (FrontDisplayLayout::ScoresOnly, false) => png!("scores-only-white-left"),
        (FrontDisplayLayout::ScoresOnly, true)  => png!("scores-only-white-right"),
    };
    image::Handle::from_bytes(bytes)
}
```

(The exhaustive `match` is the compile-time guarantee that every layout has a picture — a new `FrontDisplayLayout` variant won't compile until its PNG mapping is added.)

- [ ] **Step 2: Build the preview element** (uses the existing `Image` import pattern from `shared_elements.rs`):

```rust
let preview = container(
    Image::new(layout_preview_handle(*front_display_layout, *white_on_right))
        .width(Length::Fill)
        .height(Length::Fill)
)
.width(Length::Fill)
.height(Length::Fill);
```

- [ ] **Step 3: Rearrange the column to the D6 layout.** Replace the existing row arrangement so the final `column!` is:

```rust
column![
    make_game_time_button(/* unchanged */),
    row![sides_btn].spacing(SPACING).height(Length::Fill),                 // Row 1
    row![hide_time_btn, layout_btn].spacing(SPACING).height(Length::Fill), // Row 2
    row![                                                                  // Rows 3–4
        column![open_display_btn, brightness_btn].spacing(SPACING).width(Length::Fill),
        preview,
    ].spacing(SPACING).height(Length::Fill),
    make_cancel_apply_footer(ConfigPage::Display, settings, page_entry_snapshot), // Row 5
]
.spacing(SPACING)
.height(Length::Fill)
.into()
```

Keep each button's existing `Message` and `has_led_panel` gating exactly as before (Row-2 `layout_btn` keeps its `CycleParameter(FrontDisplayLayout)` press and the `has_led_panel` disable; `open_display_btn` keeps `OpenNewDisplay`; `brightness_btn` keeps `CycleParameter(Brightness)`). Only the **arrangement** changes.

- [ ] **Step 4: Build.** Run: `cargo build -p refbox`. Expected: compiles (the exhaustive match forces all 10 PNGs to exist).
- [ ] **Step 5: Lint.** Run: `cargo clippy -p refbox -- -D warnings`. Expected: clean.
- [ ] **Step 6: Manual verification (plain-English, human-runnable).** Run: `WAYLAND_DISPLAY= cargo run -p refbox`. Open **Settings → Display Options** and confirm:
  - The page is laid out per D6 (Starting Sides on top; Hide-Time + Display Layout; Open-New-Display over Player-Display-Brightness on the left with the picture filling the right; Cancel/Apply at the bottom).
  - The picture matches the staged layout; **cycling Display Layout** changes the picture immediately; **toggling Starting Sides** flips it.
  - The page **does not crash** (the Decision-D regression guard).
- [ ] **Step 7: Commit** (ask first): `git commit -am "feat(refbox): show layout preview image and rearrange Display Options page"`.

---

## Task 5: `just` recipes for capture + drift check

**Files:** Modify: `justfile`

- [ ] **Step 1: Add the capture recipe.**

```just
# Regenerate the front-display layout preview PNGs (run after changing any layout's look)
capture-previews:
    WAYLAND_DISPLAY= cargo run -p refbox -- --capture-previews refbox/resources/layout-previews
```

- [ ] **Step 2: Add the drift-check recipe** (regenerate into a temp dir, compare to committed):

```just
# Fail if the committed layout preview PNGs differ from a fresh capture
check-previews:
    #!/usr/bin/env bash
    set -euo pipefail
    tmp=$(mktemp -d)
    WAYLAND_DISPLAY= cargo run -p refbox -- --capture-previews "$tmp"
    if ! diff -rq "$tmp" refbox/resources/layout-previews; then
        echo "Preview PNGs are stale. Run 'just capture-previews' and commit the result." >&2
        exit 1
    fi
```

- [ ] **Step 3: Verify both recipes locally.** Run: `just capture-previews && just check-previews`. Expected: capture writes 10 files; check-previews passes (no diff against what was just committed).
- [ ] **Step 4: Decide the comparison strategy** if `diff` (exact bytes) proves flaky across machines (spec Risk 2): switch `check-previews` to a tolerance-based pixel compare. Record the decision in Deviations. (Defer the tolerant compare unless exact-diff actually fails.)
- [ ] **Step 5: Commit** (ask first): `git commit -am "chore(refbox): add capture-previews and check-previews just recipes"`.

---

## Task 6: Full local validation

**Files:** none

- [ ] **Step 1:** Run `just check` (fmt, lint, tests, audit). Expected: all clean. Note: `refbox` is bin-only — if `just test` uses `--all-targets`, expect the known pre-existing test-code lints (not failures from this work).
- [ ] **Step 2:** Re-run the Task 4 Step 6 manual verification once more on a clean build to confirm no regression.
- [ ] **Step 3:** Run `superpowers:requesting-code-review` on the full branch diff (lean process — one review at the end).

---

## Task 7: CI drift check (APPROVAL GATE — shared infrastructure)

**Files:** Modify: `.github/workflows/<ci>.yml`

- [ ] **Step 1: Ask the human** before editing any CI workflow (shared infrastructure gate).
- [ ] **Step 2: Identify the CI job** that runs on Linux and already builds `refbox`. Add a step that runs the drift check under a virtual display (the screenshot needs a windowing surface):

```yaml
      - name: Check layout previews are up to date
        run: xvfb-run -a just check-previews
```

Ensure `xvfb` is installed on the runner (add to the existing apt step, or `sudo apt-get install -y xvfb`).

- [ ] **Step 3: Push to a branch and confirm the check runs green** when previews are current, and **red** when they are stale (test by tweaking one PNG and confirming CI fails, then revert). Record results in Deviations.
- [ ] **Step 4: Commit** (ask first): `git commit -am "ci(refbox): fail build if layout preview PNGs are stale"`.

---

## Task 8: PR (APPROVAL GATE)

- [ ] **Step 1:** Confirm `just check` is green and previews verified.
- [ ] **Step 2:** Ask the human before opening the PR. Use the PR body format from `.claude/rules/pr-review.md` (What changed / Why / Scope / How to verify), written for a non-programmer reviewer. Base: `master`.

---

## Self-review notes (author)

- Spec coverage: D1 (Image not canvas → Task 4); D2 (baked-in, dev-time → Tasks 1–2, 7); D3 (10 variants → Task 2); D4 (sample data → Task 1–2 + test); D5 (Default letterbox → Task 3); D6 (page rearrange → Task 4); D7 (instant cycle/flip → Task 4 Step 6); D8 (drift check → Tasks 5, 7). Acceptance criteria 1–7 all map to a verification step.
- Known soft spots, handled by design: exact `window::screenshot` Task-chaining is confirmed in the **Task 1 spike** before generalizing (Task 2); CI screenshot needs **Xvfb** (Task 7); exact-byte vs tolerant diff decided empirically (Task 5 Step 4). These are flagged, not hidden.

## Deviations

- **Task 1 spike merged with Task 2.** The capture mechanism was proven and generalized to all 10
  variants in one pass. Confirmed API: `window::get_latest()` for the window id, a `window::frames()`
  subscription driving continuous redraws, a 3-frame settle counter before each `window::screenshot()`,
  and an `awaiting_shot` guard against overlapping captures. All 10 PNGs were eyeballed correct
  (right layout, sample data, side flip) with no stale/off-by-one frames.
- **Task 3 was a no-op.** The simulator already renders `Default` top-justified with black fill in a
  16:9 window, satisfying D5 with no code.
- **PNG encoding** uses the `image` crate (user-approved, already transitive via iced), not tiny_skia.
- **Code-review fixes (Task 6):** pinned the capture window's UI `scale_factor` to 1.0 and documented
  the scale-1.0 assumption for the drift guard. The exact-vs-tolerant diff decision (Task 5 Step 4)
  is deferred until/unless CI shows cross-machine flakiness (spec Risk 2).
- **Task 7 (CI drift check) deferred** pending explicit user approval — it edits `.github/workflows`
  (shared infrastructure). Until then, `just check-previews` is a manual/local guard only, so
  acceptance criterion 7 is not yet met.
