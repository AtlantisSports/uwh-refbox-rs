# Rugby penalty shot ending after the second half — freeze fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When a rugby penalty shot that kept the second half (or the second overtime half) open at
0:00 runs out on its own, open the score-confirmation window, exactly as tapping END TIMEOUT does
today, instead of freezing the clock, LED panel and stream overlay.

**Architecture:** One engine file. A rugby shot that runs out while its half waits at 0:00 is turned
into "the half ran out at the instant the shot ended": the game clock becomes an expired countdown
ending at the shot's end, the shot is cleared, and the ordinary score-confirmation window opens.
That's the same move `handle_rugby_pen_shot_end` already makes for a timed sudden death (PR #4554).
Both routes that reach this state use it: the clock updater's tick (`pause_for_confirm`) and the shot-end
handler (`handle_rugby_pen_shot_end`), so the handler's tail `duration(...).unwrap()` can no longer
be reached after the game has ended.

**Tech Stack:** Rust 2024, MSRV 1.85, `refbox` crate only.

**Spec:** this plan carries the behaviour rules itself (PO choice 2026-11-25, "prove with a test,
then a fix plan"). Evidence: `docs/backlog/rugby-penalty-shot-wedges-the-clock/NOTE.md` (untracked,
main checkout), section "2026-11-25 — PROVEN by test".

## Global Constraints

- MSRV 1.85: no let-chains (`if let … && …`), nothing newer than 1.85 (the toolchain is pinned
  newer, so the compiler will NOT catch it).
- Heavy process (`.claude/rules/plan-execution.md`): this is the game state machine. Per-task review,
  golden traces must stay byte-identical, `just check` before the PR.
- No `unwrap()`/`expect()` added to production code without a comment saying why it cannot fail.
- Every cargo job: `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4`, and only after the refbox
  coordinator's GO.
- Branch: `fix/refbox/rugby-shot-end-freeze`, made from the probe worktree
  `~/projects/uwh-refbox-rs-rugby-probe` (master 1fbfceb3), rebased onto fresh master before the PR.
  Delete the two `probe_` tests and `rugby_probe_extended_second_half` before the first commit:
  Task 1's tests replace them.

## Behaviour rules

1. A rugby shot runs out on its own while the second half (or the second overtime half) waits at
   0:00 → the score-confirmation window opens **whatever the score**. That's what happens today at
   the end of every second half without a shot. The window starts at the instant the shot ended, not
   at the next clock tick.
2. After the window: a decided score ends the game; a level score goes to overtime / sudden death /
   ends the game by the existing rules (`end_confirm_pause`, unchanged).
3. Unchanged: a shot that runs out during an extended **first** half (or the first overtime half)
   still goes straight to half time with no window. Ending the shot by hand with END TIMEOUT is unchanged in
   every period. The timed sudden death arm is unchanged in behaviour.

## Review Focus

- A level score at the end of the second half with overtime allowed: the window must open, then
  go to pre-overtime (not end the game).
- The second overtime half (OvertimeSecondHalf), decided: same as the second half.
- The confirm window's start is the shot's end (g+50 below), not the tick that noticed it (g+51),
  so a late tick doesn't lengthen the break.
- A clock tick at the very instant the shot ends (`now == shot end`) must take the new path, not the
  old error.
- The first-half extension (`(FirstHalf, true)`) must still go straight to half time: the golden
  trace `rugby_penalty_shot_expires` pins it and must stay byte-identical.

---

### Task 1: Open the confirm window when the shot ends a waiting second half

**Files:**
- Modify: `refbox/src/tournament_manager/mod.rs`: `pause_for_confirm` (~3340),
  `handle_rugby_pen_shot_end` (~2809), and its test module (~4266).

**Interfaces:**
- Produces (private, same file): `fn end_rugby_shot_at_period_zero(&mut self, now: Instant,
  start_time: Instant, time_remaining_at_start: Duration)`. Sets `self.clock_state =
  ClockState::CountingDown { start_time: min(now, start_time + time_remaining_at_start),
  time_remaining_at_start: Duration::ZERO }` and `self.timeout_state = None`. Nothing else.

- [ ] **Step 1: Write the failing tests** (test module, next to `updater_tick_normal_game_is_unchanged`)

Shared setup `fn rugby_shot_outlasts_half(period: GamePeriod, decided: bool) -> (TournamentManager,
Instant)`: `GameConfig { penalty_shot_duration: 45 s, ..Default::default() }` (default allows
overtime). `start_play_now(g)`, `stop_clock(g+1)`, `set_period_and_game_clock_time(period, 10 s)`,
`start_clock(g+2)`. If `decided`, `add_score(Color::Black, 1, g+3)`. `start_rugby_penalty_shot(g+5)`
(runs out at g+50). `updater_tick(g+13)` is `Ok`, and afterwards the period is unchanged and
`clock_state == Stopped { ZERO }` (the half is waiting).

Tests (each through `updater_tick`, the shipping path):
- `rugby_shot_ending_a_decided_second_half_opens_the_confirm_window`: `updater_tick(g+51)` →
  `Ok((TickKind::ConfirmScores, _))`; `in_score_confirm_pause()`; `timeout_state == None`;
  `time_pause_confirmation.unwrap().pause_began == g+50`. Then tick at g+50 + the pause duration
  + 1 s → `TickKind::AutoConfirmScores` and `current_period() == BetweenGames`.
- `rugby_shot_ending_a_level_second_half_opens_the_window_then_overtime`: `decided = false`; tick
  g+51 → `ConfirmScores`; after the window → `current_period() == PreOvertime`.
- `rugby_shot_ending_a_decided_overtime_second_half_opens_the_confirm_window`:
  `OvertimeSecondHalf`, decided; tick g+51 → `ConfirmScores`.
- `rugby_shot_ending_exactly_on_a_tick_opens_the_confirm_window`: tick at g+50 exactly →
  `ConfirmScores`.
- `rugby_shot_end_handler_does_not_panic_after_a_decided_second_half`: covers the crash route
  directly. Decided second half waiting at 0:00; call `tm.update(g+51)`. Expect `Ok(())` and
  `in_score_confirm_pause()`. Today this panics at the tail `unwrap()`, so the test FAILS by
  panicking. That is the proof the route was real.

- [ ] **Step 2: Run them and see each fail**

Run (after GO): `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 cargo test -p refbox --bin refbox rugby_shot_ -- --nocapture`
Expected: the four tick tests FAIL with `PausingDuringTimeout`; the handler test FAILS with a panic
(`called Option::unwrap() on a None value`).

- [ ] **Step 3: Implement**

- Add `end_rugby_shot_at_period_zero` (Interfaces above). Make the existing timed sudden death arm of
  `handle_rugby_pen_shot_end` call it, in place of its inline clock assignment; its behaviour must
  not change.
- `pause_for_confirm`, top of the `!in_sudden_death` branch, before the guards: if `timeout_state`
  is `RugbyPenaltyShot(CountingDown { start_time, time_remaining_at_start })` with
  `now >= start_time + time_remaining_at_start`, and `clock_state` is `Stopped { ZERO }`, and the
  period is `SecondHalf | OvertimeSecondHalf`, call the helper. The guards then pass as for any half
  ending, and `pause_inst` becomes the shot's end. (No let-chains: nest the `if let`s.)
- `handle_rugby_pen_shot_end`, `SecondHalf` and `OvertimeSecondHalf` arms: call the helper and
  `return self.pause_for_confirm(now)` in place of `end_second_half` / `end_overtime_second_half`,
  only when the half would end the game. Use the same test `timeout_end_would_end_game` uses
  (scores not equal, or no overtime and no sudden death; for the overtime half, scores not equal or
  no sudden death). Otherwise keep today's call, which leads to pre-overtime / pre-sudden-death,
  where `duration` is `Some`. Add a one-line comment on the tail `unwrap()` saying why it can no
  longer see `BetweenGames`.
- Doc comment on the helper naming both callers and the PO rule ("opens the window as END
  TIMEOUT does").

- [ ] **Step 4: Run the new tests, the engine tests and the golden traces**

Run: `… cargo test -p refbox --bin refbox tournament_manager`
Expected: all pass. `golden` tests pass with NO trace file changes (`git status` shows no change
under `refbox/src/tournament_manager/golden/`).

- [ ] **Step 5: Commit**

`git add refbox/src/tournament_manager/mod.rs` then
`git commit -m "fix(refbox): open the confirm window when a rugby shot outlasts the half"`

### Task 2: Whole-branch checks

- [ ] Rebase onto fresh `origin/master`; `cargo audit` (rule 10).
- [ ] `just check` (after GO). Expected: green.
- [ ] Pre-PR checks per `.claude/rules/pr-review.md`: `code-review` on the diff; the PO's walkthrough
  (below); Claude-driven check: not needed. The engine tests drive the shipping tick path, and
  the walk covers what's on screen.

**PO walkthrough (real time, no clock faking available here):** rugby penalty shot set to 0:45 in
settings; TIME EDIT the second half to 0:10 with Black 1–0; start the clock; tap RUGBY PENALTY SHOT
at about 0:05. Expected: the half clock holds at 0:00 while the shot counts down; when the shot
reaches 0:00, the score-confirmation screen opens by itself. Confirm → the game ends normally. Before
this fix, the screen would stay frozen at that point.

## Deviations

(none yet)
