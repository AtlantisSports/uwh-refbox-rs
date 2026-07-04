# Pi Self-Update Smoketest — Detect + Install Runbook

> Local working doc (not committed to a branch). Companion to
> `2026-06-15-self-update-on-pi-validation.md`. Scope chosen 2026-06-15:
> **detect + install only** (plan Step 0 + Tests B & C). Manual revert (D) and
> auto-revert-of-a-bad-build (E) are deliberately out of scope here — see that
> plan when you want the full safety-net validation.

## What this proves
On the spare Pi, the built-in updater can **detect** a newer published release
and **install** it — landing on the new version and leaving a backup of the old
one. This is the core happy path of self-update on real hardware.

## Important: build from `master`, not the old feature branch
The self-update feature is now merged to `master` (PR #1089), along with the two
Windows fixes (#1097, #1102). The old `feat/refbox/self-update` branch was
deleted and is **stale** (it lacks the #1102 fix). Everything below builds and
releases from **`master`**. v0.4.2 has already been re-cut from `master` as the
real release (currently an unpublished draft); the baseline deployed in A1–A2 can
be that release's `refbox-aarch64-linux` asset, or a local `just build-rpi` (same code).

---

## Before you start — prerequisites

1. **[OPEN BLOCKER] Pi program path + writability.** We need to know:
   - **`<PI_REFBOX_PATH>`** — the exact location of the refbox program file on the Pi, and
   - that **the refbox program can write to that folder** (the install saves a
     backup and drops the new program *into that same folder*).
   This is the question sent to the system developer on 2026-06-15. **If that
   folder is read-only to refbox, the Install step fails with "not writable" —
   and that result is itself the headline finding** (the feature can't self-update
   as the Pi is currently set up). Fill `<PI_REFBOX_PATH>` in below once known.
2. **Dev machine:** `cross` + Docker running (needed by `just build-rpi`), and
   `gh` logged in to GitHub.
3. **Pi access:** a way to copy a file onto the Pi (scp / USB stick) and to
   power-cycle it (its only stop/start).

---

## Part A — Dev machine  *(Claude runs A1/A3 on your OK; A2 is physical/you)*

### A1 — Build the baseline feature build for the Pi (version 0.4.2)
From a clean `master` checkout:
```
just build-rpi
```
Produces the Pi binary at:
```
target/aarch64-unknown-linux-gnu/release/refbox
```
This is **0.4.2 with the self-update feature** (and the Windows fixes). It is the
binary the Pi must run *first*, because the Pi currently runs an older release
that has no updater in it.

### A2 — Put that baseline build on the Pi  *(physical step)*
- Back up whatever is currently at `<PI_REFBOX_PATH>` (rename it aside).
- Copy the new `refbox` binary to `<PI_REFBOX_PATH>` and make it executable
  (`chmod +x <PI_REFBOX_PATH>`).
- Start refbox on the Pi (power-cycle / however it auto-starts) and confirm it
  **opens and runs a game normally**. The Pi is now on **0.4.2-with-feature**.

### A3 — Cut and PUBLISH a throwaway v0.4.3 release
This is the "newer version" the Pi will detect. Done on a throwaway branch so
`master` is untouched:
```
git switch -c throwaway/v0-4-3 origin/master
# bump the version so the built binary reports 0.4.3:
#   refbox/Cargo.toml  ->  version = "0.4.3"
#   (Cargo.lock updates to match)
git commit -am "chore(refbox): throwaway v0.4.3 for Pi smoketest"
git tag v0.4.3
git push origin v0.4.3            # pushing the TAG triggers the release workflow
```
The workflow builds `refbox-aarch64-linux` (0.4.3) + `refbox-aarch64-linux.sha256`
and creates a **DRAFT** release.
- **PUBLISH the draft release.** This is critical: the updater reads
  `/releases/latest`, which **hides drafts**. An unpublished draft is invisible to
  the Pi. (The currently-published "latest" is v0.4.1; the parked **v0.4.2 draft**
  stays hidden and does not interfere — leave it alone.)

---

## Part B — On the Pi  *(you do these)*

### Test B — Detect
- App Options → **Check Version** → **Check for Updates**.
- **EXPECT:** "Update available: 0.4.3".

### Test C — Install (happy path)
- Click **Install** (bottom-right confirm button).
- **EXPECT the sequence:** Downloading → Checking the download → Installing →
  Restarting → refbox reopens on **0.4.3**.
- **VERIFY all three:**
  1. The version now reads **0.4.3** and **stays** there (it does not bounce back).
  2. A backup of the old program named **`refbox-v0.4.2.bak`** sits next to the
     binary at `<PI_REFBOX_PATH>`.
  3. After ~20 seconds of healthy running the update is considered "proven" (the
     internal trial marker clears) — i.e. it will not auto-revert on the next boot.

---

## Part C — Cleanup  *(Claude runs the GitHub parts on your OK; Pi parts are physical)*

- **GitHub:** delete the throwaway **v0.4.3 release**, the **v0.4.3 git tag**, and
  the **`throwaway/v0-4-3` branch**.
- **Pi:** remove the leftover `refbox-v0.4.2.bak` (and any update marker files in
  the refbox config folder, e.g. `~/.config/refbox/update_*.marker`), then restore
  the Pi to its intended released build.

---

## After the smoketest passes — ship v0.4.2 for real
- **Publish the v0.4.2 draft release** (the validated build). It becomes the new
  public "latest"; from here a Pi running a feature build self-updates to it.
- The throwaway v0.4.3 is deleted in Part C. Game Block ships in a later release
  (reuses v0.4.3 or goes to v0.4.4).

---

## If something goes wrong
- **Install fails "not writable" / "Access is denied":** `<PI_REFBOX_PATH>`'s
  folder is read-only to refbox. **Stop — this is the key finding.** The updater
  needs a writable install folder; report it before going further.
- **Check shows no update (stays "up to date"):** the v0.4.3 release is probably
  still a **draft** — publish it. Also confirm the tag pushed and the workflow
  finished building the assets.
- **Download/checksum error:** confirm both `refbox-aarch64-linux` **and**
  `refbox-aarch64-linux.sha256` are attached to the published release.
- **Reopens on 0.4.2 (not 0.4.3) after restart:** the new binary didn't launch
  healthily and auto-reverted — capture the on-screen message and the Pi logs.
