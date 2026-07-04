# Refbox Self-Update — Phased Design (Restart Resilience + Self-Update Backend)

**Date:** 2026-06-15
**Status:** Design — approved in brainstorming; awaiting user review of this written spec before
writing the Phase 1 implementation plan.
**Crate:** `refbox` (Phase 1 + Phase 2 UI/backend); additive `.github/workflows/release.yml` change (Phase 2 only).

**Consolidates / supersedes for sequencing purposes:**
- `2026-06-10-refbox-self-update-design.md` (full feature design — still the reference for Phase 2 UX detail)
- `2026-06-10-refbox-self-update-adversarial-pass-2.md` (findings A–F — folded in below)
- `2026-06-10-refbox-self-update-hardware-test-script.md` (the spare-Pi Test A–E gate)
- `reference_pi_deployment_launch` memory (how the Pi boots refbox; mostly still pending the developer)

**Local working doc:** per project convention (`reference_plan_docs_not_committed`), this spec is NOT
committed to a branch or included in any PR.

---

## 1. Goal and phasing

Let an operator update the refbox on a Raspberry Pi **from inside the app, over the network**,
without copying a binary by hand or pulling the SD card. Operator-initiated, confirmed, and safe
against tournament failure modes.

The work splits along a hard dependency line — **whether it needs the Pi's boot script and a
physical Pi to design/validate**:

- **Phase 1 — Restart Resilience Hardening (build now).** Makes the *existing* restart bulletproof.
  Pi-independent to write; improves the restart that already ships today (language/mode change).
  Own branch, own PR, releasable now, **Pi-validated before publish**.
- **Phase 2 — Self-Update Backend (design-only now).** The in-app check/download/install/revert
  feature. Fully designed here with the pass-2 fixes folded in, but **blocked** on the boot script
  (for auto-revert) and a physical Pi (for validation). Gets its own implementation plan when the
  developer answers the launch question.

The UI for Phase 2 is already prototyped and user-approved (worktree `feat/refbox/self-update-ui`,
commit `890a6562`); Phase 2 wires a real backend behind that approved UI.

### Baseline / branch point
Phase 1 branches from **`origin/master`** (post-PR #1073), which already contains
`build_restart_argv` (`refbox/src/main.rs:230`) and the relaunch spawn (`~main.rs:602`). The current
working branch `feat/refbox/time-golden-trace-spike` predates #1073 and is unrelated — do **not**
branch Phase 1 from it. Fetch master first (`reference`: `feedback_fetch_master_before_branching`).

---

## 2. Phase 1 — Restart Resilience Hardening (the buildable-now spec)

### 2.1 What already exists (PR #1073, on master — do not redo)
- `build_restart_argv(&Cli)` replays all 18 CLI fields correctly (15 replayed, 3 excluded:
  `--language`, `--is-simulator`, `--capture-previews`).
- The relaunch logs spawn failure (no longer `let _ =`) and nulls stdin.
- Tests cover the never-replay fields and the unopenable-port case.

So Phase 1 is the **remaining** hardening on top of #1073, not the flag-replay itself.

### 2.2 The three changes

**Change 1 — Dark-scoreboard retry (pass-2 finding D, code half).**
When the new copy starts during a restart, the old copy may still momentarily hold the scoreboard
serial port and/or the two overlay TCP ports. Today's startup is *skip-and-log* (serial acquisition
around `main.rs:377`; TCP listeners on `--binary-port` 8001 / `--json-port` 8000), so a lost overlap
race silently brings the app up with a **dark scoreboard** and/or no overlay.

- *Fix:* on startup, if serial-open or a TCP bind fails with a "resource busy / address in use"
  class error, **wait-and-retry over a short bounded window** before giving up. Starting values:
  ~100 ms initial, exponential-ish backoff, **hard cap ~1–2 s total** per resource. On final
  failure, give up gracefully with a clear, specific log line (not a panic).
- *Scope:* the serial port (scoreboard) and both overlay TCP binds. Default timings ship now and
  are **confirmed/tuned on the Pi before publish**.
- *Note:* must not introduce a `.unwrap()`/`.expect()` panic on the busy path (project rust rules).

**Change 2 — Bounded sound-system shutdown (pass-2 finding D, shutdown half).**
`impl Drop for SoundController` (`refbox/src/sound_controller/mod.rs:611`) does a **blocking wait**
inside teardown, and teardown runs **before** the relaunch spawn. A slow/hung sound shutdown can
therefore lengthen the overlap (worsening Change 1's race) or, worst case, mean **the relaunch never
fires at all** — the old window closes and nothing comes back.

- *Fix:* put a **firm time bound** on the blocking shutdown wait so a slow/hung shutdown cannot
  delay or prevent the relaunch. On timeout, proceed with the restart regardless and log it.
- *Outcome:* a stalled sound shutdown can no longer strand the operator with a closed window.

**Change 3 — Flag-drift safety-net (pass-2 finding E).**
`build_restart_argv` reads `Cli` fields by name; a new CLI flag added later would compile cleanly
while being silently omitted from restarts — re-introducing the exact #1073 bug.

- *Fix (one line of shape):* **exhaustively destructure** `Cli` at the top of `build_restart_argv`
  (`let Cli { no_simulate, verbose, … } = args;` with **no `..`**), each field with a doc comment
  noting replay-or-exclude. Any future field then breaks the build until the author decides.

### 2.3 Deliberately NOT in Phase 1
- The explicit-exe-path relaunch refactor (pass-2 finding A) — only has behavioural value once the
  updater's rename-swap exists; it lands in Phase 2.
- Everything updater-related (check/download/verify/swap/revert/auto-revert) — Phase 2.

### 2.4 Acceptance criteria (operator-observable; validated on the spare Pi before publish)
- A restart via **language/mode change**, run **five times consecutively** (pass-2 finding D bar),
  each time returns: full-screen; scoreboard reconnected and showing the clock; buzzer working;
  overlay reconnected (if used); logs unchanged.
- Triage note baked into the test: if the old window closes and **no** new window appears, suspect
  the sound-shutdown hang first.

### 2.5 Automated tests
- Retry logic: succeeds when the resource frees within the window; **gives up gracefully** (no
  panic, clear log) when it does not, within the bound.
- Sound-shutdown bound: teardown returns within the time limit even if the underlying wait would
  block longer.
- Drift guard: covered by compile-time exhaustiveness (demonstrated in code review, not a runtime
  test).

### 2.6 Blast radius / process
Touches the restart path, hardware acquisition, and the sound controller — **heavy-process**
territory per `.claude/rules/plan-execution.md` (state-machine / hardware-adjacent). The Phase 1
plan builds it carefully with per-step verification; `just check` must pass; `just test` after any
`sound_controller`/startup change.

### 2.7 Ship flow
Own branch (proposed `fix/refbox/restart-resilience`; **branch creation needs user approval**) →
build + `just check` → PR (ask before opening) → merge → cut release → **spare-Pi 5× restart test**
→ publish if good; fix + re-release if not.

---

## 3. Phase 2 — Self-Update Backend (design-only; blocked)

> Phase 2's UX is already designed (`2026-06-10-refbox-self-update-design.md` §4) and prototyped/
> approved. This section captures the **backend** flow with the pass-2 amendments folded in, and
> marks what is blocked.

### 3.1 Update flow (operator-confirmed; any failure before the swap changes nothing)
1. **Check** — query the latest published GitHub release for `AtlantisSports/uwh-refbox-rs` (with a
   `User-Agent` header; drafts/pre-releases are excluded by the "latest" API — finding F).
2. **Pre-flight** — confirm free disk space **and** that the install directory is **writable**
   (finding F); re-check the game gate (close the check→install gap).
3. **Download** the standalone aarch64 binary to a temp file **on the same filesystem** as the
   install path; follow the asset redirect over HTTPS without leaking any auth header.
4. **Verify** the published checksum; abort and change nothing on mismatch.
5. **Smoke-test** — run the downloaded binary in a hidden `--self-check` mode **using the actual
   restart argv** (finding C): proves both "runs on this Pi" and "accepts the argv it will be
   restarted with," without opening a window, spawning a sim child, or grabbing hardware.
6. **Atomic swap — hard-link backup + single rename (finding B):**
   a. `link(current → refbox-v{prev}.bak)` (same dir/filesystem; nothing moves; one-deep).
   b. `rename(new → install path)` — atomic replace; old survives under the `.bak` name.
   There is **no instant** where the install path lacks a runnable program. (Rename-replace of a
   running binary is legal; write-in-place is not — ETXTBSY.)
7. **Restart — explicit exe path (finding A):** the single relaunch routine takes the executable
   path as a **parameter**. Plain restart passes `current_exe()`; the updater passes the
   **canonicalized install path captured *before* the swap**. (A rename makes `current_exe()` point
   at the `.bak` file under one scheme and `(deleted)` under the hard-link scheme — so a
   `current_exe()`-based respawn would relaunch the OLD version or fail outright. Empirically
   confirmed on Linux 2026-06-10.) Carries the original start-up arguments.
8. **Auto-revert safety-net** — see §3.3 (blocked).

### 3.2 Safety model (layered, outermost first)
- **Game gate** — whole feature disabled while a game is in progress, including half-time/breaks,
  timeouts, and score-review (not just live play); re-checked at install.
- **Verify-before-swap** — checksum + smoke-test pass before anything on disk changes.
- **One-deep backup** — previous version kept (same dir, version-named `refbox-v{prev}.bak`).
- **Manual revert** — "Revert to Previous Version (X)" button.
- **Best-effort auto-revert** — §3.3.
- **Manual SD-card fallback** — unchanged; the ultimate floor.

Net worst case: "the update button didn't work; update manually as today" — never a stranded
tournament.

### 3.3 BLOCKED on the boot script + a physical Pi
- **Auto-revert robustness.** A purely in-app "trial marker" (post-swap launch writes a marker;
  reaching healthy-running clears it with `fsync` — finding F; a launch that finds an uncleared
  prior trial restores the backup) only catches crashes *after* the checkpoint. A fully robust
  layer depends on **how the Pi boots refbox**. Key lever from `reference_pi_deployment_launch`:
  the Pi **auto-starts on power-up and the only stop/start is a power-cycle** — so the **boot path
  IS the recovery path**. The exact marker storage, the "healthy" definition, and whether the boot
  script itself can participate in revert are finalised once the developer shares the auto-start
  file. Question already sent (2026-06-15).
- **On-Pi validation** — Tests B–E of the hardware script (successful update, revert, failure
  handling, power-cut recoverability).

### 3.4 Architecture sketch
- **`refbox/src/updater/`** (new module) — check, download, checksum verify, smoke-test invocation,
  hard-link backup + atomic swap, revert, auto-revert marker. HTTP via `reqwest` (already used by
  the portal client) on `tokio`.
- **Relaunch routine** — the §2 Phase 1 routine, extended in Phase 2 to take the **explicit exe
  path** parameter (finding A). Plain restart and updater converge on **one** audited routine.
- **CLI** — add a hidden `--self-check` flag; keep flags a **stable interface** (finding C: add
  only; never remove/rename, or keep hidden aliases).
- **Version** — `env!("CARGO_PKG_VERSION")`; semver-aware comparison; logged at startup, shown on
  the Updates page.
- **UI** — `ConfigPage::Updates` + "Check Version" button on `make_app_config_page()`
  (`refbox/src/app/view_builders/configuration.rs`), mirroring the approved preview; new `Message`
  variants. Game-state gate is a single predicate over existing tournament-manager state.
- **Error messages** — specific/actionable, incl. captive-portal handling (non-JSON 200 →
  "couldn't reach the update server", finding F): "Couldn't reach the update server, please check
  your internet connection"; "The downloaded update wasn't valid and was not installed"; "Update
  server is busy, please try again later"; "Not enough free space to update"; "Can't update —
  the program folder isn't writable."

### 3.5 Additive CI change
`.github/workflows/release.yml`: additionally upload the standalone aarch64 Pi binary **and a
checksum file** as release assets (the binary also stays inside the combined zip). Purely additive.

### 3.6 Open dependency decision (needs human sign-off)
`self_replace` crate vs. hand-rolled hard-link+rename for the swap. Recorded as a choice, not a
decision; resolved in the Phase 2 plan.

### 3.7 Phase 2 acceptance criteria (on-Pi, before publish)
Check finds the newer version; Install downloads/verifies/restarts onto the new version
(full-screen, scoreboard, buzzer intact), version display confirms it; Revert returns to previous;
no internet → graceful message, app unchanged; Check disabled during game/break/timeout; Cancel
mid-download returns to *Update available* with nothing changed; power loss during install → the Pi
always boots a working refbox. Automated tests: version comparison, checksum verify (good +
tampered), platform/asset selection, error messages.

---

## 4. Pass-2 findings → phase mapping (traceability)

| Finding | Summary | Phase |
|--------|---------|-------|
| A | Relaunch routine takes explicit exe path (rename fools `current_exe()`) | **2** (needs the swap) |
| B | Hard-link backup + single rename (no empty-install-path window) | **2** |
| C | argv = cross-version compat contract; smoke-test with real argv; flags stable | **2** (policy noted now) |
| D | Dark-scoreboard retry/backoff (serial+TCP) + bounded sound shutdown | **1** |
| E | Exhaustive `Cli` destructure drift guard | **1** |
| F | Writable-dir pre-flight, captive-portal non-JSON, draft/pre-release, fsync marker | **2** |

---

## 5. Deferred / not now
Laptops (manual download); cryptographic signatures (checksum only for v1); multi-version backup
(one-deep only); automatic background update checks (operator-initiated; status starts "Unknown"
each session); post-update banner; release-notes snippet on the Updates page.

---

## 6. Status / deviations
- 2026-06-15: phased design approved in brainstorming (Phase 1 boundary incl. dark-scoreboard fix
  per user). Auto-revert + on-Pi validation blocked on developer's boot-script answer (question
  sent). Next: writing-plans for **Phase 1 only**.
