# Beep-Test Warmup Countdown — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Initialize the BeepTest cadence engine's clock and `time_in_next_lap` from `BeepTestPeriod::Level(0)` helpers, so the fresh state and post-reset state both show the configured warmup duration (e.g. 0:10), and the value of `time_in_next_lap` reflects the period that actually follows the warmup (Level 1).

**Architecture:** Replace two index-into-`config.levels` lookups in `cadence.rs` with the existing `BeepTestPeriod::Level(0).duration(&config)` and `next_test_period_dur(&config)` helpers. Apply in `TournamentManager::new()` and `reset_beep_test_now()`. No view-builder change needed — once the engine emits the correct `secs_in_period`, the existing TIME tile picks it up automatically.

**Tech Stack:** Rust 2024, MSRV 1.85.

**Spec:** `docs/superpowers/specs/2026-05-21-beep-test-warmup-countdown-design.md` (committed at `b2f82e2`).

**Process:** Heavy (per `.claude/rules/plan-execution.md` — state-machine change). Per-task TDD: write failing test, confirm it fails, apply minimal fix, confirm it passes, commit. Final walkthrough at the end.

---

## File Structure

| File | Role | Tasks |
|------|------|-------|
| `refbox/src/beep_test/cadence.rs` | `TournamentManager::new()`, `TournamentManager::reset_beep_test_now()`, and the existing `#[cfg(test)] mod tests` block | Tasks 1, 2 |

Two test+fix tasks (one for `new()`, one for `reset_beep_test_now()`) plus a walkthrough. The existing `test_config()` helper at the top of the test module already supplies a config with `pre = 1s`, `levels[0].duration = 10s`, `levels[1].duration = 8s` — exactly the shape we need.

---

### Task 1: TDD — `new()` initializes clock + time_in_next_lap from Level(0)

**Files:**
- Modify: `refbox/src/beep_test/cadence.rs` (the `#[cfg(test)] mod tests` block at the bottom; the `TournamentManager::new()` impl at the top)

- [ ] **Step 1: Add two failing tests**

Append to the `mod tests` block in `refbox/src/beep_test/cadence.rs` (after the existing tests, before the `impl ClockState` block):

```rust
// Test — a freshly constructed engine's clock is set to the warm-up
// duration (config.pre), not the first user level's duration. The
// stopped clock_time is what the TIME tile displays before the operator
// presses START.
#[test]
fn new_initializes_clock_to_warmup_duration() {
    let tm = TournamentManager::new(test_config());
    let now = Instant::now();
    // test_config(): pre = 1s, levels[0].duration = 10s
    assert_eq!(
        tm.clock_state.clock_time(now),
        Some(Duration::from_secs(1))
    );
}

// Test — a freshly constructed engine's `time_in_next_lap` is the
// duration of the period that follows the warm-up (Level(1), which maps
// to config.levels[0]), not config.levels[1].
#[test]
fn new_initializes_time_in_next_lap_to_first_user_level() {
    let tm = TournamentManager::new(test_config());
    // test_config(): levels[0].duration = 10s (this is Level(1)'s duration)
    assert_eq!(tm.time_in_next_lap, Duration::from_secs(10));
}
```

- [ ] **Step 2: Run the new tests; confirm they FAIL**

```
cargo test -p refbox --lib beep_test::cadence::tests::new_initializes_clock_to_warmup_duration
cargo test -p refbox --lib beep_test::cadence::tests::new_initializes_time_in_next_lap_to_first_user_level
```

Expected: both FAIL. `new_initializes_clock_to_warmup_duration` fails because `clock_time` is 10s (first level) instead of 1s (warm-up). `new_initializes_time_in_next_lap_to_first_user_level` fails because `time_in_next_lap` is 8s (`levels[1]`) instead of 10s (`levels[0]`).

If either passes unexpectedly, stop and ask — the source code may have already been changed, or the tests don't match the bug.

- [ ] **Step 3: Apply the fix in `new()`**

In `refbox/src/beep_test/cadence.rs`, find `TournamentManager::new()` (around line 50). Replace:

```rust
let initial_clock = config
    .levels
    .first()
    .map(|l| l.duration)
    .unwrap_or_default();
Self {
    time_in_next_lap: config.levels.get(1).map(|l| l.duration).unwrap_or_default(),
    current_period: BeepTestPeriod::Level(0),
    clock_state: ClockState::Stopped {
        clock_time: initial_clock,
    },
    config,
    time_state: TimeState::None,
    start_stop_tx,
    start_stop_rx,
    count: 1,
    lap_count: 0,
}
```

with:

```rust
let warmup = BeepTestPeriod::Level(0);
let initial_clock = warmup.duration(&config).unwrap_or_default();
let time_in_next_lap = warmup.next_test_period_dur(&config).unwrap_or_default();
Self {
    time_in_next_lap,
    current_period: warmup,
    clock_state: ClockState::Stopped {
        clock_time: initial_clock,
    },
    config,
    time_state: TimeState::None,
    start_stop_tx,
    start_stop_rx,
    count: 1,
    lap_count: 0,
}
```

- [ ] **Step 4: Run the two new tests; confirm they PASS**

```
cargo test -p refbox --lib beep_test::cadence::tests::new_initializes_clock_to_warmup_duration
cargo test -p refbox --lib beep_test::cadence::tests::new_initializes_time_in_next_lap_to_first_user_level
```

Expected: both PASS.

- [ ] **Step 5: Run the full beep-test test suite to confirm no regression**

```
cargo test -p refbox --lib beep_test::cadence::tests
```

Expected: all existing tests PASS plus the two new ones.

- [ ] **Step 6: Commit**

```
git add refbox/src/beep_test/cadence.rs
git commit -m "fix(refbox): initialize beep-test cadence from Level(0) helpers"
```

With Co-Authored-By footer:

```
Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

---

### Task 2: TDD — `reset_beep_test_now()` initializes `time_in_next_lap` from Level(0)

**Files:**
- Modify: `refbox/src/beep_test/cadence.rs` (the `#[cfg(test)] mod tests` block; the `reset_beep_test_now` impl around line 178)

- [ ] **Step 1: Add a failing test**

Append to the `mod tests` block (after Task 1's tests):

```rust
// Test — after reset_beep_test_now(), time_in_next_lap is the duration
// of the period that follows the warm-up (Level(1), which maps to
// config.levels[0]). Mirrors the `new()` invariant — Reset returns the
// engine to a state functionally identical to `new()`.
#[test]
fn reset_initializes_time_in_next_lap_to_first_user_level() {
    let mut tm = TournamentManager::new(test_config());
    let now = Instant::now();
    // Move the engine away from the fresh state so the reset has work to
    // do beyond "no-op" — start a beep test, advance partway.
    tm.start_beep_test_now(now).unwrap();
    tm.reset_beep_test_now(now + Duration::from_millis(500));
    // After reset, time_in_next_lap should be levels[0].duration = 10s.
    assert_eq!(tm.time_in_next_lap, Duration::from_secs(10));
}
```

- [ ] **Step 2: Run the new test; confirm it FAILS**

```
cargo test -p refbox --lib beep_test::cadence::tests::reset_initializes_time_in_next_lap_to_first_user_level
```

Expected: FAIL. `time_in_next_lap` is set to 8s (`config.levels[1]`) inside `reset_beep_test_now`, not 10s (`config.levels[0]`).

- [ ] **Step 3: Apply the fix in `reset_beep_test_now()`**

In `refbox/src/beep_test/cadence.rs`, find `reset_beep_test_now` (around line 178). Replace the `time_in_next_lap` assignment:

```rust
self.time_in_next_lap = self
    .config
    .levels
    .get(1)
    .map(|l| l.duration)
    .unwrap_or_default();
```

with:

```rust
self.time_in_next_lap = self
    .current_period
    .next_test_period_dur(&self.config)
    .unwrap_or_default();
```

(`self.current_period` was just set to `BeepTestPeriod::Level(0)` a few lines above, so this resolves to `Level(0).next_test_period_dur(&config)` = Level(1).duration = `config.levels[0].duration`.)

The `initial_clock` line just above this block is already correct (uses `self.current_period.duration(&self.config)`) — do NOT change it.

- [ ] **Step 4: Run the new test; confirm it PASSES**

```
cargo test -p refbox --lib beep_test::cadence::tests::reset_initializes_time_in_next_lap_to_first_user_level
```

Expected: PASS.

- [ ] **Step 5: Run the full beep-test test suite**

```
cargo test -p refbox --lib beep_test::cadence::tests
```

Expected: all tests PASS (existing + new from Tasks 1 and 2).

- [ ] **Step 6: Run `just check` to confirm fmt, clippy, and full test suite**

```
just check
```

Expected: PASS.

- [ ] **Step 7: Commit**

```
git add refbox/src/beep_test/cadence.rs
git commit -m "fix(refbox): reset_beep_test_now initializes time_in_next_lap from Level(0)"
```

With Co-Authored-By footer.

---

### Task 3: Walkthrough verification

**Files:** none. Smoke-test the running refbox.

- [ ] **Step 1: Launch the refbox**

```
WAYLAND_DISPLAY= cargo run --manifest-path /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign/Cargo.toml -p refbox
```

(Per memory: `--manifest-path` is preferred over `cd && cargo run` for background launches.)

- [ ] **Step 2: Walk through the spec's six acceptance criteria**

In BeepTest mode (switch from Settings → Mode if not already there):

1. **Fresh launch.** TIME tile reads `0:10` (or whatever `config.pre` is), LEVEL `0`, LAP `0`.
2. **Press START.** TIME counts down from `0:10` to `0:00` over ten seconds. Whistle at `0:05`, buzzer at `0:00`.
3. **At `0:00`.** Engine transitions to Level 1 Lap 1: LEVEL becomes `1`, LAP becomes `1`, TIME becomes the first user level's duration.
4. **Press PAUSE during the warmup countdown.** Clock holds at the current warmup remaining time.
5. **Press RESUME during the warmup.** Clock continues counting down from the held value.
6. **Press RESET at any time.** Returns to fresh state (TIME `0:10`, LEVEL `0`, LAP `0`).

Report any scenario that fails. The cadence engine timing was already correct for `start_beep_test_now` (warmup countdown), so scenarios 2–6 should already work once scenario 1 is fixed; the goal is to confirm nothing regressed.

- [ ] **Step 3: Hand back to operator**

Report walkthrough results. Do not push — branch held for stacked PR.

---

## Deviations

(Append notes here if execution deviates from the plan. Per `.claude/rules/plan-execution.md`, fold deviation notes into the code commit that introduced the deviation; no standalone doc-only deviation commits.)

### Walkthrough fix-up: RESUME from warmup pause was resetting the clock

Spec scenario 5 (RESUME during warmup → countdown continues from held value) failed during the walkthrough. The clock reset to the full warmup duration instead of resuming.

Root cause was in `refbox/src/app/mod.rs`'s `Message::BeepTestStart` handler. It dispatched between `start_beep_test_now` (which resets the clock to `Level(0).duration()`) and `start_clock` (which preserves `clock_state`) based on `current_period`. During the warmup, `current_period == Level(0)` in BOTH the fresh state AND the paused state, so the handler always took the "fresh-start" branch and clobbered the paused time.

The correct distinguisher is `beep_test_has_run`. The dispatcher now snapshots `was_run_already = self.beep_test_has_run` before setting `has_run = true`, and uses that value to decide: `false` → first-ever press of START in this session, call `start_beep_test_now`; `true` → resuming from a pause, call `start_clock`.

Fix scope was in `mod.rs` only (no cadence engine change), so the heavy-process state-machine ceremony from Tasks 1+2 was not re-applied. Walkthrough verification covers it.

---

## Self-review notes

- **Spec coverage:** Spec §Design `new()` change maps to Task 1. Spec §Design `reset_beep_test_now()` change maps to Task 2. Spec §Acceptance criteria maps to Task 3 walkthrough.
- **Type consistency:** No new types. `BeepTestPeriod::Level(0)` and its `.duration()` / `.next_test_period_dur()` helpers are already in scope inside `cadence.rs` via `use super::*` style of the existing imports (`BeepTestPeriod` is imported at the top of the file).
- **No placeholders:** all steps show concrete code/commands.
- **Heavy process:** per-task TDD discipline (test → fail → fix → pass → commit). Final walkthrough.
- **Test isolation:** Tests use the existing `test_config()` helper. The two new fields exposed by tests (`clock_state`, `time_in_next_lap`) are private but accessible to the `mod tests` submodule per Rust visibility rules.
