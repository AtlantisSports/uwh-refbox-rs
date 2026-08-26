# 025 — Vendoring `iced_winit` to fix discarded touchscreen taps

**Date:** 2026-08-26
**Status:** accepted

## Context

On the tournament Raspberry Pis, touchscreen taps were unreliable: a tap would visibly highlight
the button and then do nothing. It had been an intermittent field complaint for a long time.

A `WAYLAND_DEBUG=1` capture on field Pi `uwh-refbox-006` found the cause. iced 0.13 keeps a
**single** cursor position and writes both mouse movements and touch positions into it. Sway hides
the mouse pointer while a finger is on the screen and restores it the instant the finger lifts —
about 38 microseconds later — which winit delivers as a pointer-enter followed immediately by a
mouse-move at wherever the physical pointer was parked. That move overwrote the touch position
before the widget tree was asked to evaluate the finger lift, so `button` saw a cursor that was no
longer over it and dropped the press. The two events arrive so close together that they almost
always land in the same batch, which is why the tap almost never worked — and occasionally did.

The fault is entirely inside the third-party `iced_winit` crate, which is not reachable through
iced's public API. Any fix there requires redirecting the crate with `[patch.crates-io]`.

## Decision

Vendor `iced_winit` 0.13.0 into `vendor/iced_winit`, redirect it with `[patch.crates-io]` in the
workspace `Cargo.toml`, and fix the cursor bookkeeping there: a `CursorTracker` type that refuses
to let the synthetic mouse-move following a pointer-enter overwrite a touch position that is still
in flight.

No file under `refbox/` changes.

The vendored copy is `exclude`d from the main workspace and its tests run as their own CI step, so
the regression that caused this is guarded automatically rather than by memory.
`vendor/iced_winit/VENDORED.md` is the operational detail: what diverges from upstream, why, how to
re-vendor on an iced upgrade, and what not to do to that directory.

## Rejected alternatives

- **Patch `iced_widget`'s `button` instead.** The touch event carries its own accurate position and
  `button` throws it away; using it is a six-line change and the most upstreamable one. Rejected
  because it fixes **buttons only**. refbox also drives touch through `mouse_area` — the alarm face
  and the long-press restore — and every other touch-aware widget would have stayed broken.
- **A refbox-side wrapper widget.** Rejected on blast radius: 54 raw `button(` call sites across 12
  view-builder files, plus the two `mouse_area`-driven controls (the alarm face and the
  timeout-revive long press), and it still would not have fixed `scrollable`. That is a UI-wide
  change with regression risk on every screen, to fix an input-plumbing bug.
- **A fork of `iced_winit` under the org rather than a vendored copy.** No vendored code in the
  repo, but its tests would never run in our CI, the build would depend on an external repository,
  and offline builds for the Pi would get harder.

## Consequences

- **The vendored copy must be re-applied on every iced upgrade.** An iced 0.14 bump does not
  conflict with it — it silently stops using it, because nothing would depend on `iced_winit` 0.13
  any more, and cargo only *warns* about an unused patch. `just check-patch-applied` and a matching
  CI step exist to turn that silent regression into a hard failure.
- **A second `Cargo.lock` lives outside `cargo audit`.** The vendored crate is its own workspace
  root, so it resolves its dependencies independently; its lock is committed for determinism, but
  the `audit` job reads only the root lock. Advisories reaching that path would not be reported.
- **`-D warnings` now applies to third-party code.** A `[patch.crates-io]` path package does not get
  cargo's `--cap-lints allow`, so upstream's own warnings become our CI errors. One lint is allowed
  on the vendored package to absorb that; see divergence 3 in `VENDORED.md`.
- **One half of the fix is prophylactic.** The observed bug needed only the pointer-enter guard; the
  pointer-leave guard was kept as well because leave/enter ordering is compositor-dependent. It
  creates a residual: after a tap, the cursor keeps reporting as available at the last tap point
  until a genuine mouse move reclaims it. With no mouse attached — the tournament configuration —
  that is invisible. With a mouse attached, a click or scroll made without moving the mouse first
  acts on the widget under the last tap rather than under the pointer. Accepted knowingly; there is
  no cheap remedy, because the true pointer position is unknown until it moves.
- **The fix cannot be verified by CI.** It reproduces only on a real touchscreen under a compositor
  that warps the pointer. The unit tests prove the decision logic and the event wiring; a Pi proves
  the rest, and that hardware pass is a required gate before this ships.
