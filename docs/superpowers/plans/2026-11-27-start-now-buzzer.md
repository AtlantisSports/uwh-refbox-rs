# START NOW Sounds the Buzzer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Tapping START NOW in any break plays the buzzer the break would have played at 0:00.

**Architecture:** A pure decision function beside `period_end_sounds` says whether leaving a given period
by START NOW calls for the buzzer. The `Message::StartPlayNow` arm reads the engine's period before
`start_play_now`, and on success asks that function and calls `self.sound.trigger_buzzer()`.

**Tech Stack:** Rust 2024, iced 0.13 app (`refbox`).

**Spec:** `docs/superpowers/specs/2026-11-27-start-now-buzzer-design.md`

## Global Constraints

- MSRV 1.85: no let-chains (`if let ... && ...`), no post-1.85 std APIs.
- Clippy `-D warnings`; no `unwrap()`/`expect()` added in production code.
- Only `refbox/src/app/mod.rs` changes (plus this plan and the spec).
- Every cargo run: coordinator GO first, `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4`, "heavy done" after.
- Lean process (sound only; no engine state machine change).

## Review Focus

1. A tap that loses the race with the break's own 0:00 (`start_play_now` returns `Err`) must play nothing
   extra — the handler returns before the buzzer. Pinned by code structure (early `return` on `Err` stays
   ahead of the buzzer call); reviewer checks it.
2. START NOW while a timeout runs, or between games with no next game: the engine refuses → no buzzer.
   Same structural pin as 1.
3. No double buzz after success: the next snapshot is a play period at full time, and `maybe_play_sound`
   buzzes only at `secs_in_period == 0`. Reviewer confirms no change to `maybe_play_sound`.
4. SOUND ENABLED off: silenced inside `sound_controller` at playback — unchanged, reviewer confirms the new
   call goes through `trigger_buzzer()` and not a lower-level path.
5. Play periods (START NOW cannot be tapped there, but the engine is the backstop): the decision function
   returns false for all five, pinned by the test below.

---

### Task 1: START NOW plays the start-of-play buzzer

**Files:**
- Modify: `refbox/src/app/mod.rs` — new fn after `period_end_sounds` (~line 1519); the
  `Message::StartPlayNow` arm (~line 5197); test module `period_end_sound_tests` (~line 12370).

**Interfaces:**
- Produces: `fn start_now_plays_buzzer(left_period: GamePeriod, auto_sound_start_play: bool) -> bool`
  (private, module-level). True exactly when `period_end_sounds(left_period, false).0 &&
  auto_sound_start_play`. (`sd_counts_down` only affects `SuddenDeath`'s stop flag, never `.0`, so `false`
  is safe; say so in a one-line comment.)

- [ ] **Step 1: Write the failing test** in `period_end_sound_tests` (add `start_now_plays_buzzer` to its
  `use super::...`):

```rust
#[test]
fn start_now_buzzes_like_the_break_running_out() {
    let breaks = [
        GamePeriod::BetweenGames,
        GamePeriod::HalfTime,
        GamePeriod::PreOvertime,
        GamePeriod::OvertimeHalfTime,
        GamePeriod::PreSuddenDeath,
    ];
    let plays = [
        GamePeriod::FirstHalf,
        GamePeriod::SecondHalf,
        GamePeriod::OvertimeFirstHalf,
        GamePeriod::OvertimeSecondHalf,
        GamePeriod::SuddenDeath,
    ];
    for p in breaks {
        assert!(start_now_plays_buzzer(p, true), "{p:?} with AUTO SOUND START PLAY on");
        assert!(!start_now_plays_buzzer(p, false), "{p:?} with AUTO SOUND START PLAY off");
    }
    for p in plays {
        assert!(!start_now_plays_buzzer(p, true), "{p:?}");
        assert!(!start_now_plays_buzzer(p, false), "{p:?}");
    }
}
```

- [ ] **Step 2: Run it, expect a compile failure** (`start_now_plays_buzzer` not found).
  Run (after GO): `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 cargo test -p refbox --lib start_now_buzzes`

- [ ] **Step 3: Add `start_now_plays_buzzer`** with a doc comment: START NOW skips the 0:00 tick where
  `maybe_play_sound` would have buzzed, so it plays what that tick would have (PO ruling 2026-11-27).

- [ ] **Step 4: Wire the handler.** In `Message::StartPlayNow`: read `let left_period =
  tm.current_period();` before `tm.start_play_now(now)`. Keep the `Err` arm's early `return` unchanged.
  After `std::mem::drop(tm);` and before `self.apply_snapshot(snapshot)`:
  if `start_now_plays_buzzer(left_period, self.config.sound.auto_sound_start_play)` then
  `info!("START NOW: triggering buzzer");` and `self.sound.trigger_buzzer();`.
  Comment one line: buzz only after the engine accepted the tap, so a tap that loses the race with 0:00
  cannot double-buzz.

- [ ] **Step 5: Run the test, expect PASS** (same command as Step 2).

- [ ] **Step 6: fmt + lint the crate** (after GO): `just fmt` then
  `CARGO_BUILD_JOBS=4 cargo clippy -p refbox --all-targets -- -D warnings`. Expected: clean.

- [ ] **Step 7: Leave changes uncommitted** (the main session reviews and commits
  `feat(refbox): sound the buzzer when START NOW is tapped`).

## Deviations

- Steps 2/5: `refbox` is a binary crate, so the test command is
  `cargo test -p refbox --bin refbox start_now_buzzes` (not `--lib`).
- Step 2's red was confirmed by mutation instead of a compile failure: with the function reduced to
  `auto_sound_start_play` alone, the test failed on `FirstHalf`; restored, it passes.
