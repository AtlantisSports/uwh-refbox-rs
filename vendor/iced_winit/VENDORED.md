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

   This divergence now carries the actual behaviour fix. `state.rs`'s
   `cursor_position: Option<PhysicalPosition<f64>>` field became a `CursorTracker` value
   (defined in the new `cursor.rs`), and `state.rs` delegates to it. `CursorTracker` tracks
   two extra bits of state: whether touch or the mouse last owned the cursor
   (`touch_is_current`), and whether the next mouse move is the synthetic one winit emits
   right after a pointer enter (`suppress_next_moved`). On `entered()`, if touch currently
   owns the cursor, the tracker arms that suppression flag; the next `moved()` then consumes
   the flag and is dropped instead of overwriting the position, so the compositor's restored
   pointer can no longer clobber a touch that just landed. `left()` is now a no-op while touch
   owns the cursor, so a compositor hiding the pointer around a touch can no longer blank the
   position the finger set. Neither change affects a genuine mouse move or leave once the
   mouse has reclaimed the cursor via an unsuppressed `moved()` — a `moved()` that itself gets
   suppressed does not reclaim anything.

   The `left()` half of the fix is prophylactic rather than evidence-driven: in the capture,
   both pointer-leaves arrive *before* the touch-down, so the observed bug only ever needed
   the `entered`/`moved` half — nothing in the capture exercises `left()` blanking a live
   touch. It is kept anyway, deliberately, because leave/enter ordering between the capture's
   two pointer objects is compositor-dependent, and a `leave` landing mid-gesture on a
   different compositor would otherwise blank a touch position that is still current. The
   residual this creates: after any tap, the cursor never reports as unavailable again until
   an unsuppressed mouse move reclaims it, so on a touch-only Pi `position()` keeps reading
   "available at the last tap point" indefinitely. That is a deliberate choice, not an
   accident, and a future maintainer should know it.

Every other file must stay byte-identical to upstream so re-vendoring is a clean diff.
To re-vendor: copy the new upstream version in, then re-add the empty `[workspace]` table to
the new `Cargo.toml`, and re-apply the cursor-tracking extraction and its fix to
`src/program/state.rs` and `src/program/cursor.rs`, per whatever instructions land alongside
them.

This crate is deliberately EXCLUDED from the workspace so our `-D warnings` clippy
settings are not applied to third-party code. Its tests run via `just test-vendor`.

The cursor tracker's tests live inside `src/program`, which upstream gates behind the
`program` Cargo feature (`Cargo.toml`'s `[features]` table) — a feature this crate does not
enable by default. Without it, `src/program` is never compiled, so the tests silently do not
exist to `cargo test` rather than failing: it reports "0 tests, ok" instead of running any of
them. Because of this, `just test-vendor`'s recipe and the CI step that mirrors it both pass
`--features program` explicitly, alongside `--locked`. If that flag is ever dropped, the gate
does not fail — it just goes quiet again, so treat its presence in both places as load-bearing.

Enabling `program` also compiles upstream code that was never built before this feature flag
was added, which surfaces two pre-existing `deprecated` warnings about
`UnboundedReceiver::try_next` in `src/program.rs`. Those warnings are upstream's, not ours —
fixing them would mean editing code that must stay byte-identical, so they are left alone. If
`just test-vendor` or its CI step ever shows warnings, this is why: it is not a regression
introduced by anything in this repo.

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
