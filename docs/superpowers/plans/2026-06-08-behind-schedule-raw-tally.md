# Behind-Schedule Raw-Tally (Remove In-Game Buffer) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development or executing-plans. Checkbox steps.

**Goal:** Make the in-game behind-schedule figure a raw deviation tally (`start-lateness + freezes − removals`, floored at 0) by removing the in-game `slot_buffer` term, so skipping a half early always reduces the figure; the clawback now appears as a step-down at the break.

**Spec:** `docs/superpowers/specs/2026-06-08-behind-schedule-raw-tally-design.md`

**Process:** Heavy (game-clock read-model; many test literals change), test-first. One file: `refbox/src/tournament_manager/mod.rs`. No `uwh-common`/wire change.

---

## Task 1: In-game figure = raw deviation tally (remove the buffer)

**Files:** Modify/Test: `refbox/src/tournament_manager/mod.rs`

- [ ] **Step 1: Lead failing test** — the exact symptom the user reported (Start Now during half-time must reduce the delay even within the old slack). Add near the other `test_behind_schedule_*` tests:

```rust
#[test]
fn test_behind_schedule_start_now_during_halftime_reduces_delay() {
    initialize();
    // half 60, ht 10, sh 60 => regulation 130; block 180, min_break 5.
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
    // 20s stoppage in first half — within the OLD slack, so the old model hid it.
    tm.stop_clock(start + Duration::from_secs(10)).unwrap();
    tm.start_clock(start + Duration::from_secs(30)); // resume, 50s left on first half
    tm.update(start + Duration::from_secs(80)).unwrap(); // first half hits 0 -> HalfTime
    let t = start + Duration::from_secs(80);
    let before = tm.behind_schedule(t);
    assert_eq!(before, Duration::from_secs(20)); // raw deviation, no buffer hiding it
    tm.start_play_now(t).unwrap(); // skip the remaining 10s of half-time
    let after = tm.behind_schedule(t);
    assert_eq!(after, Duration::from_secs(10)); // dropped by the skipped 10s
}
```

- [ ] **Step 2: Run it, confirm FAIL** — `cd /home/estraily/projects/uwh-refbox-rs/.worktrees/game-block && cargo test -p refbox test_behind_schedule_start_now_during_halftime_reduces_delay`. Expected: FAIL (old model gives `before == 0` because the 20s is within the 45s slack). If it doesn't fail, STOP and report.

- [ ] **Step 3: Make the production change** in `behind_schedule`'s in-game (`else`) branch. **Delete** the `slot_buffer` computation block. Replace the final `let real_elapsed …; let projected_total …; let overrun …; let developing …; inherited + developing` with:

```rust
            // Raw deviation tally: the game's projected total wall-clock duration minus
            // the scheduled regulation play. Stoppages and edit-ups add; skipping a half
            // early and edit-downs subtract (deviation can be negative). Shown on top of
            // the lateness the game started with; floored at zero overall. The slot's
            // slack (clawback) is NOT subtracted here — it is realised between games as
            // the break compresses, so the figure steps down at the break when behind.
            let real_elapsed = now.saturating_duration_since(self.game_start_time);
            let projected_total = real_elapsed + self.remaining_regulation(now);
            let reg = self.config.regulation_play();
            if projected_total >= reg {
                inherited + (projected_total - reg)
            } else {
                inherited.saturating_sub(reg - projected_total)
            }
```

Keep `sched_start`/`inherited` as they are. Keep the `BetweenGames` branch unchanged. The `slot_buffer` and the `next_game_scheduled_start` call it used are now gone from the in-game branch (verify `next_game_scheduled_start` is still used by the between-games branch — it is; do not remove the method).

- [ ] **Step 4: Run the lead test, confirm PASS** — `cargo test -p refbox test_behind_schedule_start_now_during_halftime_reduces_delay`. Expected PASS (20 → 10).

- [ ] **Step 5: Recompute the shifted in-game test literals.** Each of these had the buffer subtracted; recompute by hand from `behind = max(0, inherited + (real_elapsed + remaining_regulation − regulation_play))` and update the literal. Show your calculation for each in your report. Do NOT weaken assertions.
  - `test_behind_schedule_grows_with_in_game_stoppage_beyond_buffer` (config: half 10, ht 3, min_break 2, block 40): the `t2` (20s stopped) assertion `5` → **`20`**; the second assertion (10s stopped) `ZERO` → **`10s`**.
  - `test_behind_schedule_frozen_while_running_clock_exceeds_period_len`: keep `a == b`; the value `15` → **`30`**.
  - `test_behind_schedule_in_game_edit_up_raises_figure_immediately`: `assert_eq!(before, …5)` → **`50`**; keep `after == before + 30s`.
  - `test_behind_schedule_in_game_edit_down_lowers_figure_immediately`: `assert_eq!(before, …45)` → **`90`**; keep `after == before − 30s`.
  - `test_behind_schedule_accrues_during_time_pause`, `test_behind_schedule_accrues_during_ref_timeout`: recompute the absolute literals (per-second climb unchanged; the buffer offset that was subtracted is gone — values increase by the buffer).
  - `test_behind_schedule_inherited_lateness_persists_in_manual_mode`, `test_behind_schedule_single_period_game`: recompute; likely unchanged where deviation is 0 (`inherited + 0`), but verify each sampled value.
  - `test_behind_schedule_frozen_through_half_time`: should stay `ZERO` (deviation 0) — verify.
  - `test_behind_schedule_climbs_in_overtime_beyond_buffer`, `test_behind_schedule_robust_to_midgame_half_length_change`: assertions are inequalities / equal-of-two-frozen-samples, so they should still hold — just run them.

- [ ] **Step 6: Rewrite the two tests whose premise changed.**
  - `test_behind_schedule_continuity_at_game_end_manual` → it now asserts a **step-down**, not continuity. Sample `behind_schedule` while still in-game just before `end_game`, then `end_game`, then sample just after. Assert the in-game value **exceeds** the between-games value by exactly the slot slack `game_block − regulation_play − minimum_break`. Compute both concrete values for the test's config and assert them. Rename to `test_behind_schedule_steps_down_by_slack_at_game_end` (and update any reference). Keep both branches honest — do not assert equality.
  - `test_behind_schedule_long_portal_gap_absorbs_overrun` → absorption now happens **at the break**, not during the game. Restructure: build an overrun during the game and assert the in-game figure is **> 0** (raw, not absorbed); then `end_game` into the long gap and assert the between-games figure is **ZERO** (the long break absorbs it). Keep using the same portal/long-gap setup the test already establishes.

- [ ] **Step 7: Full suite + clippy.** `cargo test -p refbox` (all pass — 219 expected: prior 218 + the new lead test), then `cargo clippy -p refbox -- -D warnings` (clean). If any between-games test (`..._between_games_overdue`, `..._recovered_by_long_break`, `..._between_games_climbs_when_break_overdue`, `..._between_games_follows_break_edit`, `..._zero_before_first_game_and_when_ahead`) fails, that is unexpected (the branch is unchanged) — investigate rather than edit the literal.

- [ ] **Step 8: Commit** (only this file):
```
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/game-block
git rev-parse --abbrev-ref HEAD   # must be feat/uwh-common/game-block
git add refbox/src/tournament_manager/mod.rs
git commit -m "fix(refbox): show raw behind-schedule tally in-game; clawback at the break

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: Full check and manual verification

- [ ] **Step 1:** `just check` → EXIT 0 (fmt, lint, tests, audit clean).
- [ ] **Step 2:** `WAYLAND_DISPLAY= cargo run -p refbox` (background) from the worktree. With **Show Behind Schedule Time** On:
  1. Get a game behind (a stoppage), enter half-time, click **Start Now** → the figure drops by the skipped half-time, **even if the delay was carried in from a late start**.
  2. Confirm a freeze (pause / timeout) still climbs it; editing the clock up/down still moves it immediately.
  3. When a game ends behind, the figure **steps down** at the break (clawback) — confirm that reads sensibly.
- [ ] **Step 3:** Stop the app; report observations in plain English.

## Self-review notes
- Production change is the in-game branch only; between-games unchanged. Lead test reproduces the reported symptom. All shifted literals recomputed from the one formula; two premise-changed tests rewritten. Acceptance criteria → Task 2.

## Deviations
(Record during execution.)
