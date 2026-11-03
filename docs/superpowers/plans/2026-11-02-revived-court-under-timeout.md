# Revived Court Under a Timeout: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When a REFRESH brings a game back to a finished court while a timeout is on screen, the
refbox records the game but starts no countdown; when the timeout ends, the break counts down to
that game's scheduled start (never shorter than the minimum break), exactly as a revived court with
no timeout does.

**Architecture:** The engine (`TournamentManager`) gains one owed-break flag and two methods: one
that says whether the court is parked, and one that adopts a game on a parked court, deferring the
countdown while any timeout exists. `end_timeout` pays the owed break by calling the existing
`apply_next_game_start`, instead of resuming the parked 0:00 clock. The app's schedule-receive
handler swaps its timeout-blind "is the court leaving the finished state" test for the engine's.

**Tech Stack:** Rust 2024, MSRV 1.85 (no let-chains), iced 0.13; `refbox` crate only.

**Spec:** this plan's "Approved behaviour" section. It was approved by the PO in chat on 2026-11-02,
and the rulings are in `~/.claude/projects/-home-estraily-projects-uwh-refbox-rs/coordination/rulings.md`
(entries dated 2026-11-02).

## Approved behaviour

- Setting: a court is finished (no next game; break clock parked at 0:00), a timeout is on screen,
  and a schedule arrives that names a next game for this court.
- The game is recorded at once (the next-game number shows), but no countdown starts while the
  timeout exists.
- When the timeout ends, the break counts down to that game's scheduled start, never shorter than
  the minimum break, and the game starts by itself when it reaches 0:00.
- If the timeout runs past the scheduled start, the break is the minimum break counted from the
  moment the timeout ends.
- **Unchanged:** a revived court with no timeout; the 2026-10-31 ruling that a start moved under a
  timeout on a *running* break waits for the next REFRESH (`retime_owed`); everything mid-game;
  RESTART.

## Proven faults on master (probe, 2026-11-02, origin/master 638edf30)

- **A.** A break is counting down → REF TIMEOUT (its clock counts up) → a REFRESH finishes the court
  (`set_no_next_game` parks the clock at 0:00, because `clock_is_running()` reports the running
  timeout clock) → a second REFRESH revives it. The app's test
  `was_court_finished && !tm.clock_is_running()` is false, so the app takes `set_next_game_and_retime`,
  which only marks a re-time as owed. `end_timeout` → `start_game_clock` → a 0:00 countdown → the next
  tick starts the game.
- **B.** The court is finished → REF TIMEOUT (a *stopped* timeout clock, since nothing was running) →
  a REFRESH revives it. The test is true, so `apply_next_game_start` starts the break *under* the
  timeout. At 0:00 the game kicks off with `timeout_state` still `Ref(Stopped)`, so the first half runs
  while REF TIMEOUT shows, until the operator ends it.

## Global Constraints

- `refbox` crate only, in two files: `refbox/src/tournament_manager/mod.rs` and `refbox/src/app/mod.rs`.
- MSRV 1.85: no let-chains, and nothing newer than 1.85.
- No `unwrap()`/`expect()` in production code without a why-comment; no new dependencies.
- No new UI, no new translation keys.
- Every cargo command runs with `CARGO_BUILD_JOBS=4 ... -j 4`, and **only after the refbox
  coordinator's GO** (the session asks for it; a helper never starts cargo on its own).
- Run tests with `--test-threads=1` (there's no sound card here; parallel tests red at random on ALSA).

## Review Focus

1. **The owed flag outliving its moment.** A REFRESH that finishes the court again, a TIME EDIT of the
   break, START NOW, RESTART, a site switch or a reset during the owed state must each cancel it, so a
   later `end_timeout` never overrides them. Rule: clear `break_owed_after_timeout` at **every** site that
   clears `retime_owed` today (`grep -n 'retime_owed = false'`; 10 sites on 638edf30), plus
   `set_no_next_game`. Test: a TIME EDIT during the owed state survives `end_timeout`, and a re-finish
   during the owed state leaves the clock parked after `end_timeout`.
2. **The time updater not waking.** After `end_timeout` sets the deferred countdown, the updater must
   be told the clock now runs, or the screen freezes on 0:00. `apply_next_game_start` sends `true` only
   when `clock_is_running()` was false, which needs `timeout_state` cleared **before** it is called.
   Test: the `start_stop` receiver sees `true` after `end_timeout` (copy the pattern in
   `a_reset_on_a_finished_court_tells_the_updater_the_clock_stopped`).
3. **The old score not clearing early.** The deferred countdown must arm the mid-break reset
   (`reset_game_time` = remaining − `post_game_duration`), like `a_revived_court_clears_the_old_score_early_again`.
   Test it on the deferred path.
4. **The game's timing rules.** The app copies `tm.config()` into `self.config.game` only in the
   schedule handler, so `revive_court` applies the next game's `timing` to the config **at the refresh**,
   even when the countdown is deferred. Test: after a deferred revive, `tm.config()` equals the info's timing.
5. **Other timeout kinds.** Team timeouts and penalty shots cannot start between games today
   (`can_start_team_timeout` and `can_start_penalty_shot` refuse the period). The deferral keys on
   `timeout_state.is_some()` and the payoff is a guard at the top of `end_timeout`, so they are covered
   without branch-by-branch edits. No test is needed beyond A and B.

---

### Task 1: Defer a court's revival until the timeout ends

**Files:**
- Modify: `refbox/src/tournament_manager/mod.rs`: struct field and `new()`; new methods next to
  `set_next_game_and_retime` (~line 695); `end_timeout` (~1239); every `retime_owed = false` site;
  `set_no_next_game` (~769). Tests go in `mod test`.
- Modify: `refbox/src/app/mod.rs`: `Message::RecvSchedule`, the `leaving_finished_state` block (~7459–7525).

**Interfaces:**
- Produces: `pub fn court_is_parked(&self) -> bool`. True when `current_period == BetweenGames`,
  `no_startable_next_game()`, and `!clock_state.is_running()` (the GAME clock, not `clock_is_running()`).
- Produces: `pub fn revive_court(&mut self, info: NextGameInfo, now: Instant) -> Result<bool>`. Errs
  `GameInProgress` unless the period is BetweenGames. Calls `set_next_game(info)`. With
  `timeout_state.is_some()`: applies `info.timing` to `self.config` (as `apply_next_game_start` does),
  sets `break_owed_after_timeout = true`, leaves `clock_state` alone, logs one `info!` line, and returns
  `Ok(false)`. Otherwise it returns `apply_next_game_start(now).map(|()| true)`.
- Produces: private field `break_owed_after_timeout: bool`, false in `new()`.

- [ ] **Step 1: Write the failing tests** (in `mod test`; use `initialize()`; config
  `half_play_duration 900s, minimum_break 60s, nominal_break 60s, post_game_duration 20s`; build the
  parked court as `a_revived_court_clears_the_old_score_early_again` does):
  - `a_court_revived_under_a_running_ref_timeout_waits_for_the_timeout` (A): break 300s running →
    `start_ref_timeout(+10s)` → `set_no_next_game()` → `assert!(tm.court_is_parked())` →
    `revive_court(info("13"), +30s) == Ok(false)` → clock still `Stopped { clock_time: ZERO }` →
    `end_timeout(+40s)` → `clock_state` is `CountingDown` with `time_remaining_at_start == 60s` (the
    min-break floor) → `update(+41s)` leaves `current_period == BetweenGames` → `update(+100s)` gives
    `FirstHalf` with `timeout_state == None`.
  - `a_court_revived_under_a_stopped_ref_timeout_does_not_kick_off_under_it` (B): parked →
    `start_ref_timeout(t0)` (assert `Ref(Stopped)`) → `revive_court(+5s) == Ok(false)` → `update(+70s)`
    is still `BetweenGames`, with the clock `Stopped { ZERO }` and the timeout still set → `end_timeout(+100s)`
    → `CountingDown` 60s → `update(+161s)` gives `FirstHalf` with `timeout_state == None`.
  - `a_court_revived_with_no_timeout_counts_down_at_once`: parked → `revive_court == Ok(true)` →
    `CountingDown` 60s at once.
  - `a_deferred_break_counts_to_the_scheduled_start`: `start_time: Some(OffsetDateTime::now_utc() + 20 min)`.
    After `end_timeout(Instant::now())`, the remaining time is between 19 and 20 minutes.
  - `a_deferred_break_past_its_start_is_the_minimum_break`: `start_time` 5 min in the past → 60s.
  - `a_time_edit_during_an_owed_break_survives_the_timeout_end`: deferred revive →
    `set_game_clock_time(200s)` → `end_timeout` → the clock reads 200s, not 60s (the operator's edit wins).
  - `a_court_finished_again_during_an_owed_break_stays_parked`: deferred revive → `set_no_next_game()`
    → `end_timeout` → `court_is_parked()`, and `update(+120s)` stays `BetweenGames`.
  - `ending_the_timeout_on_an_owed_break_wakes_the_updater`: the receiver sees `true` after `end_timeout`.
  - `an_owed_break_arms_the_mid_break_reset`: after `end_timeout`,
    `reset_game_time == remaining - post_game_duration`.
  - `a_deferred_revive_applies_the_games_timing_at_once`: info with `timing: Some(rule)` → after
    `revive_court`, `tm.config()` reflects the rule before `end_timeout`.
  - `court_is_parked_reads_the_game_clock_not_the_timeout_clock`: true for a parked court, with no
    timeout, under `Ref(Stopped)`, and under `Ref(CountingUp)`; false for a running break and in `FirstHalf`.

- [ ] **Step 2: Run them and see them fail** (needs a GO): `CARGO_BUILD_JOBS=4 cargo test -j 4 -p refbox court_revived court_is_parked owed_break deferred -- --test-threads=1`.
  Expected: they fail to compile (no `revive_court` / `court_is_parked`).

- [ ] **Step 3: Implement the engine side.** Add the field, `court_is_parked` and `revive_court` as
  specified. At the top of `end_timeout`: if `break_owed_after_timeout && current_period == BetweenGames
  && timeout_state.is_some()`, then set `timeout_state = None` and the flag to false, call
  `apply_next_game_start(now)?`, log one `info!` line naming the break length, and return `Ok(())`. The
  timeout must be cleared first (Review Focus 2). Add `self.break_owed_after_timeout = false;` beside
  every `retime_owed = false`, and in `set_no_next_game`. Each new doc comment says why (cite the PO
  ruling of 2026-11-02 and the two proven orders).

- [ ] **Step 4: Wire the app.** In `RecvSchedule`, compute `let leaving_finished_state = tm.court_is_parked();`
  where `was_court_finished` is read today, before anything changes the engine. Remove the timeout-blind
  `was_court_finished && !tm.clock_is_running()` line and the now-unused `was_court_finished` if nothing
  else reads it. In the `restoring` branch: when `leaving_finished_state`, call
  `tm.revive_court(info, now)` in place of `set_next_game` + `apply_next_game_start`. On `Ok(_)` keep
  the existing config-copy + snapshot tail (the config is already updated, Review Focus 4). On `Err`,
  log `error!` and fall through to the snapshot tail. The `restore_num` path stays exactly as it is.
  Update the surrounding comments that say "its break clock is held stopped at 0:00…" so they mention
  the timeout deferral.

- [ ] **Step 5: Run the tests** (GO): same command → PASS. Then the whole refbox suite:
  `CARGO_BUILD_JOBS=4 cargo test -j 4 -p refbox -- --test-threads=1` → all green.

- [ ] **Step 6: Leave the changes uncommitted.** The session reviews the diff and commits
  `fix(refbox): wait for the timeout before counting down a revived court`.

### Task 2: Walkthrough rig and steps (session, not a helper)

- [ ] Copy `/home/estraily/projects/uwh-refbox-rs/.walk-mock/` (the untracked stand-in Portal:
  `server.py` re-reads `schedule.json` on every request) into the scratchpad. Set up court 1 so its
  last game is a short one and a later game can be added by editing `schedule.json`. Launch with
  `XDG_CONFIG_HOME=<scratch>` (isolates `~/.config/refbox`), `UWH_PORTAL_URL_OVERRIDE`, `--allow-http`
  and `WAYLAND_DISPLAY=`. Book the refbox app with the coordinator unless it's isolated, and rebuild
  the binary first (a GO).
- [ ] Write the PO numbered steps for **B** (finish the court → REF TIMEOUT → add a game + REFRESH →
  wait past the minimum break: no kickoff, timeout still shown → end the timeout: the countdown starts →
  the game starts when it reaches 0:00) and for the **unchanged** no-timeout revive. **A** needs two
  REFRESHes in one timeout: offer it as an optional step. Each step must be able to fail.
- [ ] Then the pre-PR checks per `.claude/rules/pr-review.md`: `cargo audit` first after rebasing,
  `just check` (GO), the `code-review` skill on the branch diff, the dangling sweep, and the disclosure.

## Deviations

- The test `a_deferred_break_counts_to_the_scheduled_start` accepts a break of up to 20 min + 2 s,
  where the plan said exactly 20:00. The wall clock and the app's internal stopwatch are read
  separately, so the break comes out a few microseconds over 20:00.
- When the app starts up and restores a game onto a court that had finished, it now goes through
  `revive_court` too. With no timeout on screen it behaves exactly as before; under a timeout it
  follows the ruled behaviour (the break waits for the timeout to end).
- The final review found that TIME EDIT, then END REF TIMEOUT, then APPLY crashed the app on a
  court whose break was waiting for that timeout. END TIMEOUT now holds the break still while
  TIME EDIT is open, and CANCEL or APPLY resumes it (fixed in the final fix wave).
