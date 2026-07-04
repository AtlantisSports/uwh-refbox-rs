# Time-Engine Golden-Trace Regression Guard — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. This is test-only code in `refbox` → **lean process** per `.claude/rules/plan-execution.md` (no per-task code review; review once at PR; mechanical tasks skip verification ceremony). The engine itself is NEVER modified.

**Goal:** A permanent, in-crate regression test that pins the time-engine's behaviour across a curated scenario library, seeded from the last human-authored baseline (`46ec0973`), and fails the build whenever today's code computes different time-state.

**Architecture:** A fixed-step replay driver (proven by spike `45fea4a2`) runs each scenario through `TournamentManager`, advancing virtual time in fixed 100 ms steps while ticking `update`/`generate_snapshot` densely so transitions are realised. It emits a deduplicated, state-change-keyed text trace. Traces captured once from the baseline are committed as golden files; the permanent test replays every scenario through today's engine and diffs against them.

**Tech Stack:** Rust 2024, in-crate `#[cfg(test)]` modules (refbox is bin-only — integration tests can't reach the private `tournament_manager` module), golden text files read via `env!("CARGO_MANIFEST_DIR")`.

---

## Background (read before starting)

- **Why this exists / full context:** see `docs/superpowers/specs/2026-06-09-time-engine-golden-trace-guard-*.md` (critique package, two critique rounds, spike results). Summary: 370 AI-authored commits since the last human commit may have silently altered time logic; this guard detects drift from the trusted baseline.
- **The spike (`45fea4a2`)** is committed at the bottom of `refbox/src/tournament_manager/mod.rs` as `#[cfg(test)] mod golden_time_spike`. It is the working foundation. This plan generalises it into the real guard and then **deletes the spike module** (Task 1) so we don't carry two copies.
- **Two locked findings from the spike:**
  1. **Do NOT drive virtual time via `next_update_time`** — it returns `now` on whole-second boundaries and hangs a replay. Use the fixed-step model.
  2. **Key trace lines on observed state, not wall-time** — drop the elapsed-second label; the trace is the ordered sequence of *distinct* observable states (which already contain the clock value).
- **Baseline commit:** `46ec0973` (last commit by Tristan Debrunner, direct ancestor of HEAD).
- **Branch:** continue on `feat/refbox/time-golden-trace-spike`, or cut a fresh `feat/refbox/time-golden-trace-guard` off current `master` (fetch first — local master is stale). Decide at execution time; prefer a fresh branch off up-to-date master for a clean PR.

---

## File Structure

- Create: `refbox/src/tournament_manager/golden/mod.rs` — `#[cfg(test)]` child module: the driver, `Action` enum, normalization/render, golden read-write-compare harness, and the `#[test]` entry point. (Child module of `tournament_manager`, so it can call the private/`pub(super)` engine methods, exactly as the existing `mod test` does.)
- Create: `refbox/src/tournament_manager/golden/scenarios.rs` — the curated scenario library (data only: configs + action scripts).
- Create: `refbox/src/tournament_manager/golden_traces/*.trace` — one committed golden file per scenario (generated, not hand-written).
- Modify: `refbox/src/tournament_manager/mod.rs` — add `#[cfg(test)] mod golden;` near the existing `#[cfg(test)] mod test;`; **remove** the `mod golden_time_spike` block.
- Reference only (never modify): `refbox/src/app/mod.rs:4073-4116` (the real tick loop the driver mirrors).

---

## Core types (defined once; used by all tasks)

```rust
// in golden/mod.rs
use super::*; // brings TournamentManager, GameConfig, Color, PenaltyKind, Infraction, GamePeriod, Instant, Duration
use uwh_common::game_snapshot::{GameSnapshot, PenaltyTime, TimeoutSnapshot};

#[derive(Clone, Copy)]
pub(super) enum Action {
    SetupPeriod(GamePeriod, Duration), // set_period_and_game_clock_time
    StartClock,
    StopClock,
    AddScore(Color),                   // add_score(color, 0, now)
    StartPenalty(Color, u8, PenaltyKind),
    StartTeamTimeout(Color),
    StartRefTimeout,
    StartPenaltyShot,
    StartRugbyPenaltyShot,
    EndTimeout,
    SetGameClock(Duration),            // manual clock edit
}

pub(super) struct Scenario {
    pub name: &'static str,
    pub config: GameConfig,
    pub actions: &'static [(u64 /*offset secs*/, Action)],
    pub run_secs: u64,
}
```

**Normalization (resolves finding #2):** a trace is `Vec<String>` of **distinct consecutive observable states**; no timestamp label. Each state string is produced by `render(&GameSnapshot)`:
`period=<P> | clock=<secs_in_period>s | timeout=<none|Team:Black:Ns|Ref:Ns|PenaltyShot:Ns> | pens=[<canonical>]`
where penalties are ordered by (remaining desc, color, player#) and rendered `B#7:30` / `W#3:TD`. Reuse the spike's `render` minus the timestamp.

---

## Task 1: Promote the spike into a reusable driver module

**Files:**
- Create: `refbox/src/tournament_manager/golden/mod.rs`
- Modify: `refbox/src/tournament_manager/mod.rs` (add `mod golden;`, delete `mod golden_time_spike`)

- [ ] **Step 1:** Create `golden/mod.rs` with the core types above, plus the helpers carried over from the spike, generalised:
  - `snapshot_with_retry(tm, now) -> GameSnapshot` (unchanged from spike).
  - `tick(tm, now, clock_running)` — the real tick block: `could_end_game`→`pause_for_confirm`, else `pause_has_ended`→`end_confirm_pause`, else `update`. (No `next_update_time` — finding #1.)
  - `apply_action(tm, action, now, &mut clock_running)` — maps each `Action` to its real handler's `tm` calls. Mirror the handlers in `app/mod.rs` (note the game-ending `update(now + 2ms)` quirk at `app/mod.rs:2823` if a scenario ends a game). Toggle `clock_running` on start/stop/timeout. **KNOWN COUPLING POINT — document this in a comment block above `apply_action`:** this match is a hand-copy of the real action handlers. If a handler in `app/mod.rs` ever changes which `TournamentManager` methods it calls, this arm must be updated in lockstep or the golden traces will silently stop reflecting the real app. Any future PR touching an action handler should review this driver.
  - `render(&GameSnapshot) -> String` (state only, no timestamp).
  - `run(scenario: &Scenario) -> Vec<String>`:
    ```
    const STEP: Duration = Duration::from_millis(100); // finding #1: 100ms, not 250
    // NOTE: fixed-step is faithful only while `update` is idempotent w.r.t. call
    // frequency (it recomputes state from start_time+elapsed). If the engine ever
    // accumulates per-call state, this driver could diverge from the real app.
    let base = Instant::now();
    let mut clock_running = false;
    let mut trace = Vec::new(); let mut last: Option<String> = None;
    // apply setup actions at t=0 first (SetupPeriod, StartClock, ...)
    // then loop elapsed 0..=run_secs by STEP:
    //   apply due actions at their exact instants (record on change)
    //   if clock_running { tick(...); record on change }
    // record() pushes render(snapshot) iff != last.
    ```
- [ ] **Step 2:** In `mod.rs`, add `#[cfg(test)] mod golden;` adjacent to `#[cfg(test)] mod test;`, and **delete the entire `mod golden_time_spike` block**.
- [ ] **Step 3:** Add a temporary smoke test inside `golden/mod.rs` replicating the spike scenario (FirstHalf 40s, penalty B#7 @2s, stop@15/start@18, run 55s) and assert: two runs identical; trace contains `HalfTime`; trace contains `B#7:0`.
- [ ] **Step 4:** Run `cargo test -p refbox golden -- --nocapture`. Expected: PASS; trace shows FirstHalf→HalfTime→SecondHalf and penalty 30→0, deduped, no timestamp column.
- [ ] **Step 5:** `just fmt && just lint` (clippy `-D warnings`), then commit: `test(refbox): generalise golden-trace spike into driver module`.

---

## Task 2: Golden file read/write/compare harness

**Files:** Modify `refbox/src/tournament_manager/golden/mod.rs`

- [ ] **Step 1:** Add the golden harness:
  ```rust
  fn golden_path(name: &str) -> std::path::PathBuf {
      std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
          .join("src/tournament_manager/golden_traces")
          .join(format!("{name}.trace"))
  }
  // Returns Ok(()) on match; Err(diff_string) on mismatch. If UPDATE_GOLDEN is set,
  // (re)writes the file and returns Ok(()).
  fn check_or_bless(name: &str, trace: &[String]) -> Result<(), String> { /* ... */ }
  ```
  - `UPDATE_GOLDEN=1`: create the dir if needed, write `trace.join("\n") + "\n"`.
  - otherwise: read the file (missing file → `Err("no golden; run UPDATE_GOLDEN=1")`), compare line-by-line, build a unified-ish diff string on mismatch (show first N differing lines with `- expected / + actual`).
- [ ] **Step 2:** Unit-test the harness itself with a tiny synthetic trace and a temp name under `UPDATE_GOLDEN=1` then read-back match. (Or fold into Task 4's run.)
- [ ] **Step 3:** `just fmt && just lint`, commit: `test(refbox): add golden trace read/write/compare harness`.

---

## Task 3: Scenario library

**Files:** Create `refbox/src/tournament_manager/golden/scenarios.rs`

Define `pub(super) fn all() -> Vec<Scenario>` returning the curated set below. Use short, whole-second durations so traces are compact and land on clean boundaries. Each scenario's `actions` are an ordered `&[(offset_secs, Action)]`. Exact offsets are the executor's to finalise within each scenario's described intent.

- [ ] **Step 1:** Family 1 — Regulation flow:
  - `regulation_full` — FirstHalf→HalfTime→SecondHalf→BetweenGames; no penalties. (config: half=20s, half_time=8s, nominal_break small.)
  - `regulation_with_scores` — same, with `AddScore` for each team during play.
- [ ] **Step 2:** Family 2 — Penalties over time:
  - `penalty_one_minute` — single OneMinute penalty expiring mid-half (half long enough).
  - `penalty_crosses_break` — penalty started late in FirstHalf, frozen across HalfTime, resumes SecondHalf.
  - `penalty_concurrent` — two penalties (OneMinute + TwoMinute) on both teams, different expiry.
  - `penalty_during_stoppage` — penalty + clock stop/start (the spike scenario, kept).
  - `penalty_total_dismissal` — a `TotalDismissal` penalty (renders `TD`, never counts to 0).
  - `penalty_expires_at_boundary` — a penalty scripted so its remaining time hits exactly 0 at the instant a half ends (expiry coincides with the period boundary). Targets the off-by-one risk: does it clear cleanly at the boundary, linger one tick into the next period, or vanish a tick early? Distinct from `penalty_crosses_break`, which has plenty of time left at the boundary. Pin whatever the baseline does.
- [ ] **Step 3:** Family 3 — Timeouts & penalty shots:
  - `team_timeout` — `StartTeamTimeout(Black)` mid-half, `EndTimeout`, resume.
  - `ref_timeout` — `StartRefTimeout`/`EndTimeout`.
  - `penalty_shot` — `StartPenaltyShot`/`EndTimeout`.
  - `rugby_penalty_shot` — `StartRugbyPenaltyShot` (game clock keeps running) /`EndTimeout`.
  - `timeout_near_period_end` — team timeout started a few seconds before a half ends.
  - `timeout_freezes_penalty` — a penalty ticking, then a team timeout called mid-countdown, then `EndTimeout` and resume. Confirms the timeout freezes both the game clock and the penalty countdown together, and the penalty resumes from the frozen value. Pin whatever the baseline does.
- [ ] **Step 4:** Family 4 — Overtime & sudden death:
  - `overtime_full` — tie at end of regulation → PreOvertime→OvertimeFirstHalf→OvertimeHalfTime→OvertimeSecondHalf. (config: `overtime_allowed=true`.)
  - `sudden_death` — tie through overtime → PreSuddenDeath→SuddenDeath; `AddScore` ends the game. (config: `sudden_death_allowed=true`.)
  - `overtime_score_ends` — a score during overtime that ends the game.
- [ ] **Step 5:** Sub-cases:
  - `score_confirm_pause` — drive to end-of-game so `could_end_game`→`pause_for_confirm` fires; trace shows the confirm pause (`conf_pause_time` path). Mirrors the real tick's first branch.
  - `single_half` — single-half game config (Elsa's feature): `half_play_duration` only, ensure correct end.
  - `manual_clock_edit` — `SetGameClock` mid-play, confirm clock jumps and continues correctly.
- [ ] **Step 6:** `just fmt && just lint`, commit: `test(refbox): add golden-trace scenario library`.

---

## Task 4: Permanent test wiring

**Files:** Modify `refbox/src/tournament_manager/golden/mod.rs`

- [ ] **Step 1:** Replace the Task 1 smoke test with the real driver test:
  ```rust
  #[test]
  fn golden_traces_match_baseline() {
      let mut failures = Vec::new();
      for s in scenarios::all() {
          let a = run(&s);
          let b = run(&s);
          assert_eq!(a, b, "scenario '{}' is non-deterministic", s.name);
          if let Err(diff) = check_or_bless(s.name, &a) {
              failures.push(format!("--- {} ---\n{diff}", s.name));
          }
      }
      assert!(failures.is_empty(), "golden trace mismatches:\n\n{}", failures.join("\n\n"));
  }
  ```
- [ ] **Step 2:** Run `UPDATE_GOLDEN=1 cargo test -p refbox golden_traces_match_baseline` **on the current branch** to generate provisional golden files from *today's* engine (these are NOT the baseline yet — Task 5 replaces them). Confirm one `.trace` file per scenario appears under `golden_traces/`, and each looks behaviourally sane on inspection.
- [ ] **Step 3:** Run `cargo test -p refbox golden_traces_match_baseline` (no env) → PASS against the just-written files (proves the compare path).
- [ ] **Step 4:** `just fmt && just lint`. Do **not** commit the provisional golden files yet (Task 5 overwrites them with the real baseline). Commit only the code: `test(refbox): wire permanent golden-trace test`.

---

## Task 5: Bootstrap golden files from the baseline (`46ec0973`)

This is an operational procedure, run once. The driver uses only the engine's public/`pub(super)` API, which is identical between baseline and HEAD (verified), so it compiles unchanged on the baseline.

- [ ] **Step 1:** Create a baseline worktree: `git worktree add /tmp/refbox-baseline 46ec0973`.
- [ ] **Step 2:** Copy `golden/` (mod.rs + scenarios.rs) into `/tmp/refbox-baseline/refbox/src/tournament_manager/`, and add `#[cfg(test)] mod golden;` to that worktree's `mod.rs`.
- [ ] **Step 3:** In the worktree, first run `cargo check -p refbox --tests` to catch any non-common API symbol up front; then `UPDATE_GOLDEN=1 cargo test -p refbox golden_traces_match_baseline`. If either fails to compile, the driver touched a symbol that differs between baseline and HEAD — fix the driver to the common subset (do not change the baseline engine).
- [ ] **Step 4:** Copy the generated `/tmp/refbox-baseline/refbox/src/tournament_manager/golden_traces/*.trace` onto the working branch (overwriting Task 4's provisional files). These are the **trusted baseline behaviour**.
- [ ] **Step 5:** `git worktree remove /tmp/refbox-baseline`. Commit the golden files: `test(refbox): capture baseline (46ec0973) golden traces`.

---

## Task 6: Triage pass (investigation → fix/bless)

- [ ] **Step 1:** Run `cargo test -p refbox golden_traces_match_baseline` on HEAD. Each mismatch is a place today's engine diverges from the baseline.
- [ ] **Step 2:** For each mismatch, classify **with the human** (plain-English diff): "regression to fix in the engine" vs "intended change since the baseline (which feature/commit)".
- [ ] **Step 3:** For regressions: open a separate issue/branch (do NOT fix engine logic on this test-only branch — that's a different concern per `.claude/rules/scope.md`). Record them in this plan's Deviations section.
- [ ] **Step 4:** For intended changes: re-bless via `UPDATE_GOLDEN=1`, and in the PR body add one line per re-blessed scenario: `Blessed <scenario>: intended change from <feature/commit>`. Commit: `test(refbox): bless intended post-baseline time-behaviour changes`.

---

## Task 7: Re-bless rule + CI

- [ ] **Step 1:** Add a short `refbox/src/tournament_manager/golden_traces/README.md`: what these files are, how to re-bless (`UPDATE_GOLDEN=1 cargo test -p refbox golden`), and the rule: **every PR that changes a `.trace` must classify each change in one line in the PR body**.
- [ ] **Step 2:** Confirm the test runs under the existing `just test` / CI (it's an ordinary `#[cfg(test)]` test, so it does automatically — verify no `UPDATE_GOLDEN` leaks into CI env).
- [ ] **Step 3:** Commit: `docs(refbox): document golden-trace re-bless rule`.

---

## Acceptance Criteria

- `cargo test -p refbox golden_traces_match_baseline` passes on HEAD (after Task 6 triage), and fails with a readable per-scenario diff if any watched time-state changes.
- Each scenario trace is byte-identical across repeated runs (determinism asserted in-test).
- Golden files are committed, human-readable, and one per scenario.
- The guard touches only `refbox` test code + golden data; the engine and all other crates are unchanged.
- `just check` (fmt, lint, test, audit) is clean before PR.

## Scope Boundaries

- **Time-only** observable state for v1 (period, game clock, timeout type+clock, penalty remaining). Scores/fouls/warnings are out of the watched set (the `render` function is the single extension point to add them later).
- **No engine changes.** If triage finds regressions, they are fixed on separate branches, not here.
- Not building the web-vs-Rust comparison (the scenario format + render are kept implementation-neutral so they can be reused there later, but that work is out of scope).
- Not seeded-random/property testing — curated scenarios only.

## Deviations

(Record any execution deviations and triage-discovered regressions here.)

### Execution deviations (2026-06-09, Tasks 1–4)

- **Branch/worktree.** Built on a FRESH `feat/refbox/time-golden-trace-guard` off latest `master` (`ca6cdd0a`), in worktree `.worktrees/time-golden-trace-guard` — not the spike branch (it was 89 commits behind master, i.e. a stale engine; verifying it would verify the wrong code). Baseline `46ec0973` confirmed an ancestor of master.
- **Task 1 Step 2 was a no-op.** There is no `mod golden_time_spike` on a master-based branch, so nothing to delete. The driver was written fresh using the spike code (from the spike branch) as a reference.
- **Driver fidelity rework (commit `c637b19b`), a significant divergence from the plan's sketch:** the planned hand-tracked `clock_running` bool froze timeouts and confirm pauses (unfaithful). Replaced it with reading the engine's OWN start/stop latch via `tm.get_start_stop_rx()` → `*rx.borrow()`, mirroring the real `time_updater` loop in `app/mod.rs`. Result: team timeouts count down, ref/penalty-shot timeouts count up, confirm pauses resolve. This is the correct model and is documented as a KNOWN COUPLING POINT in `golden/mod.rs`.
- **Watched-state extended by one field.** `render()` now includes `conf_pause=<none|Ns>` from `GameSnapshot::conf_pause_time` (plan scoped 4 fields; this is a 5th). Needed so the score-confirmation pause is observable (scenario `score_confirm_pause` was otherwise invisible).
- **Sudden-death scoring modeled faithfully.** Added `Action::ScoreSuddenDeath(Color)` (→ `pause_for_confirm`) and `Action::ConfirmScore(Color)` (→ `set_scores` + `end_confirm_pause`) so SD goals go through the real operator score-confirmation gate (per domain-expert instruction). Regulation/overtime goals still use plain `AddScore` (correct — they don't gate). Timed-overtime goals do NOT end the game (confirmed correct with the domain expert); only SD goals end it.
- **Scenario count 20 → 30.** User approved expanding mid-execution for interaction coverage (penalty+timeout, timeout+penalty, manual-edit+active-state, OT/SD interactions, action-at-exact-transition). Kept the original 20 names unchanged and added 10; dropped two relayed duplicates (`goal_during_overtime_ends_game` = existing `overtime_score_ends`; `regulation_to_between_games` = covered by `regulation_full`). Domain note pinned: team timeouts panic in OT — use ref timeout there.
- **Harness signature.** `check_or_bless(name, trace, bless: bool)` (not the plan's 2-arg form); `bless` is computed from `UPDATE_GOLDEN` at the test call site. Chosen over `unsafe { env::set_var }` to avoid Rust-2024 unsafe + parallel-test races.
- **Task 5 de-risked.** Verified all 13 engine symbols the driver calls + the `conf_pause_time` snapshot field ALL exist at baseline `46ec0973`, so the driver should compile against the baseline without trimming to a "common subset." A scenario behaving differently at baseline because a feature post-dates it is a legit Task-6 "intended change," not a compile failure.
- **Process.** Lean process followed (test-only `refbox` code): no per-task subagent code-review; the safety-critical fidelity claims (tick mirrors `app/mod.rs` could_end_game/pause/update order; latch model) were verified by the controller directly. Full code review deferred to PR (Task 7 / pre-PR).
