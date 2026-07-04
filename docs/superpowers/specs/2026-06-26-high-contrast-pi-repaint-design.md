# Design — Fix High Contrast display-mode flicker on the Pi (no-restart repaint)

**Date:** 2026-06-26
**Branch:** `fix/refbox/high-contrast-pi-repaint`
**Crate:** `refbox` only
**Status:** Approved design — pending implementation plan
**Process weight:** Lean (refbox UI/rendering; no `uwh-common`, no wire format, no state machine).
This doc is a local working spec — per project convention it is **not** committed to the
branch/PR (see memory `reference_plan_docs_not_committed`).

---

## Problem (plain English)

On the Raspberry Pi, switching the display mode to **High Contrast** leaves the screen a
flickering patchwork of old and new colors that **persists** until the whole program is
restarted. Switching to another view mode does not clear it. On the operator's desktop and on
Windows/Mac the switch is clean.

## Confirmed root cause

Traced end-to-end in the rendering libraries (iced `0.13.1` / `iced_tiny_skia 0.13.0` /
`softbuffer 0.4.6`), and corroborated by a falsifiable test (see Evidence):

1. The Pi renders in software (tiny-skia). On Linux the app uses tiny-skia; on Windows/Mac it
   uses wgpu (`refbox/Cargo.toml` target deps). So the bug is Linux/tiny-skia-only.
2. softbuffer's **Wayland** backend is **double-buffered** (a front/back pair); the buffer the
   app draws into next is normally two frames old
   (`softbuffer .../backends/wayland/mod.rs`).
3. iced's tiny-skia `present()` forces a full clean repaint when the window background color
   changes — but it compares against a **single** "last presented background" value and updates
   that value every frame (`iced_tiny_skia .../window/compositor.rs::present`). So only **one**
   of the two buffers gets the full repaint; the other keeps the **old** background color in
   the empty (widget-free) areas, and the per-frame diff never re-clears it.
4. As the two buffers alternate on screen, the operator sees one correct frame and one
   stale-background frame → persistent flicker. Only an operation that **rebuilds the drawing
   surface** clears both buffers: a real window resize (which makes iced clear its layer
   history via `configure_surface`) or a full restart.

The app's styling already feeds the correct new background into the renderer
(`application_style` returns `window_background()`), so the bug is in how the renderer reuses
buffers, not in the app's color wiring.

### Why it only shows on the Pi

The bug requires the double-buffered **Wayland** path. The operator's desktop uses wgpu
(no bug). WSL's normal X11 path is **single-buffered** (`softbuffer .../backends/x11.rs` —
buffer age maxes at 1), so it cannot manifest. WSL's Wayland path cannot sustain a
software surface at all (dies after ~5s with "Lost when presenting surface"). The Pi runs
fullscreen under Wayland/Sway, which is the one environment that double-buffers. **Therefore
this bug can only be reproduced and verified on the actual Pi.**

### Evidence / confidence

- **Cause:** strongly supported. The mechanism is read directly from the libraries' source,
  and it passed a falsification test: the theory predicts a single-buffered backend will
  switch cleanly, and toggling High Contrast on WSL-X11 did switch cleanly (observed
  2026-06-26). Not yet observed at the mechanism level on the failing path.
- **Fix:** unproven on hardware. The renderer behavior it relies on (a real resize clears the
  layer history → both buffers repaint) is confirmed in source, but whether the chosen
  trigger actually produces a resize event on the Pi's **fullscreen Sway** window can only be
  checked on the Pi.

---

## Goal & scope

**Goal:** Switching display mode repaints the whole screen immediately on the Pi, with no
restart and no leftover stale colors.

**In scope:**
- `refbox/src/app/mod.rs` — the `Message::CycleDisplayMode` handler and a new repaint helper;
  store the `fullscreen` flag on the app so the handler can read it.

**Explicitly out of scope:**
- The High Contrast palette / what the modes look like (unchanged).
- Any change to desktop/Windows/Mac rendering (the fix is gated off there).
- The broader rendering pipeline, the LED-panel/overlay output, and Bug 1 (countdown toggle).
- No new dependencies, no iced upgrade, no patching of iced/softbuffer.

---

## Approach

When the operator changes display mode, force the screen to fully rebuild — **but only where
the bug exists** (Linux, running fullscreen, i.e. the Pi). Everywhere else the handler behaves
exactly as today (no blink, no change).

### Components

1. **Store `fullscreen` on the app.** `RefBoxApp` does not currently retain the startup
   `fullscreen` flag (it is destructured in `new()` and used only for the startup task). Add a
   `fullscreen: bool` field set from `RefBoxAppFlags` so the update handler can read it.

2. **A pure gate function** — e.g. `fn should_force_repaint(fullscreen: bool) -> bool` that
   returns `cfg!(target_os = "linux") && fullscreen`. Pure and unit-testable (the only
   automatically-testable seam in this change).

3. **A repaint helper** — `fn force_display_repaint(&self) -> Task<Message>` that returns
   `Task::none()` when the gate is false, otherwise returns the repaint task. The helper is the
   **single place** that selects the strategy, so changing strategy on the Pi is a one-line
   edit here (plus a rebuild/redeploy).

4. **Wire it into `CycleDisplayMode`.** The handler (currently sets config + atomic, persists,
   returns `Task::none()`) returns `self.force_display_repaint()` instead.

### The strategy (what the helper emits) — ship the bounce

There is no free "try all three in one session": each strategy is a separate Pi build +
deploy. So lead with the one the source analysis says is most likely to work, and keep the
others as one-line swaps in the same helper if it disappoints.

1. **Fullscreen bounce — the shipped default:** `window::get_latest()` →
   `change_mode(Windowed)` then `change_mode(Fullscreen)`, sequenced. A genuine size change →
   iced clears its layer history (`configure_surface`) → both buffers fully repaint → stale
   colors cleared. Cost: a brief on-screen blink on the mode-change tap. This is the expected
   fix.
2. **Invisible resize — alternative, low probability:** `window::get_latest()` →
   `window::get_size` → resize by one pixel and back. Would be blink-free *if* the compositor
   honored it, but a fullscreen Wayland/Sway window almost certainly ignores a client resize,
   so it is expected to be a no-op on the Pi. Only worth a try if a fully-invisible fix is
   wanted and we are willing to spend a deploy confirming it does nothing.
3. **Restart — last-resort backstop:** reuse the existing `RESTART_PENDING` + `iced::exit()`
   path (as the language change does). Guaranteed visual fix, but heavy and likely resets a
   live game — use only if the bounce fails to rebuild the surface on the Pi.

**Recommendation:** implement and ship the **fullscreen bounce**. If Pi testing shows it does
not rebuild the surface, swap the helper to the **restart** backstop (one-line change). The
resize is documented but not the default, because it is expected to no-op fullscreen.

### Data flow

`CycleDisplayMode` → set `config.display_mode` + `theme::set_display_mode` + `persist_config`
(unchanged) → `force_display_repaint()` → (Linux+fullscreen only) window task that rebuilds the
surface → both buffers repaint in the new palette.

### Error handling / safety

- All steps are `window::*` tasks already used elsewhere (`window::get_latest` +
  `window::change_mode` is the existing startup-fullscreen pattern). If the window id can't be
  fetched the task simply does nothing — no panic, no `unwrap`/`expect`.
- Off the affected platform the helper returns `Task::none()`, so behavior on desktop/Win/Mac
  is byte-for-byte unchanged.
- The mode change still persists to config first, so even a restart fallback comes back in the
  correct mode.

---

## Acceptance criteria

Because the bug is Pi-only, acceptance is split:

**On dev (desktop/WSL) — regression guard:**
- Toggling display mode behaves exactly as before: no blink, no restart, no change.
- `should_force_repaint(false)` is `false`; on non-Linux it is `false` regardless.
- `cargo test -p refbox` and `cargo clippy -p refbox -- -D warnings` pass; `just check` clean.

**On the Pi — the real verification (operator-observed):**
- Switching to High Contrast (and between all three modes) repaints the **whole** screen
  immediately; no patchwork, no flicker, no restart needed.
- The fix holds while a game clock is running and after navigating between pages.
- Any blink is brief and only on the deliberate mode-change tap.

---

## Testing

- **Automated:** unit-test the pure `should_force_repaint` gate (true only on linux+fullscreen).
  The window task itself is opaque and not unit-testable; do not over-invest in mocking iced.
- **Manual on the Pi (gating verification):** the operator toggles display modes and confirms a
  clean repaint, per acceptance criteria. Ship the bounce; if it does not rebuild the surface,
  swap the helper to the restart backstop (one-line change) and redeploy.

## Risks & open questions

- **Does the bounce trigger a resize on the Pi's fullscreen Sway window?** The one unproven
  link. If `change_mode(Windowed)` does not produce a size change/`Resized` event, the bounce
  won't rebuild the surface and we fall to the restart backstop. Pi-only check.
- **Blink severity of the bounce.** Briefly leaving fullscreen may show the desktop for a
  frame or two. If too jarring, mitigations to try on the Pi: sequence the two mode changes as
  tightly as possible, or fall back to restart. Recorded as a follow-up, not a blocker.
