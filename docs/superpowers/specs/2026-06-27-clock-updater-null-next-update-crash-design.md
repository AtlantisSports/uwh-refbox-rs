# Design: Clock updater must tolerate an empty "next update time" (FINALS-template end-of-game crash)

Date: 2026-06-27
Status: Approved (design); spec pending user review
Crate: `refbox` (game-clock updater); test in `refbox/src/tournament_manager` golden-trace harness

## Problem

The refbox crashes at the end of a game when the loaded timing rule has overtime
enabled but zero-length overtime/break durations. Observed in the field with the
portal's **"FINALS"** timing rule:

```
overtime_allowed: true, pre_overtime_break: 0ns,
ot_half_play_duration: 0ns, ot_half_time_duration: 0ns,
sudden_death_allowed: true, pre_sudden_death_duration: 60s, minimum_break: 180s
```

Reproduced twice with identical cause: live on 2026-06-27, and in the saved log on
2026-06-25.

### Root cause

When the second half ends, the game enters a short score-confirmation pause. The
pause length is derived from the timing rule:
`dur_pause = min(pre_overtime_break, minimum_break) / 2`. With
`pre_overtime_break = 0`, this is **zero**.

The background clock updater ([`refbox/src/app/mod.rs`](../../../refbox/src/app/mod.rs) `time_updater`)
then asks the game state for its next update instant:

```rust
next_time = if clock_running {
    Some(tm_.next_update_time(now).unwrap())   // mod.rs:5486  <-- panic
} else {
    None
};
```

`TournamentManager::next_update_time` ([`refbox/src/tournament_manager/mod.rs`](../../../refbox/src/tournament_manager/mod.rs))
computes the remaining confirm-pause time as
`duration_of_pause.checked_sub(now - pause_began)`. With a zero-length pause whose
`pause_began` is at/just before `now`, this subtraction underflows and returns
`None`. The `.unwrap()` panics on the tokio worker **while holding the shared game
lock**, which poisons the lock; the next UI redraw then also panics at
`RefBoxApp::view` (`mod.rs:4898`, `self.tm.lock().unwrap()`). Net effect: the whole
app dies. The second panic is a knock-on of the first.

The crash is in the background updater and is independent of whether score
confirmation is enabled — it happens regardless.

## Fix (this branch — `refbox` only)

Tolerate the empty value at the single consumption point instead of force-unwrapping
it. When `next_update_time` returns nothing (only in these degenerate zero-duration
states), the updater keeps the app alive and re-checks at its normal cadence rather
than panicking.

- Remove the `.unwrap()` at the `mod.rs:5486` crash site and handle the `None`
  branch so the updater does not crash and does not freeze (the game continues to a
  sane end state).
- Exact handling of the `None` branch (propagate `None` so the loop awaits the next
  clock event, vs. schedule a short re-poll so it self-heals) will be settled by the
  regression test during implementation — whichever leaves the game ticking and
  ending cleanly with no panic and no busy-loop.

Normal games never produce an empty next-update-time, so their behavior is unchanged.

## Testing

Add a regression test using the existing golden-trace harness
(`refbox/src/tournament_manager/golden/`):

1. Configure a FINALS-style timing rule (overtime enabled, zero pre-overtime and
   overtime durations).
2. Drive a game to the end of the second half (the exact trigger).
3. Assert the clock logic does **not** panic and the game lands in a sane end state.

The test panics on today's code and passes after the fix. All existing
confirm-pause tests (asserting 4s/6s/10s durations) must stay green, and full
`just check` must pass.

## Acceptance criteria

- A FINALS-template game played to the end of the second half no longer crashes the
  refbox.
- New regression test fails before the fix and passes after.
- `just check` is green (fmt, clippy `-D warnings`, tests, audit).
- No change to confirm-pause durations or any normal-game behavior.

## Out of scope / tracked separately

1. **Portal/schedule-side validation (separate proposal, different repo).** Propose
   that the schedule processor / portal reject a schedule whose timing rule has
   overtime enabled but zero overtime/break durations, so the degenerate config never
   reaches the refbox. To be drafted as a GitHub issue after this fix.
2. **Tied-game overtime path for zero-duration templates.** A tied finals game using
   this template would proceed into zero-length overtime periods, which may surface
   further issues downstream. Flagged for separate investigation; not addressed here.
3. No changes to `uwh-common` timing types, portal data, or the confirm-pause
   duration formula.
