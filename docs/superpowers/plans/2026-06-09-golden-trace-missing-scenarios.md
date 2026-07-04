# Golden-Trace Missing-Scenario Coverage — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax. Test-only code in `refbox` → **lean process** per `.claude/rules/plan-execution.md`. The engine is NEVER modified; `render()` is NOT changed (watched state stays time-only). Spec: `docs/superpowers/specs/2026-06-09-golden-trace-missing-scenarios-design.md`.

**Goal:** Add focused golden-trace scenarios that close the ~5 coverage gaps the cargo-mutants validation found, each proven to flip its previously-surviving mutant from missed→caught.

**Architecture:** Purely additive scenarios in `golden/scenarios.rs` (+ new `static *_ACTIONS` arrays and entries in `all()`), each generating one new `golden_traces/*.trace`. The existing 30 scenarios/traces are untouched. No new `Action` variants (natural expiry = start the timeout/shot and omit `EndTimeout`).

**Tech Stack:** Rust 2024, in-crate `#[cfg(test)]`, golden files via `UPDATE_GOLDEN=1`, kill-proof via scoped `cargo mutants`.

---

## Working location

Branch `feat/refbox/golden-trace-missing-scenarios` (worktree `.worktrees/golden-trace-missing-scenarios`), stacked on `feat/refbox/time-golden-trace-guard` (PR #1041). `cd` into the worktree before any cargo. cargo-mutants is at `~/.cargo/bin` (add to PATH).

## File structure

- Modify: `refbox/src/tournament_manager/golden/scenarios.rs` — add `static <NAME>_ACTIONS` arrays + `Scenario` entries in `all()`. Reuse `reg_config()`; override only the config fields a scenario needs.
- Create (generated, via `UPDATE_GOLDEN=1`): `refbox/src/tournament_manager/golden_traces/<name>.trace` per scenario.
- No other files.

## Per-scenario task shape (applies to Tasks 1–6)

Each task is the same five steps; only the scenario data and kill-target differ:
1. Add the `static <NAME>_ACTIONS` array and the `Scenario { name, config, actions, run_secs }` entry in `all()`.
2. Generate the golden file: `UPDATE_GOLDEN=1 cargo test -p refbox golden_traces_match_baseline` (any value of UPDATE_GOLDEN blesses). Confirm exactly one new `<name>.trace` appears and `git status` shows the existing 30 traces UNCHANGED.
3. Inspect the new trace for behavioural sanity (the expected period/clock/timeout/conf_pause/penalty sequence described in the task).
4. **Kill-proof:** re-run the previously-surviving mutant(s) for this gap, scoped, with the golden test as the only kill-check, and confirm `caught` (was `missed`):
   ```
   cd <worktree>; export PATH="$HOME/.cargo/bin:$PATH"
   cargo mutants -f refbox/src/tournament_manager/mod.rs -F '<function>' \
     --test-package refbox --cargo-test-arg golden_traces_match_baseline -j 6 --minimum-test-timeout 60
   ```
   Confirm the target line appears in `caught.txt` (not `missed.txt`). Record before(missed)→after(caught).
5. `just fmt`; commit: `test(refbox): add <name> golden scenario (closes <gap>)`.

> NOTE on `-F`: it matches whole functions; a function may host several mutants. The kill-proof passes if the SPECIFIC previously-surviving line(s) for this gap are now `caught`. Other mutants in the same function that were already out-of-scope/dead may remain missed — that is expected; the proof is line-targeted.

---

## Task 1: `sudden_death_no_overtime`

**Gap:** no scenario has overtime-disabled + sudden-death-enabled, so the SecondHalf→PreSuddenDeath confirm-pause branch (`pause_for_confirm` mod.rs:1846) is unexercised.

- [ ] **Config:** `GameConfig { overtime_allowed: false, sudden_death_allowed: true, ..reg_config() }`.
- [ ] **Actions:**
  ```rust
  static SUDDEN_DEATH_NO_OVERTIME_ACTIONS: &[(u64, Action)] = &[
      (0, SetupPeriod(GamePeriod::SecondHalf, Duration::from_secs(5))),
      (0, StartClock),
  ];
  ```
  `run_secs: 18`. Scores stay 0–0 (tie) so the game does not end at SecondHalf.
- [ ] **Expected trace:** SecondHalf 5→0 → `conf_pause` countdown (~2s, = `min(pre_sudden_death=5, minimum_break=6)/2`) → PreSuddenDeath 5→0 → SuddenDeath counting up.
- [ ] **Kill-proof:** `-F 'pause_for_confirm'`; confirm mod.rs:1846 (`replace / with *`) flips missed→caught.
- [ ] fmt + commit.

## Task 2: `manual_clock_edit_rewinds_penalty`

**Gap:** `manual_clock_edit_while_penalty_running` never edits the clock far enough to trigger the penalty-rebase branch (`set_game_clock_time` mod.rs:1781, 1783).

- [ ] **Config:** `reg_config()` (FirstHalf 20s).
- [ ] **Actions:** start a OneMinute penalty, stop the clock, then set the clock to *more* remaining than when the penalty started (rewind past its start → `time_remaining > full duration` → rebase):
  ```rust
  static MANUAL_CLOCK_EDIT_REWINDS_PENALTY_ACTIONS: &[(u64, Action)] = &[
      (0, SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(20))),
      (0, StartClock),
      (2, StartPenalty(Color::Black, 7, PenaltyKind::OneMinute)), // starts at clock=18s
      (5, StopClock),                                             // clock=15s
      (5, SetGameClock(Duration::from_secs(19))),                 // rewind to 19s (before penalty start)
      (6, StartClock),
  ];
  ```
  `run_secs: 25`.
- [ ] **Expected trace:** penalty `B#7` counts 60→57 while clock 18→15; after the edit the clock jumps to 19s and `B#7` resets to 60 (rebased); then both resume.
- [ ] **Kill-proof:** `-F 'set_game_clock_time'`; confirm mod.rs:1781 (`!=`→`==`) and/or 1783 flip missed→caught.
- [ ] fmt + commit.

## Task 3: `team_timeout_expires`

**Gap:** team timeouts are always ended via `EndTimeout`; the natural clock-expiry path (`update` mod.rs:1330) is unexercised.

- [ ] **Config:** `reg_config()` (team_timeout_duration=15s, FirstHalf 20s).
- [ ] **Actions:** start a team timeout, never end it; run past 15s so `update` expires it and resumes the game clock:
  ```rust
  static TEAM_TIMEOUT_EXPIRES_ACTIONS: &[(u64, Action)] = &[
      (0, SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(20))),
      (0, StartClock),
      (3, StartTeamTimeout(Color::Black)), // game clock stops at 17s
  ];
  ```
  `run_secs: 25` (timeout 15s expires at t=18; game clock resumes from 17s).
- [ ] **Expected trace:** clock 20→17, `timeout=Team:Black:15`→0, then `timeout=none` and clock resumes 17→… .
- [ ] **Kill-proof:** `-F 'update'`; confirm mod.rs:1330 (`+`→`-`) flips missed→caught.
- [ ] fmt + commit.

## Task 4: `rugby_penalty_shot_expires`

**Gap:** rugby penalty shots are always ended via `EndTimeout`; natural expiry (`update` mod.rs:1348 → `handle_rugby_pen_shot_end` 1443/1478) is unexercised.

- [ ] **Config:** `reg_config()` (FirstHalf 30s so the half doesn't end first; penalty_shot_duration=15s).
- [ ] **Actions:**
  ```rust
  static RUGBY_PENALTY_SHOT_EXPIRES_ACTIONS: &[(u64, Action)] = &[
      (0, SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(30))),
      (0, StartClock),
      (3, StartRugbyPenaltyShot), // game clock keeps running (rugby)
  ];
  ```
  `run_secs: 25`.
- [ ] **Expected trace:** game clock counts down continuously; `timeout=RugbyPenaltyShot:…` counts down and clears on expiry (~t=18).
- [ ] **Kill-proof:** `-F 'handle_rugby_pen_shot_end'` (and `-F 'update'` for 1348); confirm 1443/1478 (and 1348) flip missed→caught.
- [ ] fmt + commit.

## Task 5: `single_half_to_overtime`

**Gap:** the `single_half` scenario doesn't continue to overtime, so `end_first_half`'s single-half branch (mod.rs:1366-1368) is unexercised.

- [ ] **Config:** `GameConfig { single_half: true, overtime_allowed: true, sudden_death_allowed: false, ..reg_config() }`.
- [ ] **Actions:**
  ```rust
  static SINGLE_HALF_TO_OVERTIME_ACTIONS: &[(u64, Action)] = &[
      (0, SetupPeriod(GamePeriod::FirstHalf, Duration::from_secs(10))),
      (0, StartClock),
  ];
  ```
  `run_secs: 25`. Scores 0–0 (tie) → goes to overtime, not game-end.
- [ ] **Expected trace:** FirstHalf 10→0 → PreOvertime 5→0 → OvertimeFirstHalf … (NOT BetweenGames).
- [ ] **Kill-proof:** `-F 'end_first_half'`; confirm the 1366-1368 cluster flips missed→caught.
- [ ] fmt + commit.

## Task 6 (attempt; defer if fragile): `between_games_reset`

**Gap:** the between-games auto-reset path (`update` mod.rs:1181-1182) is unexercised.

- [ ] **Investigate first:** read `reset_game_time` (its config field + the reset trigger at `update` mod.rs:1180-1186). Determine the smallest config + `run_secs` that drives the game into BetweenGames and lets the break clock reach `reset_game_time` so `reset()` fires, within a deterministic fixed-step run.
- [ ] **If clean:** add `BETWEEN_GAMES_RESET_ACTIONS` (a full short game run long into BetweenGames) with the config that makes reset fire; bless; verify the trace shows the reset (e.g., `is_old_game` flips / clock jumps per `reset()`), then kill-proof `-F 'update'` for 1181/1182.
- [ ] **If fragile/non-deterministic** (needs an impractically long run, or doesn't trigger reliably): do NOT ship it. Record in the plan's Deviations section that `between_games_reset` is deferred and why, and note `update` 1181/1182 remain known-uncovered. Move on.
- [ ] fmt + commit (only if shipped).

## Task 7: Final verification

- [ ] Run the full guard test once more: `cargo test -p refbox golden_traces_match_baseline` → PASS, and confirm the existing 30 traces are unchanged in `git diff --stat` (only new `.trace` files + `scenarios.rs` edits).
- [ ] `just check` → clean (fmt, lint, tests, audit).
- [ ] Assemble the per-scenario kill-proof record (before missed → after caught) for the PR body.

## Acceptance criteria

- 5 (or 6) new scenarios added; each new `.trace` committed; existing 30 traces byte-unchanged.
- Each new scenario's targeted previously-surviving mutant is now `caught` (recorded before/after).
- `just check` clean. Engine and `render()` untouched.

## Deviations

- **7 scenarios shipped, not 5.** The single-half gap needed three configs to kill all of
  `end_first_half`'s branch mutants (the boolean clause is config×score dependent):
  `single_half_to_overtime` (tie + OT → PreOvertime, kills 1366), `single_half_decided`
  (decided + OT → end_game, kills 1368:17 `||`→`&&`), `single_half_drawn` (drawn, no OT/SD →
  end_game, kills 1368:54 `delete !`). After all three, `end_first_half` is fully caught (5/5).
- **Rugby scenario redesigned mid-execution.** First attempt (`rugby_penalty_shot_expires` with the
  shot expiring mid-half, clock running) did NOT kill 1348/1443/1478 — `handle_rugby_pen_shot_end`'s
  body is gated on `ClockState::Stopped`, unreachable while the game clock runs. Redesigned so the
  shot extends *past* the period end (clock stops at 0; ending the shot drives the transition).
  After redesign: 1348, 1443, 1478 all caught.
- **`manual_clock_edit_rewinds_penalty`: one survivor is a proven equivalent mutant.** 1781 and
  1783 (`>`→`==`, `>`→`<`) caught; 1783 `>`→`>=` survives but is equivalent (only differs when a
  penalty's remaining exactly equals its full duration, where rebasing resets to identical values —
  a no-op). Not closeable; not a gap.
- **`between_games_reset` DEFERRED (Task 6).** `reset()` clears penalties (watched) but only fires
  in BetweenGames gated by an internal `has_reset` flag that is false only in a multi-game lifecycle
  the fixed-step scenarios don't naturally reach; its remaining observable effect is `is_old_game`
  (not watched). Mutants `update`:1181/1182 remain uncovered. Deferred to the watched-state-expansion
  follow-up (B), where `is_old_game` becomes observable. Recorded, not shipped.
- **Other `update` survivors NOT targeted by this branch** (out of follow-up A's 5 gaps, left as-is):
  1198:32 (`<`→`<=`, sub-second boundary in the rugby-extend check — likely equivalent at 1 s
  resolution), 1213:52 (`+`→`-` in the BetweenGames→start_game arg), 1314:55 (`&`→`|`/`^` in the
  SD-confirm-pause guard). These are separate from the 5 closed gaps; note for future coverage.
- **Commits:** per-scenario for Tasks 1-2; grouped (amended) commit for the rest. Lean process.
