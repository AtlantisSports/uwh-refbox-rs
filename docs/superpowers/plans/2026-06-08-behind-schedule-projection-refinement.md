# Behind-Schedule Projection Refinement — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Make the in-game behind-schedule figure forward-projecting so a clock edit moves it immediately, replacing the stopped-time accumulator with one projection helper.

**Architecture:** In-game overrun becomes `saturating((real_elapsed + remaining_regulation(now)) − regulation_play)`, where `remaining_regulation` is the live remaining on the current regulation period plus the full length of later regulation periods (ZERO in extra time). Identical to the accumulator off the edit path; reacts instantly to edits. The `regulation_played` accumulator and its 16 settle sites are deleted.

**Spec:** `docs/superpowers/specs/2026-06-08-behind-schedule-projection-refinement-design.md`

**Process:** Heavy (game-clock state machine), test-first. Confined to `refbox/src/tournament_manager/mod.rs`. No `uwh-common`/wire-format change.

---

## Task 1: In-game figure projects remaining play; remove the accumulator

**Files:** Modify/Test: `refbox/src/tournament_manager/mod.rs`

- [ ] **Step 1: Write the failing in-game-edit tests** (add near the other `test_behind_schedule_*` tests)

```rust
#[test]
fn test_behind_schedule_in_game_edit_up_raises_figure_immediately() {
    initialize();
    // half 60, ht 10, sh 60 => regulation 130; block 180, min_break 5 => buffer 45.
    let config = GameConfig {
        half_play_duration: Duration::from_secs(60),
        half_time_duration: Duration::from_secs(10),
        minimum_break: Duration::from_secs(5),
        game_block: Duration::from_secs(180),
        overtime_allowed: false,
        sudden_death_allowed: false,
        ..Default::default()
    };
    let mut tm = TournamentManager::new(config);
    let start = Instant::now();
    tm.start_clock(start);
    tm.start_play_now(start).unwrap();
    // 50s of stoppage so we are already behind (overrun 50 - buffer 45 = 5).
    tm.stop_clock(start + Duration::from_secs(10)).unwrap();
    let t = start + Duration::from_secs(60); // 50s stopped, clock frozen at 50
    let before = tm.behind_schedule(t);
    assert_eq!(before, Duration::from_secs(5));
    // Add 30s of play to the (stopped) game clock — the app's edit path.
    tm.set_game_clock_time(tm.game_clock_time(t).unwrap() + Duration::from_secs(30))
        .unwrap();
    let after = tm.behind_schedule(t);
    assert_eq!(
        after,
        before + Duration::from_secs(30),
        "edit up did not raise the figure by the edited amount"
    );
}

#[test]
fn test_behind_schedule_in_game_edit_down_lowers_figure_immediately() {
    initialize();
    let config = GameConfig {
        half_play_duration: Duration::from_secs(60),
        half_time_duration: Duration::from_secs(10),
        minimum_break: Duration::from_secs(5),
        game_block: Duration::from_secs(180),
        overtime_allowed: false,
        sudden_death_allowed: false,
        ..Default::default()
    };
    let mut tm = TournamentManager::new(config);
    let start = Instant::now();
    tm.start_clock(start);
    tm.start_play_now(start).unwrap();
    // 90s stopped => overrun 90 - buffer 45 = 45.
    tm.stop_clock(start + Duration::from_secs(10)).unwrap();
    let t = start + Duration::from_secs(100);
    let before = tm.behind_schedule(t);
    assert_eq!(before, Duration::from_secs(45));
    // Remove 30s of play from the (stopped) game clock.
    tm.set_game_clock_time(tm.game_clock_time(t).unwrap() - Duration::from_secs(30))
        .unwrap();
    let after = tm.behind_schedule(t);
    assert_eq!(
        after,
        before - Duration::from_secs(30),
        "edit down did not lower the figure by the edited amount"
    );
}
```

- [ ] **Step 2: Run them, confirm they FAIL**

`cd /home/estraily/projects/uwh-refbox-rs/.worktrees/game-block && cargo test -p refbox test_behind_schedule_in_game_edit`
Expected: FAIL — the accumulator-based figure ignores the clock value, so `after == before` (e.g. `5 == 5`, not `35`).

- [ ] **Step 3: Add the `remaining_regulation` helper** (place near `behind_schedule`, ~where the deleted helpers were)

```rust
    /// Scheduled regulation play still to come before the game reaches the end of
    /// regulation: the live remaining on the current regulation period plus the full
    /// length of any regulation periods after it. ZERO in extra time (overtime /
    /// sudden death and their breaks) and between games. Lets `behind_schedule`
    /// project when the game will end, so a clock edit moves the figure immediately.
    fn remaining_regulation(&self, now: Instant) -> Duration {
        let remaining_current = self.clock_state.clock_time(now).unwrap_or(Duration::ZERO);
        match self.current_period {
            GamePeriod::FirstHalf => {
                if self.config.single_half {
                    remaining_current
                } else {
                    remaining_current
                        + self.config.half_time_duration
                        + self.config.half_play_duration
                }
            }
            GamePeriod::HalfTime => remaining_current + self.config.half_play_duration,
            GamePeriod::SecondHalf => remaining_current,
            _ => Duration::ZERO,
        }
    }
```

- [ ] **Step 4: Rewrite the in-game overrun in `behind_schedule`**

In the `else` (in-game) branch, replace the lines computing `real_elapsed`/`overrun`/`developing` with:

```rust
            // Forward projection: the game's projected total wall-clock duration is the
            // time it has taken so far plus the regulation play still to come. Overrun is
            // the part of that beyond the scheduled regulation length. Frozen while a
            // regulation clock runs (real time and remaining move together); climbs while
            // stopped and in extra time; and an in-game clock edit shifts `remaining` —
            // and the figure — immediately. Reads the live remaining clock directly, so
            // it is immune to clock-reading-vs-period-length.
            let real_elapsed = now.saturating_duration_since(self.game_start_time);
            let projected_total = real_elapsed + self.remaining_regulation(now);
            let overrun = projected_total.saturating_sub(self.config.regulation_play());
            let developing = overrun.saturating_sub(slot_buffer);
            inherited + developing
```

(Keep `sched_start`, `inherited`, and `slot_buffer` exactly as they are. The `BetweenGames` branch is unchanged.)

- [ ] **Step 5: Delete the accumulator machinery** (the compiler will flag every remaining reference)

- Remove fields `regulation_played: Duration,` and `regulation_mark: Instant,` from the struct.
- Remove their initialisers in `new()` (`regulation_played: Duration::ZERO,` and `regulation_mark: Instant::now(),`).
- Remove the reset in `start_game()` (`self.regulation_played = Duration::ZERO;` and `self.regulation_mark = start_time;`).
- Delete the methods `in_regulation_progress`, `settle_regulation_played`, and `regulation_played_now`.
- Delete every `self.settle_regulation_played(now);` call (16 of them, at the tops of `update`, `start_clock`, `stop_clock`, `halt_clock`, `start_play_now`, `pause_for_confirm`, `end_confirm_pause`, `start_team_timeout`, `start_ref_timeout`, `start_penalty_shot`, `start_rugby_penalty_shot`, `switch_to_ref_timeout`, `switch_to_rugby_penalty_shot`, `end_timeout`, `reset_game`, `apply_next_game_start`).

- [ ] **Step 6: Build**

`cargo build -p refbox`
Expected: clean. If anything still references a removed item, remove that reference (do not re-introduce the accumulator).

- [ ] **Step 7: Adjust the clock-exceeds-period test literal**

In `test_behind_schedule_frozen_while_running_clock_exceeds_period_len`, keep `assert_eq!(a, b)` (still frozen while running). Replace `assert_eq!(a, Duration::ZERO);` with:

```rust
    // Projection correctly reports the over-long clock as a frozen, nonzero figure:
    // a 40s clock in a 10s half projects ~30s of extra play; 30 - 15 buffer = 15.
    assert_eq!(a, Duration::from_secs(15));
```

- [ ] **Step 8: Run the new tests, then the whole behind-schedule suite, then the crate**

`cargo test -p refbox test_behind_schedule_in_game_edit` → PASS.
`cargo test -p refbox behind_schedule` → all PASS.
`cargo test -p refbox` → all PASS (216 expected — accumulator tests were already removed in the prior plan; no test count change beyond the two new ones).
If any prior `test_behind_schedule_*` now differs, re-derive its expected value from the projection model by hand (it should be identical off the edit path). Only change a literal if the projection genuinely yields it; record the calculation in your report. Do NOT weaken assertions.

- [ ] **Step 9: Clippy**

`cargo clippy -p refbox -- -D warnings` → clean (no dead code; no leftover `#[allow]`).

- [ ] **Step 10: Commit** (only this file)

```
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/game-block
git rev-parse --abbrev-ref HEAD   # must be feat/uwh-common/game-block
git add refbox/src/tournament_manager/mod.rs
git commit -m "fix(refbox): project remaining play so in-game clock edits move the delay

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: Full check and manual verification

**Files:** none (verification only)

- [ ] **Step 1:** `just check` → fmt, lint, tests, audit all clean (EXIT 0).
- [ ] **Step 2:** Launch from the worktree: `WAYLAND_DISPLAY= cargo run -p refbox` (background). With **Show Behind Schedule Time** On and a short game in progress and already behind:
  1. Open the time edit, **add** time to the game clock, apply → the delay figure jumps **up** by the edited amount immediately.
  2. Open the time edit, **remove** time, apply → the figure drops by that amount immediately.
  3. Confirm the earlier behaviours still hold: frozen during regulation play and half-time; climbs while paused / during a normal timeout; climbs in overtime past the slack; between-games break edit still moves it.
- [ ] **Step 3:** Stop the app; report observations to the human in plain English.

## Self-review notes

- Spec coverage: projection (Task 1 Steps 3-4), accumulator removal (Step 5), in-game-edit tests (Step 1), adjusted clock>len literal (Step 7), full regression (Step 8), acceptance criteria (Task 2).
- Off the edit path the projection equals the prior model algebraically, so the other behind-schedule tests are expected to pass unchanged.

## Deviations

(Record here during execution.)
