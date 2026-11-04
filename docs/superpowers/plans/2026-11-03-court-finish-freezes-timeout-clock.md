# Court-finish freezes the ref timeout clock — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (native: one task,
> within the "tiny task stays in the main session" limit). Steps use checkbox (`- [ ]`) syntax.

**Goal:** A schedule refresh that finishes the court while a REF TIMEOUT is counting up between
games no longer freezes the timeout clock on screen.

**Architecture:** `set_no_next_game` parks the break at 0:00 and tells the time updater "the
clock stopped" whenever `clock_is_running()` is true. Between games under a counting-up ref
timeout that call reports the TIMEOUT clock, so the updater is told "stopped" while the timeout
clock still runs, and stops redrawing it. Keep the park (PR #4144's `court_is_parked` and the
order-A revive depend on the break sitting at 0:00), but only send "stopped" when nothing on
screen is still running. The updater must then hear "stopped" when that timeout ends on a
finished court and the refused restart leaves nothing running, so `end_timeout` sends it there.

**Tech Stack:** Rust 2024, refbox crate, `TournamentManager` (refbox/src/tournament_manager/mod.rs).

**Spec:** PO rulings 2026-11-03 (engine session): problem 4 = fix on its own branch; problem 1
moved to branch C; problem 5 = leave as is. Caller audit: every other `clock_is_running` caller
means "the clock on screen" and is correct.

**Base:** `origin/master` AFTER PR #4144 merges (it rewrites `end_timeout`'s top and adds
`court_is_parked`/`revive_court`, which this fix must stay compatible with). Branch
`fix/refbox/court-finish-freezes-timeout-clock`, own worktree. Build with `CARGO_BUILD_JOBS=4`,
coordinator GO; a fresh-worktree build is announced to the coordinator first.

## Global Constraints

- MSRV 1.85 / edition 2024; clippy `-D warnings`; no new deps; no `unwrap()` outside tests.
- Engine-only change: `refbox/src/tournament_manager/mod.rs`. No app, UI, translation or `uwh-common` change.
- Heavy process (state machine): run the whole `tournament_manager` test module, then `just check`.

## Review Focus

1. Order A (#4144): break running → REF TIMEOUT → finishing REFRESH → reviving REFRESH → END REF
   TIMEOUT must still count the owed break, not kick off at 0:00 — #4144's order-A tests must stay green.
2. A finishing refresh during a plain running break (no timeout) must still tell the updater
   "stopped" — existing test at the `set_no_next_game` park (`rx.has_changed()` + `!borrow`).
3. Ending the ref timeout on a finished court with no revive: break stays at 0:00, no game
   starts, updater told "stopped" (else the hot loop `set_no_next_game`'s comment describes) — Task 1 test B.
4. A court flagged finished at kickoff keeps its game clock — existing test
   `a_court_flagged_finished_at_kickoff_keeps_its_game_clock`.
5. A ref timeout that is STOPPED when the court finishes: no change (nothing was running).

---

### Task 1: Don't tell the updater "stopped" while a timeout clock still runs

**Files:**
- Modify: `refbox/src/tournament_manager/mod.rs` — `set_no_next_game` park block; `end_timeout`
  Ref/PenaltyShot `CountingUp` arm (and the Team `CountingDown` arm only if a team timeout can
  exist between games — check `can_start_team_timeout`).
- Test: same file, `#[cfg(test)]` module, next to the existing `set_no_next_game` park test.

- [x] **Step 1: Write failing tests**
  - A `a_finishing_refresh_under_a_running_ref_timeout_keeps_the_timeout_clock_running`:
    BetweenGames, next game "11", break 150 s counting down, `start_ref_timeout` (counting up);
    take `get_start_stop_rx()`, `borrow_and_update()`; `set_no_next_game()`. Assert
    `tm.clock_is_running()` (timeout clock), `game_clock_time == Some(0)` (break parked), and the
    updater was NOT told stopped: `!rx.has_changed().unwrap() || *rx.borrow_and_update()`.
  - B `ending_that_timeout_on_a_finished_court_parks_and_tells_the_updater`: continue A;
    `borrow_and_update()`; `end_timeout(later)`. Assert period `BetweenGames`,
    `!tm.clock_is_running()`, `game_clock_time == Some(0)`, `rx.has_changed().unwrap()` and
    `!*rx.borrow_and_update()`.
- [x] **Step 2: Run, confirm A fails** (updater told stopped) — B may pass or fail; record which.
  `CARGO_BUILD_JOBS=4 cargo test -p refbox --lib tournament_manager -- --test-threads=1 finishing_refresh_under ending_that_timeout`
- [x] **Step 3: Fix.** In `set_no_next_game`, keep the park condition and parking; replace the
  unconditional `self.send_clock_running(false)` with a send only when `!self.clock_is_running()`
  after parking, and update its comment. In `end_timeout`'s `CountingUp` arm, after clearing the
  timeout, `send_clock_running(false)` when `!self.clock_state.is_running()`.
- [x] **Step 4: Run the whole module:** `CARGO_BUILD_JOBS=4 cargo test -p refbox --lib tournament_manager -- --test-threads=1` — all green.
- [x] **Step 5: `just check`** (coordinator GO) — green.
- [x] **Step 6: Commit** `fix(refbox): keep the ref timeout clock running when the court finishes`

## Deviations

- Test command: refbox has no lib target, so `cargo test -p refbox --bin refbox` replaces
  `--lib` (same tests).
- Team `CountingDown` arm of `end_timeout` left unchanged: `can_start_team_timeout` allows team
  timeouts only during a half, so none exists between games where the restart can be refused.
- #4144's `ending_a_running_timeout_on_an_owed_break_wakes_the_updater` asserted, as a fixture
  check, that parking told the updater "stopped" under a running ref timeout — problem 4
  itself. Rewritten as `..._keeps_the_updater_running`: the updater agrees with the engine
  before and after the timeout ends.
