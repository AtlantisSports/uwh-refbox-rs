# TIME EDIT Holds a Schedule Reply — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Status:** APPROVED by the PO 2026-11-06 as written (option 1: an untouched APPLY with a held reply behaves like CANCEL). Written 2026-11-05 by the engine lane. Branch
`fix/refbox/time-edit-holds-schedule-reply` NOT created yet (needs PO OK). On approval, copy this file into the
worktree's `docs/superpowers/plans/2026-11-05-time-edit-holds-schedule-reply.md` as the first commit.

**Goal:** A Portal/Custom schedule reply that lands while TIME EDIT is open no longer moves any clock (and so can no
longer crash APPLY); it is held and acted on the moment TIME EDIT closes, as if it arrived then.

**Architecture:** App-only (`refbox/src/app/mod.rs`). `Message::RecvSchedule` is parked in a new `held_schedule` field
while `AppState::TimeEdit` is showing. `Message::TimeEditComplete` with a held reply runs in this order: put the clocks
back exactly as TIME EDIT found them → restore the page → replay the held reply through `self.update` → re-apply only
the clock values the operator actually changed. No engine code changes.

**Tech Stack:** Rust 2024, iced 0.13, refbox crate.

**Spec:** PO ruling 2026-11-05 in `coordination/rulings.md` ("TIME EDIT is a PAUSE"). Worked example agreed by the PO:
parked court, reply "game 13 in 15:00", operator leaves TIME EDIT 2 s later → CANCEL counts 14:58 to game 13; APPLY
10:00 counts 10:00 to game 13.

**Bug being fixed (proven 2026-11-05 by a throwaway engine probe on origin/master 5aa232a2):** parked court (break
stopped at 0:00, no next game) → `Message::EditTime` (clock stays stopped, page holds 0:00) → `RecvSchedule` revives the
court (`revive_court` → `apply_next_game_start` starts the break) → APPLY → `set_game_clock_time` returns
`Err(ClockIsRunning)` → `.unwrap()` at app/mod.rs ~4664 PANICS. Two routes: (a) REFRESH then TIME EDIT before the
reply; (b) **app start-up while Portal-linked**: `RefBoxApp::new` parks the clock at 0:00 with no next game
(app/mod.rs ~4492-4500), so the FIRST schedule download is a revive — TIME EDIT during that download + APPLY crashes.
The restore path (`restoring` → `apply_next_game_start`, ~7579) starts the clock the same way.

## Global Constraints

- MSRV 1.85: NO let-chains (`if let … && …`); every gate passes with them (toolchain is 1.96) — reject in review.
- `cargo clippy -D warnings`; no new `unwrap()`/`expect()` without a "why this cannot panic" comment.
- Every cargo job: coordinator's heavy GO first, `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4`.
- No `uwh-common`, no engine (`tournament_manager`) production change, no translations, no UI change.
- Heavy process applies (clock/state behaviour): task review + PO walkthrough + code-review skill before push.

## Decision the PO must confirm at plan approval (flag it in the approval question)

**An APPLY that changed nothing behaves like CANCEL** when a reply was held. The PO ruled "APPLY = whatever is on the
screen"; on a parked court the screen shows the 0:00 it opened with. Re-applying an untouched 0:00 AFTER the held
revive would set the new break to 0:00 and kick game 13 off at once. This plan re-applies only values the operator
changed (the same rule `set_game_clock_time` already uses for an owed re-time: a DONE that changed nothing is not the
operator choosing a break time). Ask the PO: keep this, or apply the untouched value literally.

## Review Focus

1. Untouched APPLY on a parked court with a held reply → counts down to the scheduled start, never 0:00 kickoff.
2. APPLY that changed ONLY the timeout clock → the revived break countdown is not overwritten.
3. Two replies during one TIME EDIT → only the latest is replayed (each is a full schedule).
4. A held reply whose site changed meanwhile → still discarded by `reply_is_current` on replay (hold BEFORE that check,
   so the replay runs every existing check).
5. A held reply with no TIME EDIT change on a RUNNING break (ordinary between-games TIME EDIT) → after CANCEL the clock
   runs again, then the reply's re-time applies exactly as an un-held reply would.

---

### Task 1: Hold the reply during TIME EDIT, replay it on exit

**Files:**
- Modify: `refbox/src/app/mod.rs` — struct field (~177), `new` init (~4341), `Message::TimeEditComplete` (~4659),
  top of `Message::RecvSchedule` (~7389), plus a pure fn + its tests in this file's test module.
- Test: `refbox/src/tournament_manager/mod.rs` test module (the revived-court tests, ~14140-14310) — engine sequence
  tests only, no production change there.

**Interfaces:**
- Produces: field `held_schedule: Option<(EventId, Schedule, u64)>` on `RefBoxApp` (the `RecvSchedule` payload,
  message.rs:241), initialised `None`.
- Produces: `fn time_edit_changes(canceled: bool, old: (Duration, Option<Duration>), new: (Duration, Option<Duration>)) -> (Option<Duration>, Option<Duration>)`
  — free fn in app/mod.rs; returns (game clock to set, timeout clock to set), each `Some` only when not canceled AND
  that value differs from `old`. `old` is `self.time_edit_old` (which the EndTimeout hold at ~7258 may already have
  moved — that is intended).

- [ ] **Step 1: Write the failing tests**

Pure fn (app/mod.rs tests):
```rust
#[test]
fn time_edit_changes_returns_only_what_the_operator_changed() {
    let s = Duration::from_secs;
    assert_eq!(time_edit_changes(true, (s(0), Some(s(30))), (s(600), Some(s(5)))), (None, None));
    assert_eq!(time_edit_changes(false, (s(0), None), (s(0), None)), (None, None));
    assert_eq!(time_edit_changes(false, (s(0), Some(s(30))), (s(600), Some(s(30)))), (Some(s(600)), None));
    assert_eq!(time_edit_changes(false, (s(0), Some(s(30))), (s(0), Some(s(5)))), (None, Some(s(5))));
}
```
Engine sequence tests (tournament_manager tests, reuse `parked_court`, `revived_game`, `assert_counting_down`; the
fixture's minimum break is 60 s) — they pin the ORDER the app will use:
- `time_edit_cancel_then_held_revive_counts_to_the_start`: parked; `stop_clock(start)` Ok (EditTime on a parked
  clock); `revive_court(revived_game("13"), start + 2s) == Ok(true)`; `assert_counting_down(&tm, 60)`.
- `time_edit_apply_after_held_revive_keeps_the_operator_time`: parked; revive at `start`; then `stop_clock(start+2s)`
  Ok; `set_game_clock_time(600s) == Ok(())`; `start_clock(start+2s)`; `assert_counting_down(&tm, 600)`.
- `apply_before_revive_is_what_crashed`: parked; revive; `set_game_clock_time(Duration::ZERO) == Err(TournamentManagerError::ClockIsRunning)`
  (documents the crash this plan removes; the old app order).

- [ ] **Step 2: Run** `cargo test -p refbox --bin refbox time_edit_` (heavy GO) — expect the pure-fn test to FAIL to
  compile (fn missing); the engine tests pass already (they pin existing engine behaviour the new order relies on).

- [ ] **Step 3: Implement**
  - `time_edit_changes` as specified.
  - Top of `Message::RecvSchedule`, BEFORE `reply_is_current`: if `matches!(self.app_state, AppState::TimeEdit(..))`,
    `info!` that the schedule is held until TIME EDIT closes (say if it replaced an earlier held one), store it in
    `held_schedule`, `return Task::none()`.
  - `Message::TimeEditComplete`: if `self.held_schedule.take()` is `None`, the existing body runs UNCHANGED. If
    `Some(held)`: (1) lock tm; if `was_running`, `start_clock(now)` + `update(now)` — values NOT applied yet;
    snapshot → `apply_snapshot`; drop tm; (2) `self.app_state = self.last_app_state.clone()` (must precede the
    replay, or the replay is held again); (3) `let replay = self.update(Message::RecvSchedule(e, s, i))`;
    (4) `let (game, timeout) = time_edit_changes(canceled, self.time_edit_old, (game_time, timeout_time))`; if either is
    `Some`: lock tm, `let running = tm.clock_is_running()`, `stop_clock(now)`, set the `Some` values, if `running`
    `start_clock(now)` + `update(now)`, snapshot → `apply_snapshot`; (5) return `Task::batch` of all tasks.
    Use `?`-free error handling: any `Err` from the engine is logged with `error!`/`warn!`, never unwrapped
    (the crash class is exactly an unwrap meeting a state it did not expect).

- [ ] **Step 4: Run** `cargo test -p refbox --bin refbox` (heavy GO) — all pass; then `just lint`.

- [ ] **Step 5: Commit** `fix(refbox): hold a schedule reply while TIME EDIT is open` (stage the two files by name).

## Verification

- **Leg 2 (PO walk, mandatory — visible effect):** the crash needs a reply to land DURING TIME EDIT, which a live
  portal answers too fast to hit by hand. Use a mock portal whose schedule reply is DELAYED ~15 s (check
  `.walk-mock/` and the archived walk rigs under `docs/audit-archive/` for a delay knob before writing one), isolated
  with its own `XDG_CONFIG_HOME` and `UWH_PORTAL_URL_OVERRIDE`. Steps: launch → open TIME EDIT during the delayed first
  download → wait for the reply (log line "held") → (a) CANCEL: break counts to the game; relaunch → (b) APPLY with
  10:00: counts from 10:00; relaunch → (c) APPLY untouched: counts to the game, no kickoff. Each step can fail: on
  master (a)-(c) crash or kick off.
- **Leg 3:** not needed beyond the probe; the walk covers the wiring a unit test cannot reach (no RefBoxApp test
  harness exists — verified 2026-11-05).
- `just check`, `cargo audit` after rebase, code-review skill on the worktree diff (give it the worktree path + base
  SHA explicitly).

## Deviations

(none yet)
