# Touchscreen Taps Discarded by Cursor Warp — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make a touchscreen tap reliably trigger the button it lands on, instead of being silently
discarded when the compositor restores the mouse pointer immediately after the finger lifts.

**Architecture:** Vendor `iced_winit` 0.13.0 into `vendor/`, redirect it with `[patch.crates-io]`,
and extract its cursor bookkeeping into a small pure `CursorTracker` that refuses to let the single
synthetic `CursorMoved` following a `CursorEntered` overwrite an in-flight touch position. No refbox
source changes at all.

**Tech Stack:** Rust 2024, `iced` 0.13 (`iced_winit` 0.13.0, `iced_widget` 0.13.4,
`iced_core` 0.13.2), `winit` 0.30.12, Wayland/Sway on Raspberry Pi (aarch64), tiny-skia renderer.

**Spec:** This document. The root cause section below *is* the spec — it was established
empirically on field Pi `uwh-refbox-006` on 2026-08-26 and every link is cited to source.

## Global Constraints

- **MSRV 1.85**, edition 2024. No APIs newer than 1.85.
- **`cargo clippy --workspace --all-targets --all-features -- -D warnings`** must pass on Linux,
  Windows and macOS. Vendored third-party code must be kept *out* of the workspace so our lint
  settings are not applied to it.
- **Do NOT upgrade iced to a new major version.** It stays on 0.13. This plan pins `iced_winit` to
  the exact vendored 0.13.0 source with a minimal diff.
- **No `unwrap()`/`expect()`** in new non-test code without a comment justifying it.
- **No new third-party dependencies.**
- `cargo fmt --all` before every commit. Commit format `type(scope): description`, lowercase,
  imperative, ≤72 chars.
- Branch: `fix/refbox/touch-tap-cursor-warp`.

---

## Root cause

Confirmed 2026-08-26 by a `WAYLAND_DEBUG=1` capture on `uwh-refbox-006`, then confirmed a second
time by a positive prediction (see "Confirming prediction" below).

The captured protocol traffic for **one tap at (530, 190)**:

```
wl_pointer#53.leave(959, wl_surface#35)
wl_pointer#23.leave(959, wl_surface#35)
wl_touch#27.down(960, 1691925, wl_surface#35, 0, 530.00000000, 190.12500000)
wl_touch#27.motion(1691937, 0, 529.00000000, 190.12500000)      ×4, all ~529/190
wl_touch#27.up(961, 1691959, 0)                       @ 3261750.655
wl_pointer#23.enter(962, wl_surface#35, 100.0, 100.0) @ 3261750.693   <-- 0.038 ms later
wl_pointer#53.enter(962, wl_surface#35, 100.0, 100.0) @ 3261750.711
```

Sway hides the cursor when a finger lands (`wl_pointer.leave`) and **restores it the instant the
finger lifts, emitting `wl_pointer.enter` at the physical pointer's parked position (100, 100)** —
nowhere near the touched button.

The chain, every link read in source:

1. `wl_touch.down` at (530, 190) → iced's cursor is set there → the button under it sets
   `is_pressed = true`. **This is the visible press highlight**, and it is why the symptom looked
   like "the tap registered and was then thrown away".
2. `winit-0.30.12/src/platform_impl/linux/wayland/seat/pointer/mod.rs:127-139` — a pointer `Enter`
   pushes `WindowEvent::CursorEntered` **and then `WindowEvent::CursorMoved { position }`** with the
   enter position.
3. `iced_winit-0.13.0/src/program/state.rs:169-176` — `CursorMoved` and `Touch { location }` both
   write the **same** `cursor_position` field. iced exposes one cursor, shared between mouse and
   touch. It becomes (100, 100). `CursorEntered` is handled nowhere in the crate.
4. `iced_winit-0.13.0/src/program.rs:1027-1034` — `ui.update(&window_events, window.state.cursor(), …)`
   evaluates a whole **batch** of events against a **single, final** cursor value. So `FingerLifted`
   is judged with the cursor already warped.
5. `iced_widget-0.13.4/src/button.rs:312-329` — the release arm publishes `on_press` only
   `if cursor.is_over(bounds)`. (100, 100) is not over the button → **the action is silently
   dropped**. `is_pressed` is still cleared, and `button.rs:356-368` then redraws `Status::Active`,
   so the highlight disappears as though nothing happened.

**Why it is intermittent** (the property nothing else explained): if `touch.up` and `pointer.enter`
land in *different* iced event batches — pure frame-timing luck — `FingerLifted` is evaluated
*before* the warp, `is_over` is true, and the tap works. They arrive ~0.04 ms apart so they almost
always share a batch. Hence "almost never works, very occasionally does". This is the long-standing
intermittent field complaint.

**Confirming prediction (passed).** If taps only survive where the cursor already is, then parking
the physical mouse pointer *on* a button should make that button tappable while distant buttons stay
dead. Tested on the Pi: exactly that behaviour. Diagnosis confirmed.

**Diagnostic trap worth recording:** `libinput debug-events` shows a *perfect* touch-only stream with
no pointer events whatsoever, because `wl_pointer.enter` is **synthesised by Sway**, not by libinput.
A clean libinput capture does not rule out pointer interference. Only `WAYLAND_DEBUG=1` sees it.

**Not this bug:** iced issue #1392 is an older, different fault (winit reporting no location on
touch-up). It was fixed by winit PR #2255 and `winit-0.30.12/.../touch/mod.rs:97` correctly reports
the last-known location. Do not conflate them.

## Why the fix goes in `iced_winit`

Three candidate layers were considered:

| Layer | Change size | Widgets fixed | refbox churn |
|---|---|---|---|
| `iced_winit` cursor tracking | ~10 lines + tests | **all of them** | **none** |
| `iced_widget` `button` (use the touch event's own `position`) | ~6 lines | `button` only | none |
| refbox wrapper widget | new `Widget` impl | only where applied | **54 call sites, 12 files** |

`iced_core::touch::Event` *does* carry an accurate `position` on every variant
(`iced_core-0.13.2/src/touch.rs:9-18`), and `button.rs:313` throws it away with `{ .. }`. Fixing
that is the most upstreamable change, but it fixes **buttons only** — refbox also uses `mouse_area`
(`main_view.rs:206` for the alarm face, `shared_elements.rs:226` for the long-press restore), and
every other touch-aware widget would stay broken.

The refbox-side wrapper was rejected on blast radius: 54 raw `button(` call sites across 12
view-builder files (13 in `shared_elements.rs`, 41 elsewhere), plus 2 `mouse_area` sites, and it
still would not fix `scrollable`. That is a UI-wide change with real regression risk on every
screen, to fix an input-plumbing bug.

Fixing the cursor bookkeeping is one small change, in one place, that fixes every widget at once and
touches no refbox source.

## The vendoring decision — APPROVAL REQUIRED BEFORE TASK 1

`iced_winit` is not reachable through iced's public API, so it must be redirected via
`[patch.crates-io]`. Two ways:

- **(A) Vendor into `vendor/iced_winit/` (recommended).** 10 files, 3457 lines, 180 KB — small. It
  is `exclude`d from the workspace so `cargo clippy --workspace -D warnings` never lints third-party
  code, and a dedicated `just` recipe runs its tests so **the regression test runs in our CI**. Eric
  can watch a test fail before the fix and pass after, which is the verifiability the project rules
  require. Cost: 3457 vendored lines in the repo, and a manual re-vendor if iced is ever bumped.
- **(B) Git fork under the org.** No vendored code, but its tests never run in our CI, we depend on
  an external repo at build time, and offline/Pi builds get harder.

**Recommended: (A).** Both `Cargo.toml` changes and vendoring are shared infrastructure, so this
needs Eric's explicit sign-off before Task 1 begins.

## File structure

- `vendor/iced_winit/**` — verbatim copy of `iced_winit` 0.13.0. Only two files diverge from
  upstream; everything else must be byte-identical so a future re-vendor is a clean diff.
- `vendor/iced_winit/src/program/cursor.rs` — **new.** The `CursorTracker` type and its unit tests.
  Pure value logic, no `winit::window::Window`, so it is testable without a real window.
- `vendor/iced_winit/src/program/state.rs` — modified. `cursor_position` field replaced by a
  `CursorTracker`; the `CursorMoved` / `CursorLeft` arms delegate to it; a new `CursorEntered` arm
  is added.
- `Cargo.toml` (workspace root) — adds `[patch.crates-io]` and `exclude`.
- `justfile` — adds a `test-vendor` recipe and calls it from `check`.
- No files under `refbox/` are touched by this plan.

---

### Task 1: Vendor `iced_winit` with no behaviour change

Goal: prove the patch plumbing works and the app is byte-for-byte equivalent *before* changing any
logic. If this task is green, every later failure is attributable to our diff.

**Files:**
- Create: `vendor/iced_winit/**` (copy of the crate)
- Modify: `Cargo.toml` (workspace root)
- Modify: `Justfile` (note the capital J)

**Interfaces:**
- Consumes: nothing.
- Produces: a workspace that builds `iced_winit` from `vendor/iced_winit` instead of crates.io.

- [ ] **Step 1: Confirm approval has been given for option (A)**

Do not proceed without it. `Cargo.toml` changes and vendoring are shared infrastructure.

- [ ] **Step 2: Copy the crate in verbatim**

```bash
mkdir -p vendor
cp -r ~/.cargo/registry/src/index.crates.io-*/iced_winit-0.13.0 vendor/iced_winit
chmod -R u+w vendor/iced_winit
rm -f vendor/iced_winit/.cargo-ok
```

- [ ] **Step 3: Record the upstream provenance**

Create `vendor/iced_winit/VENDORED.md`:

```markdown
# Vendored `iced_winit` 0.13.0

Verbatim copy of `iced_winit` 0.13.0 from crates.io, with ONE deliberate divergence:
the cursor bookkeeping in `src/program/state.rs` plus the new `src/program/cursor.rs`.

Why: iced 0.13 exposes a single cursor position shared between mouse and touch. Wayland
compositors restore the mouse pointer immediately after a touch ends, and that synthetic
pointer position overwrote the touch position before the widget tree saw the finger lift,
so touchscreen taps were silently discarded. See
`docs/superpowers/plans/2026-08-26-touch-tap-discarded-cursor-warp.md`.

Every other file must stay byte-identical to upstream so re-vendoring is a clean diff.
To re-vendor: copy the new upstream version in, then re-apply the `cursor.rs` +
`state.rs` diff.

This crate is deliberately EXCLUDED from the workspace so our `-D warnings` clippy
settings are not applied to third-party code. Its tests run via `just test-vendor`.
```

- [ ] **Step 4: Wire up the patch and the exclusion**

In the workspace root `Cargo.toml`, add `"vendor/iced_winit"` to the existing
`[workspace] exclude` list (it already contains `"wireless-remote"`), and append:

```toml
# `iced_winit` is vendored to carry a single fix: iced 0.13 shares one cursor
# position between mouse and touch, and a compositor restoring the pointer after
# a touch discarded the tap. See vendor/iced_winit/VENDORED.md.
[patch.crates-io]
iced_winit = { path = "vendor/iced_winit" }
```

- [ ] **Step 5: Add the vendored test recipe**

In `Justfile` (capital J), add the recipe:

```make
# Run the vendored iced_winit tests (excluded from the workspace, so `cargo test
# --workspace` does not reach them)
test-vendor:
    cargo test --manifest-path vendor/iced_winit/Cargo.toml
```

Then add it to the aggregate check at `Justfile:12`, so the regression test runs in CI. It becomes:

```make
check: fmt-check lint test test-vendor audit
```

- [ ] **Step 6: Verify the patch is actually in effect**

Run: `cargo tree -i iced_winit`
Expected: shows `iced_winit v0.13.0 (<repo>/vendor/iced_winit)` — a path, not a registry entry.
If it still shows a registry source, the patch is not applied. Stop and fix before continuing.

- [ ] **Step 7: Verify nothing else changed**

Run: `just check`
Expected: PASS, exactly as on `master`. Note the pre-change result first so you are comparing
like with like — `.claude/rules` records that `just lint` is not `--all-targets` and that a
pre-existing `player_grid.rs` error fails the strict form without failing CI. Do not "fix" that
here.

- [ ] **Step 8: Commit**

```bash
git add vendor/iced_winit Cargo.toml Cargo.lock Justfile
git commit -m "chore(workspace): vendor iced_winit 0.13.0 unchanged"
```

---

### Task 2: Extract cursor bookkeeping into a testable `CursorTracker`

Goal: a pure, unit-testable home for the decision, with tests that lock in **today's** behaviour.
Still no behaviour change — this task must be a refactor only.

**Files:**
- Create: `vendor/iced_winit/src/program/cursor.rs`
- Modify: `vendor/iced_winit/src/program/state.rs`
- Test: `vendor/iced_winit/src/program/cursor.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Consumes: Task 1's vendored crate.
- Produces: `pub(crate) struct CursorTracker` with
  `position(&self) -> Option<PhysicalPosition<f64>>`, `touched(&mut self, PhysicalPosition<f64>)`,
  `moved(&mut self, PhysicalPosition<f64>)`, `left(&mut self)`, and `entered(&mut self)`.
  `Default` yields no position. Task 3 changes only the bodies of `entered` and `moved`.

- [ ] **Step 1: Write the tests for current behaviour**

Create `vendor/iced_winit/src/program/cursor.rs`:

```rust
//! Tracks the single cursor position that iced exposes to widgets.
//!
//! iced writes both mouse movements and touch positions into one cursor, so the
//! two input methods can overwrite each other. This type owns that decision so
//! it can be reasoned about and tested on its own.

use winit::dpi::PhysicalPosition;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct CursorTracker {
    position: Option<PhysicalPosition<f64>>,
}

impl CursorTracker {
    /// The position widgets are evaluated against, if the cursor is available.
    pub(crate) fn position(&self) -> Option<PhysicalPosition<f64>> {
        self.position
    }

    /// A finger landed, moved, or lifted at `location`.
    pub(crate) fn touched(&mut self, location: PhysicalPosition<f64>) {
        self.position = Some(location);
    }

    /// The mouse pointer moved to `position`.
    pub(crate) fn moved(&mut self, position: PhysicalPosition<f64>) {
        self.position = Some(position);
    }

    /// The mouse pointer entered the window.
    pub(crate) fn entered(&mut self) {}

    /// The mouse pointer left the window.
    pub(crate) fn left(&mut self) {
        self.position = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: f64, y: f64) -> PhysicalPosition<f64> {
        PhysicalPosition::new(x, y)
    }

    #[test]
    fn starts_with_no_position() {
        assert_eq!(CursorTracker::default().position(), None);
    }

    #[test]
    fn a_touch_sets_the_position_to_the_finger() {
        let mut c = CursorTracker::default();
        c.touched(at(530.0, 190.0));
        assert_eq!(c.position(), Some(at(530.0, 190.0)));
    }

    #[test]
    fn a_mouse_move_sets_the_position() {
        let mut c = CursorTracker::default();
        c.moved(at(12.0, 34.0));
        assert_eq!(c.position(), Some(at(12.0, 34.0)));
    }

    #[test]
    fn a_pointer_leave_clears_the_position() {
        let mut c = CursorTracker::default();
        c.moved(at(12.0, 34.0));
        c.left();
        assert_eq!(c.position(), None);
    }
}
```

- [ ] **Step 2: Run the tests to verify they pass**

Run: `just test-vendor`
Expected: PASS — 4 tests. They describe existing behaviour, so they must pass immediately. If any
fails, the extraction does not match what `state.rs` does today; stop and reconcile.

- [ ] **Step 3: Declare the module**

In `vendor/iced_winit/src/program.rs`, alongside the existing `mod state;` and
`mod window_manager;` at lines 2-3, add:

```rust
mod cursor;
```

- [ ] **Step 4: Swap `state.rs` over to the tracker**

In `vendor/iced_winit/src/program/state.rs`, replace the `cursor_position` field with the tracker,
and delegate. The field declaration (around line 20) becomes:

```rust
    cursor: super::cursor::CursorTracker,
```

Its initialiser (around line 71) becomes:

```rust
            cursor: super::cursor::CursorTracker::default(),
```

The `Debug` impl's `.field("cursor_position", &self.cursor_position)` (around line 36) becomes:

```rust
            .field("cursor", &self.cursor)
```

The `cursor()` accessor (around lines 106-116) becomes:

```rust
    /// Returns the current cursor position of the [`State`].
    pub fn cursor(&self) -> mouse::Cursor {
        self.cursor
            .position()
            .map(|cursor_position| {
                conversion::cursor_position(
                    cursor_position,
                    self.viewport.scale_factor(),
                )
            })
            .map(mouse::Cursor::Available)
            .unwrap_or(mouse::Cursor::Unavailable)
    }
```

And the event arms (around lines 169-176) become:

```rust
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor.moved(*position);
            }
            WindowEvent::Touch(Touch { location, .. }) => {
                self.cursor.touched(*location);
            }
            WindowEvent::CursorEntered { .. } => {
                self.cursor.entered();
            }
            WindowEvent::CursorLeft { .. } => {
                self.cursor.left();
            }
```

- [ ] **Step 5: Verify the refactor changed nothing**

Run: `just test-vendor && just check`
Expected: PASS. This task must be behaviour-neutral — the app should behave exactly as after
Task 1.

- [ ] **Step 6: Commit**

```bash
cargo fmt --all
git add vendor/iced_winit Cargo.lock
git commit -m "refactor(workspace): extract iced_winit cursor tracking"
```

---

### Task 3: Stop a restored pointer from discarding a touch

Goal: the actual fix, driven by a test that replays the exact Sway sequence from the capture.

**Files:**
- Modify: `vendor/iced_winit/src/program/cursor.rs`
- Test: `vendor/iced_winit/src/program/cursor.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Consumes: Task 2's `CursorTracker`.
- Produces: no signature changes. Only the bodies of `entered`, `moved`, `touched` and `left`
  change, plus two private fields.

- [ ] **Step 1: Write the failing regression test**

Append to the `tests` module in `vendor/iced_winit/src/program/cursor.rs`:

```rust
    /// Replays the exact Wayland sequence captured on field Pi uwh-refbox-006
    /// on 2026-08-26 for a single tap at (530, 190). Sway hides the cursor when
    /// the finger lands and restores it 0.04 ms after the finger lifts, sending
    /// a pointer enter at the physical pointer's parked position of (100, 100).
    /// That must NOT become the cursor, or the widget tree evaluates the
    /// finger-lift against (100, 100) and silently discards the tap.
    #[test]
    fn a_restored_pointer_cannot_overwrite_a_touch() {
        let mut c = CursorTracker::default();

        c.left();                    // wl_pointer.leave, cursor hidden for touch
        c.touched(at(530.0, 190.125));   // wl_touch.down
        c.touched(at(529.0, 190.125));   // wl_touch.motion
        c.touched(at(529.0, 191.625));   // wl_touch.motion
        c.touched(at(529.0, 191.625));   // wl_touch.up
        c.entered();                 // wl_pointer.enter -> CursorEntered
        c.moved(at(100.0, 100.0));   // ...immediately followed by CursorMoved

        assert_eq!(
            c.position(),
            Some(at(529.0, 191.625)),
            "the restored pointer overwrote the touch position"
        );
    }

    #[test]
    fn only_the_first_move_after_an_enter_is_suppressed() {
        let mut c = CursorTracker::default();

        c.touched(at(530.0, 190.0));
        c.entered();
        c.moved(at(100.0, 100.0)); // synthetic, suppressed
        c.moved(at(101.0, 102.0)); // the user really is moving the mouse now

        assert_eq!(c.position(), Some(at(101.0, 102.0)));
    }

    #[test]
    fn a_pointer_enter_with_no_preceding_touch_is_honoured() {
        let mut c = CursorTracker::default();

        c.entered();
        c.moved(at(100.0, 100.0));

        assert_eq!(c.position(), Some(at(100.0, 100.0)));
    }

    #[test]
    fn a_pointer_leave_cannot_blank_a_touch_position() {
        let mut c = CursorTracker::default();

        c.touched(at(530.0, 190.0));
        c.left();

        assert_eq!(c.position(), Some(at(530.0, 190.0)));
    }

    #[test]
    fn a_mouse_move_reclaims_the_cursor_from_touch() {
        let mut c = CursorTracker::default();

        c.touched(at(530.0, 190.0));
        c.moved(at(10.0, 10.0)); // no enter first: a genuine mouse move
        c.left();

        assert_eq!(c.position(), None, "mouse should own the cursor again");
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `just test-vendor`
Expected: FAIL. `a_restored_pointer_cannot_overwrite_a_touch` fails with
`Some(PhysicalPosition { x: 100.0, y: 100.0 })`, and
`a_pointer_leave_cannot_blank_a_touch_position` fails with `None`. **This is the bug reproduced in
a test.** Do not proceed until you have seen it fail.

- [ ] **Step 3: Implement the fix**

Replace the struct and its `impl` in `vendor/iced_winit/src/program/cursor.rs` with:

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct CursorTracker {
    position: Option<PhysicalPosition<f64>>,
    /// Whether touch, rather than the mouse, last owned the cursor.
    touch_is_current: bool,
    /// Whether the next `moved` is the synthetic one that winit emits directly
    /// after a pointer enter, and must therefore be ignored.
    suppress_next_moved: bool,
}

impl CursorTracker {
    /// The position widgets are evaluated against, if the cursor is available.
    pub(crate) fn position(&self) -> Option<PhysicalPosition<f64>> {
        self.position
    }

    /// A finger landed, moved, or lifted at `location`.
    pub(crate) fn touched(&mut self, location: PhysicalPosition<f64>) {
        self.position = Some(location);
        self.touch_is_current = true;
    }

    /// The mouse pointer entered the window.
    ///
    /// Wayland compositors hide the cursor while a finger is on the screen and
    /// restore it the moment the finger lifts. winit turns that restore into a
    /// `CursorEntered` immediately followed by a `CursorMoved` at wherever the
    /// physical pointer was parked. Unfiltered, that move overwrites the touch
    /// position before the widget tree sees the finger lift, and the tap is
    /// discarded — so suppress exactly that one move.
    pub(crate) fn entered(&mut self) {
        self.suppress_next_moved = self.touch_is_current;
    }

    /// The mouse pointer moved to `position`.
    pub(crate) fn moved(&mut self, position: PhysicalPosition<f64>) {
        if self.suppress_next_moved {
            self.suppress_next_moved = false;
        } else {
            self.position = Some(position);
            self.touch_is_current = false;
        }
    }

    /// The mouse pointer left the window.
    ///
    /// Ignored while touch owns the cursor: a compositor hiding the pointer
    /// around a touch must not blank a position a finger just set.
    pub(crate) fn left(&mut self) {
        if !self.touch_is_current {
            self.position = None;
        }
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `just test-vendor`
Expected: PASS — all 9 tests.

- [ ] **Step 5: Verify the workspace is still clean**

Run: `just check`
Expected: PASS, same as the Task 1 baseline.

- [ ] **Step 6: Commit**

```bash
cargo fmt --all
git add vendor/iced_winit
git commit -m "fix(workspace): keep touch position when pointer is restored"
```

---

### Task 4: Verify on real hardware — MANUAL GATE

**This cannot be verified locally and the plan will not pretend otherwise.** refbox dies under WSL's
Wayland (`iced_winit] Error Lost when presenting surface`), and WSL-X11 has no touch device at all,
so there is no way to reproduce a touchscreen tap on the development machine. The unit tests in
Task 3 prove the decision logic; only a Pi proves the wiring.

**Files:** none.

- [ ] **Step 1: Build for the Pi — NOT possible on the development machine**

`just build-rpi` is `cross build --target aarch64-unknown-linux-gnu`, and `cross` needs Docker,
which is unreachable from this WSL distro. `cargo check --target aarch64-unknown-linux-gnu -p refbox`
does not substitute either: it fails on `openssl-sys`, which needs cross-compiled system libraries.
(Note that cargo prints `warning: build failed, ...`, so the word "warning" makes a hard failure
look benign — read the exit code, not the last line.)

What *was* verified here is narrower and should be stated as such: the patched crate alone
cross-compiles for the Pi's architecture —
`cargo check --manifest-path vendor/iced_winit/Cargo.toml --locked --features program --target
aarch64-unknown-linux-gnu` exits 0.

So producing the deployable Pi binary requires a Docker-capable machine or the release pipeline.
**That build is part of this gate, not a formality:** until it has been produced and run on the Pi,
nothing below has been checked, and the PR must say so plainly rather than implying a Pi build was
done. Once built, confirm with `file` that it reports `ELF 64-bit LSB pie executable, ARM aarch64` —
matching what the field Pi runs.

- [ ] **Step 2: Confirm which version the Pi is on**

The field Pi's binary was dated 20 Aug 2026, newer than the newest `refbox-v0.4.7.bak`. Get
`~/refbox --version` from it before comparing behaviour, so the before/after is on the same
baseline.

- [ ] **Step 3: Deploy alongside, not over, the existing binary**

Copy to the Pi as `~/refbox-touchfix` and run it directly rather than replacing `~/refbox`, so the
working copy is untouched and rollback is instant. Carry the environment
`start-refbox.sh` sets — **especially `UWH_PORTAL_URL_OVERRIDE`**, without which refbox talks to the
production portal and erases the Pi's saved portal login:

```bash
export RUST_BACKTRACE=1
export UWH_PORTAL_URL_OVERRIDE=https://api.dev.uwhportal.com
~/refbox-touchfix -n -f --serial-port /dev/ttyUSB1 --num-old-logs 50 --log-location /home/pi/refbox-logs
```

- [ ] **Step 4: Run the acceptance checks**

1. With the mouse pointer parked far from the button, tap a navigation button. **It must act.**
   (Before the fix it does nothing.)
2. Tap twenty different buttons in a row. All twenty must act — the bug was intermittent, so a
   single success proves nothing.
3. Mouse clicks must still work normally.
4. If a mouse is attached, move it after tapping; the pointer must still control the cursor.
5. Unplug the mouse and keyboard entirely, then repeat check 2. This is the real tournament
   configuration.
6. Confirm the alarm face (`mouse_area`, not `button`) still responds to touch, including a
   long press.
7. **Mouse click without moving, after a tap.** With a mouse attached, tap a button, then click the
   mouse *without moving it first*. The click must land on the widget under the pointer, not on the
   button that was just tapped. This is the residual documented in `vendor/iced_winit/VENDORED.md`:
   `MouseInput` and `MouseWheel` carry no position, so iced evaluates them against the tracked
   cursor, which after a tap still reads as the tap point until a genuine mouse move reclaims it.
   The tournament Pi runs with no mouse attached, which is why the residual is accepted — but it is
   worth seeing once so the behaviour is known rather than assumed.
8. **Alarm hold and release by finger.** Press and hold the alarm face with a finger, then lift.
   The buzzer must stop. `refbox/src/app/view_builders/main_view.rs:206` gives the alarm face
   `on_press`/`on_release` and **no** `on_exit`, and the global safety net at
   `refbox/src/app/mod.rs:6375` only catches `mouse::Event::ButtonReleased` — never a touch lift.
   So before this fix a touched-and-held alarm could lose its release to the same warp and the
   buzzer could stick. Confirm it does not.
9. **Timeout-revive long press.** Hold a used-up team-timeout button until it revives, then release
   by lifting the finger. Its `mouse_area`
   (`refbox/src/app/view_builders/shared_elements.rs:226-250`) previously reached its release
   message via `on_exit` — caused by the warp — and after the fix reaches it via `on_release`. Same
   message, different path, so it should behave identically; worth exercising once to be sure.

- [ ] **Step 5: Confirm the Sway stopgap is not masking the result**

If `swaymsg seat seat0 hide_cursor 0` was applied while testing the stopgap, it must be **reverted**
before this verification, or a passing result proves nothing about the code fix. Reverting is a
reboot, since the command is runtime-only.

- [ ] **Step 6: Record the outcome in this plan**

Append a "Hardware verification" section stating the date, the Pi, the refbox version, and each
acceptance check's result. `.claude/rules/embedded.md` requires hardware testing to be documented or
explicitly stated as not done.

---

## Deviations

_(Record any divergence from this plan here rather than in standalone commits — see
`.claude/rules/plan-execution.md`.)_

## Follow-ups — deliberately out of scope

- **Document the eGalax udev rule.** A separate fault found on the same Pi: udev tags the eGalax
  USB TouchController as `ID_INPUT_TABLET`, so libinput refuses it entirely and Sway never sees the
  touchscreen. Fixed by `/etc/udev/rules.d/91-libinput-egalax-local.rules`. Upstream libinput closed
  this WON'T FIX, so the rule is permanent and belongs in the Pi build instructions. Note that
  `/etc/udev/rules.d/` is a system path, so `./overlayfs.sh disable` is required first on Pis running
  the read-only overlay, or the rule vanishes on the next reboot.
- **Retire tslib on the Pis.** `ts_uinput` was adopted to work around the misdetection above and
  became a fragile single point of failure (upstream `libts/tslib#92`, unresolved). With the udev
  rule in place it should be removable.
- **Upstream the fix to iced.** The right long-term home. Worth filing with the `WAYLAND_DEBUG`
  capture attached, so the vendored copy can eventually be dropped.
- **Two `wl_pointer` objects.** The capture shows `wl_pointer#23` and `#53` both entering. Worth
  understanding whether that Pi has two seats configured.
