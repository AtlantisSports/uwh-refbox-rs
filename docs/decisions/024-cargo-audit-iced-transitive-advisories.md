# 024 — Accepting iced-transitive cargo-audit advisories

**Date:** 2026-07-16
**Status:** accepted

## Context

`cargo audit` (the `audit` CI job) began failing after the RustSec advisory database
published new advisories affecting transitive dependencies. Three were flagged that are not
covered by the existing ignore list:

- **RUSTSEC-2026-0204** — `crossbeam-epoch` 0.9.18: invalid pointer dereference in the
  `fmt::Pointer` impl for `Atomic`/`Shared` when the underlying pointer is invalid. Fixed in
  `>=0.9.20`. Pulled transitively via `crossbeam-deque` → `rayon` → `image`.
- **RUSTSEC-2026-0194 / RUSTSEC-2026-0195** — `quick-xml` 0.37.5 (severity 7.5, high):
  quadratic run time when checking a start tag for duplicate attribute names. Fixed in
  `>=0.41.0`. Pulled **only** by `wayland-scanner` (a build-time proc-macro) →
  `smithay-client-toolkit` → `winit` → `iced`.
- **RUSTSEC-2025-0052** — `async-std` 1.13.2: unmaintained (a *warning*, not a vulnerability).
  Pulled by `iced_futures` → `iced`.

## Decision

- **Fix what is cleanly fixable.** Bump `crossbeam-epoch` to 0.9.20 via a lockfile update
  (`cargo update -p crossbeam-epoch --precise 0.9.20`). This is a semver-compatible patch bump,
  no code change, and stays within the MSRV 1.85.
- **Accept (ignore) the iced-locked advisories** in the `audit` CI job's ignore list:
  - `RUSTSEC-2026-0194`, `RUSTSEC-2026-0195` (`quick-xml`)
  - `RUSTSEC-2025-0052` (`async-std`)

## Rationale

- `quick-xml` and `async-std` reach the tree only through `iced`/`winit`. Their fixed versions
  require newer `winit`/`iced` releases, and upgrading `iced` is out of scope (a major, breaking
  change requiring separate discussion — see the project rules on iced upgrades).
- The `quick-xml` advisory is a denial-of-service vector against *untrusted* XML input. In this
  project `quick-xml` is used only by `wayland-scanner`, a **build-time proc-macro** that parses
  the trusted, shipped Wayland protocol XML at compile time. It never processes untrusted input,
  so the real-world exposure is effectively nil.
- `async-std`'s advisory is an "unmaintained" warning, not a vulnerability.

## Consequences

- The `audit` CI job passes again (`cargo audit` exits 0 with the updated ignore list).
- When `iced`/`winit` are next upgraded (a separate decision), re-evaluate and remove the
  `quick-xml` and `async-std` entries from the ignore list. Renovate may also resolve them via
  its own dependency-update branches; drop the ignores once the upgraded versions land.
- This acceptance is scoped to the advisories listed above; any new advisory must be evaluated
  on its own merits rather than assumed covered.
