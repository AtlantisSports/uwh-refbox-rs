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
3. Download `refbox.zip` from the draft and **verify the packaging** — see below. Do not skip
   this: "test the platform builds" used to be all this step said, and a macOS app that could
   not launch shipped in five consecutive releases before anyone noticed.
4. When satisfied, **publish** the draft. (Keep `--draft=true` on any `gh release edit`, or it
   publishes early.)

## Verify the draft before publishing

Passing artifacts between jobs **strips the executable bit** — GitHub stores each artifact as a
zip and does not preserve file permissions, so everything arrives as `644`. A `chmod` step in
`release.yml` restores it. That step is the only automated guard: it fails the job if a path ever
stops matching, but **no PR-time CI covers packaging at all**, because `release.yml` runs only on a
`v*.*.*` tag. So the draft is the first and last chance to catch a bad build.

Check the zip itself, not just that it downloads:

```bash
unzip -Z refbox.zip | grep -E 'MacOS/refbox$|Raspberry Pi/refbox$'
```

- [ ] **Exactly three lines** come back — Arm bundle, Intel bundle, Pi binary. Fewer means a build
      is missing, and an empty result must not be read as a pass.
- [ ] Every one of them starts with `-rwxr-xr-x`. If any reads `-rw-r--r--`, the build is broken —
      **do not publish.**
- [ ] Folder names read `Mac (Arm processor)` / `Mac (Intel processor)` — no stray backslash.
- [ ] `Windows/refbox.exe` is a file, not a folder containing another `refbox.exe`.
- [ ] The release carries `refbox-aarch64-linux` and `refbox-aarch64-linux.sha256` as **loose
      assets** alongside `refbox.zip`. In-app self-update looks these up by exact name
      (`BIN_ASSET` / `SUM_ASSET` in `refbox/src/updater/release.rs`) and fails if either is
      missing or renamed. Copies of the Pi binary and its checksum also appear *inside* the zip
      under `Raspberry Pi/` and `rpi-sha256/` — that is normal and not a problem.
- [ ] A macOS user opens `refbox.app` and it launches.

On that last point: macOS will warn about an unidentified developer, because the app is ad-hoc
signed rather than notarised with a paid Apple certificate. That is expected and unrelated to the
packaging. Clear it via **System Settings → Privacy & Security → Open Anyway**. What you are
checking for is the *different* error — *"The application 'refbox' can't be opened."* — which means
the executable bit is missing.

### The overlay assets

The overlay ships on this same release as separate assets. It has no self-update, so someone copies
the binary to the streaming machine by hand.

```bash
unzip -Z overlay.zip | grep -E 'Raspberry Pi/overlay$'
```

- [ ] **Exactly one line** comes back. An empty result is a FAILURE, not a pass.
- [ ] It starts with `-rwxr-xr-x`. If it reads `-rw-r--r--`, do not publish.
- [ ] `file` on the extracted binary reports `ELF 64-bit LSB ... ARM aarch64`.
- [ ] Its SHA-256 matches `overlay-aarch64-linux.sha256`.
- [ ] `overlay-aarch64-linux` and `overlay-aarch64-linux.sha256` are present as loose assets.
      **These always download non-executable** — GitHub serves release assets with no Unix mode, so
      whoever installs one runs `chmod +x`. Expected, not a defect. `overlay.zip` is the copy that
      arrives runnable.
- [ ] `refbox.zip` contains **no** overlay files. The overlay is deliberately separate.
