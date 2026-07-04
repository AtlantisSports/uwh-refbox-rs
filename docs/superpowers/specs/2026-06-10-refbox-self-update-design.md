# Refbox Raspberry-Pi Self-Update — Design Spec

**Date:** 2026-06-10
**Status:** Design — awaiting user review (no implementation until approved; the prerequisite fix
in §3 lands first, on its own branch)
**Related artifacts:**
- Confidence report: `2026-06-10-refbox-self-update-review-confidence-report.md`
- Spare-Pi hardware test script: `2026-06-10-refbox-self-update-hardware-test-script.md`
- External reviewer briefing: `2026-06-10-refbox-self-update-external-review-package.md`
- Prerequisite bug memory: `project_restart_drops_cli_args_bug`

---

## 1. Goal

Let an operator update the refbox on a Raspberry Pi **from inside the app, over the network**,
without manually copying a binary or pulling the SD card. Operator-initiated, with confirmation,
and safe against the failure modes a tournament can throw at it.

## 2. Scope boundary

**In scope (crate `refbox`):**
- A new in-app Updates page + the entry button on the App Options page.
- An updater module (version check, download, verify, smoke-test, atomic swap, revert).
- Exposing/logging the app version; a hidden `--self-check` (smoke-test) mode.
- Hardening the existing restart path (see §3, done first as a separate branch).

**In scope (CI):** an **additive** change to `.github/workflows/release.yml` to also publish the
standalone aarch64 Pi binary + a checksum file (the combined all-platform zip is unchanged).

**Out of scope (explicitly):**
- Laptops (Windows/Mac) — they keep the manual download method.
- `wireless-remote` firmware — untouched.
- Cryptographic signatures (checksum only for v1; signature recorded as a future option).
- Keeping more than one previous version (one-deep revert only).
- Automatic background update checks (operator-initiated only; status starts "Unknown" each
  session).

## 3. Prerequisite (separate branch, lands first): fix the restart

The self-update relies on the app's existing restart, which has a confirmed bug:
`refbox/src/main.rs:520` does `Command::new(exe).spawn()` with **no arguments**, so a restart
relaunches with clap defaults — dropping `--fullscreen`, `--serial-port`, `--no-simulate`,
`--binary-port`/`--json-port`, `--log-location`, `--language`, `--baud-rate`, etc. On a tournament
Pi a restart would return windowed, with no LED panel, in simulation mode. This already affects the
shipped mode/language restarts.

**Prerequisite fix (own branch, hardware-tested before self-update builds on it):**
1. Reconstruct the original start-up arguments on relaunch (extend the existing `build_sim_argv`
   pattern at `main.rs:185-222`, or capture and replay the parsed args).
2. Stop silently swallowing a failed relaunch (`let _ = ...spawn()`): detect and log failure.
3. Make hardware acquisition on startup resilient to a brief overlap with the exiting old process —
   the serial-port open must not `.unwrap()`-panic if the port is momentarily still held; retry
   with short backoff (≈100 ms → 1 s, bounded) for serial and TCP binds; explicitly release
   hardware in the old process before restart (and bound/avoid the blocking call in
   `SoundController::drop`).

This fix is independently valuable (it repairs today's mode/language restarts) and is a hard
precondition for everything below.

## 4. User experience

### Entry point
- The **App Options** page gains a **"Check Version"** button placed **between CANCEL and APPLY**.
- It is **disabled (greyed)** while a game is in progress (the gate — see §6). It opens the Updates
  page. Unsaved App Options edits are **preserved** (not applied, not discarded) across the visit.

### Updates page
- Shows **"Current version: X"**.
- One **yellow** button labelled **"Check for Updates"**. Page **status starts at "Unknown"** each
  session.
- A lower-corner button: **"Back"** when idle; relabels to **"Cancel"** while an update is in
  progress; **disabled** during the final swap+restart.

### States
- Press **Check for Updates** → status becomes **"Up to date."** or **"Update available: Y"**, and
  the yellow button transforms in place into **"Install Update"**.
- Press **Install Update** → final confirm **"This will restart the refbox. Continue?"** → progress
  (*Downloading… / Checking the download… / Installing… / Restarting…*).
- A **"Revert to Previous Version (X)"** button appears whenever a one-deep backup exists; same
  confirm + restart behaviour.
- Restart lands on the normal **main page** (startup screen), now on the new (or reverted) version.
  No startup-sequence change; no post-update banner (kept silent by decision).

### On a laptop (non-Pi)
- The page shows the current version but replaces the yellow button with: *"Automatic updates are
  available on the Raspberry Pi. On this computer, download updates from the releases page."*

## 5. Update flow (under the hood)

Operator-confirmed install, in order; any failure before the swap leaves the running program
untouched:

1. **Check** — query the latest GitHub release for `AtlantisSports/uwh-refbox-rs` (with a
   `User-Agent` header — required), compare to the running version.
2. **Pre-flight** — confirm enough free disk space; re-check the game gate (close the
   check→install gap).
3. **Download** the standalone aarch64 binary to a temp file **on the same filesystem** as the
   install path.
4. **Verify** the published checksum; abort and change nothing on mismatch. Follow the release
   asset redirect (`objects.githubusercontent.com`) over HTTPS without leaking any auth header.
5. **Smoke-test** — mark the temp file executable and run it in a hidden `--self-check` mode that
   initialises and exits **without** opening a window, spawning a sim child, or grabbing hardware.
   (v1 may start with a trivial `--version` check and grow to a fuller self-check.)
6. **Atomic swap** — capture the real install path **before** swapping (canonicalize, so a symlink
   or PATH launch resolves to the true file; never re-resolve after). Rename the current binary to a
   one-deep backup, then rename the new file into place. Rename only — never overwrite-in-place
   (ETXTBSY). The backup lives **in the same directory** as the binary (same filesystem is required
   for an atomic rename) and is **named with the previous version** (e.g. `refbox-v{prev}.bak`) so
   the Revert button can display the version it would return to.
7. **Restart** via the (now-hardened) restart path, carrying the original start-up arguments.
8. **Auto-revert safety net** (see §6).

## 6. Safety model

Layered, outermost-first:

- **Game gate** — the whole feature is disabled while a game is in progress. "In progress" must
  include half-time and other break periods and active timeouts/score-review states, not just live
  play. Re-checked at the moment of install.
- **Verify-before-swap** — checksum + smoke-test must pass before anything on disk changes.
- **One-deep backup** — the previous version is kept as the revert target (same directory,
  version-named per §5), replaced only by the next update's backup. Still one-deep — the version in
  the name is for display/inspection, not version history.
- **Manual revert** — operator-pressed "Revert to Previous Version (X)".
- **Best-effort auto-revert** *(new)* — if the updated app does not reach a healthy running state
  within a short window, the previous version is restored automatically, without a person present.
  Mechanism: the post-swap launch writes a "trial" marker; reaching healthy-running clears it; a
  launch that finds an uncleared prior trial restores the backup and relaunches. **Honest
  limitation:** a purely in-app marker only catches crashes that occur *after* the check-point; a
  fully robust version depends on how the Pi is launched (service vs. by hand) — to be confirmed
  during spare-Pi testing, and the robust layer added there if the launch setup supports it. The
  exact marker storage location and the precise "healthy" definition (reached the main page with
  hardware initialised, vs. a timeout) are finalised together with the launch-method question (§9),
  since they interact with what survives a power-loss reboot.
- **Manual SD-card fallback** — unchanged; the ultimate floor if everything else fails.

Net worst case: "the update button didn't work; keep updating manually as today" — never a stranded
tournament.

## 7. Architecture sketch

- **`refbox/src/updater/`** (new module) — version check, download, checksum verify, smoke-test
  invocation, atomic swap, backup/revert, auto-revert marker logic. HTTP via `reqwest` (already
  used by the portal client) on `tokio`. Consider the `self_replace` crate for the rename dance
  (a new dependency — needs sign-off; recorded as an option, not a decision).
- **Version source** — `env!("CARGO_PKG_VERSION")` (currently 0.4.1). Logged at startup and shown
  on the Updates page. Add semver-aware comparison.
- **CLI** — a hidden `--self-check` flag (and rely on existing `--version`) in
  `refbox/src/main.rs` clap `Cli`.
- **Restart path** — the §3-hardened `main.rs` relaunch (args carried forward, failure logged,
  resilient acquisition). The self-update and the existing restart converge on **one** audited
  relaunch routine.
- **UI** — new `ConfigPage::Updates` + the "Check Version" button on `make_app_config_page()` in
  `refbox/src/app/view_builders/configuration.rs`, using existing button helpers
  (`make_button`/`make_value_button`); new `Message` variants for check/install/revert/progress.
- **Game-state gate** — a single clear predicate read from the tournament-manager state the app
  already holds, covering live play, half-time/breaks, timeouts, and score-review.
- **Release pipeline** — `.github/workflows/release.yml`: additionally upload the standalone Pi
  binary and a checksum file as release assets (the binary also stays inside the combined zip).
- **Error messages** are specific and actionable, e.g. *"Couldn't reach the update server, please
  check your internet connection"*, *"The downloaded update wasn't valid and was not installed"*,
  *"Update server is busy, please try again later"* (rate limit), *"Not enough free space to update"*.

**Captured for the implementation plan (not design-level):** semver-aware version comparison
crate; the `User-Agent` header value; the `reqwest` redirect policy that drops auth across hosts;
download-progress reporting via an iced command; bounded retry/backoff numbers; the `--self-check`
tiering (start at `--version`); CI asset naming; and the `self_replace`-vs-hand-rolled decision
(a dependency choice needing human sign-off).

## 8. Acceptance criteria (operator-observable)

Tied to the spare-Pi hardware test script:
- After the §3 fix, a language/mode change restart returns **full-screen, with the scoreboard and
  buzzer working**, logs unchanged.
- Check finds the newer version; Install downloads, verifies, restarts onto the new version
  (full-screen, scoreboard, buzzer intact); the version display confirms it.
- Revert returns to the previous version.
- No internet → *"Couldn't reach the update server, please check your internet connection"*; app
  unchanged.
- The Check Version button is disabled during a game, a break/half-time, and a timeout.
- Cancel mid-download returns to *Update available* with nothing changed.
- Power loss during install → the Pi always boots a working refbox (new or previous).
- Automated tests pass for: version comparison, checksum verify (good + tampered), platform/asset
  selection, and the error messages.

## 9. Open questions to settle on real hardware

- How is the Pi launched (background service vs. by hand)? Determines how robust auto-revert can be.
- Does the relaunch reliably start the new binary after a rename-based swap?
- Does the old process release the serial/SPI/TCP/audio resources before the new one acquires them,
  or is retry/backoff doing the work?
- Does the app exit cleanly when its window closes on the Pi (sound-controller blocking drop)?

## 10. Deferred / not now

Laptops; cryptographic signature; multi-version backup; automatic startup update checks; a
post-update confirmation banner; a release-notes/changelog snippet on the Updates page. All recorded
as possible later additions.
