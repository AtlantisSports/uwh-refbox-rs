# High Contrast Pi Repaint Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When the operator changes display mode, the whole screen repaints immediately on the Pi (no restart, no leftover patchwork), while desktop/Windows/Mac behavior is unchanged.

**Architecture:** The display-mode handler currently just updates state and returns `Task::none()`. We make it return a "force full repaint" task that — only on Linux when running fullscreen (the Pi) — briefly toggles the window out of and back into fullscreen. That genuine size change makes iced's tiny-skia renderer clear its per-buffer layer history (`configure_surface`), so both double-buffered frames repaint in the new palette instead of one keeping stale background pixels. The trigger is gated by a pure, unit-tested predicate; everywhere else the handler behaves exactly as before.

**Tech Stack:** Rust 2024, `iced 0.13` (tiny-skia on Linux, wgpu on Win/Mac), `refbox` crate only.

**Design spec:** `docs/superpowers/specs/2026-06-26-high-contrast-pi-repaint-design.md`

## Global Constraints

- **MSRV:** Rust 1.85 — no newer language/std features.
- **Edition:** Rust 2024.
- **Clippy:** `cargo clippy -p refbox -- -D warnings` must pass — **zero warnings**, including `dead_code`. Note CI does **not** use `--all-targets` here, so any field/fn read only by tests counts as dead code in the normal build.
- **No `unwrap()`/`expect()`** in production code without a justifying comment.
- **No new dependencies**, no iced upgrade, no edits to `uwh-common`, the LED-panel/overlay wire format, or `wireless-remote`.
- **Scope:** `refbox/src/app/mod.rs` only. Do not change palettes, translations, or any other file.
- **Verification commands** (this crate is bin-only):
  - Tests: `cargo test -p refbox` (no `--lib`)
  - Lint: `cargo clippy -p refbox -- -D warnings` (no `--all-targets`, mirrors CI)
  - Full gate before PR: `just check`

---

### Task 1: Force a full repaint on display-mode change (Pi only)

All four pieces (gate predicate, stored `fullscreen` flag, repaint helper, handler wiring) land in one commit because each is dead code without the others under `-D warnings`.

**Files:**
- Modify: `refbox/src/app/mod.rs`
  - Struct field: `RefBoxApp` definition (`refbox/src/app/mod.rs:133`), add field near `list_all_events: bool` (`:198`).
  - Constructor: `Self { … }` literal in `new()` (`refbox/src/app/mod.rs:1830`), add `fullscreen,` near `list_all_events,` (`:1861`).
  - Helper method: inside the `impl RefBoxApp` block, immediately after `application_style` (`refbox/src/app/mod.rs:5254-5264`).
  - Free predicate fn: module level, immediately after that `impl` block closes (`:5265`), before `decide_restore` (`:5270`).
  - Handler: `Message::CycleDisplayMode` arm (`refbox/src/app/mod.rs:2841-2847`).
  - Test: new `#[cfg(test)] mod repaint_gate_tests` at end of `refbox/src/app/mod.rs`.

**Interfaces:**
- Produces:
  - `fn should_force_repaint(fullscreen: bool) -> bool` — module-private free fn; `true` only on Linux when `fullscreen`.
  - `RefBoxApp::force_display_repaint(&self) -> Task<Message>` — returns the repaint task when gated on, else `Task::none()`.
  - `RefBoxApp.fullscreen: bool` — private field, set once in `new()` from `RefBoxAppFlags.fullscreen`.
- Consumes: existing `window::get_latest`, `window::change_mode`, `window::Mode` (already imported at `:19` and used at `:1920`); `Task` (already in scope, used at `:2846`).

- [ ] **Step 1: Write the failing unit test for the gate predicate**

Add at the very end of `refbox/src/app/mod.rs`:

```rust
#[cfg(test)]
mod repaint_gate_tests {
    use super::*;

    #[test]
    fn windowed_never_repaints() {
        // A windowed window has no stale second buffer to clear, on any platform.
        assert!(!should_force_repaint(false));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn linux_fullscreen_repaints() {
        // The Pi (Linux/tiny-skia, fullscreen) is the one place the flicker occurs.
        assert!(should_force_repaint(true));
    }

    #[test]
    #[cfg(not(target_os = "linux"))]
    fn non_linux_never_repaints() {
        // Windows/Mac use wgpu and don't have the bug; never force a repaint.
        assert!(!should_force_repaint(true));
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p refbox repaint_gate_tests`
Expected: FAIL — compile error, `cannot find function should_force_repaint in this scope`.

- [ ] **Step 3: Add the pure gate predicate**

Immediately after the `impl RefBoxApp` block that ends at `refbox/src/app/mod.rs:5265` (i.e. before `fn decide_restore` at `:5270`), add:

```rust
/// Whether changing the display mode must force a full-screen repaint.
///
/// Only the Pi hits the flicker: Linux uses the tiny-skia software renderer
/// with a double-buffered Wayland surface, and a palette-only change repaints
/// just one of the two buffers (the other keeps stale background pixels). On
/// Windows/Mac (wgpu) and on windowed/single-buffered setups there is no such
/// bug, so we never force a repaint — avoiding a needless on-screen blink.
const fn should_force_repaint(fullscreen: bool) -> bool {
    cfg!(target_os = "linux") && fullscreen
}
```

- [ ] **Step 4: Add the `fullscreen` field to `RefBoxApp`**

In the `RefBoxApp` struct, immediately after `list_all_events: bool,` (`refbox/src/app/mod.rs:198`), add:

```rust
    /// `true` when started with `--fullscreen` (the Pi). Read by
    /// `force_display_repaint` to decide whether a display-mode change needs a
    /// full-screen repaint. See `should_force_repaint`.
    fullscreen: bool,
```

- [ ] **Step 5: Set the field in the constructor**

In the `let mut new = Self { … }` literal in `new()`, immediately after `list_all_events,` (`refbox/src/app/mod.rs:1861`), add the shorthand (the `fullscreen` local already exists from the flags destructure at `:1651`):

```rust
            fullscreen,
```

- [ ] **Step 6: Add the repaint helper method**

Inside the `impl RefBoxApp` block, immediately after the `application_style` method (after `refbox/src/app/mod.rs:5264`, before the block's closing `}` at `:5265`), add:

```rust
    /// Force the whole window to repaint after a display-mode change.
    ///
    /// On the Pi (Linux/tiny-skia, fullscreen) a palette-only change otherwise
    /// leaves stale background pixels in one of the double-buffered frames,
    /// producing a persistent flicker. Briefly leaving and re-entering
    /// fullscreen is a genuine surface resize, which makes iced clear its
    /// per-buffer layer history so both buffers repaint in the new palette.
    /// Gated off everywhere else (no bug there; would just blink for nothing).
    /// Mirrors the startup fullscreen task at the top of `new()`.
    fn force_display_repaint(&self) -> Task<Message> {
        if !should_force_repaint(self.fullscreen) {
            return Task::none();
        }
        window::get_latest().and_then(|w| {
            window::change_mode(w, window::Mode::Windowed)
                .chain(window::change_mode(w, window::Mode::Fullscreen))
        })
    }
```

- [ ] **Step 7: Wire the helper into the `CycleDisplayMode` handler**

In the `Message::CycleDisplayMode` arm (`refbox/src/app/mod.rs:2841-2847`), replace the trailing `Task::none()` with `self.force_display_repaint()`. The arm becomes:

```rust
            Message::CycleDisplayMode => {
                let next = self.config.display_mode.next();
                self.config.display_mode = next;
                crate::app::theme::set_display_mode(next);
                self.persist_config();
                self.force_display_repaint()
            }
```

- [ ] **Step 8: Run the gate test to verify it passes**

Run: `cargo test -p refbox repaint_gate_tests`
Expected: PASS (2 tests run on this Linux dev machine: `windowed_never_repaints`, `linux_fullscreen_repaints`).

- [ ] **Step 9: Run lint and the full test suite**

Run: `cargo clippy -p refbox -- -D warnings`
Expected: finishes with no warnings (no `dead_code` — the field, fn, helper, and handler all reference each other).

Run: `cargo test -p refbox`
Expected: all tests pass.

- [ ] **Step 10: Commit**

```bash
git add refbox/src/app/mod.rs
git commit -m "fix(refbox): repaint whole screen on display-mode change

On the Pi the High Contrast switch left a persistent patchwork because
tiny-skia's double-buffered Wayland surface repaints only one buffer on a
palette-only change. Force a full repaint (brief fullscreen bounce) on
display-mode change, gated to Linux+fullscreen so desktop/Win/Mac are
unaffected.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: Verify on the Pi (manual — no code)

This is the real acceptance gate. The fix cannot be reproduced or proven on WSL/desktop (single-buffered / wgpu); only the Pi exercises the double-buffered Wayland path.

- [ ] **Step 1: Build and deploy to the Pi**

Run: `just build-rpi`, copy the binary + `.sha256` to the spare Pi (loose, not zipped — see `reference_release_asset_structure`), and launch it the normal way (`--fullscreen`).

- [ ] **Step 2: Observe the fix**

On the Pi: Settings → tap **VIEW MODE** through Light → Dark → High Contrast.
Expected: each switch repaints the **whole** screen cleanly — no patchwork, no flicker, no restart. A brief blink on the tap is acceptable. Confirm it also holds with a game clock running and after navigating between pages.

- [ ] **Step 3: If the bounce does NOT clear the flicker (fallback)**

If toggling fullscreen does not rebuild the surface on the Pi's Sway compositor, swap the helper body in `force_display_repaint` (Task 1, Step 6) to the restart backstop instead of the bounce:

```rust
        // Fallback: full restart (guaranteed surface recreation). Reuses the
        // same path as the language-change restart. Heavier and resets state,
        // so only if the fullscreen bounce fails to rebuild the surface.
        RESTART_PENDING.store(true, Ordering::Relaxed);
        iced::exit()
```

Use the exact `RESTART_PENDING.store(true, Ordering::Relaxed);` form already used at `refbox/src/app/mod.rs:1365`, followed by `iced::exit()`. Persist config first (the handler already calls `self.persist_config()` before this). Rebuild, redeploy, re-test from Step 1. Record the outcome in the design spec's Risks section.

---

## Self-Review

**1. Spec coverage:**
- Repaint on mode change, Pi-only gate → Task 1 (gate fn + helper + handler). ✓
- Store `fullscreen` flag (was startup-local) → Task 1 Steps 4-5. ✓
- Desktop/Win/Mac unchanged (no blink) → gate `cfg!(linux) && fullscreen`; test `non_linux_never_repaints`, `windowed_never_repaints`. ✓
- Fullscreen bounce as shipped default; restart as one-line fallback → Task 1 Step 6 + Task 2 Step 3. ✓
- Automated test of the pure gate; manual Pi verification → Task 1 Step 1 + Task 2. ✓
- No new deps, refbox-only, no palette change → Global Constraints + scope. ✓

**2. Placeholder scan:** No TBD/TODO; all code shown in full. ✓

**3. Type consistency:** `should_force_repaint(bool) -> bool`, `force_display_repaint(&self) -> Task<Message>`, field `fullscreen: bool` — names identical across struct, constructor, helper, handler, and tests. `and_then(|w| …)` + `chain(…)` mirror the verified iced API and the existing pattern at `:1920`. ✓
