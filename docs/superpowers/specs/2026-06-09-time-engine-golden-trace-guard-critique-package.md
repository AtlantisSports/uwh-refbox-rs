# Critique Package: Golden-Trace Regression Guard for the Refbox Time Engine

**Status:** Draft design, seeking external critique before it becomes an implementation plan.
**Date:** 2026-06-09
**Audience:** An AI agent (or engineer) unfamiliar with this project. This document is self-contained — you should not need repository access to critique it, though file paths are given so you can verify claims if you do have access.

---

## 0. What I'm asking you to do

Critique the design in Section 4. I want you to be adversarial about it. Specifically:

- Find ways the proposed test harness could produce **traces that do not reflect how the real application behaves** (false confidence), or **flag differences that aren't real** (false alarms / noise).
- Challenge the assumptions in Section 5 and the risks in Section 6 — tell me which are underestimated, which are missing, and which are non-issues.
- Tell me if a fundamentally different approach to the stated goal (Section 1) would be materially better, given the constraints in Section 3.
- Call out anywhere the plan is over-engineered for its goal, or under-specified to the point of being un-implementable.

Do **not** assume I want a generic "best practices" lecture. Engage with the specifics below.

---

## 1. The goal

This is the software that runs underwater-hockey (UWH) referee operations at tournaments: it manages the game clock, scores, fouls, penalties, timeouts, and overtime, and drives a poolside LED scoreboard and a stream overlay. **The single most critical part of the system is the time logic** — the calculation of the game clock, period transitions, timeout clocks, and penalty countdowns.

Over roughly the last 8 months, a large amount of work (370 commits) has been done on this codebase, and **all of it was AI-authored** (authored by a non-programmer domain expert directing an AI). The concern: AI-introduced changes may have silently altered the core time calculations in ways that ordinary feature testing wouldn't catch, and a full human code review is not currently feasible.

**The chosen strategy** (selected from a menu of options that also included property-based testing, mutation testing, and coverage analysis) is **differential / golden-master testing against a trusted baseline**: pin down how the last human-authored version of the engine behaved, and automatically detect when today's code behaves differently.

**Decision already made:** the deliverable is a **durable regression guard** (a maintained test suite that runs in CI permanently), not a one-time investigation script. The initial run doubles as the investigation.

**Watch scope already decided:** **time-related state only** for the first version (game period, game clock, timeout clock/state, penalty countdown times), built so that scores/fouls/warnings can be added to the watched set later as a small change.

**Scenario coverage already decided** (four families, confirmed by the domain expert):
1. Regulation game flow (first half → half-time break → second half → game end, plus next-game-start timing).
2. Penalties over time (1/2/5-minute penalties, penalties crossing a period boundary or break, expiry, multiple concurrent penalties, penalty time during stoppages).
3. Timeouts & penalty shots (team timeout, ref timeout, penalty shot, rugby penalty shot, switching between them, timeout near end of period).
4. Overtime & sudden death (overtime halves with break, sudden-death timing, a timeout/score that ends the game during these phases).
Plus time-relevant sub-cases: score-confirmation pause, single-half games, manual clock-time edits.

---

## 2. Concrete technical facts I have verified (challenge these if they seem wrong)

These were established by inspecting the repo. They are load-bearing for the design.

**Baseline commit.**
- The trusted baseline is commit `46ec0973` (2025-10-08), authored by Tristan Debrunner — the last commit on the current history authored by a human (non-AI). Verified it is a direct ancestor of `HEAD`.
- There are **370 commits between that baseline and HEAD; 365 are AI-authored**, the rest are dependency bots. So the cut between "trusted" and "under suspicion" is clean.

**Where the time logic lives.**
- The engine is a single struct, `TournamentManager`, in `refbox/src/tournament_manager/mod.rs`. That file is **7,016 lines**. Supporting files: `tournament_manager/penalty.rs` (571 lines), and the observable output type `GameSnapshot` in `uwh-common/src/game_snapshot.rs` (1,186 lines).
- The real application's update loop lives in `refbox/src/app/mod.rs` (~4,150 lines).

**The engine's interface is time-injected and deterministic-friendly.**
- ~60 public methods. Crucially, every state-changing and state-reading method that depends on time takes the current time as an explicit parameter, e.g.:
  - `add_score(&mut self, color, player_num, now: Instant)`
  - `start_clock(&mut self, now: Instant)`, `stop_clock(&mut self, now: Instant)`
  - `start_penalty(...)`, `start_team_timeout(color, now)`, `start_ref_timeout(now)`, `start_penalty_shot(now)`, `start_rugby_penalty_shot(now)`
  - `game_clock_time(&self, now: Instant) -> Option<Duration>`
  - `timeout_clock_time(&self, now: Instant) -> Option<Duration>`
  - `current_period(&self) -> GamePeriod`
  - `generate_snapshot(&mut self, now: Instant) -> Option<GameSnapshot>`
  - `next_update_time(&self, now: Instant) -> Option<Instant>`
  - `pause_has_ended(&self, now)`, `could_end_game(&self, now)`, `timeout_end_would_end_game(&self, now)`
- Because time is injected, the same scripted sequence of (timestamp, action) calls is **reproducible** — no reliance on the wall clock.

**The baseline's API is virtually identical to today's.**
- A `pub fn` listing of `tournament_manager/mod.rs` at the baseline commit vs. HEAD shows the same method names, same signatures, and nearly the same line numbers. The 370 commits changed *internal logic*, not the public interface. (This is the key enabler: one scenario format and one driver work against both versions unchanged.)

**Existing test infrastructure.**
- `tournament_manager/mod.rs` already contains **62 in-module tests** (`#[cfg(test)] mod test`). There is an established pattern for constructing a `TournamentManager` and asserting on clock state in tests.

**A hard structural constraint: `refbox` is a binary-only crate.**
- `tournament_manager` is a **private module of the `refbox` binary**, not a public library crate. Therefore an external integration test (in `refbox/tests/`) **cannot reach it**. Any test that drives `TournamentManager` must be an **in-crate unit test** (`#[cfg(test)]` inside the binary), which is exactly how the existing 62 tests are written.
- Consequence: the replay/compare logic must live *inside* the `refbox` crate. Golden data files can live on disk (e.g. `refbox/tests/golden_time/`) and be read by the in-crate test via `CARGO_MANIFEST_DIR`.

**Async / channels.**
- `TournamentManager` exposes `get_start_stop_rx() -> watch::Receiver<bool>` (a `tokio::sync::watch` channel). This signals the rest of the app when the clock starts/stops. I have **not** yet fully confirmed whether the engine itself spawns async tasks or whether it can be fully driven synchronously from a test by feeding it instants. This is an open risk (see Section 6).

---

## 3. Constraints

- **Language/toolchain:** Rust, edition 2024, MSRV 1.85, `clippy -D warnings`, must pass on Linux/Windows/macOS.
- **`uwh-common` and `matrix-drawing` are `no_std`.** This design intends to touch neither.
- **Scope discipline is strongly enforced in this project.** The change must stay within `refbox` (new test module + golden data files). No edits to `uwh-common`, `wireless-remote`, the UI, or the LED/overlay/remote wire formats.
- **No new runtime dependencies** without discussion; test-only dev-dependencies are less sensitive but still want justification.
- **Reviewer is a non-programmer domain expert.** The golden files and diffs should be human-readable so they can make the "bug vs. intended change" call.

---

## 4. The proposed design

### 4.1 Components

**(a) Scenario scripts.** Each scenario is version-neutral data: an ordered list of `(elapsed_time, action)` steps, the game configuration it runs under (expressed only via the time-relevant settings that exist in *both* versions), and an observation cadence. Actions map to the common public API (start/stop clock, add score, start penalty, start/switch/end timeouts, penalty shots, manual clock edits, score-confirm pause, etc.). Scenarios cover the four confirmed families plus the time-relevant sub-cases.

**(b) Replay driver.** Test code that takes one scenario, constructs a `TournamentManager` from the scenario's config, and advances a *virtual* clock: it computes instants as `base + elapsed` from a single fixed base instant, performs each action at its timestamp, and at each observation point records one normalized line capturing period, game clock, timeout clock/state, and each active penalty's remaining time. Output: a deterministic multi-line text trace per scenario.

**(c) Golden trace files.** The recorded traces committed as plain text under `refbox/tests/golden_time/`. These files are the durable regression record — reviewable in diffs and defended by CI.

**(d) The permanent test.** An in-crate `#[cfg(test)]` test (alongside the existing 62) that, for every scenario, replays it through *today's* engine and asserts the produced trace equals the saved golden file, printing a line-by-line diff on mismatch.

### 4.2 Data flow

`scenario script → replay driver → normalized text trace → compare to committed golden → pass / fail-with-diff`

### 4.3 One-time bootstrap (seeding the trusted baseline)

Because the public API is identical between baseline and HEAD, the same driver source compiles and runs on both. Procedure:
1. Create a temporary git worktree checked out at baseline `46ec0973`.
2. Add the driver/scenario code there (it uses only the common API), run it in "record" mode (`UPDATE_GOLDEN=1`) to emit the baseline traces.
3. Copy those trace files onto the working branch as the initial golden record.

The baseline commit is touched exactly once, only to seed these files.

### 4.4 Workflow this produces

1. **Investigate:** with golden = baseline traces, run the test against today's code. Every reported difference is either a regression or an intentional feature change.
2. **Triage:** go through each difference with the domain expert; classify as "bug to fix" or "intended new behaviour."
3. **Re-bless:** intended changes update the affected golden file (one command, `UPDATE_GOLDEN=1`). Bugs get fixed in the engine until the test passes against the *original* baseline trace.
4. **Guard forever:** after triage, goldens reflect agreed-correct behaviour; CI fails any future unintended change to time behaviour. Re-blessing in a PR is a visible, reviewable diff.

### 4.5 Scope boundaries

- Watches time-related state only for v1 (period, game clock, timeout clock/state, penalty countdowns); extensible to scores/fouls/warnings later.
- Touches only `refbox` (new in-crate test module + golden data files).
- Baseline fixed at `46ec0973`; not re-litigating whether the baseline's behaviour was itself "correct" — it is the reference by definition.
- This is the differential-testing approach only; not property/mutation/coverage testing.

---

## 5. Assumptions the critic should pressure-test

1. **The engine can be driven deterministically and synchronously from a unit test** by feeding injected `Instant`s, without spawning the app's async runtime, and without the engine reading the wall clock internally.
2. **Sampling the public getters (`game_clock_time`, `current_period`, `timeout_clock_time`, penalty times) at observation points faithfully reproduces what the real app shows**, including time-driven auto-transitions (e.g., a period ending when the clock runs out, a penalty expiring).
3. **The public API being identical between baseline and HEAD means the driver source is portable to both** with no per-version code.
4. **A text-trace comparison is the right granularity** — fine enough to catch real timing regressions, coarse enough not to drown the triage in formatting noise.
5. **`GameConfig` differences between versions can be handled** by specifying only time-relevant settings in the scenario and letting each version default the rest; any time-relevant default that diverged is itself a legitimate finding, not harness breakage.
6. **The baseline (Tristan's last commit) is the right trust anchor** — i.e., its time behaviour was correct/field-tested enough to pin against.

---

## 6. Risks I already see (tell me which are worse than I think, and what I'm missing)

1. **Harness fidelity to the real update loop (highest concern).** The real app does not just call getters on a fixed cadence — it uses `next_update_time(now)` to schedule ticks and calls `generate_snapshot(now)` to advance/emit state. Some time-driven transitions (period rollover at clock zero, penalty expiry, pause-has-ended) may only be realized when the app ticks at the right instants. **If the replay driver samples on a naive fixed cadence instead of emulating the real tick scheduling, the traces may not reflect real behaviour** — risking both false confidence and false alarms. The driver may need to replicate the app's `next_update_time`-driven loop. How faithfully must it mirror `refbox/src/app/mod.rs`? Is there a risk the driver itself becomes a second, divergent implementation of the update loop that needs its own testing?
2. **Hidden non-determinism in traces.** Penalty list ordering, `Option`/rounding of `Duration` values, sub-second boundary effects, or any internal `Instant::now()` / RNG / HashMap iteration could make traces unstable across runs or platforms. Normalization rules need to be airtight.
3. **`GameConfig` divergence beyond the time-relevant subset.** If a default that *does* affect timing changed between versions, the bootstrap and the current run start from subtly different configs; this surfaces as a diff that looks like an engine regression but is a config-default change. Triage must distinguish these.
4. **Re-bless erodes the guard.** The whole value depends on disciplined triage. A careless `UPDATE_GOLDEN=1` that blesses a real regression silently destroys the protection. Is the visible-PR-diff safeguard enough, or does re-blessing need a stronger gate?
5. **Bootstrap portability gotchas.** If the driver inadvertently references any HEAD-only type/method, it won't compile at baseline. Keeping it on the strict common subset is a discipline the plan relies on.
6. **Coverage illusion.** A curated scenario library only tests the situations someone thought to script. Time bugs in unscripted edge cases (e.g., unusual action orderings) go undetected. Should this be paired with seeded-random scenario generation for breadth, or does that undermine the human-readable-golden goal?
7. **Single 7,016-line file / private-module constraint.** The test must live in-crate. Does packing more test machinery into an already-huge module create maintainability problems, and is there a cleaner in-crate home for it?

---

## 7. Specific questions for the critic

1. Is emulating the app's `next_update_time` tick loop in the driver the right call, or is there a simpler observation strategy that still captures time-driven transitions faithfully? (Risk #1.)
2. Is golden-master with curated scenarios the right primary tool here, or would seeded-random differential testing (same baseline, random action sequences, compared on the fly) catch materially more for comparable effort — given the constraint that two versions of the crate can't easily co-compile?
3. Are there failure modes of golden-master testing specific to *timing* logic (as opposed to pure functions) that this design doesn't account for?
4. Given the bin-only constraint forces in-crate tests, is there a better architectural option I'm missing (e.g., proposing that `tournament_manager` be extracted into a small library crate so it can be tested from outside)? Note: extracting it would expand scope significantly and touch the structure of the app — is that trade-off worth it?
5. What's the single weakest point of this plan that, if it fails, makes the whole effort worthless?
