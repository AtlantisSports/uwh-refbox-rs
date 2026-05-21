# Beep-Test Warmup Countdown — Design

**Date:** 2026-05-21
**Status:** Approved through brainstorming; awaiting implementation plan
**Owner:** e-straily (with Claude)
**Branch:** `feat/refbox/beep-test-redesign` (continuing after Chunk 4 at `046af44`)
**Chunk:** 5 of 6 follow-on improvements to the beep-test mode
**Process gate:** Heavy (per `.claude/rules/plan-execution.md`) — touches the cadence engine state machine. Per-task verification, unit tests required.

---

## Goal

The BeepTest main page's fresh state should show a 10-second warmup
countdown (TIME 0:10, LEVEL 0, LAP 0), and pressing START should tick the
warmup down to 0:00 before transitioning to Level 1 Lap 1. Today the
engine internally treats Level(0) as the warmup but initializes its
display clock to the first user level's duration (e.g., 36 s), making the
fresh state look wrong.

---

## Motivation

Operator walkthrough of Chunk 4 surfaced that, on a fresh launch, the
TIME tile reads `0:36` (the first user level's lap duration) instead of
`0:10` (the warmup duration). The cadence engine's run-time behavior is
already correct — pressing START in this state calls
`start_beep_test_now`, which correctly resets the clock to
`Level(0).duration() == config.pre == 10s` before counting down. The bug
is purely in the *displayed* fresh-state value, which comes from
`clock_state.clock_time` as set by `TournamentManager::new()`.

While investigating, an adjacent pre-existing off-by-one was found:
`time_in_next_lap` is initialized to `config.levels.get(1)` (second user
level) instead of `config.levels.first()` (first user level, which is
what comes *after* the warmup). It doesn't currently manifest visibly
because `update()` recomputes it via `next_test_period_dur` before any
transition reads it, but it's wrong-by-construction and worth fixing while
the engine is being touched.

---

## Scope

### Files touched

- `refbox/src/beep_test/cadence.rs` — `TournamentManager::new()` and
  `TournamentManager::reset_beep_test_now()`. Unit tests added (heavy
  process).

### Not touched

- The view layer (`refbox/src/app/view_builders/beep_test.rs`). The TIME
  tile already reads `snapshot.secs_in_period`; once the engine emits 10
  on the fresh state, the tile shows 0:10 automatically.
- The LED panel display (Chunk 6).
- The sound controller (no timing changes).
- Translations.
- Any other engine method.

---

## Design

### The minimal-scope fix (Approach A)

Two spots in `cadence.rs` both pull from `config.levels` index lookups for
the warmup-state setup. Replace those with the `BeepTestPeriod::Level(0)`
helpers that already exist:

- `BeepTestPeriod::Level(0).duration(&config)` returns `Some(config.pre)`
  — the warmup duration.
- `BeepTestPeriod::Level(0).next_test_period_dur(&config)` returns
  `Some(config.levels[0].duration)` — the first user level's duration
  (the period immediately following the warmup).

### `TournamentManager::new()` change

Today:

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
    ...
}
```

After:

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
    ...
}
```

### `TournamentManager::reset_beep_test_now()` change

Today the `initial_clock` line is already correct
(`self.current_period.duration(&self.config)`). Only `time_in_next_lap`
needs to change from:

```rust
self.time_in_next_lap = self
    .config
    .levels
    .get(1)
    .map(|l| l.duration)
    .unwrap_or_default();
```

to:

```rust
self.time_in_next_lap = self
    .current_period
    .next_test_period_dur(&self.config)
    .unwrap_or_default();
```

(Since `self.current_period` was just set to `BeepTestPeriod::Level(0)` a
few lines above, this is equivalent to the `warmup.next_test_period_dur`
expression in `new()`.)

### Unit tests

Per heavy-process rule for state-machine changes, add tests in the
existing `#[cfg(test)] mod tests` block of `cadence.rs`. Three new tests:

1. **`new_initializes_clock_to_warmup_duration`** — Build a manager with
   a config where `pre = 10s` and `levels[0].duration = 36s`. Assert that
   `clock_state.clock_time(Instant::now())` returns `Some(Duration::from_secs(10))`.
2. **`reset_returns_to_warmup_duration`** — Build a manager, advance it
   partway through the warmup, then call `reset_beep_test_now(now)`.
   Assert the same invariant.
3. **`new_initializes_time_in_next_lap_to_first_level`** — Build a
   manager with `levels[0].duration = 36s` and `levels[1].duration = 30s`.
   Assert `time_in_next_lap == Duration::from_secs(36)` (the duration of
   the period that comes after the warmup, which is Level(1) = levels[0]).

Tests should follow the existing test helper pattern in the file. If
constructing a config requires more wiring, the third test may be merged
into the first by asserting both fields at once.

---

## Acceptance criteria

Walking through the running refbox in BeepTest mode:

1. **Fresh launch.** TIME tile reads `0:10` (or whatever `config.pre`
   is), LEVEL `0`, LAP `0`.
2. **Press START.** TIME counts down from `0:10` to `0:00` over 10
   seconds. Whistle at `0:05` and buzzer at `0:00` per existing sound
   logic.
3. **At 0:00.** Engine transitions to Level 1 Lap 1: LEVEL becomes `1`,
   LAP becomes `1`, TIME becomes the first user level's duration.
4. **Press PAUSE during the warmup countdown.** Clock holds at the
   current warmup remaining time. Tile shows PAUSE → RESUME per Chunk 2.
5. **Press RESUME during the warmup.** Clock continues counting down from
   the held value.
6. **Press RESET at any time.** Returns to fresh state (TIME `0:10`,
   LEVEL `0`, LAP `0`).

Heavy-process verification: `just check` must pass with the new unit
tests included; the walkthrough above is the second verification gate.

---

## Out of scope (intentionally deferred)

- Any change to the LED panel display (Chunk 6).
- Any change to sound timing.
- Any change to how `secs_in_period` is computed inside `update()` (the
  fix is purely in initial values).
- Any change to `BeepTestPeriod` enum or its helpers.
- The audio-delay investigation (deferred separately).
