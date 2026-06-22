# Release Checklist

How to cut a release of this project.

## How a release is built

Pushing a git tag of the form `vX.Y.Z` triggers `.github/workflows/release.yml`, which builds
native **Windows**, **macOS** (Arm + Intel), and **Raspberry Pi** binaries and assembles them
into a **draft** GitHub release (plus the loose Pi binary + `.sha256` for self-update). The
release is created as a draft — it is not public until someone publishes it.

## Version bump (do this first, on its own commit/PR)

Bump **every** crate version in lockstep — and this **includes `wireless-remote`**, which is a
*separate* Cargo workspace and therefore easy to forget. Always bump it too.

Crates to bump (own `version`, plus any internal path-dependency `version = "X.Y.Z"` references):

- `fonts`
- `matrix-drawing`
- `uwh-common`
- `wireless-modes`
- `overlay`
- `led-panel-sim`
- `schedule-processor`
- `refbox`
- **`wireless-remote`** ← separate workspace; do not skip it

Steps:

1. In each crate's `Cargo.toml`, change `version = "<old>"` to `version = "<new>"` (this also
   updates the internal path-dependency references, which use the same `version = "<old>"`
   string).
2. Run `cargo check --workspace` from the repo root to regenerate the main `Cargo.lock`. This
   does **not** touch `wireless-remote` (separate workspace) — leave `wireless-remote/Cargo.lock`
   as-is, matching previous releases. Do **not** run cargo inside `wireless-remote/` (different
   toolchain — see `.claude/rules/embedded.md`).
3. Commit as `chore(workspace): bump version to <new>` and merge to `master` via the merge queue.

> The wireless-remote bump is a version number only — no firmware code change, and the physical
> remotes do **not** need re-flashing for a refbox release.

## Cut the release

1. With the bump merged on `master`, push the tag: `git tag vX.Y.Z <master-sha> && git push origin vX.Y.Z`.
2. Wait for `release.yml` to finish; a **draft** release appears under Releases.
3. Download `refbox.zip` from the draft to test the platform builds (e.g. `Windows/refbox.exe`).
4. When satisfied, **publish** the draft. (Keep `--draft=true` on any `gh release edit`, or it
   publishes early.)
