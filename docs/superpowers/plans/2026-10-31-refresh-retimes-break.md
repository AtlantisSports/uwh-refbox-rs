# Refresh Re-times the Break Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Between games, a freshly received schedule whose next-game start time differs from the one the break is counting down to re-times that break.

**Architecture:** One new engine method on `TournamentManager` stores the next game (as `set_next_game` does) and, when the start moved and a between-games countdown is running, restarts the countdown from the new start — keeping the moment the old score clears fixed. The app's `Message::RecvSchedule` handler calls it in place of `set_next_game` and pushes a fresh snapshot when it re-timed. All decision logic lives in the engine so it is unit-testable (app `Task`s cannot be observed by tests).

**Tech Stack:** Rust 2024, MSRV 1.85, iced 0.13, `time::OffsetDateTime`.

**Spec:** No separate spec document — the behaviour was agreed with the PO in chat on 2026-10-31 and is restated in "Agreed behaviour" below. Origin: PO report from the 2026 Milwaukee Halloween Tournament (portal timezone fixed and re-published mid game 1; every start moved 1 h earlier; the break after game 1 still counted ~1 h). PO ruling: "a refresh should re-pull and fix this sort of issue".

## Agreed behaviour (PO, 2026-10-31)

- Trigger: any schedule received between games — REFRESH, and the automatic fetch at game end.
- Re-time **only when the next game's start time actually changed** (PO chose option 1). A schedule with the same start leaves the clock alone, so an operator's TIME EDIT survives unchanged refreshes.
- Earlier start shortens the break, later lengthens it; a start already past or too close floors at the event's minimum break (existing `calc_time_to_next_game` rule).
- A different next game (reordered schedule) counts down to that game's start.
- Silent: log line only, no new screen or message.
- During a game: nothing changes (the end-of-game fetch picks it up).
- Claude's calls, shown to and approved by the PO: the old score clears at the **same moment** it would have anyway (except when the new break is shorter than the time left before the clear: then it clears at kickoff, as `end_game` already does for short breaks). The court-finished revive path is unchanged.
- **SUPERSEDED by the re-ask below (Fix round 2).** ~~Stopped break clock (PO ruling, 2026-10-31, after the final review):~~ in hockey mode the break clock is only stopped while TIME EDIT is open (the stop button exists only in rugby mode). If the moved start arrives then, the clock is left exactly as it is and the operator sets the time. The new start is still stored, so later breaks are timed from the corrected schedule; that one break is not re-timed by a later REFRESH either (accepted).
- **Stopped break clock, re-asked (PO, 2026-10-31 ~16:3x, option 1 "Fix it on resume"):** the earlier premise was false — in hockey, REF TIMEOUT works between games and stops the break clock. A refresh leaves a stopped break countdown alone; once it runs again, the next refresh re-times the break to the moved start. **Confirmed by the PO after the second pre-PR review (option 2, "Keep it as walked"): the owed re-time is paid by the next REFRESH or game-end fetch, NOT at the moment the timeout ends.** A TIME EDIT cancels it only when it changes the break time.

## Global Constraints

- MSRV 1.85: **no let-chains** (`if let … && …`), they pass every local gate but break MSRV. `let … else` is fine.
- No `unwrap()`/`expect()` in production code without a comment saying why it cannot panic.
- Clippy `-D warnings`; no `#[allow]`.
- **Coordinator GO (lane rule 9):** before ANY cargo build/test/clippy/check, message the coordinator and wait for GO; tell it "heavy done" after. One command at a time.
- Tests: `cargo test -p refbox --lib -- --test-threads=1 <filter>` (this machine has no sound card; parallel refbox tests fail randomly on ALSA).
- Helpers leave changes **uncommitted**; never commit or push.
- Scope: `refbox/src/tournament_manager/mod.rs` and `refbox/src/app/mod.rs` only. No `uwh-common`, no translations, no UI.

## Review Focus

1. **A re-publish that does not move the next game** (organiser edits a team name) — must not touch the clock; an operator's TIME EDIT survives. Test: `retime_leaves_an_unchanged_start_alone`.
2. **Start moved to before now** (fix arrives late into the break) — floor at `minimum_break`, never zero, never a panic. Test: `retime_floors_a_past_start_at_minimum_break`.
3. **Re-time shorter than the remaining score-display time** — score clears at 0:00 at the latest, no underflow. Test: `retime_shorter_than_the_score_display_clears_by_kickoff`.
4. **Schedule arriving mid-game or during a timeout** — stores the info, clock untouched. Test: `retime_does_nothing_during_a_game`.
5. **Same instant written in a different UTC offset** (the Milwaukee fix changed the zone) — `OffsetDateTime` equality compares instants, so an offset-only change is "unchanged". Test: `retime_treats_the_same_instant_in_another_offset_as_unchanged`.

---

### Task 1: Engine method `set_next_game_and_retime`

**Files:**
- Modify: `refbox/src/tournament_manager/mod.rs` — new method directly after `set_next_game` (~line 646); tests in the existing `mod test` (~line 3814), near `an_ordinary_mid_break_game_change_leaves_the_reset_point_alone` (~line 12236).

**Interfaces:**
- Produces: `pub fn set_next_game_and_retime(&mut self, info: NextGameInfo, now: Instant) -> bool` — always stores `info` exactly as `set_next_game` does; returns `true` only when it re-timed a running break.

**Rule the method implements:**
1. `old = self.next_game.as_ref().and_then(|i| i.start_time)`; `new = info.start_time`; call `self.set_next_game(info)`.
2. Return `false` unless all hold: `new.is_some()` and `new != old`; `current_period == BetweenGames`; `timeout_state.is_none()`; `clock_state` is `ClockState::CountingDown { .. }`.
3. `remaining_now = self.game_clock_time(now)` (return `false` on `None`). `until_clear = remaining_now.saturating_sub(self.reset_game_time)`.
4. `new_remaining = self.calc_time_to_next_game(now, now)` (applies the minimum-break floor and `MAX_TIME_VAL` cap).
5. `clock_state = CountingDown { start_time: now, time_remaining_at_start: new_remaining }`. If `!self.has_reset`, set `reset_game_time = new_remaining.saturating_sub(until_clear)` (keeps the score-clear moment fixed in wall time). Do NOT call `arm_mid_break_reset`. (Originally "do NOT send `send_clock_running`"; superseded by Fix round 2, F7: the re-time now sends it so the updater recomputes its wake.)
6. `info!` one line with old and new remaining; return `true`.

Doc comment: say why the start-changed check exists (an operator's TIME EDIT must survive refreshes that do not move the game — PO ruling 2026-10-31) and why the reset point is shifted rather than re-armed (see the comment in `apply_next_game_start`).

**Test notes:** `calc_time_to_next_game` reads `OffsetDateTime::now_utc()` and `Instant::now()` itself, so tests use `let now = Instant::now();` and starts as `OffsetDateTime::now_utc() + time::Duration::minutes(n)`, and assert remaining times within **2 s** tolerance. Set up a running break with `tm.set_next_game(old_info)`, `tm.set_period_and_game_clock_time(GamePeriod::BetweenGames, d)`, `tm.start_clock(now)` (as the existing reset-point tests do). Config: `minimum_break: 60 s`, `post_game_duration: 120 s`.

- [x] **Step 1: Write the failing tests** (all in `mod test`, each calling `initialize()`):
  - `retime_moves_an_earlier_start_into_the_running_break` — break running 3600 s, next game "2" with start +60 min; call with game "2" start +5 min → returns `true`; `game_clock_time(now)` ≈ 300 s (±2 s). *(The Milwaukee case.)*
  - `retime_moves_a_later_start_into_the_running_break` — +5 min → +20 min → `true`, ≈ 1200 s.
  - `retime_leaves_an_unchanged_start_alone` — break running 1000 s (≠ the start, standing in for a TIME EDIT), start +60 min; call with the same start → `false`, `game_clock_time(now) == 1000 s` exactly.
  - `retime_floors_a_past_start_at_minimum_break` — new start −10 min → `true`, `game_clock_time(now) == 60 s`.
  - `retime_follows_a_different_next_game` — game "2" at +60 min → game "3" at +10 min → `true`, ≈ 600 s, `next_game_number() == "3"`.
  - `retime_keeps_the_score_clear_moment` — break 600 s started at `now`, `reset_game_time == 480 s`; at `now + 30 s` call with a start giving ≈ 300 s → `reset_game_time == game_clock_time(now+30s) − 90 s` exactly (90 s = 570 − 480).
  - `retime_shorter_than_the_score_display_clears_by_kickoff` — same setup, new start in the past (→ 60 s) → `reset_game_time == 0`, no panic.
  - `retime_leaves_a_stopped_break_clock_stopped` — break clock stopped at 900 s, start moves → `false`, still stopped at 900 s, `next_game_info()` holds the new start.
  - `retime_does_nothing_during_a_game` — period `FirstHalf`, clock running → `false`, clock unchanged, info stored.
  - `retime_ignores_a_schedule_with_no_start` — new `start_time: None` → `false`, clock unchanged.
  - `retime_treats_the_same_instant_in_another_offset_as_unchanged` — old start `t`, new start `t.to_offset(UtcOffset::from_hms(-5, 0, 0)?)` (not the `offset!` macro — the `time` macros feature may not be enabled) → `false`.

- [x] **Step 2: (GO first) run and see them fail** — `cargo test -p refbox --lib -- --test-threads=1 retime_`. Expected: compile error, `set_next_game_and_retime` not found.
- [x] **Step 3: Implement the method** per the rule above.
- [x] **Step 4: (GO first) run and see them pass** — same command; 11 passed (13 after the final-review tests). Then the whole engine module: `cargo test -p refbox --lib -- --test-threads=1 tournament_manager` — all green.
- [x] **Step 5: Leave uncommitted;** the main session reviews and commits `fix(refbox): re-time a running break when the next game's start moves`.

### Task 2: Wire it into `RecvSchedule` (main session — two edits)

**Files:** Modify `refbox/src/app/mod.rs`, `Message::RecvSchedule` handler, the `if let (Some(game), Some(timing)) = found` branch (~lines 7234–7262).

**Interfaces:** Consumes `set_next_game_and_retime` from Task 1.

- [x] **Step 1:** Hoist `let now = Instant::now();` above the `set_next_game` call; replace `tm.set_next_game(NextGameInfo { … })` with `let retimed = tm.set_next_game_and_retime(NextGameInfo { … }, now);`. The existing restore/leaving-finished branch reuses `now`.
- [x] **Step 2:** After that branch, add `if retimed { … }` that generates a snapshot (`tm.generate_snapshot(now)` — handle `None` without unwrap: skip the push), drops `tm`, pushes `self.apply_snapshot(snapshot)` onto `roster_tasks` and returns `Task::batch(roster_tasks)` — the same shape as the restore branch, minus the config copy (the timing rule is not applied here).
- [x] **Step 3: (GO first)** `cargo clippy -p refbox --all-features -- -D warnings`, then `just check` (GO each). Known pre-existing `--all-targets` reds: `player_grid.rs:126`, `app/mod.rs` ~8923 — not ours.
- [x] **Step 4:** Commit `fix(refbox): re-time the break when a fetched schedule moves the next game`.

### Task 3: Walk rig + walkthrough (main session; rig untracked)

- [x] Copy `coordination/walk-mock/next-game-timing.py` to `coordination/walk-mock/retime-break.py` (port 8093): game 1 starts ~2 min after server start with short halves; game 2 starts +60 min. Add `GET /move?game=2&minutes=N` that shifts game 2's `startsOn` by N minutes and prints the new value. Never committed.
- [x] Book refbox with the coordinator; rebuild (GO); launch with `UWH_PORTAL_URL_OVERRIDE` + own `XDG_CONFIG_HOME`.
- [x] Walk steps for the PO (each must be able to fail): (a) during game 1 move game 2 to +5 min → break after game 1 ends ≈ 5 min (end-of-game fetch); (b) mid-break move to +15 min, press REFRESH → countdown jumps to ≈ 15 min; (c) TIME EDIT the break, REFRESH with no move → the edit stays; (d) move into the past, REFRESH → minimum break.
- [x] Close refbox, tell the coordinator "refbox free".

### Task 4: Pre-PR (main session)

- [x] Code review (`code-review` skill against `origin/master...HEAD`, run from the worktree), plus the `.claude/rules/pr-review.md` checklist. Fix or answer every finding. (Run twice: before and after Fix round 2.)
- [ ] Dangling-work sweep; rebase onto `origin/master` first (master moves fast); disclosure; ask the PO to push.

### Fix round 2: owed re-time + pre-PR review findings F4–F7 (implementer, one task)

**Files:** Modify `refbox/src/tournament_manager/mod.rs` (engine + tests), `refbox/src/app/mod.rs` (RecvSchedule block, ~7234-7290).

**Design: an owed re-time flag** (chosen over "record the start each countdown was timed against": same failure class, but it leaves every walked case and the existing test fixture exactly as they are).
- New private field `retime_owed: bool` on `TournamentManager`, initialised `false`, with a comment saying what it means: a moved start arrived between games while the break clock could not be re-timed.
- `set_next_game_and_retime`: `moved = new_start.is_some() && new_start != old_start`. Between games, if `moved` but a timeout is active or the clock is not `CountingDown`: set `retime_owed = true`, return `false`. The re-time now runs when `(moved || self.retime_owed)`, `new_start` is `Some`, between games, no timeout, clock `CountingDown`. On re-time: `retime_owed = false`, and `self.send_clock_running(true)` so the updater recomputes its next wake (F7). Update the doc comment.
- Clear `retime_owed = false` wherever the break is set afresh from the schedule or by the operator: `end_game`, `apply_next_game_start`, `start_game`, `wait_for_scheduled_game`, `set_no_next_game`, and `set_game_clock_time` when `current_period == BetweenGames` (TIME EDIT: the operator's time stands — Claude's call). Grep every other writer of a between-games `clock_state` and say in the report why each one does or does not clear it.
- App (F5): compute `restoring` BEFORE storing the info. When restoring, call plain `tm.set_next_game(..)` then `apply_next_game_start` (as before the branch); only the non-restoring path calls `set_next_game_and_retime`. Removes a redundant re-time and a false "start moved" log on every startup restore.
- App (F6): the comment over the call says it re-times a running break when the next game's start moved, or once the clock runs again after a move that arrived while it was stopped.

**Tests** (all named `retime_*`, using `retime_fixture` / `next_game_at`):
- F4: rewrite `retime_does_nothing_during_a_between_games_timeout` to enter the timeout with `tm.start_ref_timeout(now)`, not by assigning `timeout_state`.
- `retime_owed_from_a_ref_timeout_applies_after_it_ends`: fixture start 60 min, break 3600 s; `start_ref_timeout`; retime to 5 min → `false`, clock unchanged; `end_timeout(now)`; retime with the SAME 5-min info → `true`, `assert_about(.., 300)`.
- `retime_owed_from_a_stopped_clock_applies_once_it_runs`: `stop_clock`; retime(5) → `false`; `start_clock`; retime(5) → `true`, about 300.
- `retime_owed_is_paid_once`: continue the above; a further retime with the same info → `false`.
- `retime_owed_is_cancelled_by_time_edit`: `stop_clock`; retime(5) → `false`; `set_game_clock_time(900 s)`; `start_clock`; retime(5) → `false`, clock = 900 s.
- `retime_owed_is_cancelled_by_a_fresh_break`: `stop_clock`; retime(5) → `false`; `apply_next_game_start(now)`; retime(5) → `false`.
- `retime_wakes_the_clock_updater`: take `get_start_stop_rx()`, `borrow_and_update()`; a re-time → `has_changed()` is `Ok(true)`.

**Verify (each needs the coordinator's GO; one at a time):** `cargo test -p refbox --bins retime_ -- --test-threads=1` (all pass, 19 tests); `cargo clippy -p refbox --all-targets -- -D warnings` (only the 2 pre-existing reds: app/mod.rs ~8946, player_grid.rs:126). Then main session: re-review, `just check`, RE-WALK (state-machine change), pre-PR code-review.

**Review Focus for this round:** a ref timeout started while the break clock was already stopped (the clock stays stopped after it ends, so the move stays owed); a TIME EDIT cancelled rather than applied (owed survives — correct); no refresh after the timeout ends (break stays wrong until one — accepted by the ruling's wording).

## Branch

`fix/refbox/refresh-retimes-break`, worktree `/home/estraily/projects/uwh-refbox-rs/.claude/worktrees/fix+refbox+refresh-retimes-break` off `origin/master`. This plan is the branch's first commit at `docs/superpowers/plans/2026-10-31-refresh-retimes-break.md`.

## Deviations

- Test commands: `refbox` has no lib target, so every `cargo test -p refbox --lib` above is `cargo test -p refbox --bins`.
- Task 1: `retime_keeps_the_score_clear_moment` expects ~330 s, not ~300 s (it re-times at `now + 30 s` against a start 6 min out); the exact reset-point assertion is as planned.
- Task 2 Step 2 shipped as `else if retimed` beside the restore branch; the final review then merged both into one snapshot tail.
- Final review (code-review skill) added: two tests (`has_reset` already true; a between-games timeout) and a corrected doc comment on when the old score clears.
- Walk rig is `coordination/walk-mock/retime-break.py` on port 8093 (`/at`, `/move`, `/show`), untracked.
- **Open (pre-PR review, 2026-10-31):** the stopped-clock ruling above rests on a premise that turned out false — in hockey the REF TIMEOUT button works between games and stops the break clock, so a moved start arriving during a between-games ref timeout is stored, never applied, and later REFRESHes see "unchanged". Re-asking the PO with the corrected premise before the push. Smaller follow-ups queued with it: no misleading "start moved" re-time on a startup restore; wake the clock updater after a re-time; replace the unreachable between-games-timeout test with one through `start_ref_timeout`. Parked as a suggestion: a REFRESH during a game does not hand the corrected start to the engine (out of scope; relies on the game-end fetch).
- **Fix round 2 (2026-10-31):** an owed re-time flag instead of recording the start each countdown was timed against (same failure class; keeps every walked case and the test fixture as they were). The implementer also clears it in `reset_game`, `start_nominal_break` and `reset_for_site_switch` (each sets the break afresh). Re-walked 7/7.
- **Second pre-PR review (2026-10-31):** a TIME EDIT clears an owed re-time only when the break time actually changes (DONE re-sends it unchanged after a timeout-only edit); a missing snapshot after setting the break from the schedule now logs a warning. Answered without change: the floor at minimum break (agreed behaviour), None-to-Some counting as moved and the current game's minimum break (both ruled at the first final review).
