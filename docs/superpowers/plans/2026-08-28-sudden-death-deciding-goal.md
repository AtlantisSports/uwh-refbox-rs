# Sudden-Death Deciding Goal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A goal that ends sudden death reaches the game engine when it is scored, so the game ends correctly whether the operator confirms it or the confirmation window simply expires.

**Architecture:** The three sudden-death score handlers in `app/mod.rs` currently hold the new score in UI state and tell the engine only "pause for confirmation", so the background clock task — which resolves the window and cannot see UI state — finds an unchanged tie. Two new `TournamentManager` operations own the fix: they open the confirmation window **first**, then apply the change, which is the only safe order. The three handlers call them.

**Tech Stack:** Rust 2024, `refbox` crate only. `cargo test -p refbox`, `just check`. Golden-trace regression harness at `refbox/src/tournament_manager/golden/`.

**Spec:** `docs/superpowers/specs/2026-08-28-sudden-death-deciding-goal-design.md` — read it before starting; it records what was rejected and why.

## Global Constraints

- **Blast radius:** game state machine → **heavy process** per `.claude/rules/plan-execution.md`: per-task verification, per-task review, strict deviation tracking.
- **Crates touched:** `refbox` only. Do **not** touch `uwh-common`, `overlay`, `wireless-remote`, or the LED panel wire format.
- **MSRV:** Rust 1.85. **Edition:** 2024. No APIs newer than 1.85.
- **Clippy:** `cargo clippy --workspace --all-targets --all-features -- -D warnings` must be clean. No `#[allow(...)]` to silence anything.
- **No `unwrap()`/`expect()` in non-test code without a comment explaining why it cannot panic.** Every `unwrap` this plan adds comes with that comment already written — copy it.
- **No new dependencies.**
- **Out of scope, do not change:** the confirmation window's length, the confirmation screen's layout or wording, the `confirm_score` setting, `Message::ScoreConfirmation`, `Message::ScoreEditComplete`'s `is_confirmation` branch.
- **Baseline:** 599 `refbox` tests pass on this branch at `79d79f35`. Any red after a task is yours.
- **Approval gates:** ask the human before any commit, and before any push or PR.

---

### Task 1: Engine — two operations that own the order

**Files:**
- Modify: `refbox/src/tournament_manager/mod.rs` (`add_score` at ~105-123; new fns after it; new tests in the `tests` module)
- Modify: `refbox/src/tournament_manager/game_stats.rs` (test-only accessor after `add_goal` at ~44-63)

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces:
  - `pub fn add_deciding_goal(&mut self, color: Color, player_num: u8, now: Instant) -> Result<()>`
  - `pub fn set_deciding_scores(&mut self, scores: BlackWhiteBundle<u8>, now: Instant) -> Result<()>`
  - `fn add_score_at(&mut self, color: Color, player_num: u8, clock_time: Option<Duration>, now: Instant)` (private)
  - `#[cfg(test)] pub(crate) fn goals(&self) -> Vec<(u8, String, GamePeriod, f32)>` on `GameStats`

- [ ] **Step 1: Add the test-only goal accessor**

`GameStats.events` is private to `game_stats.rs` and `Event` is a private enum, so the tests in `mod.rs` cannot read it. Add this inside `impl GameStats`, directly after `add_goal`. It returns only public types, so `Event` stays private.

```rust
    /// Test-only view of the recorded goals in order: (cap number, side, period, period time).
    /// Deliberately returns public types so `Event` need not become visible outside this module.
    #[cfg(test)]
    pub(crate) fn goals(&self) -> Vec<(u8, String, GamePeriod, f32)> {
        self.events
            .iter()
            .filter_map(|e| match e {
                Event::Goal {
                    player_cap_number,
                    side,
                    game_period,
                    period_time,
                    ..
                } => Some((
                    *player_cap_number,
                    side.clone(),
                    *game_period,
                    *period_time,
                )),
                _ => None,
            })
            .collect()
    }
```

- [ ] **Step 2: Write the four failing tests**

Add to the `#[cfg(test)] mod tests` block in `refbox/src/tournament_manager/mod.rs`, next to `test_sd_score_confirm_with_clock_stopped_does_not_crash` (~line 8334) so the sudden-death confirm tests stay together.

```rust
    // ── Deciding sudden-death goal (2026-08-28) ───────────────────────────────
    // A goal that ends sudden death must reach the engine when it is SCORED, not when the
    // operator confirms it. Before this fix the three score handlers in app/mod.rs held the
    // new score in UI state; the background clock task that resolves the confirmation window
    // cannot see UI state, so a window that expired untouched discarded the goal and
    // end_confirm_pause's SuddenDeath arm kept the period at SuddenDeath forever.

    /// Shared setup: sudden death, tied 1-1, clock counting up from 30 s, window = 10 s.
    fn sd_tied_at_30s() -> (TournamentManager, Instant, Instant) {
        let config = GameConfig {
            overtime_allowed: false,
            sudden_death_allowed: true,
            minimum_break: Duration::from_secs(20), // → confirm window = 20/2 = 10 s
            post_game_duration: Duration::from_secs(20),
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);
        let start = Instant::now();
        let now = start + Duration::from_secs(1);

        tm.set_period_and_game_clock_time(GamePeriod::SuddenDeath, Duration::from_secs(30));
        tm.set_scores(BlackWhiteBundle { black: 1, white: 1 }, start);
        tm.start_game_clock(start);

        (tm, start, now)
    }

    #[test]
    fn test_deciding_goal_is_recorded_without_ending_the_game() {
        initialize();
        let (mut tm, _start, now) = sd_tied_at_30s();

        tm.add_deciding_goal(Color::Black, 7, now).unwrap();

        // The score reaches the engine straight away and the window is open...
        assert_eq!(tm.get_scores(), BlackWhiteBundle { black: 2, white: 1 });
        assert!(tm.in_score_confirm_pause());
        // ...but the game has NOT ended. Ending it is the window's job, either route.
        assert_eq!(tm.current_period, GamePeriod::SuddenDeath);
    }

    /// The reported bug. The operator scores the deciding goal and never taps CORRECT; only the
    /// clock updater's `end_confirm_pause` runs. That must end the game exactly as CORRECT does.
    #[test]
    fn test_deciding_goal_ends_game_when_window_expires_untouched() {
        initialize();
        let (mut tm, _start, now) = sd_tied_at_30s();

        tm.add_deciding_goal(Color::Black, 7, now).unwrap();

        let before_end = now + Duration::from_secs(9);
        let after_end = now + Duration::from_secs(11);
        assert!(!tm.pause_has_ended(before_end));
        assert!(tm.pause_has_ended(after_end));

        // No operator input at all.
        tm.end_confirm_pause(after_end).unwrap();

        assert_eq!(tm.current_period, GamePeriod::BetweenGames);
        assert_eq!(tm.get_scores(), BlackWhiteBundle { black: 2, white: 1 });
        assert!(!tm.in_score_confirm_pause());
    }

    /// Guards the "read the clock BEFORE arming the window" half of `add_deciding_goal`:
    /// `pause_for_confirm` freezes `clock_state` at 10 ms, so recording the goal afterwards
    /// would stamp it 0.01 s into sudden death instead of its real reading.
    #[test]
    fn test_deciding_goal_records_scorer_and_real_clock_reading() {
        initialize();
        let (mut tm, _start, now) = sd_tied_at_30s();

        // The sudden-death clock was 30 s and has run 1 s, so the goal is 31 s into sudden death.
        tm.add_deciding_goal(Color::Black, 7, now).unwrap();

        let goals = tm.current_game_stats.goals();
        assert_eq!(goals.len(), 1, "the deciding goal was not recorded");
        let (cap_number, side, period, period_time) = &goals[0];
        assert_eq!(*cap_number, 7, "the scorer's cap number was dropped");
        assert_eq!(side.as_str(), "dark");
        assert_eq!(*period, GamePeriod::SuddenDeath);
        assert!(
            (*period_time - 31.0).abs() < 0.001,
            "expected the real sudden-death reading of 31 s, got {period_time} \
             (0.01 means the frozen confirm-window clock was recorded)"
        );
    }

    /// The score-edit route into the window has the same defect and the same fix. An edit is the
    /// operator stating a score, not scoring one, so no goal is recorded for it.
    #[test]
    fn test_deciding_score_edit_is_recorded_without_ending_the_game() {
        initialize();
        let (mut tm, _start, now) = sd_tied_at_30s();

        tm.set_deciding_scores(BlackWhiteBundle { black: 3, white: 1 }, now)
            .unwrap();

        assert_eq!(tm.get_scores(), BlackWhiteBundle { black: 3, white: 1 });
        assert!(tm.in_score_confirm_pause());
        assert_eq!(tm.current_period, GamePeriod::SuddenDeath);
        assert!(
            tm.current_game_stats.goals().is_empty(),
            "an edited score must not fabricate a goal"
        );

        tm.end_confirm_pause(now + Duration::from_secs(11)).unwrap();
        assert_eq!(tm.current_period, GamePeriod::BetweenGames);
    }
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p refbox deciding_goal`
Expected: FAIL to compile — `no method named 'add_deciding_goal' found for struct 'TournamentManager'` and the same for `set_deciding_scores`. A compile failure is the correct red here; the methods do not exist yet.

- [ ] **Step 4: Split `add_score` and add the two operations**

Replace the whole of `add_score` (currently ~lines 105-123) with the following four items. `add_score`'s observable behaviour is unchanged — it just captures the clock reading and delegates.

**Deliberate, do not "fix" it back:** the sudden-death path now increments with `scores[color] += 1`, the same as every other goal, rather than the `saturating_add` the three screens used. Sharing one goal-recording routine is the point of the change, and the difference needs 255 goals in one game to matter. The original `add_score` also called `game_clock_time(now)` twice; calling it once and reusing the value is identical, since it is a pure function of `clock_state` and `now`.

```rust
    pub fn add_score(&mut self, color: Color, player_num: u8, now: Instant) {
        let clock_time = self.game_clock_time(now);
        self.add_score_at(color, player_num, clock_time, now);
    }

    /// Record a goal whose period-clock reading was captured earlier.
    ///
    /// `add_deciding_goal` must read the clock BEFORE it opens the score-confirmation window,
    /// because `pause_for_confirm` freezes `clock_state` at 10 ms; recording the goal afterwards
    /// would stamp it 0.01 s into sudden death instead of its real reading.
    fn add_score_at(
        &mut self,
        color: Color,
        player_num: u8,
        clock_time: Option<Duration>,
        now: Instant,
    ) {
        info!(
            "{} Score by {color} player #{player_num}",
            self.status_string(now)
        );
        self.current_game_stats
            .add_goal(self.current_period, clock_time, color, player_num, now);
        self.recent_goal = clock_time.map(|time| (color, player_num, self.current_period, time));
        let mut scores = self.scores;
        scores[color] += 1;
        self.set_scores(scores, now);
    }

    /// Record the goal that ends sudden death, behind the score-confirmation window.
    ///
    /// **The order of the two steps is load-bearing.** `set_scores` ends the game immediately for
    /// an untied sudden-death score unless a confirm pause is already armed, which moves the
    /// period to `BetweenGames`; `pause_for_confirm` would then reach its
    /// `GamePeriod::BetweenGames => unreachable!()` arm and panic while the game-state lock is
    /// held, taking the app down on the operator's next touch. Arming the window first makes the
    /// goal safe to record and leaves the window intact, so the game ends when the window
    /// resolves — whether the operator confirms it or it simply expires.
    ///
    /// `pause_for_confirm` does not change `current_period`, so the goal is still recorded
    /// against `SuddenDeath`.
    ///
    /// # Errors
    /// Only from `pause_for_confirm`, which cannot fail in sudden death: it accepts a stopped
    /// clock and clears any active timeout. The `?` is a backstop, not an expected path.
    pub fn add_deciding_goal(
        &mut self,
        color: Color,
        player_num: u8,
        now: Instant,
    ) -> Result<()> {
        let clock_time = self.game_clock_time(now);
        self.pause_for_confirm(now)?;
        self.add_score_at(color, player_num, clock_time, now);
        Ok(())
    }

    /// Apply an edited score that ends sudden death, behind the score-confirmation window.
    ///
    /// Same load-bearing order as [`Self::add_deciding_goal`], for the same reason. No goal is
    /// recorded: an edit is the operator stating a score, not scoring one.
    ///
    /// # Errors
    /// As [`Self::add_deciding_goal`].
    pub fn set_deciding_scores(
        &mut self,
        scores: BlackWhiteBundle<u8>,
        now: Instant,
    ) -> Result<()> {
        self.pause_for_confirm(now)?;
        self.set_scores(scores, now);
        Ok(())
    }
```

- [ ] **Step 5: Run the new tests to verify they pass**

Run: `cargo test -p refbox deciding_goal`
Expected: PASS — 4 tests.

- [ ] **Step 6: Run the whole crate suite**

Run: `cargo test -p refbox`
Expected: 603 passed, 0 failed (599 baseline + 4 new). `golden_traces_match_baseline` must still be green: the harness has not changed yet, so no trace should move. **If a golden trace moved here, stop** — it means `add_score`'s split changed behaviour, which it must not.

- [ ] **Step 7: Lint**

Run: `cargo clippy -p refbox --all-targets --all-features -- -D warnings`
Expected: no warnings.

- [ ] **Step 8: Commit (ask the human first)**

```bash
git add refbox/src/tournament_manager/mod.rs refbox/src/tournament_manager/game_stats.rs
git commit -m "fix(refbox): record a deciding sudden-death goal when it is scored"
```

---

### Task 2: App — the three screens call the ordered operations

**Files:**
- Modify: `refbox/src/app/mod.rs` — `Message::AddNewScore` (~2765-2782), `Message::ScoreEditComplete` (~2818-2830), `Message::AddScoreComplete` (~3366-3382)

**Interfaces:**
- Consumes: `add_deciding_goal`, `set_deciding_scores` from Task 1.
- Produces: nothing later tasks call. Task 3 mirrors these three handlers in the golden harness.

**Testability note, state it in the commit body:** the iced `update()` message loop is not unit-tested in this crate (`refbox/tests/features/*.feature` are written specs, not executed). The proof for this task is Task 3's golden harness, which is a documented hand-copy of these same handlers, plus the human's walkthrough in Task 4. Do not invent a UI-driver harness for this.

- [ ] **Step 1: `Message::AddNewScore` — sudden-death branch**

Replace the sudden-death branch. It also gains the snapshot generate-and-apply that its own non-sudden-death branch and both branches of `Message::AddScoreComplete` already have — that is what pushes the new score to the LED panel and the overlay promptly.

```rust
                    let mut tm = self.tm.lock().unwrap();
                    let now = Instant::now();
                    if tm.current_period() == GamePeriod::SuddenDeath {
                        // Safe: add_deciding_goal's only Err comes from pause_for_confirm, which
                        // cannot fail in sudden death — it accepts a stopped clock and clears any
                        // active timeout. The period check above guarantees we are in it.
                        tm.add_deciding_goal(color, 0, now).unwrap();
                        let scores = tm.get_scores();
                        let snapshot = tm.generate_snapshot(now).unwrap(); // TODO: Remove this unwrap
                        std::mem::drop(tm);
                        let task = self.apply_snapshot(snapshot);
                        self.app_state = AppState::ConfirmScores(scores);
                        task
                    } else {
                        tm.add_score(color, 0, now);
                        let snapshot = tm.generate_snapshot(now).unwrap(); // TODO: Remove this unwrap
                        std::mem::drop(tm);
                        let task = self.apply_snapshot(snapshot);
                        self.app_state = AppState::MainPage;
                        task
                    }
```

- [ ] **Step 2: `Message::AddScoreComplete` — sudden-death branch**

The snapshot is already generated and applied below for both branches; do not add another.

```rust
                        let app_state = if tm.current_period() == GamePeriod::SuddenDeath {
                            // Safe: see Message::AddNewScore — pause_for_confirm cannot fail in
                            // sudden death, and the period check above guarantees we are in it.
                            tm.add_deciding_goal(color, player.try_into().unwrap(), now)
                                .unwrap();
                            AppState::ConfirmScores(tm.get_scores())
                        } else {
                            tm.add_score(color, player.try_into().unwrap(), now);
                            AppState::MainPage
                        };
```

`TEAM SCORE` needs no handling: `BoolGameParameter::TeamScore` already forces `player_num` to 0 when it is selected, so this is the identical expression to the non-sudden-death line below it.

- [ ] **Step 3: `Message::ScoreEditComplete` — non-confirmation sudden-death branch**

Leave the `is_confirmation` branch above it completely alone.

```rust
                    } else if !canceled {
                        if tm.current_period() == GamePeriod::SuddenDeath
                            && (scores.black != scores.white)
                        {
                            // Safe: see Message::AddNewScore — pause_for_confirm cannot fail in
                            // sudden death, and the period check above guarantees we are in it.
                            tm.set_deciding_scores(scores, now).unwrap();
                            AppState::ConfirmScores(scores)
                        } else {
                            tm.set_scores(scores, now);
                            AppState::MainPage
                        }
```

- [ ] **Step 4: Confirm nothing else in the app needed changing**

Run: `grep -n "GamePeriod::SuddenDeath" refbox/src/app/mod.rs`
Expected: the three call sites you just edited, plus the pre-existing whistle/buzzer `match` arms near lines 773-799 and the settings list at ~2641 and ~7110. If a **fourth** score-holding site appears, stop and report it — the spec accounted for exactly three.

- [ ] **Step 5: Build and lint**

Run: `cargo clippy -p refbox --all-targets --all-features -- -D warnings`
Expected: no warnings.

- [ ] **Step 6: Run the crate suite**

Run: `cargo test -p refbox`
Expected: 603 passed, 0 failed. Golden traces still green — the harness mirrors the *old* handlers until Task 3, and it drives the engine directly rather than through `update()`, so nothing should move yet.

- [ ] **Step 7: Commit (ask the human first)**

```bash
git add refbox/src/app/mod.rs
git commit -m "fix(refbox): give a sudden-death goal to the engine as it is scored"
```

---

### Task 3: Golden harness lockstep, and the regression scenario

`golden/mod.rs` carries a KNOWN COUPLING POINT warning that `apply_action` hand-copies the handlers Task 2 just changed. This task updates it and adds the scenario that proves the bug is gone.

**Files:**
- Modify: `refbox/src/tournament_manager/golden/mod.rs` — `Action` enum (~35-105), the coupling comment (~170-205), `apply_action` (~222-320), `tests` module (~590-620)
- Modify: `refbox/src/tournament_manager/golden/scenarios.rs` — `use` at line 20, `SUDDEN_DEATH_ACTIONS` (~267-278), the three Family-11 consts (~646-686), `all()` (~1105-1120)
- Create: `refbox/src/tournament_manager/golden_traces/sudden_death_score_auto_confirm.trace`
- Modify: 4 existing `.trace` baselines

**Interfaces:**
- Consumes: `add_deciding_goal` from Task 1; the handler shapes from Task 2.
- Produces: golden scenario `sudden_death_score_auto_confirm`; `Action::ConfirmScore` is **removed**.

- [ ] **Step 1: Add the new scenario**

In `scenarios.rs`, after `SD_SCORE_DURING_PENALTY_SHOT_ACTIONS` (~line 686):

```rust
// E) The reported bug: a deciding SD goal is scored and then LEFT ALONE — the operator never taps
//    CORRECT, so only the clock updater's auto-confirm resolves the window. Before the fix the
//    goal lived in UI state, the engine still saw a tie, and end_confirm_pause's SuddenDeath arm
//    kept the period at SuddenDeath forever: score frozen at B0/W0 and no game-start signal for
//    the next game. To reproduce that failure, revert apply_action's Action::ScoreSuddenDeath arm
//    to a bare `tm.pause_for_confirm(now).unwrap()` and re-run.
//
//    minimum_break=20 s → window = 10 s. Goal at t=15, so the window auto-confirms at ~t=25.
static SD_SCORE_AUTO_CONFIRM_ACTIONS: &[(u64, Action)] = &[
    (
        0,
        SetupPeriod(GamePeriod::OvertimeSecondHalf, Duration::from_secs(5)),
    ),
    (0, StartClock),
    // ~t=5 OvertimeSecondHalf ends → PreSuddenDeath (5 s) → SuddenDeath starts ~t=10.
    (15, ScoreSuddenDeath(Color::Black)),
    // Deliberately no confirm action: the window must expire on its own.
];
```

In `all()`, after the `sudden_death_score_during_penalty_shot` entry:

```rust
        Scenario {
            name: "sudden_death_score_auto_confirm",
            config: GameConfig {
                overtime_allowed: true,
                sudden_death_allowed: true,
                pre_overtime_break: Duration::from_secs(5),
                ot_half_play_duration: Duration::from_secs(5),
                ot_half_time_duration: Duration::from_secs(3),
                pre_sudden_death_duration: Duration::from_secs(5),
                minimum_break: Duration::from_secs(20),
                ..reg_config()
            },
            actions: SD_SCORE_AUTO_CONFIRM_ACTIONS,
            run_secs: 35,
        },
```

- [ ] **Step 2: Add the baseline-independent assertion test**

In `golden/mod.rs`'s `mod tests`, after `golden_traces_match_baseline`:

```rust
    /// Baseline-independent guard for the sudden-death auto-confirm bug (2026-08-28).
    ///
    /// `golden_traces_match_baseline` would also catch a regression here, but only against a
    /// blessed file — and a careless `UPDATE_GOLDEN=1` re-blesses the bug along with the fix.
    /// This test states the requirement outright: when an unattended confirmation window ends a
    /// sudden-death game, the game must actually end, and it must end with the goal counted.
    ///
    /// Asserts on the FIRST `BetweenGames` state rather than the last line, because the
    /// between-games auto-reset later zeroes the scores by design.
    #[test]
    fn sudden_death_goal_survives_an_unattended_confirm_window() {
        let scenario = scenarios::all()
            .into_iter()
            .find(|s| s.name == "sudden_death_score_auto_confirm")
            .expect("scenario 'sudden_death_score_auto_confirm' is missing");
        let trace = run(&scenario);

        let ended = trace
            .iter()
            .find(|line| line.contains("period=BetweenGames"))
            .unwrap_or_else(|| {
                panic!(
                    "an unattended confirm window never ended the game; full trace:\n{}",
                    trace.join("\n")
                )
            });

        assert!(
            ended.contains("score=B1/W0"),
            "the game ended without the deciding goal; first BetweenGames state:\n  {ended}\n\
             full trace:\n{}",
            trace.join("\n")
        );
    }
```

- [ ] **Step 3: Run it and CAPTURE the failure — this is the fail-before evidence**

Run: `cargo test -p refbox sudden_death_goal_survives`
Expected: **FAIL.** `apply_action`'s `ScoreSuddenDeath` arm still mirrors the pre-fix handlers (it holds the score locally and only calls `pause_for_confirm`), so the trace never leaves `SuddenDeath` and the panic message is "an unattended confirm window never ended the game".

**Paste the failure output into the task report.** It is the evidence that this test would have failed before the fix, and the spec commits to showing it.

- [ ] **Step 4: Update `apply_action`'s `ScoreSuddenDeath` arm**

```rust
        Action::ScoreSuddenDeath(color) => {
            // Mirrors app/mod.rs Message::AddNewScore / Message::AddScoreComplete, SD branch:
            // tm.add_deciding_goal(color, 0, now). The goal is recorded immediately — so the
            // panel and the overlay show it, as they do for any other goal — behind the
            // score-confirmation window, which then ends the game by either route.
            tm.add_deciding_goal(color, 0, now).unwrap();
        }
```

- [ ] **Step 5: Merge `ConfirmScore` into `ConfirmGameEnd`**

With the score applied when it is entered, `ConfirmScore`'s increment is wrong and the two actions become identical. Delete `ConfirmScore` and retarget `ConfirmGameEnd`.

In the `Action` enum, delete the `ConfirmScore(Color)` variant and its doc comment, and replace `ConfirmGameEnd`'s doc comment with:

```rust
    /// Operator confirms the score on the confirmation screen — mirrors
    /// `Message::ScoreConfirmation { correct: true }` (app/mod.rs): `set_scores` with the
    /// engine's own current score, a no-op write now that scores are applied when entered,
    /// then `end_confirm_pause`, which finishes the game.
    ///
    /// Also the only way to finish a game-ending timeout's confirm pause, which the driver's
    /// tick loop cannot auto-end because `end_game_ending_timeout` → `halt_clock` drops the
    /// start/stop latch, so no tick fires to reach `pause_has_ended`.
    ConfirmGameEnd,
```

In `apply_action`, delete the `Action::ConfirmScore(color)` arm and replace `ConfirmGameEnd`'s body comment to match the doc above; the body itself is unchanged:

```rust
        Action::ConfirmGameEnd => {
            // Mirrors Message::ScoreConfirmation { correct: true }: the engine already holds the
            // score, so this re-set is a no-op, and end_confirm_pause finishes the game.
            let s = tm.get_scores();
            tm.set_scores(s, now);
            tm.end_confirm_pause(now).unwrap();
        }
```

In the KNOWN COUPLING POINT comment block, replace the `ScoreSuddenDeath`, `ConfirmScore` and `ConfirmGameEnd` cross-reference lines with:

```rust
//   ScoreSuddenDeath     → Message::AddNewScore / AddScoreComplete, SD branch:
//                          add_deciding_goal(color, 0, now) — record the goal, then open the
//                          score-confirmation window; the window ends the game, either route.
//   ConfirmGameEnd       → Message::ScoreConfirmation { correct: true }: set_scores(current)
//                          (a no-op write) + end_confirm_pause(now). Also the only way to finish
//                          a game-ending timeout's confirm pause (halt_clock stopped the latch,
//                          so the tick loop cannot auto-confirm it).
```

In `scenarios.rs`: remove `ConfirmScore` from the `use` list on line 20, and replace all four `ConfirmScore(Color::Black)` actions with `ConfirmGameEnd` — at the `SUDDEN_DEATH_ACTIONS`, `SD_SCORE_CLOCK_STOPPED_ACTIONS`, `SD_SCORE_DURING_REF_TIMEOUT_ACTIONS` and `SD_SCORE_DURING_PENALTY_SHOT_ACTIONS` entries. Update the four inline comments and the `sudden_death` scenario header comment (~line 259 and ~842), which describe `ScoreSuddenDeath` as holding the score.

- [ ] **Step 6: Run the assertion test to verify it now passes**

Run: `cargo test -p refbox sudden_death_goal_survives`
Expected: PASS.

- [ ] **Step 7: Re-bless the baselines and READ every diff**

Run: `UPDATE_GOLDEN=1 cargo test -p refbox golden_traces_match_baseline`
Then: `git diff --stat refbox/src/tournament_manager/golden_traces/`

Expected: exactly five files — one new (`sudden_death_score_auto_confirm.trace`) and four modified: `sudden_death`, `sudden_death_score_clock_stopped`, `sudden_death_score_during_ref_timeout`, `sudden_death_score_during_penalty_shot`.

Then: `git diff refbox/src/tournament_manager/golden_traces/`

Every change in the four existing files must be the score moving **earlier** — appearing at the `ScoreSuddenDeath` offset instead of at the confirm offset. **Any other kind of change — a period appearing or disappearing, a clock value moving, a penalty changing — means something unintended happened. Stop and report it rather than accepting the bless.**

- [ ] **Step 8: Full suite and lint**

Run: `cargo test -p refbox`
Expected: 604 passed, 0 failed (599 baseline + 4 engine + 1 assertion).

Run: `cargo clippy -p refbox --all-targets --all-features -- -D warnings`
Expected: no warnings.

- [ ] **Step 9: Show the human the five trace diffs, then commit (ask first)**

The spec commits to reviewing these diffs with the human rather than silently blessing them. Present the `git diff` output before asking to commit.

```bash
git add refbox/src/tournament_manager/golden/ refbox/src/tournament_manager/golden_traces/
git commit -m "test(refbox): lock the unattended sudden-death confirm window"
```

---

### Task 4: Full gate, review, and walkthrough handoff

No code. This task closes out the repo's pre-PR criteria, which live in two documents — `.claude/rules/pr-review.md` (Claude's side, before a PR exists) and `docs/review-checklist.md` (the human's side, once it does).

- [ ] **Step 1: Full workspace gate**

Run: `just check`
Expected: fmt, lint, tests and audit all clean, across the whole workspace — not just `refbox`.

If `just audit` is red, check whether master is red too before touching anything: the advisory database moves independently of this branch.

- [ ] **Step 2: Code review**

Invoke `superpowers:requesting-code-review` — once, for the whole change. `.claude/rules/plan-execution.md` mandates this skill specifically; a checklist plus a green build is not a code review. Address findings before proceeding.

- [ ] **Step 3: Rebuild the binary the human will actually drive**

`just check` builds a *test* binary. Build the real one:

Run: `cargo build -p refbox --release`

- [ ] **Step 4: Hand over the walkthrough — then STOP**

A manual walkthrough of every UI change is mandatory in this workspace before any PR, and it is the human's to perform, not Claude's. Launch the app for them (`WAYLAND_DISPLAY=` unset for X11; back up `~/.config/refbox` first if the walkthrough needs settings changed) and hand over numbered steps covering the spec's five acceptance criteria, one at a time:

1. Sudden death is reachable: configure sudden death on, a short second half, and a 30-second minimum break (giving a 15-second confirmation window).
2. Score a goal in sudden death → the score updates on the display immediately and the confirmation screen appears with its countdown.
3. Tap CORRECT → the game ends and the between-games break starts.
4. Repeat, then **wait for the countdown to expire without touching anything** → the game must end the same way. *This is the bug; on today's build it stays in Sudden Death.*
5. Repeat, tap INCORRECT, restore the tie, confirm → play continues in sudden death.

Wait for their report on each before moving on. Do not describe any of this as verified by your own runs.

- [ ] **Step 5: Only then, discuss a PR**

Nothing is PR-ready before it is pushed, because CI has not run — CI green is step 1 of the human's own checklist. When reporting status, report against those two repo documents' line items, and say plainly what is outstanding.

---

## Deviations

Record any divergence from this plan here as it happens, rather than in standalone commits.

**1. Tasks 1 and 2 become a single commit.** Task 1's two new engine methods are dead code until
Task 2 calls them, so `just lint` fails at the end of Task 1 with "methods `add_deciding_goal` and
`set_deciding_scores` are never used". Silencing it with `#[allow(dead_code)]` would be the wrong
fix — `.claude/rules/rust.md` forbids `#[allow]` without discussion, and it would have to come
straight back out one task later. Task 1's own verification is unaffected: its four tests were
written first, observed failing to compile, and now pass. Only the commit boundary moved, so
Task 1 Step 8's commit is folded into Task 2 Step 7.

**3. Task 4's code review found a spec error, not an implementation error — and it added scope.**
The spec treated "the window expires" as one route. It is two: the window only self-resolves while
the clock-running latch is true, so a deciding goal scored with the clock **stopped** opens a window
that waits indefinitely. The organiser ruled that deliberate (*"if the operator stops the clock that
is an intentional action and we should not try to circumvent it"*), so no behaviour changed, but
three things were added beyond the plan: a spec section recording the ruling, an engine test
(`test_deciding_goal_with_clock_stopped_is_recorded_and_waits`), and golden scenario
`sudden_death_score_stopped_clock_waits` — the last two so a future reader who spots the asymmetry
cannot quietly "fix" it. Two further review findings went into the spec's accepted-limitations list
(a slow INCORRECT is lost once the window expires; the overlay's recent-goal flag is inconsistent for
a deciding goal). Minor findings fixed in place: a missing `unwrap` justification, missing `# Panics`
docs plus `debug_assert`s on both new operations, stale comments in the pre-existing R2 test block,
a `goals()` doc note, and the Family-11 comment letters reordered to read A–F.

**2. The lint command in Tasks 1, 2 and 3 is `just lint`, not the `--all-targets --all-features`
form the plan wrote.** The strict form has two failures that pre-date this branch and do not fail
CI: `items after a test module` at `refbox/src/app/view_builders/keypad_pages/player_grid.rs:116`
and `field assignment outside of initializer` at `refbox/src/app/mod.rs:6531`. `just lint` is
`cargo clippy --all -- -D warnings` plus a `--no-default-features` pass, which is what CI runs,
and it still catches the dead-code error above — so nothing is lost by using it.
