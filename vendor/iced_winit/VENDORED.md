# Vendored `iced_winit` 0.13.0

Verbatim copy of `iced_winit` 0.13.0 from crates.io, with TWO deliberate divergences:

1. The cursor bookkeeping in `src/program/state.rs` plus the new `src/program/cursor.rs`.

   Why: iced 0.13 exposes a single cursor position shared between mouse and touch. Wayland
   compositors restore the mouse pointer immediately after a touch ends, and that synthetic
   pointer position overwrote the touch position before the widget tree saw the finger lift,
   so touchscreen taps were silently discarded. See
   `docs/superpowers/plans/2026-08-26-touch-tap-discarded-cursor-warp.md`.

2. An empty `[workspace]` table added near the top of `Cargo.toml`.

   Why: this crate is excluded from the main workspace, but an excluded package's manifest
   is not automatically its own workspace root — cargo keeps walking up the directory tree
   looking for one. In a normal checkout that walk finds nothing above the repo root and the
   crate is treated as standalone without issue. But when this repo is checked out as a
   worktree nested inside another checkout of itself (as some development sandboxes do),
   that walk reaches the *outer* checkout's `Cargo.toml`, which knows nothing about this path,
   and cargo refuses to proceed ("current package believes it's in a workspace when it's
   not"). An empty `[workspace]` table makes this crate its own workspace root unconditionally,
   which is the fix cargo's own error message recommends, and it works identically in both a
   normal checkout and a nested one.

Every other file must stay byte-identical to upstream so re-vendoring is a clean diff.
To re-vendor: copy the new upstream version in, then re-apply the `cursor.rs` + `state.rs`
diff and re-add the empty `[workspace]` table to the new `Cargo.toml`.

This crate is deliberately EXCLUDED from the workspace so our `-D warnings` clippy
settings are not applied to third-party code. Its tests run via `just test-vendor`.
