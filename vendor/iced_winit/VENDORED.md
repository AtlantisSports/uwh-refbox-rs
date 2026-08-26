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
