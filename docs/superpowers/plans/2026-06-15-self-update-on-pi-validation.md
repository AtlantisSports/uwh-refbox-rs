# Self-Update — On-Pi Validation Plan (Tests B–E)

> Local working doc. PR #1089 is now **MERGED to `master`** (with Windows fixes
> #1097/#1102), so every build and release below comes from **`master`** — not the
> old `feat/refbox/self-update` branch (deleted). The detect+install subset (the
> chosen first smoketest) is written up as a runnable companion:
> `2026-06-15-pi-smoketest-runbook.md`. Execute on a spare Pi.

**Goal:** Validate the in-app self-update end-to-end against REAL GitHub releases
on a spare Raspberry Pi before relying on it at a tournament. Uses throwaway
releases that are deleted afterward. The mechanics were already validated locally
(x86, via a temporary local-source override) — all four behaviours passed — so
this gate is specifically about the real GitHub path + real aarch64 binary + the
actual Pi launch/path/permissions.

## Prerequisites (resolve BEFORE starting)
1. **Pi launch path + writability** — confirm WHERE the Pi's refbox binary lives
   and that its **directory is writable by the refbox process** (the updater
   hard-links a backup and renames the new binary *in the same directory*). The
   auto-start command/path was asked of the system developer on 2026-06-15
   (see memory `reference_pi_deployment_launch`) — need that answer. **If the
   binary lives in a root-owned / read-only location, the swap fails with
   "not writable" and the feature can't work there** — that itself is a key
   finding to surface.
2. **Config dir writable** — the trial/rolled-back markers live in the confy
   config dir (`~/.config/refbox` on Linux). Confirm it's writable.
3. **Pi must already be running a build that CONTAINS this feature** — it
   currently runs an older release without it. See Step 0.

## Step 0 — Get a feature build onto the spare Pi
- The feature is on `master` (version 0.4.2). Get the Pi binary either from the
  re-cut **v0.4.2 release** (`refbox-aarch64-linux` asset) or by `just build-rpi`
  on `master` (same code), and copy it to the Pi's launch path the manual way.
  The Pi now runs 0.4.2-with-feature. (One-time bootstrap — once a Pi runs a
  feature build it self-updates thereafter.)

## Step 1 — Cut a throwaway v0.4.3 release (the GOOD update)
- From `master` (throwaway branch), bump `refbox/Cargo.toml` to `0.4.3`, commit,
  push tag `v0.4.3` (workflow triggers on `v*.*.*`).
- The release workflow builds and creates a **DRAFT** release with assets
  `refbox-aarch64-linux` + `refbox-aarch64-linux.sha256` (the Task-14 step).
- **PUBLISH the draft** — critical: the updater queries `/releases/latest`,
  which HIDES drafts. An unpublished draft is invisible to the Pi.

## Test B — Detect
- Pi (0.4.2): App Options → Check Version → Check for Updates.
- EXPECT: "Update available: 0.4.3".

## Test C — Install (happy path)
- Click Install → EXPECT Downloading → Checking the download → Installing →
  Restarting → reopens on **0.4.3**.
- VERIFY: version reads 0.4.3 and STAYS (no auto-revert); a `refbox-v0.4.2.bak`
  exists next to the binary; after ~20s the trial marker clears (proven healthy).

## Test D — Manual revert
- On 0.4.3: App Options → Check Version → "Revert to Previous Version (0.4.2)"
  → Revert → EXPECT restart back to **0.4.2**; backup consumed.

## Test E — Auto-revert of a BAD build
- Cut a SECOND throwaway release (e.g. `v0.4.4`) from a branch where the binary
  **passes `--self-check` but FAILS to run healthily** — e.g. add a `panic!` or
  `std::process::exit(1)` in the normal UI-startup path, placed AFTER the
  `--self-check` early-return (so the smoke test still passes). Publish it.
- On the Pi: Check → install v0.4.4 → it swaps + restarts → the bad build
  crashes on real startup → **power-cycle the Pi** → EXPECT: auto-reverts to the
  prior version and opens on the Updates page with *"Reverted to the previous
  version because the update didn't start correctly, please try again."*
- Note: the Pi only restarts via power-cycle, so auto-revert happens on the next
  power-up after the bad build fails.

## Cleanup
- DELETE the throwaway GitHub releases (v0.4.3, v0.4.4) AND their git tags.
- Remove any leftover `refbox-v*.bak` / markers on the Pi.
- Restore the Pi to its intended released build.

## Key gotchas
- Draft releases are invisible to the updater — must publish.
- The standalone-asset workflow step is on `master` now (merged via #1089; its
  download-path bug fixed in #1110), so releases cut from `master` include
  `refbox-aarch64-linux` + `.sha256`.
- For Test E the bad build must pass `--self-check` yet fail real startup.
- Swap needs the binary's directory writable by the refbox process (Prereq 1).
