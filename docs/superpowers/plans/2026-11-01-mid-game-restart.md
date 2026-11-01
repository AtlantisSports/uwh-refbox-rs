# Mid-game RESTART Button Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** While a game is in play, the Game Info page's REFRESH button becomes RESTART. After a confirmation page it wipes the game without sending anything to the Portal, puts the court back into the pre-game break for the same game, and pulls a fresh schedule.

**Architecture:** One new engine operation, `TournamentManager::restart_game`, carries out the whole state change in the right order. The app gets a new confirmation kind and a handler that calls it. That handler publishes the resulting snapshot without asking whether a game ended, so no result can ever be filed, and then fires the same `request_schedule` that REFRESH uses. A fresh schedule then arrives through the existing between-games `RecvSchedule` path. That path keeps game N because the engine already holds it, re-times the break if the start moved, and stores the new timing rule, which `start_game` applies at kickoff.

**Tech Stack:** Rust 2024, iced 0.13, Fluent translations (`fl!`).

**Spec:** Approved by the PO in brainstorming, 2026-11-01. The source is memory `project_refresh_retimes_break_branch.md`, last section, reproduced in full below so it travels with this plan:

> - Between games (incl. court finished): button stays REFRESH, unchanged.
> - First half → game end (half-time, timeouts, OT, SD incl.): button reads RESTART → confirmation page "Restart this game? The score, fouls and penalties will be lost and nothing is sent to the Portal." green GO BACK / red RESTART GAME → wipe game (nothing to Portal), pre-game break for the SAME game, countdown = max(time to scheduled start, minimum_break) then auto-start (PO option 1), fire a fresh `request_schedule`.
> - Approved calls: a failed refresh still restarts on the old rules; RESTART only where REFRESH shows (`uses_remote`); the expired-login grey-out applies; the 14 non-English translations are machine-drafted and marked unchecked in the PR.

**Scope Card**
Task: implement the approved mid-game RESTART button in `refbox`.
Explicitly not doing: changing timing rules while a game is in play; the parked revived-court bug; how REFRESH fetches between games, including the wrong health-check comment in `refresh_court_schedule`.

## Global Constraints

- MSRV 1.85: no let-chains (`if let … && …`). Every gate passes with them anyway, so check by eye.
- Clippy `-D warnings`; no `unwrap()`/`expect()` in production code without a `// why this cannot panic` comment.
- Copy is literal (en-US):
  - `restart = RESTART`
  - `restart-game-confirm = Restart this game? The score, fouls and penalties will be lost and nothing is sent to the Portal.`
  - `go-back = GO BACK`
  - `restart-game = RESTART GAME`
- Every new key goes into all 15 `refbox/translations/*/refbox.ftl`. An en-US-only key fails `just check` (build.rs coverage check).
- Button colours: GO BACK uses `green_button`, RESTART GAME uses `red_button`, and RESTART on Game Info stays `blue_button` like REFRESH.
- Crate scope: `refbox` only. No `uwh-common`, no dependency changes.
- **Every cargo command (build, test, clippy, `just *`) needs the refbox coordinator's GO first**, one at a time. Helpers must ask the main session, never run one unasked.
- Helpers leave changes uncommitted. The main session reviews and commits.

## Review Focus

1. **The game runs out while the confirmation page is up.** The result has already been filed, so RESTART GAME must change nothing: no wipe, no second break. *Pinned by* Task 1 `restart_refused_between_games` plus the Task 3 handler re-check.
2. **A clock tick queued before the restart is applied after it.** The time updater's channel holds 100 messages. A stale in-play tick would read as "game started", and `handle_game_start` would then overwrite next-game N with N+1, so the break would kick off the wrong game. *Pinned by* Task 2 `a_tick_from_a_game_the_engine_has_left_is_stale`.
3. **Restarting the court's last game.** Here `next_game` is `None` and `no_next_game` is true, so a naive reset parks the clock at 0:00 and numbers the stats blank. *Pinned by* Task 1 `restart_last_game_on_court`.
4. **Game N is missing from the stored schedule, so there is no start time.** The countdown must be the minimum break, not a count to game N+1's Game Block slot, which is what `next_scheduled_start` holds mid-game. *Pinned by* Task 1 `restart_without_start_time_uses_minimum_break`.
5. **Restarting during a ref timeout or penalty shot, or with the game clock stopped.** The timeout clears and the break still runs, rather than staying frozen. *Pinned by* Task 1 `restart_during_ref_timeout`.

---

### Task 1: Engine — `restart_game`

**Files:**
- Modify: `refbox/src/tournament_manager/mod.rs`. Add the method beside `reset_game` (~line 787), add the error variant to `TournamentManagerError` (~3754), and put the tests in the existing `#[cfg(test)]` module.

**Interfaces:**
- Produces: `pub fn restart_game(&mut self, info: NextGameInfo, now: Instant) -> Result<()>`
- Produces: `TournamentManagerError::NotInGame`, with message text "No game is in progress to restart". Fix any exhaustive matches the compiler reports.

Order inside `restart_game` matters, and each step below fixes a real trap:
1. `current_period == BetweenGames` → `Err(NotInGame)`, with nothing changed.
2. `let this_slot = self.current_scheduled_start;` (game N's own slot. `reset()` clears it.)
3. `self.set_next_game(info)` **before** the reset. `reset()` numbers the new stats from `next_game_number()`. Reset first, and mid-game that reads N+1, or blank on the court's last game. This step also clears `no_next_game`.
4. `self.reset_game(now)`. This clears the period, timeouts, scores, penalties, fouls and warnings, sets `has_reset`, and clears `retime_owed`.
5. `if this_slot.is_some() { self.next_scheduled_start = this_slot; }`. This puts the Game Block projection back to game N's slot, the state before N kicked off. Without it, a missing `start_time` counts to N+1's slot, and `start_game` mis-dates the restarted game's DELAY.
6. `self.apply_next_game_start(now)?`. This applies `info.timing` to the config and sets the countdown to max(start − now, minimum_break).
7. `self.send_clock_running(true)` unconditionally. If the clock was already running, the updater sleeps until the next whole second of the countdown it last saw, the same reason `set_next_game_and_retime` gives.
8. `info!` one line: game number, and the new break length.

It must NOT call `end_game`. That is what writes `last_game_info`, and so what files a result.

- [ ] **Step 1: Write the failing tests.** Build each one from the existing helpers in this module that start a game and score. Assertions:
  - `restart_from_first_half_counts_to_scheduled_start`: mid first half, with a goal each side, a foul, a warning and a penalty. `start_time = now_utc + 10 min`, `minimum_break = 2 min`. After the restart: period `BetweenGames`; scores 0–0; no penalties, fouls or warnings; `game_clock_time` within 1 s of 600 s; `game_number == "N"`; `next_game_number() == "N"`; `last_game_info()` unchanged from before the restart.
  - `restart_with_start_in_past_uses_minimum_break`: `start_time = now_utc − 5 min` → clock == `minimum_break`.
  - `restart_refused_between_games`: between games → `Err(NotInGame)`, with clock state, next game and scores unchanged.
  - `restart_last_game_on_court`: `set_no_next_game()` mid-game, then restart → `Ok`, clock counting down, `next_game_number() == "N"`, and the stats being recorded name game N.
  - `restart_without_start_time_uses_minimum_break`: kick off with a Game Block slot set, then restart with `start_time: None` → clock == `minimum_break`.
  - `restart_during_ref_timeout`: ref timeout running mid-game, clock stopped → after the restart, no timeout and the clock counting down.
  - `restart_applies_the_timing_rule`: `info.timing` with a different `minimum_break` → `config().minimum_break` equals it.
  - `restarted_game_kicks_off_as_the_same_game`: tick past the end of the break → `FirstHalf`, `game_number == "N"`, team timeouts used == 0.
  - `restart_wakes_the_updater`: take `get_start_stop_rx()` beforehand, with the clock running in the first half → the receiver has changed and holds `true` after the restart.
- [ ] **Step 2: Run them (GO first): `cargo test -p refbox restart_` — all FAIL, because `restart_game` does not exist.**
- [ ] **Step 3: Implement `restart_game` and `NotInGame` per the order above.**
- [ ] **Step 4: Run (GO first): `cargo test -p refbox restart_` → all PASS. Then `cargo test -p refbox tournament_manager` → no regressions.**
- [ ] **Step 5: Main session reviews and commits** `feat(refbox): add an engine operation to restart the game in play`

### Task 2: App rules — stale-tick guard and the Game Info button choice

Two pure functions, so both rules are tested without an app (there is no app-level test harness here).

**Files:**
- Modify: `refbox/src/app/mod.rs`. `SnapshotTransition` / `snapshot_transition` (~1530-1567), `apply_tick` (~1578), and their tests.
- Modify: `refbox/src/app/view_builders/game_info.rs`.

**Interfaces:**
- Produces: `SnapshotTransition::Stale`, and `snapshot_transition(prev, next, caught_up, engine_period: GamePeriod)`, with the extra parameter last.
- Produces: in `game_info.rs`, `enum MiddleButton { Hidden, Refreshing, Refresh, Restart }` and `fn middle_button(uses_remote: bool, is_refreshing: bool, period: GamePeriod) -> MiddleButton`.

Stale rule: if `prev` is `BetweenGames`, `next` is in play, **and** `engine_period == BetweenGames`, return `Stale`. A real kickoff always has the engine in play when its tick is applied. Keep the `caught_up` arms first: a catch-up marker is taken in the same lock as its snapshot, so it is never stale. In `apply_tick`, `Stale` → `warn!` one line naming both game numbers, then `return Task::none()`. Do not store or send the snapshot, because the tick that follows carries the true state. Read `engine_period` with one `self.tm.lock().current_period()` at the top of `apply_tick`. **Trap:** that lock does not re-enter. Grep every caller of `apply_tick`/`apply_snapshot` and confirm each one drops its `tm` guard before the call (`RecvSchedule` and `EndGameAndApply` do); a caller that still holds it deadlocks the app.

Button rule: `!uses_remote` → `Hidden`; otherwise `is_refreshing` → `Refreshing`; otherwise `BetweenGames` → `Refresh`; otherwise `Restart`. `Restart` sends `Message::RequestGameRestart` (Task 3). Until Task 3 lands, wire it to `RequestPortalRefresh` so this task compiles on its own. The `token_expired` grey-out applies to both `Refresh` and `Restart`.

- [ ] **Step 1: Failing tests.**
  - In the `snapshot_transition` tests: `a_tick_from_a_game_the_engine_has_left_is_stale`: prev `BetweenGames`, next `FirstHalf`, engine `BetweenGames`, no marker → `Stale`. `a_real_kickoff_is_still_game_started`: same, with engine `FirstHalf` → `GameStarted`. `a_catch_up_is_never_stale`: `Some(CatchUp::NewGame)` with engine `BetweenGames` → `CaughtUpToNewGame`. Update every existing call site to pass a realistic `engine_period`, normally `next.current_period`.
  - In `game_info.rs`: `middle_button_rules`, a table over (uses_remote, refreshing, period) → `Hidden` for `(false, _, _)`; `Refreshing` for `(true, true, FirstHalf)`; `Refresh` for `(true, false, BetweenGames)`; `Restart` for each of `FirstHalf, HalfTime, SecondHalf, PreOvertime, OvertimeFirstHalf, OvertimeHalfTime, OvertimeSecondHalf, PreSuddenDeath, SuddenDeath` with `(true, false, _)`.
- [ ] **Step 2: Run (GO first): `cargo test -p refbox stale middle_button` → FAIL, because neither function exists.**
- [ ] **Step 3: Implement both, and add the `restart = RESTART` key to all 15 locales.**
- [ ] **Step 4: Run (GO first): the same tests → PASS; then `cargo test -p refbox app::` → no regressions.**
- [ ] **Step 5: Main session reviews and commits** `feat(refbox): show RESTART on game info while a game is in play`

### Task 3: Confirmation page and the restart handler

**Files:**
- Modify: `refbox/src/app/message.rs`. Add `Message::RequestGameRestart`, and add it to every match arm the compiler flags (~443, 552, 801). Add `ConfirmationOption::RestartGame`.
- Modify: `refbox/src/app/mod.rs`. Add `ConfirmationKind::RestartGame`, the `RequestGameRestart` arm, routing in `ConfirmationSelected` (~6740), and the handler.
- Modify: `refbox/src/app/view_builders/confirmation.rs`. Header and buttons.
- Modify: `refbox/src/app/view_builders/game_info.rs`. Point `Restart` at `RequestGameRestart`.
- Modify: `refbox/translations/*/refbox.ftl` ×15. Add `restart-game-confirm`, `go-back` and `restart-game` next to `go-back-to-editor`.

**Interfaces:**
- Consumes: `restart_game` and `NotInGame` (Task 1); `MiddleButton::Restart` (Task 2).
- Produces: `fn restart_current_game(&mut self) -> Task<Message>`, and `fn publish_snapshot(&mut self, snapshot: GameSnapshot)`. The second is the tail of `apply_tick`: everything after the transition match (event id, portal base URL, sound, display send, store). `apply_tick` calls it, so there is one copy.

Behaviour:
- `RequestGameRestart` → `AppState::ConfirmationPage(ConfirmationKind::RestartGame)`.
- In `ConfirmationSelected`, before the `*FromApply` dispatch, `RestartGame` is handled on its own: `GoBack` → `GameDetailsPage(false)`; `RestartGame` → `restart_current_game()`. Any other option is `unreachable!` with a reason, matching the style of the existing arms.
- Confirmation page: header `fl!("restart-game-confirm")`; buttons `(fl!("go-back"), green_button, GoBack)` and `(fl!("restart-game"), red_button, RestartGame)`.
- `restart_current_game`:
  1. Re-take the check. If `!self.uses_remote()` or the engine is `BetweenGames` (the game ended while the page was up), `warn!` and go to `MainPage`, changing nothing.
  2. Build the info for game N = the engine's current game number: `self.schedule.get_game_and_timing(&n)` → `NextGameInfo { number: n, timing: timing.cloned(), start_time: game.map(|g| g.start_time) }`. A missing game gives `None`/`None` and Task 1 handles that.
  3. `tm.restart_game(info, now)`. On `Err`, `warn!` and go to `MainPage`. Then set `self.config.game = tm.config().clone()` and generate the snapshot, dropping the lock first.
  4. `self.publish_snapshot(snapshot)`. **Not** `apply_snapshot`: that would read in-play → between-games as `GameEnded` and run `handle_game_end`. That function is currently kept from filing only because `last_game_info` happens to name a different game.
  5. Set `app_state = MainPage`, where the operator sees the countdown.
  6. If there is a `current_event_id` and `uwhportal_client.is_some()`, return `self.request_schedule(event_id)`. Otherwise return `Task::none()`. A failed fetch changes nothing: the restart on the old rules stands.
- Live Updates needs no code. The next frame is the ordinary pre-game frame for game N (scores null), and no stats frame is sent between games. No result or stats are sent, so the copy holds.

- [ ] **Step 1:** There is no app harness, so the handler is proven by the walk below. The rules it relies on are already pinned in Tasks 1-2. Build (GO first) `cargo build -p refbox` → compiles.
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Run (GO first): `cargo test -p refbox` → green. Use `--test-threads=1` if a random few ALSA tests red, which is a known no-sound-card artefact here.**
- [ ] **Step 4: Main session reviews and commits** `feat(refbox): restart the game in play from the game info page`

### Task 4: Branch checks and walkthrough (main session)

- [ ] Final whole-branch review: the `.claude/rules/pr-review.md` check 1 (`code-review` skill) on `origin/master...HEAD`, plus `just check` (GO first).
- [ ] PO walkthrough. Book the refbox app with the coordinator, or isolate with `XDG_CONFIG_HOME`; link to the dev portal with `UWH_PORTAL_URL_OVERRIDE`. Steps the PO walks, each able to fail:
  1. Between games, Game Info shows REFRESH, which works as before.
  2. Start a game, score a goal, give a foul → Game Info shows RESTART.
  3. RESTART → the confirmation page with the exact wording, green GO BACK and red RESTART GAME. GO BACK → back on Game Info, with the game and score untouched.
  4. RESTART → RESTART GAME → main page, score 0–0, foul gone, break counting toward the scheduled start (or the minimum break if that start has passed), same game number.
  5. Let the break run out → the SAME game starts.
  6. The Portal shows no result for that game (dev portal event page).
  7. During half-time and during a ref timeout, the button still reads RESTART and the restart works.
- [ ] PR disclosure per `pr-review.md`: 14 translations machine-drafted and unchecked.

## Deviations

- **Task 1:** `restart_game` also clears a pending score-confirmation pause (`time_pause_confirmation`). Left set, a restart tapped during the end-of-game or deciding-goal confirm window stopped the break reading as running and stopped the restarted game's second half ending on its own. Test `restart_during_score_confirm_pause`. `reset_game` keeps the same gap on the existing END CURRENT GAME path, which is out of scope here and offered separately.
- **Task 3:** the `ConfirmScores` and `AutoConfirmScores` handlers also test the stale rule before changing the screen. A score-confirmation tick queued before a restart would otherwise raise a 0–0 score page for the abandoned game, or end a pause the restart had already cleared.
- **Task 3:** the ja-JP and zh-CN confirmation text says the score will be "erased" rather than "lost". The refbox's built-in CJK font subset (checked at build time) lacks わ, 失 and 任.
- **Task 2:** the stale rule is not specific to RESTART. It also drops an in-play tick queued behind END CURRENT GAME AND APPLY, which used to run a spurious kickoff refresh.
- **Accepted:** a sleep catch-up tick queued at the exact instant of RESTART GAME bypasses the stale rule (catch-up markers always do) and is applied.
