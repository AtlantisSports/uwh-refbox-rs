# Rust Patterns

These rules govern Rust-specific behaviour in this workspace. They reflect the project's
technical requirements and the standards enforced by CI.

## Edition and MSRV

- **Edition:** Rust 2024 (all crates)
- **MSRV:** Rust 1.85 (all crates, including wireless-remote)
- Do not use language features or standard library APIs introduced after Rust 1.85
- Do not change the edition or MSRV without explicit discussion

## Formatting

Run `cargo fmt --all` before every commit. The pre-commit hook enforces this automatically, but
run it manually if needed with `just fmt`.

Never manually reformat code that you are not otherwise changing — let `cargo fmt` do it.

## Linting

All code must pass `cargo clippy --workspace --all-targets --all-features -- -D warnings` with
zero warnings. This is enforced by CI across Linux, Windows, and macOS.

Run `just lint` locally before committing. Fix every warning — do not use `#[allow(...)]`
attributes to silence warnings without explicit discussion and justification.

## Safety and Correctness

**No `unwrap()` or `expect()` in non-test production code** without a comment explaining why it
is guaranteed not to panic. If a panic would indicate a programming error (not a runtime
condition), document it.

**No new `unsafe` blocks** without explicit discussion. If `unsafe` is unavoidable, explain why
in a comment at the `unsafe` block.

## `no_std` Compatibility

`uwh-common` (core) and `matrix-drawing` must compile without the standard library. This is
required for embedded targets.

When adding dependencies to these crates:
- Check that the dependency supports `no_std` (look for `default-features = false` support)
- Gate any `std`-only functionality behind a feature flag (e.g., `#[cfg(feature = "std")]`)
- Never add a dependency that pulls in `std` unconditionally to these crates

## Feature Flags

- Document any new feature flag you add in a comment explaining what it enables
- Do not add feature flags speculatively — only add them when they are immediately needed
- Test both with and without the feature enabled (CI does this via `--no-default-features`)

## Iced (GUI Framework)

Used in `refbox` and `beep-test`. Version: 0.13.

- Use the existing theme and style patterns in `refbox/src/app/theme/` — do not introduce new
  styling approaches without discussion
- Follow the existing message/update/view pattern already established in the codebase
- Do not upgrade to a new major version of `iced` without explicit discussion (it involves
  significant breaking changes)

## Dependencies

- Do not add new dependencies without discussion, especially to `uwh-common` or the
  utility crates
- Prefer existing dependencies already in the workspace over adding a new one
- Renovation (`renovate.json`) handles routine dependency updates automatically — do not
  manually bump versions unless fixing a specific issue

## Testing

- Tests live alongside the code they test (in the same file using `#[cfg(test)]` modules)
  or in `tests/` directories within each crate
- Write tests for any bug fix (a test that would have failed before the fix)
- Run `just test` to verify all tests pass before committing
