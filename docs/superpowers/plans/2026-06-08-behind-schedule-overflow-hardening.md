# Behind-Schedule Overflow Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the two new operator-facing read-model `Instant + Duration` sites panic-proof on arbitrary input, and add two regression tests documenting far-future-portal and zero-length-regulation behaviour.

**Architecture:** Replace `now + d` with `now.checked_add(d).unwrap_or(now)` in `next_game_scheduled_start` and `behind_schedule`. Both methods are `&self` read-model helpers used only by the gated "behind schedule" display; the guard is defense-in-depth that cannot change observable behaviour on any realistic input. Add two characterization tests in the existing `#[cfg(test)] mod test`.

**Tech Stack:** Rust 2024, `std::time::{Instant, Duration}`, `time::OffsetDateTime`, refbox crate test harness (`cargo test -p refbox`).

---

## Context the executor needs

- File under change: `refbox/src/tournament_manager/mod.rs` (the game clock / state machine — the most critical file in the app). These two methods are **read-model only**: they take `&self`, cannot mutate state, and feed exactly one config-gated display site (`refbox/src/app/mod.rs:3803`). They do **not** drive the clock, the scheduling formula, or the wire/snapshot format. Do not touch any other method.
- **Out of scope (do not touch):** the pre-existing `Instant::now() + dur` in `calc_time_to_next_game` (proven byte-identical to master; a separate concern), the `next_scheduled_start` formula, `game_block`, and anything in `uwh-common`.
- Why `checked_add` and not `saturating_add`: `std::Instant` has no `saturating_add`; `checked_add` returns `Option<Instant>`, and falling back to `now` yields a sensible "scheduled = now" reading rather than a panic. This mirrors the existing `checked_sub(...).unwrap_or(now)` already used two lines above in `next_game_scheduled_start`.
- **Honest note on testability:** on Linux, a *valid* `OffsetDateTime` delta cannot make `now + d` overflow `Instant` (a Timespec-backed `Instant` represents thousands of years), so the `checked_add → None → now` fallback branch cannot be forced deterministically in a portable test. Therefore both tests are **characterization/regression tests**: they pass *before and after* the production guard, proving the guard changes nothing observable while documenting graceful handling of extreme-but-valid inputs. The guard's value is that it makes a panic impossible *in principle* for the now operator-watched figure. The executor should expect both tests to be green even against the un-hardened code; that is correct and expected, not a TDD failure.
- Test module conventions in this file: each test starts with `initialize();`; configs are built with `GameConfig { ... , ..Default::default() }`; `NextGameInfo` is constructed as `NextGameInfo { number: "..".to_string(), timing: None, start_time: Some(OffsetDateTime::now_utc() + time::Duration::hours(1)) }` (see the existing `test_behind_schedule_long_portal_gap_absorbs_overrun`). `OffsetDateTime` and `time::Duration` are already imported in this module.

---

## File Structure

- Modify: `refbox/src/tournament_manager/mod.rs:902` — guard `now + d` in `next_game_scheduled_start`.
- Modify: `refbox/src/tournament_manager/mod.rs:2084` — guard `now + remaining_break` in `behind_schedule`.
- Test: `refbox/src/tournament_manager/mod.rs` (the in-file `#[cfg(test)] mod test`) — two new tests, placed next to the other `test_behind_schedule_*` tests.

---

### Task 1: Add the two regression tests (characterization)

**Files:**
- Test: `refbox/src/tournament_manager/mod.rs` (inside `mod test`, near the other `test_behind_schedule_*` tests)

- [ ] **Step 1: Write the far-future-portal test**

Place after `test_behind_schedule_long_portal_gap_absorbs_overrun`:

```rust
#[test]
fn test_behind_schedule_far_future_portal_time_is_safe() {
    // A portal scheduled start an extreme distance in the future (e.g. a fat-fingered
    // date) must never panic and must produce a finite, sensible figure. With the next
    // game ~1000 years out, an ended game is trivially "on time" => ZERO. This exercises
    // the `now + d` path in next_game_scheduled_start with a very large positive delta.
    initialize();
    let mut tm = TournamentManager::new(behind_test_config());
    let start = Instant::now();
    tm.start_clock(start);
    tm.start_play_now(start).unwrap();
    tm.set_next_game(NextGameInfo {
        number: "2".to_string(),
        timing: None,
        start_time: Some(OffsetDateTime::now_utc() + time::Duration::days(365 * 1000)),
    });
    tm.stop_clock(start).unwrap();
    tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(0));
    let end = start + Duration::from_secs(30);
    tm.end_game(end);
    // No panic, and the absurdly-distant next start reads as on-time.
    assert_eq!(tm.behind_schedule(end), Duration::ZERO);
}
```

- [ ] **Step 2: Write the zero-length-regulation test**

Place immediately after the test from Step 1:

```rust
#[test]
fn test_behind_schedule_zero_length_regulation_is_safe() {
    // Degenerate config: zero-length halves and half-time => regulation_play() == 0.
    // The in-game raw tally is real_elapsed + remaining_regulation(=0) - reg(=0), all
    // via guarded/saturating arithmetic: must not panic and must equal real elapsed.
    initialize();
    let config = GameConfig {
        half_play_duration: Duration::from_secs(0),
        half_time_duration: Duration::from_secs(0),
        minimum_break: Duration::from_secs(2),
        game_block: Duration::from_secs(10),
        overtime_allowed: false,
        sudden_death_allowed: false,
        ..Default::default()
    };
    let mut tm = TournamentManager::new(config);
    let start = Instant::now();
    tm.start_clock(start);
    tm.start_play_now(start).unwrap(); // on time => inherited 0
    tm.stop_clock(start).unwrap();
    // real_elapsed 7 + remaining_regulation 0 - reg 0 = 7; inherited 0.
    assert_eq!(
        tm.behind_schedule(start + Duration::from_secs(7)),
        Duration::from_secs(7)
    );
}
```

- [ ] **Step 3: Run the two new tests (expect PASS even pre-guard)**

Run: `cargo test -p refbox far_future_portal_time_is_safe zero_length_regulation_is_safe`
Expected: both PASS. (They are characterization tests; passing against the current code is correct — see the honest note above.)

- [ ] **Step 4: Commit**

```bash
git add refbox/src/tournament_manager/mod.rs
git commit -m "test(refbox): cover far-future portal time and zero-length regulation in behind_schedule"
```

---

### Task 2: Guard `now + d` in `next_game_scheduled_start`

**Files:**
- Modify: `refbox/src/tournament_manager/mod.rs:902`

- [ ] **Step 1: Apply the guard**

Replace exactly this line (currently at line 902):

```rust
                delta.try_into().ok().map(|d: Duration| now + d)
```

with:

```rust
                delta
                    .try_into()
                    .ok()
                    .map(|d: Duration| now.checked_add(d).unwrap_or(now))
```

- [ ] **Step 2: Verify tests still pass**

Run: `cargo test -p refbox`
Expected: PASS, 226 tests (224 existing + 2 new). No behaviour change on any existing test.

- [ ] **Step 3: Commit**

```bash
git add refbox/src/tournament_manager/mod.rs
git commit -m "fix(refbox): guard next_game_scheduled_start against Instant overflow"
```

---

### Task 3: Guard `now + remaining_break` in `behind_schedule`

**Files:**
- Modify: `refbox/src/tournament_manager/mod.rs:2084`

- [ ] **Step 1: Apply the guard**

Replace exactly this line (currently at line 2084):

```rust
            let projected_next_start = now + remaining_break;
```

with:

```rust
            let projected_next_start = now.checked_add(remaining_break).unwrap_or(now);
```

- [ ] **Step 2: Verify tests still pass**

Run: `cargo test -p refbox`
Expected: PASS, 226 tests. Existing between-games behind-schedule tests (e.g. `between_games_follows_break_edit`, `between_games_climbs_when_break_paused`) unchanged.

- [ ] **Step 3: Commit**

```bash
git add refbox/src/tournament_manager/mod.rs
git commit -m "fix(refbox): guard behind_schedule break projection against Instant overflow"
```

---

### Task 4: Full verification

- [ ] **Step 1: Run the full refbox suite**

Run: `cargo test -p refbox`
Expected: PASS, 226 tests, 0 failed.

- [ ] **Step 2: Run clippy as CI does for this bin crate**

Run: `cargo clippy -p refbox -- -D warnings`
Expected: clean, zero warnings. (Note: `-p refbox` without `--all-targets`, mirroring `just lint`.)

- [ ] **Step 3: Confirm the scope boundary held**

Run: `git diff master...HEAD -- refbox/src/tournament_manager/mod.rs | grep -E "^@@" `
Expected: the only *new* production hunks vs the prior review are inside `next_game_scheduled_start` and `behind_schedule`. `calc_time_to_next_game` and all 23 previously-verified clock methods remain untouched.

---

## Deviations

(Record any execution deviations here, per the lean-process rule. Do not create standalone deviation commits.)
