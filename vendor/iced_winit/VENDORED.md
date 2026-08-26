# Vendored `iced_winit` 0.13.0

Verbatim copy of `iced_winit` 0.13.0 from crates.io, with TWO deliberate divergences so far:

1. An empty `[workspace]` table added near the top of `Cargo.toml`.

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

2. Cursor bookkeeping extracted out of `src/program/state.rs` into a new
   `src/program/cursor.rs`.

   Why: iced 0.13 exposes a single cursor position shared between mouse and touch. Wayland
   compositors restore the mouse pointer immediately after a touch ends, and that synthetic
   pointer position overwrites the touch position before the widget tree sees the finger lift,
   so touchscreen taps are silently discarded. See
   `docs/superpowers/plans/2026-08-26-touch-tap-discarded-cursor-warp.md`. This vendoring
   exists so that fix can be applied here.

   This divergence is **not** the fix itself — it is a pure extraction, with no behaviour
   change, made so the fix can land in a small, unit-tested place. `state.rs`'s
   `cursor_position: Option<PhysicalPosition<f64>>` field became a `CursorTracker` value
   (defined in the new `cursor.rs`), and `state.rs` now delegates to it; the tracker's tests
   lock in today's behaviour, including that `entered()` is presently an empty method, exactly
   as the old code did nothing on that event. The actual behaviour change is a later change,
   not this one.

Every other file must stay byte-identical to upstream so re-vendoring is a clean diff.
To re-vendor: copy the new upstream version in, then re-add the empty `[workspace]` table to
the new `Cargo.toml`, and re-apply the cursor-tracking extraction (and the fix, once it exists)
to `src/program/state.rs` and `src/program/cursor.rs`, per whatever instructions land alongside
them.

This crate is deliberately EXCLUDED from the workspace so our `-D warnings` clippy
settings are not applied to third-party code. Its tests run via `just test-vendor`.

## Dependency resolution and the committed lock

Because this crate is its own workspace root (see above), `cargo test --manifest-path
vendor/iced_winit/Cargo.toml` resolves its dependencies independently of the main workspace's
`Cargo.lock` — the pinned versions used everywhere else in this repo do not apply here.
`vendor/iced_winit/Cargo.lock` is committed, and both `just test-vendor` and the CI step pass
`--locked`, purely to keep that resolution deterministic: a new upstream package release can't
silently change what this check compiles, and neither `just test-vendor` nor CI has to hit the
network to pick versions.

This is **not** an MSRV guarantee. A fresh resolve of this crate's dependencies today pulls in
transitive packages (e.g. `wayland-protocols`) that require a newer rustc than this project's
1.85 MSRV floor. That is fine here because this path only ever runs under the pinned toolchain
in `rust-toolchain.toml` (currently 1.96) — never at the bare 1.85 MSRV. The committed lock is
about determinism for this test-only path, not about MSRV compliance.

`cargo audit` does not scan this crate's lock — only the main workspace `Cargo.lock` is audited
(see the `audit` recipe in `Justfile` and the `audit` job in `.github/workflows/rust.yml`).
