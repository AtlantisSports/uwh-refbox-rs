# Self-Update Review — Confidence Report

**Date:** 2026-06-10
**Subject:** Confidence in the proposed Raspberry-Pi self-update feature, and in the existing
restart mechanism it would build on.
**Audience:** Tournament organiser (non-programmer). Plain English throughout.

---

## How this review was done

An exhaustive multi-agent code review was run over (a) the existing in-app restart mechanism and
(b) the proposed self-update design. Seven independent reviewers each took a different angle
(restart correctness, Linux binary-replace behaviour, hardware hand-off, brick scenarios,
game-state/config persistence, download/verification, and a completeness sweep), producing **49
findings**. An adversarial verification pass then began checking each finding.

**Honesty note:** the review run was interrupted partway through the verification/synthesis step
when VS Code shut down. The 49 findings were fully recovered from the saved run journal. The
automatic "is this real?" verdicts were only partially recorded and could not be cleanly matched
back to findings, so **every finding below was judged on its own merits against the actual code**,
not on the interrupted auto-verdicts. The single most important finding was re-verified by hand
directly in the source.

---

## Overall confidence: **moderate, and conditional**

The *design* (operator-initiated, Pi-only, disabled during a game, verify-before-swap, one-deep
revert, manual SD fallback) is sound. But the review found that the **foundation it stands on —
the app's restart mechanism — has a real, confirmed bug**, and that the file-swap step has several
strict rules that must be followed exactly or a Pi could be left in a bad state.

The honest summary: **this is buildable at good confidence, but only after (1) a prerequisite fix
to the restart, (2) the must-fix hardening list below is built in, and (3) a hands-on test on a
real spare Pi.** Without those, confidence is low. With them, the design's layered safety nets
(verify, smoke-test, keep-a-backup, revert, manual SD fallback) make the worst realistic case
"the new button didn't work, keep updating manually" — not a bricked tournament.

---

## The one confirmed bug (verified by hand) — fix this first

**When the refbox restarts itself, it relaunches with none of its start-up settings.**

- The restart runs the program with **no command-line options** (`Command::new(exe).spawn()` at
  [refbox/src/main.rs:520](../../../refbox/src/main.rs#L520)), whereas the original was started
  *with* options. Compare the simulator spawn right above it, which deliberately rebuilds its
  options ([refbox/src/main.rs:214-222](../../../refbox/src/main.rs#L214-L222)).
- On a Pi started for a tournament (full-screen, scoreboard serial port, real-hardware mode), a
  restart would bring the app back **windowed, with no LED-panel connection, and in simulation
  mode** — with log files going to a different place too.
- This already affects the **shipped** in-app restarts (switching app mode, switching language).
  Because the restart has never run at a real event, it is almost certainly latent right now.

**Decision (agreed):** fix this as a standalone bug on its own branch, hardware-tested, *before*
self-update is built on top of it.

---

## Must-fix-before-build (hardening the design)

Grouped by theme. These are decisions to lock in before any code is written.

### Restart foundation
- Relaunch must **carry forward the original start-up settings** (the bug above).
- A **failed relaunch is currently silent** — if the new copy fails to start, there is no running
  app and nothing logged ([main.rs:519-520](../../../refbox/src/main.rs#L519-L520)). The update
  path must detect and report this.
- The new copy and the old one can briefly **fight over the scoreboard port and the network
  ports**; the scoreboard port is opened with a hard `.unwrap()` that **crashes** the app if the
  port is still held. Restart must ensure the old copy has fully released hardware first.

### File-swap rules (Linux/Pi specifics)
- You **cannot overwrite a running program in place** on Linux (it errors, "ETXTBSY"). The swap
  **must be a rename**, not an overwrite.
- The downloaded file **must be marked runnable** (executable bit) before the swap.
- The install path must be **captured before the swap**, and the swap must be atomic (write to a
  temp file on the *same* disk, then rename into place) so a power cut can never leave the Pi with
  a half-written or missing program.
- Check there is **enough free space** before downloading/backing up, so a full SD card can't
  produce a truncated binary with no backup.

### The "disabled during a game" gate
- The gate's definition of "in a game" must include **half-time and other break periods** and
  **active timeouts / score-review states**, not just live play.
- Close the **gap between "Check" and "Install"**: a game could start in between, so re-check the
  game state at the moment of install.

### Download & verification
- The version check **requires a "User-Agent" header** or GitHub will reject it outright.
- A plain checksum proves the file is **intact**, not that it is **genuine**; if the release
  account were ever compromised, a checksum wouldn't catch it. Decide whether a cryptographic
  signature is warranted (recommended to at least record this as a known limitation).
- Release downloads **redirect to a different host** — verification must follow the redirect
  safely and not leak any credentials across it.
- The release currently publishes **no standalone Pi file or checksum** (and is a draft) — the
  pipeline change must ship before the feature has anything to fetch.

### Operability
- The app **does not show or log its own version** — add this so an operator can confirm an update
  actually took effect.
- There is **no safe way to "test-launch" the new file** without popping a stray window and
  grabbing hardware — the smoke-test needs a dedicated, headless self-check mode.
- After a restart, **nothing confirms the scoreboard/hardware came back** — add an observable
  check.

---

## What only a real spare Pi can settle

- Does the relaunched app actually return to full-screen and reconnect the scoreboard once the
  restart carries settings forward?
- Does the old copy release the scoreboard serial port and network ports before the new copy grabs
  them, or is there a crash-inducing race?
- Does the app exit cleanly when its window closes on the Pi (the sound system does a blocking
  shut-down step that is untested on hardware)?
- After a rename-based swap, does the relaunch start the **new** program (not a stale/"deleted"
  reference)?
- How exactly is the Pi launched (by a background service, or by hand)? This determines whether a
  failed relaunch self-recovers — and it is the one fact not recorded anywhere in the code.

---

## Bottom line

The review did its job: instead of rubber-stamping, it found a real defect in the restart
foundation and produced a concrete hardening list. The path to a confident release is clear —
fix the restart bug first, build the hardening in, and prove it on a spare Pi — and every failure
path is designed to leave you no worse off than today's manual method.
