# Per-platform release downloads

**Date:** 2026-09-11
**Status:** Approved in principle by Eric 2026-09-11; awaiting spec review
**Crates touched:** none — CI and docs only

## Goal

Replace the single `refbox.zip` — which today contains the Windows build, two separate macOS
builds, the Raspberry Pi build and five PDF guides — with one download per platform. As part of
that, the two macOS builds become a single universal app that runs natively on both Intel and
Apple-silicon Macs, delivered as a `.dmg`.

## Decisions already taken

These were settled in conversation on 2026-09-11 and are not open for re-litigation during
implementation.

| Decision | Ruling | Who |
|---|---|---|
| Apple code signing / notarisation | **Dropped.** Not a dependency of this work. macOS will warn "unidentified developer" and users right-click → Open. | Eric |
| Universal binary and `.dmg` | Both. Universal first. | Eric |
| Release shape | Split per platform. | Eric |
| The five PDF guides | **In every platform download**, each in its own folder, so platform-specific documents can be added later without rework. | Eric |
| Changeover | **Clean break.** `refbox.zip` disappears in the same release the new downloads appear. Eric confirmed he knows what written instructions point at it. | Eric |
| Windows code signing | Still undecided, and explicitly **out of scope** here. | — |

## Target release assets

| Asset | Contents |
|---|---|
| `refbox-windows.zip` | `refbox.exe` + `Documents/` |
| `refbox-macos.dmg` | universal `refbox.app` + `Applications` shortcut + `Documents/` |
| `refbox-raspberry-pi.zip` | `refbox` binary, executable bit set + `Documents/` |
| `refbox-aarch64-linux` | **unchanged** |
| `refbox-aarch64-linux.sha256` | **unchanged** |
| `overlay.zip`, `overlay-aarch64-linux`, `overlay-aarch64-linux.sha256` | **unchanged** |
| `refbox.zip` | **removed** |

`Documents/` is the same five PDFs in each: Getting Started (English), Getting Started (Spanish),
Providing App Credentials, Refbox User Manual, Refbox User Manual with Fouls.

## Constraints that must not be violated

1. **`refbox-aarch64-linux` and `refbox-aarch64-linux.sha256` keep those exact names.** The in-app
   updater looks them up by exact string (`refbox/src/updater/release.rs:4-5`). Renaming either
   breaks self-update on every Pi already deployed.
2. **The macOS bundle step keeps running from `refbox/`.** `cargo-bundle` resolves the icon path
   against the process working directory, not the package root, and drops the icon *silently* on
   no match. This was fixed in `81fd427c`; the fix must survive the job restructure. The existing
   assertion that `.icns` and `CFBundleIconFile` are present must survive with it.
3. **No code signing.** Nothing in this work may introduce a dependency on a certificate.
4. **The overlay is untouched.** It already ships as separate assets and stays that way.

## Design

### macOS: one job instead of two

`build-macos-arm` and `build-macos-x86` collapse into a single `build-macos` job:

1. Bundle the app for `aarch64-apple-darwin` with `--format osx`, run from `refbox/` (constraint 2).
2. Build the `x86_64-apple-darwin` binary separately.
3. `lipo -create` the two executables into one and write it over
   `refbox.app/Contents/MacOS/refbox`.
4. Assert `lipo -archs` on the result reports **both** `x86_64` and `arm64`. A silent failure to
   fuse must fail the release, not ship a half-built app.
5. Assert the icon is present, as today.
6. Stage `refbox.app`, an `Applications` symlink and `Documents/`, then `hdiutil create` the
   `.dmg`. `hdiutil` is used rather than `cargo bundle --format dmg` because the contents need to
   be controlled explicitly.
7. Upload the finished `.dmg` as the job's artifact.

The `.dmg` is built on the macOS runner, from the app that was compiled there moments earlier.

### A new job to fetch the PDFs

The guides are currently downloaded from Google Drive in `upload-release`, which runs on Linux.
The macOS job now needs them too, so the download moves into its own `fetch-docs` job that
retrieves them once and publishes them as an artifact. `build-macos` and `upload-release` both
consume it.

### Windows and Pi

Both keep their existing build jobs unchanged. `upload-release` assembles each into its own zip
alongside a copy of `Documents/`. The Pi binary still needs `chmod +x` before zipping — GitHub
normalises artifact permissions to `0644` — and the loose Pi assets are staged exactly as today.

### Two workarounds that disappear

- The macOS `chmod +x` restoration in `upload-release` is no longer needed: the app never
  round-trips through the artifact store as loose files.
- Extracting `refbox.zip` on Windows currently strips the executable bit from the macOS app,
  producing "the application refbox can't be opened". A `.dmg` carries its own filesystem and is
  opaque to Windows, so the corruption path is closed rather than patched.

## Verification

**Nothing here can be verified locally.** There is no Mac on this machine, and `just check` only
exercises the host platform. The only real proof is a release run.

1. Push a throwaway tag (e.g. `v0.5.2-rc1`). The workflow builds and creates a **draft** release,
   which is not public. A draft cannot reach Pis in the field: the in-app updater queries
   `/releases/latest`, which excludes drafts and pre-releases (`refbox/src/updater/net.rs:6,16`).
2. Inspect the draft: all eight expected assets are present — three platform downloads, two
   loose Pi assets, three overlay assets — `refbox.zip` is absent, each zip
   contains `Documents/` with five PDFs, and the Pi binary is executable.
3. **Draft releases cannot be shared.** GitHub shows a draft only to users with write access and
   its asset links require authentication. Eric downloads `refbox-macos.dmg` from the draft and
   sends the file to testers directly. Passing it through Windows and a cloud service is exactly
   the path that used to corrupt the Mac build, so the act of sharing is itself the test.
4. Testers need at least one Intel Mac and one Apple-silicon Mac between them. They must be told
   in advance that macOS will refuse the app on first launch — right-click → Open — because it is
   unsigned. Otherwise a working build gets reported as broken.
5. On an Apple-silicon Mac: Activity Monitor → find refbox → the **Kind** column must read
   **Apple**. If it reads **Intel**, the universal build failed and the Mac is running it under
   translation.
6. Delete the throwaway tag and draft afterwards.

## Files changed

- `.github/workflows/release.yml`
- `docs/release-checklist.md` — rewritten; it asserts on `refbox.zip`'s exact contents and would
  be actively wrong after this change.

## Explicitly out of scope

- Apple code signing and notarisation (dropped).
- Windows code signing — SignPath vs Certum is still an open question of Eric's, untouched here.
- Cross-platform self-update. The updater is Pi-only today and this change deliberately leaves its
  asset names alone.
- The overlay's packaging.

## Accepted trade-offs

- **PDFs inside the `.dmg` get left behind.** People drag the app to Applications and the
  documents stay on the disk image. Inherent to the format; accepted knowingly.
- **A clean break can strand written instructions.** The Getting Started guides live on Google
  Drive and could not be read from here, so whether any of them names `refbox.zip` is unverified.
  Eric accepted this risk on the basis that he knows what is written down.
