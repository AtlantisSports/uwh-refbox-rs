# Behind-Schedule Calculation Rework — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the behind-schedule figure so it freezes while the run is moving through scheduled time and climbs whenever it is not — by measuring real stopped/overrun time directly instead of reconstructing it from clock readings.

**Architecture:** Replace the brittle `accumulated_overrun` reconstruction with a directly-measured `regulation_played` accumulator (wall-time the game spends counting down a regulation period with no timeout, capped at `regulation_play`). `behind_schedule`'s in-game branch becomes `inherited + saturating(real_elapsed − regulation_played − clawback)`; its between-games branch becomes `saturating((now + live_break_clock) − scheduled_next_start)`, which reacts to pauses and edits. Remove the `behind_at_game_end` frozen snapshot.

**Tech Stack:** Rust 2024, refbox crate (`src/tournament_manager/mod.rs`), `cargo test -p refbox`.

**Spec:** `docs/superpowers/specs/2026-06-08-behind-schedule-calc-rework-design.md`

**Process:** Heavy (game-clock state machine) — test-first, per-task verification with real unit tests. Confined to `refbox/src/tournament_manager/mod.rs`. No `uwh-common` / wire-format change.

---

## File Structure

- Modify: `refbox/src/tournament_manager/mod.rs`
  - Struct fields (~line 40–67): add `regulation_played`, `regulation_mark`; remove `behind_at_game_end`.
  - `new()` (~line 70–99): init new fields; drop `behind_at_game_end`.
  - `reset()` (~line 227–233): drop `behind_at_game_end` clear.
  - New private helpers: `in_regulation_progress`, `settle_regulation_played`, `regulation_played_now`.
  - `settle_regulation_played(now)` call inserted at the top of every `&mut self` method that takes `now` and can change the clock / timeout / period.
  - `start_game()` (~line 1050–1089): reset the accumulator.
  - `end_game()` (~line 980–1048): drop the `behind_at_game_end` assignment.
  - `behind_schedule()` (~line 2114–2142): rewrite both branches.
  - Delete `accumulated_overrun()` (~line 2062–2107) and its three tests.
  - Tests: add/replace `#[cfg(test)]` cases.

There is exactly one non-test consumer of `accumulated_overrun` (inside `behind_schedule`) and one of `behind_at_game_end` (inside `behind_schedule`), so both can be removed cleanly.

---

## Task 1: Bug 1 — figure must not climb while a regulation clock runs

Replaces the indirect overrun reconstruction with a direct measurement. This is the core change.

**Files:**
- Modify/Test: `refbox/src/tournament_manager/mod.rs`

- [ ] **Step 1: Write the failing test** (add inside the `#[cfg(test)] mod test` block, near the other `test_behind_schedule_*` tests, ~line 2800)

```rust
#[test]
fn test_behind_schedule_frozen_while_running_clock_exceeds_period_len() {
    // Reproduces the live bug: clock reading (40s) greater than the configured
    // half length (10s). The old reconstruction (period_len − clock) saturated to
    // zero and made the figure climb with real time while the clock ran.
    initialize();
    // Slot 40; regulation 2*10+3 = 23; min_break 2 => buffer 15.
    let config = GameConfig {
        half_play_duration: Duration::from_secs(10),
        half_time_duration: Duration::from_secs(3),
        minimum_break: Duration::from_secs(2),
        game_block: Duration::from_secs(40),
        overtime_allowed: false,
        sudden_death_allowed: false,
        ..Default::default()
    };
    let mut tm = TournamentManager::new(config);
    let start = Instant::now();
    tm.start_clock(start);
    tm.start_play_now(start).unwrap(); // FirstHalf, clock = 10, running
    tm.stop_clock(start).unwrap(); // must be stopped to set the clock
    tm.set_game_clock_time(Duration::from_secs(40)).unwrap(); // clock = 40 > half len
    tm.start_clock(start); // resume: counting down from 40, running

    // While the regulation clock runs, the figure must hold steady, whatever the
    // clock reads. Sample two points (both within regulation_play=23 of real time,
    // so still genuinely "playing regulation"): under the bug these differ (climb).
    let a = tm.behind_schedule(start + Duration::from_secs(16));
    let b = tm.behind_schedule(start + Duration::from_secs(19));
    assert_eq!(a, b, "figure climbed while the clock was running");
    assert_eq!(a, Duration::ZERO);
}
```

- [ ] **Step 2: Run it and confirm it fails**

Run: `cargo test -p refbox test_behind_schedule_frozen_while_running_clock_exceeds_period_len`
Expected: FAIL — `a` (≈1s) ≠ `b` (≈4s) under the current reconstruction.

- [ ] **Step 3: Add the new fields**

In the struct (~line 61, where `behind_at_game_end` is declared) **remove** `behind_at_game_end: Duration,` and its doc comment, and **add**:

```rust
    /// Wall-clock time this game has spent making *scheduled regulation* progress —
    /// counting down first half / half-time / second half with no timeout active —
    /// capped at `regulation_play`. Reset at `start_game`. `behind_schedule` uses it
    /// to measure overrun directly instead of reconstructing it from clock readings.
    regulation_played: Duration,
    /// Instant `regulation_played` was last folded up to (see `settle_regulation_played`).
    regulation_mark: Instant,
```

In `new()` (~line 91) **remove** `behind_at_game_end: Duration::ZERO,` and **add**:

```rust
            regulation_played: Duration::ZERO,
            regulation_mark: Instant::now(),
```

In `reset()` (~line 230) **remove** the line `self.behind_at_game_end = Duration::ZERO;`.

- [ ] **Step 4: Add the predicate and accumulator helpers**

Add these three private methods (place them just above `pub fn accumulated_overrun`, ~line 2061):

```rust
    /// True while the game is moving through *scheduled regulation* time at the
    /// scheduled pace: a regulation period's clock counting down, no timeout active.
    /// This — not "any clock running" — is what freezes the behind-schedule figure.
    fn in_regulation_progress(&self) -> bool {
        matches!(
            self.current_period,
            GamePeriod::FirstHalf | GamePeriod::HalfTime | GamePeriod::SecondHalf
        ) && self.timeout_state.is_none()
            && self.clock_state.is_running()
    }

    /// Fold the interval since the last mark into `regulation_played` when the game
    /// was making regulation progress, then advance the mark. Called at the top of
    /// every `&mut self` method that takes `now` and can change the clock/timeout/
    /// period. Over-calling is harmless: a non-progress interval only moves the mark.
    fn settle_regulation_played(&mut self, now: Instant) {
        if self.in_regulation_progress() {
            let played = now.saturating_duration_since(self.regulation_mark);
            self.regulation_played =
                (self.regulation_played + played).min(self.config.regulation_play());
        }
        self.regulation_mark = now;
    }

    /// `regulation_played` plus the not-yet-settled tail when currently progressing,
    /// so read-only callers (which cannot settle) see a live value between settles.
    /// Capped at `regulation_play`.
    fn regulation_played_now(&self, now: Instant) -> Duration {
        let mut played = self.regulation_played;
        if self.in_regulation_progress() {
            played += now.saturating_duration_since(self.regulation_mark);
        }
        played.min(self.config.regulation_play())
    }
```

- [ ] **Step 5: Reset the accumulator when a game starts**

In `start_game()` (~line 1080, right after `self.game_start_time = start_time;`) add:

```rust
        self.regulation_played = Duration::ZERO;
        self.regulation_mark = start_time;
```

- [ ] **Step 6: Settle at every clock/timeout/period transition**

Insert `self.settle_regulation_played(now);` as the **first statement** of each method below. (Over-calling is safe; the goal is never to *miss* a transition.) Methods and their current first lines:

- `update` (~1176) — the periodic tick; covers running accrual, auto-stops, and all period auto-advances.
- `start_clock` (~1543)
- `stop_clock` (~1589)
- `halt_clock` (~1639)
- `start_play_now` (~1698)
- `pause_for_confirm` (~1828)
- `end_confirm_pause` (~1902)
- `start_team_timeout` (~390), `start_ref_timeout` (~409), `start_penalty_shot` (~426), `start_rugby_penalty_shot` (~443)
- `switch_to_ref_timeout` (~471), `switch_to_rugby_penalty_shot` (~494)
- `end_timeout` (~540)
- `reset_game` (~207)
- `apply_next_game_start` (~950)
- `set_period_and_game_clock_time` (~1996)

For methods that early-return on a guard before mutating (e.g. `start_team_timeout` checks `can_start_team_timeout`), placing the settle first is still correct — the mark simply advances.

- [ ] **Step 7: Rewrite the in-game branch of `behind_schedule`**

Replace the `else { … }` branch body (the non-`BetweenGames` arm, ~line 2121–2141) with:

```rust
        } else {
            let Some(sched_start) = self.current_scheduled_start else {
                return Duration::ZERO;
            };
            let inherited = self.game_start_time.saturating_duration_since(sched_start);
            // Slack this game's slot has before its stoppages/overtime push the next
            // game late (manual mode = `game_block_buffer`); see the 2026-06-06 spec.
            let slot_buffer = match self.next_game_scheduled_start(now) {
                Some(sched_next) => sched_next
                    .saturating_duration_since(sched_start)
                    .saturating_sub(self.config.regulation_play() + self.config.minimum_break),
                None => self.config.game_block_buffer(),
            };
            // Overrun = real time the game has taken beyond the scheduled regulation
            // play it has actually completed. Frozen while regulation runs (both grow
            // together); climbs while stopped and while in extra time (regulation
            // capped). Measured directly, so immune to clock-reading-vs-period-length.
            let real_elapsed = now.saturating_duration_since(self.game_start_time);
            let overrun = real_elapsed.saturating_sub(self.regulation_played_now(now));
            let developing = overrun.saturating_sub(slot_buffer);
            inherited + developing
        }
```

(Leave the `BetweenGames` arm unchanged for now — Task 2 rewrites it. It still references `self.behind_at_game_end`, which still exists until Task 2; **to keep this task compiling**, temporarily replace the `BetweenGames` arm body `self.behind_at_game_end` with `Duration::ZERO` and note it is finished in Task 2.)

Concretely, the whole method after this step reads:

```rust
    pub fn behind_schedule(&self, now: Instant) -> Duration {
        if self.current_period == GamePeriod::BetweenGames {
            Duration::ZERO // TODO(Task 2): projected-next-start formula
        } else {
            // ... in-game branch above ...
        }
    }
```

- [ ] **Step 8: Run the Bug-1 test and confirm it passes**

Run: `cargo test -p refbox test_behind_schedule_frozen_while_running_clock_exceeds_period_len`
Expected: PASS.

- [ ] **Step 9: Run the in-game behind-schedule tests that should still hold**

Run: `cargo test -p refbox test_behind_schedule_grows_with_in_game_stoppage_beyond_buffer test_behind_schedule_accrues_during_time_pause test_behind_schedule_accrues_during_ref_timeout test_behind_schedule_inherited_lateness_persists_in_manual_mode test_behind_schedule_single_period_game`
Expected: PASS. (These assert in-game stoppage growth and inherited lateness, which the new direct measurement reproduces. If any now computes a different number, confirm against the spec model and update the test's expected value — recording why in the plan's Deviations section — only if the spec model genuinely yields the new value.)

- [ ] **Step 10: Commit**

```bash
git add refbox/src/tournament_manager/mod.rs docs/superpowers/plans/2026-06-08-behind-schedule-calc-rework.md
git commit -m "fix(refbox): measure behind-schedule overrun directly, not from clock"
```

---

## Task 2: Bug 2 — between-games figure must track the live break clock

**Files:**
- Modify/Test: `refbox/src/tournament_manager/mod.rs`

- [ ] **Step 1: Write the failing tests** (add near the other `test_behind_schedule_*` tests)

```rust
#[test]
fn test_behind_schedule_between_games_climbs_when_break_overdue() {
    initialize();
    // Manual mode, block 40, regulation 23, min_break 2 => buffer 15.
    let config = GameConfig {
        half_play_duration: Duration::from_secs(10),
        half_time_duration: Duration::from_secs(3),
        minimum_break: Duration::from_secs(2),
        game_block: Duration::from_secs(40),
        overtime_allowed: false,
        sudden_death_allowed: false,
        ..Default::default()
    };
    let mut tm = TournamentManager::new(config);
    let start = Instant::now();
    tm.start_clock(start);
    tm.start_play_now(start).unwrap();
    // Run a long stoppage so the game ends well behind, then end the game.
    tm.stop_clock(start + Duration::from_secs(5)).unwrap();
    let end = start + Duration::from_secs(60);
    tm.end_game(end);
    assert_eq!(tm.current_period, GamePeriod::BetweenGames);

    // The break counts down from `end`. While it counts down normally the figure
    // holds; once it has run out and we keep sitting, the figure must climb.
    let v_at_end = tm.behind_schedule(end);
    let v_one_min_later = tm.behind_schedule(end + Duration::from_secs(60));
    assert!(
        v_one_min_later > v_at_end,
        "between-games figure did not climb when overdue: {v_at_end:?} -> {v_one_min_later:?}"
    );
}

#[test]
fn test_behind_schedule_between_games_follows_break_edit() {
    initialize();
    let config = GameConfig {
        half_play_duration: Duration::from_secs(10),
        half_time_duration: Duration::from_secs(3),
        minimum_break: Duration::from_secs(2),
        game_block: Duration::from_secs(40),
        overtime_allowed: false,
        sudden_death_allowed: false,
        ..Default::default()
    };
    let mut tm = TournamentManager::new(config);
    let start = Instant::now();
    tm.start_clock(start);
    tm.start_play_now(start).unwrap();
    let end = start + Duration::from_secs(30);
    tm.end_game(end);

    let before = tm.behind_schedule(end);
    // Extending the break pushes the next game later => figure rises by the edit.
    tm.set_game_clock_time(tm.game_clock_time(end).unwrap() + Duration::from_secs(10))
        .unwrap();
    let after = tm.behind_schedule(end);
    assert_eq!(after, before + Duration::from_secs(10));
}
```

- [ ] **Step 2: Run them and confirm they fail**

Run: `cargo test -p refbox test_behind_schedule_between_games_climbs_when_break_overdue test_behind_schedule_between_games_follows_break_edit`
Expected: FAIL — the placeholder `Duration::ZERO` arm returns 0 for both samples.

- [ ] **Step 3: Rewrite the between-games branch**

Replace the `if self.current_period == GamePeriod::BetweenGames { … }` arm body (the `Duration::ZERO` placeholder from Task 1) with:

```rust
        if self.current_period == GamePeriod::BetweenGames {
            // Project the next game's start from the *live* break clock. While the
            // break counts down normally the projection holds steady (frozen);
            // pausing it, sitting past zero, or editing it slides the projection —
            // and the figure — by exactly that amount. Floored at zero (on-time/ahead).
            let Some(sched_next) = self.next_game_scheduled_start(now) else {
                return Duration::ZERO;
            };
            let remaining_break = self.clock_state.clock_time(now).unwrap_or(Duration::ZERO);
            let projected_next_start = now + remaining_break;
            projected_next_start.saturating_duration_since(sched_next)
        } else {
```

- [ ] **Step 4: Remove the now-unused `behind_at_game_end` write**

In `end_game()` (~line 986) remove the line `self.behind_at_game_end = self.behind_schedule(now);` and its preceding comment block (~983–985). The field itself was already removed in Task 1.

- [ ] **Step 5: Run the new tests and confirm they pass**

Run: `cargo test -p refbox test_behind_schedule_between_games_climbs_when_break_overdue test_behind_schedule_between_games_follows_break_edit`
Expected: PASS.

- [ ] **Step 6: Update the existing between-games tests to the new model**

The old `test_behind_schedule_frozen_during_break` asserts the *frozen* behaviour we are deliberately removing; `test_behind_schedule_between_games_overdue` and `test_behind_schedule_recovered_by_long_break` assert the snapshot/growing-formula model. Re-derive each expected value from the new projected formula (`(now + live_break_clock) − sched_next`, floored). For `..._recovered_by_long_break` the figure should still read `ZERO` once a long scheduled break makes `projected_next_start ≤ sched_next`; keep that assertion. Delete `test_behind_schedule_frozen_during_break` (its premise — a frozen break — is now incorrect) and replace it with the two new tests from Step 1 (already added). Run the trio:

Run: `cargo test -p refbox test_behind_schedule_between_games_overdue test_behind_schedule_recovered_by_long_break test_behind_schedule_continuity_at_game_end_manual test_behind_schedule_long_portal_gap_absorbs_overrun`
Expected: PASS (adjust expected literals to the projected formula where they differ; the continuity test must still pass unchanged — the branches agree at game end by construction).

- [ ] **Step 7: Commit**

```bash
git add refbox/src/tournament_manager/mod.rs
git commit -m "fix(refbox): between-games delay tracks live break clock, not a snapshot"
```

---

## Task 3: Remove the dead reconstruction and add model coverage

**Files:**
- Modify/Test: `refbox/src/tournament_manager/mod.rs`

- [ ] **Step 1: Delete `accumulated_overrun` and its tests**

Remove the method `pub fn accumulated_overrun(&self, now: Instant) -> Duration { … }` (~line 2062–2107) and the three tests `test_accumulated_overrun`, `test_accumulated_overrun_zero_between_games`, `test_accumulated_overrun_sudden_death_only_not_suppressed` (~line 2736–2798).

- [ ] **Step 2: Build to confirm nothing else referenced them**

Run: `cargo build -p refbox`
Expected: builds clean (the only caller was `behind_schedule`, already rewritten).

- [ ] **Step 3: Write model-coverage tests** (add to the test module)

```rust
#[test]
fn test_behind_schedule_frozen_through_half_time() {
    initialize();
    let config = GameConfig {
        half_play_duration: Duration::from_secs(10),
        half_time_duration: Duration::from_secs(6),
        minimum_break: Duration::from_secs(2),
        game_block: Duration::from_secs(40), // buffer 12
        overtime_allowed: false,
        sudden_death_allowed: false,
        ..Default::default()
    };
    let mut tm = TournamentManager::new(config);
    let start = Instant::now();
    tm.start_clock(start);
    tm.start_play_now(start).unwrap();
    // Advance into half-time (first half 10s, then half-time begins).
    tm.update(start + Duration::from_secs(11)).unwrap();
    assert_eq!(tm.current_period, GamePeriod::HalfTime);
    // Half-time is scheduled time => figure frozen while it counts down.
    let a = tm.behind_schedule(start + Duration::from_secs(12));
    let b = tm.behind_schedule(start + Duration::from_secs(14));
    assert_eq!(a, b);
    assert_eq!(a, Duration::ZERO);
}

#[test]
fn test_behind_schedule_climbs_in_overtime_beyond_buffer() {
    initialize();
    // Tight slot so overtime quickly exceeds the buffer.
    // regulation = 2*10 + 4 = 24; block 30, min_break 2 => buffer 4.
    let config = GameConfig {
        half_play_duration: Duration::from_secs(10),
        half_time_duration: Duration::from_secs(4),
        minimum_break: Duration::from_secs(2),
        game_block: Duration::from_secs(30),
        overtime_allowed: true,
        pre_overtime_break: Duration::from_secs(2),
        ot_half_play_duration: Duration::from_secs(10),
        ot_half_time_duration: Duration::from_secs(2),
        sudden_death_allowed: false,
        ..Default::default()
    };
    let mut tm = TournamentManager::new(config);
    let start = Instant::now();
    tm.start_clock(start);
    tm.start_play_now(start).unwrap();
    // Drive the clock to the end of regulation and into pre-overtime/overtime.
    // Regulation wall time = 24s with no stoppage; sample a few seconds into OT.
    let in_ot = start + Duration::from_secs(24 + 8);
    tm.update(in_ot).unwrap();
    // 8s of (unscheduled) extra time, buffer 4 => figure ~4s and rising.
    let v1 = tm.behind_schedule(in_ot);
    let v2 = tm.behind_schedule(in_ot + Duration::from_secs(3));
    assert!(v1 > Duration::ZERO, "overtime did not push the figure past the buffer");
    assert!(v2 > v1, "figure did not climb during overtime");
}

#[test]
fn test_behind_schedule_robust_to_midgame_half_length_change() {
    initialize();
    // Start with a 10s half, then shrink config to a 5s half mid-game (the kind of
    // config/clock mismatch that triggered the original bug). Running => frozen.
    let config = GameConfig {
        half_play_duration: Duration::from_secs(10),
        half_time_duration: Duration::from_secs(3),
        minimum_break: Duration::from_secs(2),
        game_block: Duration::from_secs(40),
        overtime_allowed: false,
        sudden_death_allowed: false,
        ..Default::default()
    };
    let mut tm = TournamentManager::new(config);
    let start = Instant::now();
    tm.start_clock(start);
    tm.start_play_now(start).unwrap();
    tm.stop_clock(start).unwrap();
    let mut shrunk = tm.config().clone();
    shrunk.half_play_duration = Duration::from_secs(5); // now clock (10) > half (5)
    tm.set_config(shrunk).unwrap();
    tm.start_clock(start);
    let a = tm.behind_schedule(start + Duration::from_secs(2));
    let b = tm.behind_schedule(start + Duration::from_secs(4));
    assert_eq!(a, b, "figure climbed while running after a half-length change");
}
```

(If `GameConfig` field names for overtime durations differ, read the struct in `uwh-common/src/config.rs` and use the exact names; if `tm.config()` does not exist, read the field directly via an existing accessor pattern used by sibling tests.)

- [ ] **Step 4: Run the new model tests**

Run: `cargo test -p refbox test_behind_schedule_frozen_through_half_time test_behind_schedule_climbs_in_overtime_beyond_buffer test_behind_schedule_robust_to_midgame_half_length_change`
Expected: PASS.

- [ ] **Step 5: Run the whole tournament-manager suite**

Run: `cargo test -p refbox`
Expected: all pass (refbox suite was 214). If a pre-existing `behind_schedule`/`accumulated_overrun` test now encodes the old model, re-derive its expected value from the spec and update it, noting the change in the Deviations section.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/tournament_manager/mod.rs
git commit -m "test(refbox): cover half-time, overtime, and config-change behind-schedule cases"
```

---

## Task 4: Full check and manual verification

**Files:** none (verification only)

- [ ] **Step 1: Run the full gate**

Run: `just check`
Expected: fmt, lint, tests, audit all clean. (refbox is bin-only — `just lint` runs `cargo clippy -p refbox -- -D warnings`, not `--all-targets`.)

- [ ] **Step 2: Launch the app from the worktree and verify by hand**

Run (background): `WAYLAND_DISPLAY= cargo run -p refbox`
Verify, with **Show Behind Schedule Time** On and a short-game config:
1. Start a game; while the clock counts down (including half-time) the figure does **not** climb.
2. Stop the clock / take a ref timeout — the figure climbs ~1/sec.
3. Let a game run into overtime — the figure starts climbing once the slot's slack is used.
4. Between games: a normally-counting break holds the figure; pause it or sit past zero → it climbs; edit the break time → it moves by that amount.

- [ ] **Step 3: Stop the app.** Report results to the human in plain English (what was observed at each step) before requesting code review.

---

## Self-review notes

- **Spec coverage:** Bug 1 (Task 1), Bug 2 (Task 2), overtime climb + half-time frozen + robustness + dead-reconstruction removal (Task 3), continuity (Task 2 Step 6), acceptance criteria (Task 4). Clawback definition unchanged (reused in Task 1 Step 7).
- **Type consistency:** new methods `in_regulation_progress`/`settle_regulation_played`/`regulation_played_now`, fields `regulation_played`/`regulation_mark`, used consistently across tasks.
- **No silent caps:** `regulation_played` is capped at `regulation_play` by design (overtime/edited-up play becomes overrun); this is the intended model, not a dropped case.

## Deviations

(Record here during execution if expected test literals or method names needed adjustment.)
