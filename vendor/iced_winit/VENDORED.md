# Vendored `iced_winit` 0.13.0

Verbatim copy of `iced_winit` 0.13.0 from crates.io, with THREE deliberate divergences:

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
   `src/program/cursor.rs`, declared by a one-line `mod cursor;` added to
   `src/program.rs`.

   Why: iced 0.13 exposes a single cursor position shared between mouse and touch. Wayland
   compositors restore the mouse pointer immediately after a touch ends, and that synthetic
   pointer position overwrites the touch position before the widget tree sees the finger lift,
   so touchscreen taps are silently discarded. See
   `docs/superpowers/plans/2026-08-26-touch-tap-discarded-cursor-warp.md`. This vendoring
   exists so that fix can be applied here.

   This divergence now carries the actual behaviour fix. `state.rs`'s
   `cursor_position: Option<PhysicalPosition<f64>>` field became a `CursorTracker` value
   (defined in the new `cursor.rs`), and `state.rs` delegates to it: `update()` calls
   `self.cursor.handle(event)` once, before its own `match`, and upstream's cursor arms are
   gone from that `match` entirely. The event mapping — which `WindowEvent` means what to the
   cursor — therefore lives in `cursor.rs` alongside the logic, where the tests can reach it.
   That placement is deliberate: upstream handles `CursorMoved` and `Touch` in ONE shared arm
   and has no `CursorEntered` arm at all, so a re-vendor that reinstated upstream's arm shape
   would leave `CursorTracker`'s methods intact and the fix inert. With the mapping tested,
   that mutation fails three tests instead of passing silently. Nothing else in `update()`'s
   `match` reads or writes the cursor, so calling `handle` before it cannot change behaviour.

   `CursorTracker` tracks two extra bits of state: whether touch or the mouse last owned the
   cursor (`touch_is_current`), and whether the next mouse move is the synthetic one winit emits
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

3. `deprecated = "allow"` added to the existing `[lints.rust]` table in `Cargo.toml`.

   Why: cargo passes `--cap-lints allow` only to packages it fetches from a registry or a git
   source. This crate is reached through `[patch.crates-io]` as a **path** package, which is a
   local unit and gets no cap, so lint settings apply to it in full. CI sets
   `RUSTFLAGS: "-D warnings"` for the entire workflow (`.github/workflows/rust.yml`), and that
   has two separate consequences here:

   - **Today:** built with `--features program`, upstream's two `UnboundedReceiver::try_next`
     deprecations become hard errors, so the CI step that runs this crate's tests — the
     regression gate whose existence justified vendoring rather than forking — could not
     compile at all. It passed locally only because `just test-vendor` runs without
     `RUSTFLAGS`.
   - **Latent:** the main workspace compiles this same crate as a local path unit, with the
     `program` feature enabled by `iced`, under the same `RUSTFLAGS`. That is clean today only
     because the root `Cargo.lock` pins `futures-channel 0.3.31`, which predates the
     deprecation, while this crate's own lock pins `0.3.34`, which carries it. The day a
     dependency update bumps the root lock's `futures-channel` to 0.3.34 or later,
     `cargo build --all` fails on all three CI platforms with an error pointing into
     third-party code that nobody would connect to this vendoring.

   Allowing the lint on the package closes both faces at once: cargo emits a package's
   `[lints]` on the rustc command line in a position that beats `RUSTFLAGS`, verified by
   running the CI command with `RUSTFLAGS="-D warnings"` before and after (101, then 0).
   Setting `env: RUSTFLAGS: ""` on the CI step instead would have fixed only the first face and
   left the second armed.

Every other file must stay byte-identical to upstream so re-vendoring is a clean diff.
To re-vendor: copy the new upstream version in, then re-add the empty `[workspace]` table to
the new `Cargo.toml`, and re-apply the cursor-tracking extraction and its fix to
`src/program/state.rs` and `src/program/cursor.rs`, per whatever instructions land alongside
them.

This crate is deliberately EXCLUDED from the main workspace, which keeps it out of
`cargo clippy --all`'s primary set, so our clippy lints are never reported against
third-party code. Note what exclusion does NOT do: before vendoring, this crate came from a
registry, and cargo builds registry packages with `--cap-lints allow`, which silenced every
lint in it. A `[patch.crates-io]` **path** package is a local unit and gets no such cap, so
`RUSTFLAGS` and the package's own `[lints]` tables now apply to it in full — see divergence 3.
Its tests run via `just test-vendor`.

The cursor tracker's tests live inside `src/program`, which upstream gates behind the
`program` Cargo feature (`Cargo.toml`'s `[features]` table) — a feature this crate does not
enable by default. Without it, `src/program` is never compiled, so the tests silently do not
exist to `cargo test` rather than failing: it reports "0 tests, ok" instead of running any of
them. Because of this, `just test-vendor`'s recipe and the CI step that mirrors it both pass
`--features program` explicitly, alongside `--locked`. If that flag is ever dropped, the gate
does not fail — it just goes quiet again, so treat its presence in both places as load-bearing.

Enabling `program` also compiles upstream code that was never built before this feature flag
was added, which surfaces two pre-existing `deprecated` warnings about
`UnboundedReceiver::try_next` in `src/program.rs`. Those are upstream's, not ours, and fixing
them would mean editing code that must stay byte-identical — so divergence 3 allows the lint
on the package instead. Under CI's workflow-wide `RUSTFLAGS: "-D warnings"` they are not
warnings but hard **errors**, which is why that divergence exists rather than being a matter
of tidiness.

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
