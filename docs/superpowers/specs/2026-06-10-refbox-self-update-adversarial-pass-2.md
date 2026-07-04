# Self-Update & Restart — Adversarial Pass 2 (post-merge)

**Date:** 2026-06-10
**Subject:** Fresh adversarial analysis of the merged restart fix (PR #1073) and the self-update
design, run after PR #1073 merged. Goal: find what the first review (49 findings) missed.
**Verified against:** the actual merged code on `origin/master` and the four 2026-06-10 design
artifacts.

---

## State verification (do not trust memory)

- PR #1073 (`fix/refbox/restart-preserve-cli-args`) is **MERGED** into master (4 commits, 2 files).
- The session memory and the plan both described it as "gated on the spare-Pi smoke test."
  **Whether Test A of the hardware script was actually run before merge is unconfirmed** — this is
  a question only the operator can answer, and it determines whether the design's own gate has
  been satisfied.

## What was checked and holds (credit where due)

- `build_restart_argv` covers **all 18** CLI fields: 15 replayed, 3 deliberately and correctly
  excluded (`--language`, `--is-simulator`, `--capture-previews`). No missed flag today.
- Replaying `--baud-rate` only alongside `--serial-port` is correct (meaningless without it).
- Spawn failure is now logged, stdin nulled; tests are meaningful (incl. the
  never-replay test and the unopenable-port test).
- The descoping of retry/backoff + SoundController work was **recorded with reasoning** in the
  plan ("Deferred: hardware-validated resilience") — a documented deferral, not a process miss.

---

## New findings (ranked by tournament impact)

### A. After a rename-swap, the merged restart block relaunches the BACKUP, not the new version (HIGH — design-level, must fix before implementation)

On Linux, the kernel's record of "which file is this program running from" (`/proc/self/exe`,
what `std::env::current_exe()` reads) **follows the file when it is renamed**. The designed swap
renames the running binary to `refbox-v{prev}.bak` and renames the new file into place. At
respawn time the merged code calls `current_exe()` — which now points at the **`.bak` file** —
so the "restart onto the new version" deterministically restarts the **old** version, while the
UI told the operator the update succeeded. The Updates page would then show the old version
again (confusing but honest), and the backup file is simultaneously the running binary (so a
subsequent "Revert" reasoning gets tangled).

The design already says "capture the install path before swapping … never re-resolve after"
(§5.6) and separately says the updater and the restart "converge on one audited relaunch
routine" (§7). **Those two statements conflict as written**, because today's audited relaunch
routine re-resolves via `current_exe()` at spawn time.

**Design amendment:** the single relaunch routine must take the executable path as an explicit
parameter. Plain restart passes `current_exe()`; the updater passes the canonicalized install
path captured before the swap. This also analytically answers open question §9.2 ("does the
relaunch start the new binary after a rename-based swap?") — as designed today: **no**.

**Empirically confirmed (2026-06-10, WSL2 Linux 6.6):** a live process's `/proc/self/exe` was
observed to follow the rename — after `mv prog prog-v0.4.1.bak; mv new prog`, the running
process's exe link reads `…/prog-v0.4.1.bak`. With the hard-link backup variant (finding B), it
instead reads `…/prog (deleted)` — i.e. a `current_exe()`-based respawn launches the OLD version
under the rename scheme and FAILS OUTRIGHT under the hard-link scheme. Either way the explicit
path parameter is mandatory; the hard-link scheme at least fails loudly. Re-confirm on the Pi
during hardware testing, but the Linux semantics are now observed, not assumed.

### B. The two-rename swap has a moment with NO program at the install path (MEDIUM-HIGH — contradicts acceptance criterion E)

§5.6 swaps as: rename(current → backup), then rename(new → install path). Between those two
steps the install path is **empty**. A power cut in that window leaves a Pi that boots with no
refbox at all — exactly the "never fail to start" case Test E promises to prevent. The window is
tiny, but the acceptance criterion says "always," and SD-card power cuts at venues are real.

**Design amendment:** make the backup a **hard link** (or copy) instead of a rename:
1. `link(current → refbox-v{prev}.bak)` — the same file now has two names; nothing moved.
2. `rename(new → install path)` — atomic replace; the old file survives under the `.bak` name.
There is then **no instant** at which the install path lacks a runnable program. Same directory,
same filesystem, still one-deep, no extra disk space. (ETXTBSY does not apply — rename-replace
of a running binary is legal; only write-in-place is not.)

Note: with this scheme `current_exe()` in the running old process shows the install path as
"(deleted)"-suffixed after the swap — which would make a `current_exe()`-based respawn **fail
outright** rather than launch the backup. Finding A's fix (explicit path parameter) handles both
schemes.

### C. Replayed argv becomes a cross-version compatibility contract (MEDIUM)

The restart replays the **old** binary's arguments into the **new** binary. If any future
release renames or removes a CLI flag, the post-update relaunch dies at argument parsing — the
app simply never comes back, after telling the operator the update worked. The planned
`--self-check` smoke test does not catch this, because it doesn't use the replayed argv.

**Design amendments:**
1. Treat the CLI flag set as a **stable interface**: flags may be added, never removed/renamed
   (or old ones kept as hidden aliases).
2. Run the smoke test as `new-binary --self-check <the actual restart argv>` — one invocation
   then proves both "binary runs on this Pi" and "binary accepts the argv it will be restarted
   with."

### D. The silent dark-scoreboard chain, and a hole in the hardware test (MEDIUM)

Three merged/observed facts combine:
1. Serial-port acquisition is now **skip-and-log** (no retry — deferral recorded).
2. The sound system's shutdown does a **blocking wait** inside teardown, and teardown runs
   **before** the respawn line — so a slow old-process exit lengthens any overlap, and a hung
   sound shutdown means **the respawn never fires at all** (old window closes, nothing returns).
3. TCP listeners also degrade silently if the old process still holds the ports.

If the overlap race is ever lost, the new instance comes up looking normal but with a **dark
scoreboard** (and possibly no overlay connectivity), with the only evidence one log line. The
prior review knew about the race; what's new is that the failure mode changed from *crash*
(loud) to *silent degradation*, and that **a race is intermittent — the hardware script runs the
restart exactly once** (Test A, step 5). One clean pass proves little about a race.

**Test-script amendment:** run Test A's restart **five times consecutively**, checking
scoreboard + buzzer each time. Add to Test A's failure triage: "if the old window closes and no
new window ever appears, suspect the sound-system shutdown hang first."
**Design note:** the deferred retry/backoff should be treated as *expected* work, not
contingency — silent panel loss at a tournament is the exact failure class this feature exists
to avoid.

### E. Nothing forces future flags into the restart argv (LOW-MEDIUM — drift inoculation)

`build_restart_argv` reads fields by name; adding a new CLI flag tomorrow compiles cleanly while
silently omitting it from restarts — which is precisely the original bug, waiting to re-happen.

**Code amendment (one line of shape):** exhaustively destructure the parsed args at the top of
`build_restart_argv` (`let Cli { no_simulate, verbose, … } = args;` with no `..`). Any future
field then breaks the build until the author decides replay-or-exclude, with the doc comment
explaining the choice.

### F. Smaller design gaps (LOW)

- **Writable install dir pre-flight:** if the binary lives somewhere the app user can't write
  (e.g. root-owned `/usr/local/bin`), the swap fails at rename time. Cheap to check in §5.2
  pre-flight alongside disk space, with a specific error message.
- **Captive-portal Wi-Fi:** venue networks often answer any HTTPS request with a login page. The
  version check must fail gracefully on non-JSON 200 responses (message: "couldn't reach the
  update server"), not just on network errors.
- **Draft/pre-release releases:** GitHub's "latest release" API excludes drafts and pre-releases;
  the current release being a draft means the feature sees nothing until a real release exists
  (already noted in the first review; re-confirmed it also shapes *testing* — the spare-Pi test
  needs a real published release or a URL override).
- **Auto-revert false trigger:** a power cut after the new version is healthy but before the
  trial-marker clear is flushed to the SD card would revert a healthy update on next boot.
  Acceptable (it fails safe), but flush the marker-clear (`fsync`) to minimize it.

---

## Effect on the confidence claim

The first review's bottom line — worst case is "the button didn't work, update manually" — is
**slightly overstated as the design stands**, because findings A–C each describe a path where
the worst case is worse: A = silent non-update (operator believes Pi updated when it didn't),
B = a Pi that doesn't boot into refbox after an unlucky power cut, C = an app that never comes
back after an otherwise-successful future update. All three have cheap, concrete fixes listed
above; with them (plus the repeated-restart hardware test), the original claim becomes accurate.

None of this requires reopening the merged PR #1073: findings A–C are design-doc amendments for
the unbuilt updater; D's code half was already a recorded deferral; E is a small hardening
follow-up.

## Open question for the operator

Was hardware Test A (the restart test on the spare Pi) run before PR #1073 merged? If not, it is
still the gate for relying on restart at an event — merged ≠ tournament-tested.
