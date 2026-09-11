# Per-platform release downloads — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the single `refbox.zip` with one download per platform — a Windows zip, a universal macOS `.dmg`, and a Raspberry Pi zip — each carrying the PDF guides.

**Architecture:** The two macOS jobs collapse into one that builds both architectures, fuses them with `lipo`, re-signs ad-hoc, and assembles the `.dmg` on the macOS runner. A new `fetch-docs` job downloads the PDF guides once and shares them as an artifact. `upload-release` assembles per-platform archives instead of one combined zip.

**Tech Stack:** GitHub Actions, `cargo-bundle`, Apple `lipo` / `codesign` / `hdiutil`, `zip`.

**Spec:** `docs/superpowers/specs/2026-09-11-per-platform-release-downloads-design.md`

## Global Constraints

- **`refbox-aarch64-linux` and `refbox-aarch64-linux.sha256` keep those exact asset names.** The in-app updater matches them by exact string (`refbox/src/updater/release.rs:4-5`). Renaming either breaks self-update on every deployed Pi.
- **The macOS bundle step must run with `working-directory: refbox`.** `cargo-bundle` resolves the icon path against the process working directory and drops the icon *silently* on no match. The existing `.icns` / `CFBundleIconFile` assertion must survive every restructure.
- **No code signing with a certificate.** Ad-hoc signing (`codesign --sign -`) only. Apple signing was dropped on 2026-09-11.
- **The overlay jobs and assets are untouched.**
- **Final asset list is exactly eight:** `refbox-windows.zip`, `refbox-macos.dmg`, `refbox-raspberry-pi.zip`, `refbox-aarch64-linux`, `refbox-aarch64-linux.sha256`, `overlay.zip`, `overlay-aarch64-linux`, `overlay-aarch64-linux.sha256`. `refbox.zip` is gone.
- **`release.yml` runs only on a `v*.*.*` tag.** No PR-time CI exercises any of this. Local verification is limited to YAML validity; the real gate is the draft-release rehearsal in Task 5.

## Note on testing

This plan has no unit tests, because a GitHub Actions workflow has none to write — there is no local runner for it and `just check` does not read it. Substituting fake tests would be worse than admitting that. Each task's verification is: the YAML still parses, the diff says what it should, and the workflow stays internally coherent (every `needs:` and every artifact name resolves). The genuine proof is Task 5.

---

### Task 1: One universal macOS job

Replaces `build-macos-arm` and `build-macos-x86` with a single `build-macos` job producing a universal `refbox.app`. `upload-release` is updated in the same task so the workflow never references an artifact that no longer exists.

**Files:**
- Modify: `.github/workflows/release.yml:24-85` (delete both macOS jobs, add one)
- Modify: `.github/workflows/release.yml:149` (`needs:` list)
- Modify: `.github/workflows/release.yml:156-163` (macOS download steps)
- Modify: `.github/workflows/release.yml:186-190` (chmod step)

**Interfaces:**
- Produces: artifact `refbox-macos` containing `refbox.app` with a universal executable.
- Consumes: nothing from other tasks.

- [ ] **Step 1: Replace lines 24-85 with the single job**

```yaml
  build-macos:
    name: Build the universal macOS app
    runs-on: macos-latest

    steps:
    - uses: actions/checkout@v5
    - uses: Swatinem/rust-cache@v2
    - name: Install cargo-bundle from crates.io
      uses: baptiste0928/cargo-install@v3
      with:
        crate: cargo-bundle
    - name: Versions
      run: cargo --version && rustc --version && cargo fmt -- --version && cargo clippy -- --version && cargo bundle --version
    - run: rustup target add x86_64-apple-darwin
    # cargo-bundle globs the `icon` path from `[package.metadata.bundle]` against
    # the PROCESS working directory, not the package root — it calls glob::glob
    # on the raw pattern with no join. Run from `refbox/` so
    # `resources/AppIcon.png` resolves; from the workspace root it matched
    # nothing and the icon was dropped SILENTLY (no warning), which is why the
    # app had no icon for five releases. The bundle still lands under the
    # workspace `target/` because cargo-bundle takes its output directory from
    # `cargo metadata`. `--format osx` produces the .app only; the .dmg is
    # assembled explicitly further down so its contents can be controlled.
    - name: Bundle the macOS app (Apple silicon)
      working-directory: refbox
      run: cargo bundle --release --target aarch64-apple-darwin --format osx
    - name: Build the Intel binary
      run: cargo build --release --target x86_64-apple-darwin -p refbox
    # Fuse the two executables into one that carries both architectures, then
    # re-sign. macOS REFUSES to launch an arm64 binary with no valid signature,
    # and rewriting the executable inside a bundle can invalidate the ad-hoc
    # signature the linker applied. Re-signing is cheap, idempotent, and the
    # alternative is a build that passes CI and dies on launch.
    - name: Fuse into a universal binary
      run: |
        APP='target/aarch64-apple-darwin/release/bundle/osx/refbox.app'
        lipo -create \
          target/aarch64-apple-darwin/release/refbox \
          target/x86_64-apple-darwin/release/refbox \
          -output "$APP/Contents/MacOS/refbox"
        codesign --force --sign - "$APP"
    - name: Verify the app is universal, signed, and has its icon
      run: |
        APP='target/aarch64-apple-darwin/release/bundle/osx/refbox.app'
        ARCHS=$(lipo -archs "$APP/Contents/MacOS/refbox")
        echo "Architectures: $ARCHS"
        echo "$ARCHS" | grep -qw arm64
        echo "$ARCHS" | grep -qw x86_64
        codesign --verify --verbose "$APP"
        ls "$APP/Contents/Resources/"*.icns
        grep -q CFBundleIconFile "$APP/Contents/Info.plist"
    - uses: actions/upload-artifact@v4
      with:
        name: refbox-macos
        path: target/aarch64-apple-darwin/release/bundle/osx/refbox.app
```

- [ ] **Step 2: Update the `needs:` list**

Change `needs: [build-windows, build-macos-arm, build-macos-x86, build-rpi, build-overlay-rpi]`
to `needs: [build-windows, build-macos, build-rpi, build-overlay-rpi]`

- [ ] **Step 3: Replace the two macOS download steps with one**

Delete both `refbox-macos-arm` / `refbox-macos-x86` download steps and insert:

```yaml
    - uses: actions/download-artifact@v5
      with:
        name: refbox-macos
        path: 'release/Mac/refbox.app'
```

- [ ] **Step 4: Update the chmod step**

```yaml
    - name: Restore executable permissions
      run: |
        chmod +x 'release/Mac/refbox.app/Contents/MacOS/refbox'
        chmod +x 'release/Raspberry Pi/refbox'
```

- [ ] **Step 5: Verify the YAML parses and no stale names remain**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml')); print('YAML OK')"
grep -n 'macos-arm\|macos-x86\|Arm processor\|Intel processor' .github/workflows/release.yml
```

Expected: `YAML OK`, and the grep prints **nothing**. Any output is a stale reference and a bug.

- [ ] **Step 6: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "feat(ci): build one universal macOS app instead of two"
```

---

### Task 2: Fetch the guides once, and assemble the .dmg on the Mac

The `.dmg` must be built on macOS, but the PDFs are fetched on Linux at the end of the run. This task moves the download into its own job so both consumers can have it, and assembles the disk image.

**Files:**
- Modify: `.github/workflows/release.yml` — add `fetch-docs` job, add `needs:` + dmg steps to `build-macos`, remove the PDF download from `upload-release`, add a docs download to `upload-release`.

**Interfaces:**
- Produces: artifact `refbox-docs` (five PDFs, flat); artifact `refbox-macos-dmg` containing `refbox-macos.dmg`.
- Consumes: the `refbox-macos` artifact from Task 1 is no longer uploaded — the `.dmg` replaces it.

- [ ] **Step 1: Add the `fetch-docs` job above `upload-release`**

```yaml
  # The guides live on Google Drive, not in this repo. They are fetched once here
  # and shared, because the macOS disk image is assembled on the macOS runner
  # while the zips are assembled on Linux — both need the same files.
  fetch-docs:
    name: Fetch the PDF guides
    runs-on: ubuntu-latest

    steps:
    - name: Download PDFs from Google Drive
      run: |
        mkdir -p docs-pdfs
        curl -L "https://drive.usercontent.google.com/download?id=1Momxds_2oyPTEZHgcuSxnTxI_HbGeaN8&export=download" -o "docs-pdfs/Getting Started Guide - English.pdf"
        curl -L "https://drive.usercontent.google.com/download?id=13eHCCmBH-IEUwIODaqpbbz_LiZQBBh0p&export=download" -o "docs-pdfs/Getting Started Guide - Spanish (Guía de Inicio).pdf"
        curl -L "https://drive.usercontent.google.com/download?id=1sJTdKHKjul6Pa7eV44APEQBFaxwQSvYy&export=download" -o "docs-pdfs/Providing App Credentials - English.pdf"
        curl -L "https://drive.usercontent.google.com/download?id=1skrUYTEYBgHs4T-dl3AobwklfJR3ZGE1&export=download" -o "docs-pdfs/Refbox User Manual.pdf"
        curl -L "https://drive.usercontent.google.com/download?id=1FLtJwc8gb5vuSRVPUNLxKhT1ufcOSa-w&export=download" -o "docs-pdfs/Refbox User Manual with Fouls.pdf"
    - uses: actions/upload-artifact@v4
      with:
        name: refbox-docs
        path: docs-pdfs
```

- [ ] **Step 2: Make `build-macos` wait for the guides**

Add to the `build-macos` job, immediately under `runs-on: macos-latest`:

```yaml
    needs: [fetch-docs]
```

- [ ] **Step 3: Add the docs download and dmg assembly to `build-macos`**

Replace the `upload-artifact` step at the end of `build-macos` with:

```yaml
    - uses: actions/download-artifact@v5
      with:
        name: refbox-docs
        path: docs-pdfs
    # `ditto` rather than `cp -R`: it is Apple's own tool for copying bundles and
    # preserves the bundle's structure and permissions exactly. The `Applications`
    # symlink is what lets someone drag the app across in the mounted window.
    - name: Assemble the disk image
      run: |
        APP='target/aarch64-apple-darwin/release/bundle/osx/refbox.app'
        mkdir -p dmg-staging/Documents
        ditto "$APP" 'dmg-staging/refbox.app'
        ln -s /Applications dmg-staging/Applications
        cp docs-pdfs/*.pdf dmg-staging/Documents/
        hdiutil create -volname 'refbox' -srcfolder dmg-staging -ov -format UDZO refbox-macos.dmg
    - name: Verify the disk image mounts and carries what it should
      run: |
        MOUNT=$(mktemp -d)
        hdiutil attach refbox-macos.dmg -mountpoint "$MOUNT" -nobrowse -readonly
        test -x "$MOUNT/refbox.app/Contents/MacOS/refbox"
        test "$(ls "$MOUNT/Documents/"*.pdf | wc -l)" -eq 5
        test -L "$MOUNT/Applications"
        hdiutil detach "$MOUNT"
    - uses: actions/upload-artifact@v4
      with:
        name: refbox-macos-dmg
        path: refbox-macos.dmg
```

- [ ] **Step 4: In `upload-release`, swap the macOS download for the dmg**

Replace the `refbox-macos` download step with:

```yaml
    - uses: actions/download-artifact@v5
      with:
        name: refbox-macos-dmg
        path: .
```

- [ ] **Step 5: In `upload-release`, remove the macOS chmod line**

The app now travels inside the disk image and never round-trips through the artifact store as loose files, so its executable bit is never stripped. Leave the Pi line.

```yaml
    - name: Restore executable permissions
      run: chmod +x 'release/Raspberry Pi/refbox'
```

- [ ] **Step 6: In `upload-release`, delete the `Download PDFs from Google Drive` step and download the artifact instead**

Place this **with the other downloads at the top of the job** — immediately after the
`refbox-rpi-sha256` download and before the overlay downloads. It must come before every step
that copies out of `docs-pdfs/`, which Task 3 adds further down. Leaving it where the old curl
step was (near the end) puts it *after* those copies and the job dies on `cp`.

```yaml
    - uses: actions/download-artifact@v5
      with:
        name: refbox-docs
        path: docs-pdfs
```

- [ ] **Step 7: Add `fetch-docs` to `upload-release`'s `needs:` list**

`needs: [build-windows, build-macos, build-rpi, build-overlay-rpi, fetch-docs]`

- [ ] **Step 8: Verify**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml')); print('YAML OK')"
grep -n 'drive.usercontent' .github/workflows/release.yml
```

Expected: `YAML OK`, and the Google Drive URLs appear **only** inside the `fetch-docs` job.

- [ ] **Step 9: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "feat(ci): assemble a macOS disk image with the guides"
```

---

### Task 3: Split into per-platform downloads

**Files:**
- Modify: `.github/workflows/release.yml` — `upload-release` packaging and asset list.

**Interfaces:**
- Produces: the final eight release assets.

- [ ] **Step 1: Repoint the Windows and Pi downloads**

```yaml
    - uses: actions/download-artifact@v5
      with:
        name: refbox-windows
        path: 'windows'
    - uses: actions/download-artifact@v5
      with:
        name: refbox-rpi
        path: 'raspberry-pi'
    - uses: actions/download-artifact@v5
      with:
        name: refbox-rpi-sha256
        path: 'raspberry-pi'
```

Delete the now-unused second `refbox-rpi` download into `pi-standalone` and the `refbox-rpi-sha256` download into `release/rpi-sha256`.

- [ ] **Step 2: Replace the refbox chmod, zip and Pi-staging steps — leave the overlay's `Package the overlay` and `Stage standalone overlay assets` steps exactly as they are**

```yaml
    # GitHub normalises artifact permissions to 0644, so the Pi binary arrives
    # unrunnable and the bit has to be restored before zipping. A zip preserves
    # it; a loose release asset cannot, because GitHub serves assets as opaque
    # HTTP blobs with no Unix mode — so whoever installs the loose binary runs
    # `chmod +x`. Expected, and documented in the release checklist.
    - name: Package the Windows download
      run: |
        mkdir -p windows/Documents
        cp docs-pdfs/*.pdf windows/Documents/
        cd windows && zip -r ../refbox-windows.zip .
    - name: Package the Raspberry Pi download
      run: |
        chmod +x raspberry-pi/refbox
        mkdir -p raspberry-pi/Documents
        cp docs-pdfs/*.pdf raspberry-pi/Documents/
        cd raspberry-pi && zip -r ../refbox-raspberry-pi.zip .
    # These two asset names are load-bearing: in-app self-update looks them up by
    # exact string (BIN_ASSET / SUM_ASSET in refbox/src/updater/release.rs).
    # Renaming either breaks updates on every Pi already in the field.
    - name: Stage standalone Pi assets
      run: |
        cp raspberry-pi/refbox refbox-aarch64-linux
        cp raspberry-pi/refbox.sha256 refbox-aarch64-linux.sha256
```

- [ ] **Step 3: Replace the release asset list**

```yaml
    - uses: softprops/action-gh-release@v2
      with:
        files: |
          refbox-windows.zip
          refbox-macos.dmg
          refbox-raspberry-pi.zip
          refbox-aarch64-linux
          refbox-aarch64-linux.sha256
          overlay.zip
          overlay-aarch64-linux
          overlay-aarch64-linux.sha256
        draft: true
        generate_release_notes: true
```

- [ ] **Step 4: Update the stale comment on the `build-overlay-rpi` job**

Line 116 says the overlay is "Deliberately NOT added to refbox.zip". Replace `refbox.zip` with `the refbox downloads`.

- [ ] **Step 5: Verify no trace of the old bundle remains**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml')); print('YAML OK')"
grep -n 'refbox\.zip\|pi-standalone\|rpi-sha256/' .github/workflows/release.yml
```

Expected: `YAML OK`, and the grep prints **nothing**. `refbox.zip` must not appear at all.

- [ ] **Step 6: Confirm the job graph and every artifact name actually resolve**

Two lists side by side cannot fail for a missing `needs:` edge — a job that downloads an artifact
from a job it does not wait for passes a name comparison and then loses the race at release time.
Assert both properties instead:

```bash
python3 - <<'EOF'
import yaml
jobs = yaml.safe_load(open('.github/workflows/release.yml'))['jobs']
for name, job in jobs.items():
    for dep in job.get('needs', []):
        assert dep in jobs, f"{name} needs a job that does not exist: {dep}"
assert 'fetch-docs' in jobs['build-macos'].get('needs', []), "build-macos must wait for fetch-docs"
uploads = {s['with']['name'] for j in jobs.values() for s in j['steps']
           if 'upload-artifact' in str(s.get('uses', ''))}
for name, job in jobs.items():
    for s in job['steps']:
        if 'download-artifact' in str(s.get('uses', '')):
            assert s['with']['name'] in uploads, f"{name} downloads an artifact nobody uploads: {s['with']['name']}"
print("job graph and artifact names resolve")
EOF
```

Expected: `job graph and artifact names resolve`. Break it on purpose once — delete
`needs: [fetch-docs]` from `build-macos` and confirm it goes red — before trusting it.

- [ ] **Step 7: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "feat(ci): ship one download per platform instead of refbox.zip"
```

---

### Task 4: Rewrite the release checklist

`docs/release-checklist.md` asserts on `refbox.zip`'s exact contents throughout and is actively wrong after Task 3.

**Files:**
- Modify: `docs/release-checklist.md` — the "How a release is built" section (lines 5-15) and everything from "Verify the draft before publishing" (line 66) to line 103. The version-bump section is untouched.

- [ ] **Step 1: Update "How a release is built"**

State that the workflow produces one download per platform: `refbox-windows.zip`, `refbox-macos.dmg` (universal — one app for both Intel and Apple-silicon Macs), and `refbox-raspberry-pi.zip`, each containing the PDF guides under `Documents/`, plus the loose Pi binary and checksum for self-update. Keep the existing paragraph about the overlay's three separate assets, changing "not inside `refbox.zip`" to "not inside any of the refbox downloads".

- [ ] **Step 2: Rewrite the verification section**

Preserve the existing discipline — every check must be able to fail, and an empty result is never a pass. Replace the `refbox.zip` checks with:

```bash
unzip -Z refbox-windows.zip | grep -E 'refbox\.exe$|Documents/.*\.pdf$'
unzip -Z refbox-raspberry-pi.zip | grep -E 'refbox$|Documents/.*\.pdf$'
```

- [ ] `refbox-windows.zip` lists `refbox.exe` at the top level plus five PDFs under `Documents/`.
- [ ] `refbox-raspberry-pi.zip` lists `refbox` starting `-rwxr-xr-x` plus five PDFs under `Documents/`. If it reads `-rw-r--r--`, do not publish.
- [ ] Mounting `refbox-macos.dmg` shows `refbox.app`, an `Applications` shortcut, and a `Documents` folder with five PDFs.
- [ ] The old `refbox.zip` is **absent**. It was retired on 2026-09-11.
- [ ] `refbox-aarch64-linux` and `refbox-aarch64-linux.sha256` are present as loose assets — self-update matches these by exact name and fails if either is renamed.
- [ ] On the Pi, Settings → App → Check Version shows the yellow **Check for Updates** button. (Unchanged — keep the existing wording and rationale.)

- [ ] **Step 3: Add the universal-binary check, which is new and can fail**

- [ ] On an **Apple-silicon** Mac, launch refbox, open Activity Monitor, find `refbox`, and read the **Kind** column. It must say **Apple**. **Intel** means the universal build silently failed and the Mac is running it under translation.
- [ ] On an **Intel** Mac, the same app launches. One machine cannot prove a universal build; both are required.

- [ ] **Step 4: Correct the unsigned-app instructions**

The app is ad-hoc signed, never notarised — Apple signing was dropped on 2026-09-11 and is not coming back unless that decision changes. macOS will warn about an unidentified developer. Clear it via **System Settings → Privacy & Security → Open Anyway**. Note explicitly that the right-click → Open shortcut is unreliable on recent macOS versions. The *different* error to watch for — *"The application 'refbox' can't be opened"* — means a missing executable bit, which the disk image is designed to prevent.

- [ ] **Step 5: Verify**

```bash
grep -n 'Arm processor\|Intel processor' docs/release-checklist.md
```

Expected: **nothing**.

- [ ] **Step 6: Commit**

```bash
git add docs/release-checklist.md
git commit -m "docs(ci): rewrite the release checklist for per-platform downloads"
```

---

### Task 5: Verification against the real release draft

**Eric's decision, 2026-09-11:** no throwaway rehearsal tag. The branch merges, and verification
happens against the draft of the next real release, which is **not published** until Mac users
confirm it works. That publish gate is what makes merging before verification safe.

A draft is safe to leave sitting: it is not public, and the in-app updater cannot see it — it
queries `/releases/latest`, which excludes drafts and pre-releases
(`refbox/src/updater/net.rs:6,16`).

- [ ] **Step 1: Merge the PR**, then cut the release as `docs/release-checklist.md` describes.
- [ ] **Step 2: Work the rewritten checklist** against the draft — the eight expected assets, the
      contents and permission bits of both zips, and the absence of `refbox.zip`.
- [ ] **Step 3: Send the file, not the link.** A draft is visible only to people with write access
      and its asset links need authentication. Eric downloads `refbox-macos.dmg` and sends it.
      Passing it through Windows or a cloud drive is the exact path that used to corrupt the old
      zip, so doing so is itself a test.
- [ ] **Step 4: Brief the testers first.** macOS will refuse the app on first launch; it is cleared
      via **System Settings → Privacy & Security → Open Anyway**, not the right-click → Open
      shortcut, which is unreliable on recent macOS versions. Without this a working build gets
      reported as broken. At least one Intel and one Apple-silicon Mac are needed between them.
- [ ] **Step 5: Testers report** — does it launch, does the icon appear, and on Apple silicon does
      Activity Monitor's **Kind** column read **Apple** rather than **Intel**.
- [ ] **Step 6: Publish only once they confirm.**

---

## Deviations

Record anything that diverged from this plan here, rather than in separate commits.

- **Declined addition (Task 2):** a check that each downloaded guide really is a PDF was proposed and **declined by Eric on 2026-09-11** as outside the approved spec. Do not re-add it. The pre-existing risk is unchanged and accepted: if Google Drive ever answers a download with an error page, the release ships a broken guide silently.
- **Carried over from the old packaging (Task 3, Step 2):** the Pi zip keeps `refbox.sha256` alongside the binary. The spec's asset table listed only the binary and the guides; dropping the checksum would have been a silent regression.
- **Commits were not made per task.** All four tasks were applied to the working tree first; splitting intermingled edits to a single workflow file into four commits afterwards carried more risk than value. Committed by file instead.
- **Code review, 2026-09-11, four findings.** Three fixed: the Pi checklist grep never matched the binary (`/refbox$` requires a folder that no longer exists); the dmg's PDF count was a hardcoded `5` and now compares guides-in against guides-out; the executable-bit comment sat above the Windows step rather than the Pi step that performs the chmod. One reported and **not** acted on: the Google Drive `curl -L` calls lack `--fail`, so an HTTP error is written into the `.pdf` and curl still exits 0. That is pre-existing behaviour moved unchanged, in the same area as the PDF-validity check Eric declined — his call, not a silent fix.
- **`zip` is not installed on this machine**, so the corrected Pi grep could not be tested locally. The regex was chosen to match whether the archive stores the entry as `refbox` or `./refbox`, removing the dependency on that untested detail. It is first exercised for real during the Task 5 rehearsal.
- **Second review, 2026-09-11, over the prose documents.** Eight findings, seven fixed: the spec's "no code signing" constraint contradicted the ad-hoc `codesign` the implementation depends on (the worst of them — building from the spec alone would have shipped an app that dies on Apple silicon); the plan placed the docs download after the steps that copy from it; "replace the chmod, zip and staging steps" read as including the overlay's; Task 3 Step 6 could not fail for the missing-`needs:` defect it claimed to catch; the spec's asset table omitted the Pi checksum; "each in its own folder" read as one folder per PDF; and both documents still described the abandoned rehearsal-tag flow.
- **Not fixed, deliberately:** nothing automated asserts that exactly five guides are present. The dmg step compares guides-in against guides-out, which proves the copy but not that Drive returned all five, and `curl` still lacks `--fail`. Restoring a count check means putting a guard back into the job where Eric declined one on 2026-09-11. The five-guide count is asserted by the human checklist instead, and this matches pre-existing behaviour.
