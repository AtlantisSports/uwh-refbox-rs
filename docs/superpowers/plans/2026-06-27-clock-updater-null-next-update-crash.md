# Clock Updater Null Next-Update-Time Crash — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop the refbox from crashing at the end of a game when the loaded timing rule has overtime enabled but a zero pre-overtime break (the portal "FINALS" rule), by tolerating an empty "next update time" in the background clock updater instead of force-unwrapping it.

**Architecture:** The crash is a `next_update_time(now).unwrap()` on a `None` in `refbox/src/app/mod.rs`'s `time_updater` async stream. Extract the wake-time decision into a small pure helper `next_updater_wake(...)` that never panics (falls back to a short re-poll when there is no concrete next instant), unit-test the helper, and wire it into `time_updater`. Add a characterization test in the tournament manager proving the FINALS-style config actually produces the empty value.

**Tech Stack:** Rust 2024, `iced` 0.13 stream, `tokio`; tests are in-file `#[cfg(test)]` modules run via `cargo test -p refbox`.

## Global Constraints

- MSRV Rust 1.85; edition 2024. Do not use newer APIs.
- `just check` must pass: `cargo fmt` clean, `cargo clippy --workspace --all-targets --all-features -- -D warnings` zero warnings, all tests pass, `cargo audit` clean.
- No new `unwrap()`/`expect()` in production code without a justifying comment. (This change *removes* an unwrap; the helper uses `unwrap_or`, which cannot panic.)
- No new dependencies. No changes to `uwh-common`, portal data, or the confirm-pause duration formula.
- `refbox` is a binary crate: iterate with `cargo test -p refbox <name>`; final gate is `just check`.
- Planning docs (this file, the spec) stay local and uncommitted per project convention.
- Branch off the latest `origin/master`. Stage only the source files named below — never `CLAUDE.md` or `docs/`.

---

## Setup: branch (confirm name with the user before running)

Proposed branch: `fix/refbox/clock-updater-null-next-update`

- [ ] **Confirm the branch name with the user**, then create it off the latest master:

```bash
git fetch origin master
git checkout -b fix/refbox/clock-updater-null-next-update origin/master
git rev-parse --abbrev-ref HEAD   # expect: fix/refbox/clock-updater-null-next-update
```

Note: a pre-existing `M CLAUDE.md` and untracked `docs/` will carry over in the working tree — leave them unstaged throughout.

---

### Task 1: Characterization test — FINALS-style config yields an empty next-update-time

Proves the degenerate timing rule really produces the `None` that the fix must tolerate, and guards the confirm-pause duration formula (so a future change that floors it is noticed).

**Files:**
- Test: `refbox/src/tournament_manager/mod.rs` (add a `#[test]` in the existing `#[cfg(test)]` module, e.g. next to `test_pause_score_confirm_*` around line 7600+)

**Interfaces:**
- Consumes (existing, in scope in that test module): `GameConfig`, `TournamentManager::new`, `set_period_and_game_clock_time`, `set_game_start`, `start_game_clock`, `set_scores`, `could_end_game`, `pause_for_confirm`, `next_update_time`, `time_pause_confirmation`, `BlackWhiteBundle`, `GamePeriod`, `Duration`, `Instant`, `initialize`.
- Produces: nothing consumed by later tasks.

- [ ] **Step 1: Write the characterization test**

Add to the `#[cfg(test)] mod tests` in `refbox/src/tournament_manager/mod.rs`:

```rust
#[test]
fn test_finals_template_yields_empty_next_update_time() {
    initialize();
    // A FINALS-style timing rule: overtime enabled, but the pre-overtime
    // break is zero. This is the degenerate config that crashed the refbox.
    let config = GameConfig {
        overtime_allowed: true,
        sudden_death_allowed: true,
        pre_overtime_break: Duration::ZERO,
        pre_sudden_death_duration: Duration::from_secs(60),
        minimum_break: Duration::from_secs(180),
        ..Default::default()
    };
    let mut tm = TournamentManager::new(config);

    let start = Instant::now();
    let game_end = start + Duration::from_secs(30);

    tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(30));
    tm.set_game_start(start);
    tm.start_game_clock(start);
    tm.set_scores(BlackWhiteBundle { black: 1, white: 2 }, start);

    assert_eq!(Ok(true), tm.could_end_game(game_end));
    tm.pause_for_confirm(game_end).unwrap();

    // Confirm pause is zero-length: min(pre_overtime_break, minimum_break)/2 == 0.
    let confirm = tm.time_pause_confirmation.as_ref().unwrap();
    assert_eq!(confirm.duration_of_pause, Duration::ZERO);

    // One tick after the pause begins, the remaining-pause computation
    // underflows, so there is no concrete next-update instant. The app must
    // tolerate this empty value instead of unwrapping it.
    let after = game_end + Duration::from_millis(1);
    assert_eq!(tm.next_update_time(after), None);
}
```

- [ ] **Step 2: Run the test**

Run: `cargo test -p refbox test_finals_template_yields_empty_next_update_time -- --nocapture`
Expected: PASS (this characterizes current behavior — it locks the trigger; it does not turn red).

- [ ] **Step 3: Commit**

```bash
git add refbox/src/tournament_manager/mod.rs
git commit -m "test(refbox): lock FINALS zero-break empty next-update-time trigger"
```

---

### Task 2: Fix — tolerate the empty next-update-time in the clock updater

**Files:**
- Modify: `refbox/src/app/mod.rs` — add `UPDATER_NO_NEXT_TIME_FALLBACK` const + `next_updater_wake` fn just above `fn time_updater()` (~line 5402); add a `#[cfg(test)] mod updater_wake_tests`; replace the unwrap at the crash site (~line 5485-5489).

**Interfaces:**
- Produces: `fn next_updater_wake(clock_running: bool, next_update_time: Option<Instant>, now: Instant) -> Option<Instant>` and `const UPDATER_NO_NEXT_TIME_FALLBACK: Duration`.
- Consumes: `Instant`, `Duration` (already in module scope: see `const REQUEST_TIMEOUT: Duration` near line 70 and `Instant::now()` in `time_updater`).

- [ ] **Step 1: Write the failing helper tests**

Add to `refbox/src/app/mod.rs` (place immediately after where `next_updater_wake` will go, see Step 3):

```rust
#[cfg(test)]
mod updater_wake_tests {
    use super::*;

    #[test]
    fn stopped_clock_has_no_scheduled_wake() {
        let now = Instant::now();
        assert_eq!(next_updater_wake(false, Some(now), now), None);
        assert_eq!(next_updater_wake(false, None, now), None);
    }

    #[test]
    fn running_clock_uses_concrete_next_time() {
        let now = Instant::now();
        let next = now + Duration::from_secs(1);
        assert_eq!(next_updater_wake(true, Some(next), now), Some(next));
    }

    #[test]
    fn running_clock_with_empty_next_time_falls_back_without_panicking() {
        // The FINALS-template crash case: clock running but no concrete next
        // update instant. Must NOT panic; schedules a short re-poll instead.
        let now = Instant::now();
        assert_eq!(
            next_updater_wake(true, None, now),
            Some(now + UPDATER_NO_NEXT_TIME_FALLBACK)
        );
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p refbox updater_wake -- --nocapture`
Expected: FAIL — compile error `cannot find function 'next_updater_wake'` (and `UPDATER_NO_NEXT_TIME_FALLBACK`).

- [ ] **Step 3: Add the const and helper**

In `refbox/src/app/mod.rs`, immediately before `fn time_updater() -> impl Stream<Item = Message> {` (~line 5403):

```rust
/// Fallback re-poll delay for the time updater when the clock is running but
/// the game state has no concrete next-update instant. This happens only in
/// degenerate zero-duration timing rules (e.g. the portal "FINALS" rule, whose
/// pre-overtime break is zero, producing a zero-length score-confirm pause).
/// Re-polling soon lets the state machine advance; it must never panic.
const UPDATER_NO_NEXT_TIME_FALLBACK: Duration = Duration::from_millis(100);

/// Decide when [`time_updater`] should next wake.
///
/// - Clock stopped: `None` — await the next clock-running change.
/// - Clock running with a concrete next-update instant: wake at that instant.
/// - Clock running but no next-update instant (degenerate state): re-poll after
///   [`UPDATER_NO_NEXT_TIME_FALLBACK`] so the state machine can advance.
///
/// Replaces an earlier `next_update_time(now).unwrap()` that crashed the whole
/// app (poisoning the shared game lock) when the value was absent.
fn next_updater_wake(
    clock_running: bool,
    next_update_time: Option<Instant>,
    now: Instant,
) -> Option<Instant> {
    clock_running.then(|| next_update_time.unwrap_or(now + UPDATER_NO_NEXT_TIME_FALLBACK))
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p refbox updater_wake -- --nocapture`
Expected: PASS (all 3 tests).

- [ ] **Step 5: Wire the helper into `time_updater`**

In `refbox/src/app/mod.rs`, replace this block (~line 5485-5489):

```rust
                next_time = if clock_running {
                    Some(tm_.next_update_time(now).unwrap())
                } else {
                    None
                };
```

with:

```rust
                next_time = next_updater_wake(clock_running, tm_.next_update_time(now), now);
```

- [ ] **Step 6: Build the runnable binary and run the full check**

Run: `cargo build -p refbox`
Expected: builds clean (no warnings).

Run: `just check`
Expected: fmt clean, clippy zero warnings, all tests pass (including Task 1 + the 3 helper tests), audit clean.

- [ ] **Step 7: Commit**

```bash
git add refbox/src/app/mod.rs
git commit -m "fix(refbox): tolerate empty next-update-time in time updater"
```

---

## Manual verification (optional, recommended before PR)

The automated tests prove the fix logic. To confirm end-to-end on the real UI:

- [ ] Rebuild: `cargo build -p refbox` (the runnable binary, not the test binary).
- [ ] Launch with logging: `WAYLAND_DISPLAY= RUST_BACKTRACE=full ./target/debug/refbox -v` (capture console to a file).
- [ ] Reproduce: load a FINALS game, play to the end of the second half (clock to 0), and confirm the app does **not** crash. Without the fix this panics at `mod.rs` `next_update_time(now).unwrap()`.

## Self-review notes

- **Spec coverage:** root-cause tolerance (Task 2 helper + wireup), regression proof (Task 1 characterization + Task 2 helper tests), acceptance "no crash / `just check` green" (Task 2 Step 6). ✓
- **No behavior change to normal games:** normal configs never yield an empty next-update-time, so `next_updater_wake` returns the same `Some(t)` it does today. ✓
- **Out of scope (do NOT touch here):** confirm-pause duration formula; `uwh-common`/portal types; the tied-game zero-duration overtime path; portal/schedule-side validation (separate follow-up issue).

---

## Deviations (recorded during execution)

The null-handling fix (Task 1 + Task 2) shipped as planned, but a smoketest then
revealed a **second crash from the same zero-length-pause root**: with the updater
no longer crashing at `next_update_time`, the zero-length pause auto-ends
instantly, so `end_confirm_pause` gets called twice and panics `NotPaused`
(`app/mod.rs:3842`). Null-handling alone was therefore insufficient.

Per the user's direction (avoid an "artificial minimum pause"; fix the root), a
third commit adds the **primary fix**: treat a timing rule that enables overtime
but has zero-length overtime periods as having **no overtime** (a tied finals game
goes to sudden death). Implemented as
`TournamentManager::normalize_degenerate_overtime(&mut config)`
(`overtime_allowed && ot_half_play_duration.is_zero()` ⇒ `overtime_allowed=false`),
called at both `timing.into()` adoption sites (`apply_next_game_start` ~1049 and the
start-play-now path ~1152). The pause is then derived from sudden-death/break timing
(30s), so it is never zero-length; the tied-game zero-overtime cascade is avoided.
Null-handling is retained as defense-in-depth. The tied-game path moved from
"out of scope" to "covered" by this fix.

Tests added: `test_normalize_degenerate_overtime`,
`test_finals_next_game_disables_overtime` (production wiring),
`test_finals_normalized_nonzero_pause_and_tie_to_sudden_death`. `just check` green.

**Smoketest-verified 2026-06-28:** real FINALS game (event 2305-A, game 51) played
to end of second half → "Pause Duration: 30s" → Between Games, **no panic**; the
game-info table shows **Overtime: NO / Sudden Death: YES**. Branch
`fix/refbox/clock-updater-null-next-update`, 3 commits, not yet merged. Upstream
portal/schedule validation remains a separate follow-up proposal.
