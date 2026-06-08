# Design: Behind-schedule in-game figure becomes forward-projecting (edit-aware)

Date: 2026-06-08
Crates: `refbox` only (`src/tournament_manager/mod.rs`).
Status: approved (brainstorm) — pending written-spec review.
Refines: `2026-06-08-behind-schedule-calc-rework-design.md`. That rework introduced a directly-measured
`regulation_played` accumulator for the in-game branch. This refinement **replaces** that accumulator
with a forward projection, so the figure reacts to in-game clock edits the way the between-games
branch already reacts to break edits. The between-games branch, the display, the setting, `inherited`,
and the clawback buffer are unchanged.

## Problem

The in-game figure looks **backward** (real time already taken minus regulation already played), while
the between-games figure looks **forward** (projected next start). So a clock edit during a game —
which changes the *future* remaining play — does not move the figure when applied; its effect only
appears as the added/removed time is actually played. Editing the between-games break, by contrast,
moves the figure immediately. The operator wants the in-game edit to behave like the break edit:
**add play time → figure rises immediately; remove play time → figure drops immediately.**

The production time-edit path (`Message::EditTime` → `ChangeTime` → `TimeEditComplete`) only ever edits
a *clock value* (the game clock via `set_game_clock_time`, or a timeout clock via
`set_timeout_clock_time`); it never changes the period (`set_period_and_game_clock_time` is
`#[cfg(test)]`). So the only scenarios in scope are clock-value edits.

## Approach: project the remaining regulation play

Replace the in-game overrun term with a forward projection of when the game will end:

```
overrun(now) = saturating( (real_elapsed + remaining_regulation(now)) − regulation_play )
behind      = inherited + saturating( overrun(now) − slot_buffer )      // unchanged structure
```

- `real_elapsed = now − game_start_time`.
- `remaining_regulation(now)` = scheduled regulation play still to come:
  - `FirstHalf`  → `clock_time(now)` + (`half_time_duration` + `half_play_duration`) unless `single_half`
  - `HalfTime`   → `clock_time(now)` + `half_play_duration`
  - `SecondHalf` → `clock_time(now)`
  - any extra-time or break period (`PreOvertime`, `OvertimeFirstHalf`, `OvertimeHalfTime`,
    `OvertimeSecondHalf`, `PreSuddenDeath`, `SuddenDeath`) → `Duration::ZERO` (regulation is done)
  - `clock_time(now)` is the current period's remaining (live clock), `unwrap_or(ZERO)` if past zero.
- `inherited` and `slot_buffer` are exactly as in the prior spec (inherited lateness is still shown in
  full for a late-started game; the slot's slack still absorbs this game's overrun only).

`real_elapsed + remaining_regulation(now)` is the **projected total wall-clock duration of the game**;
subtracting `regulation_play` gives the projected overrun beyond the scheduled game length.

### Why this is correct and edit-aware

- **Regulation play, clock running:** as `now` advances by `dt`, `clock_time` drops by `dt`, so
  `real_elapsed + remaining_regulation` is constant → overrun frozen → figure frozen. (Fixes the
  original "climbs while running" bug by construction, with no period-length subtraction.)
- **Clock stopped (pause, team/ref timeout, normal penalty shot, time-edit screen open):** `clock_time`
  is frozen, `real_elapsed` grows → overrun climbs.
- **Rugby penalty shot:** the main game clock keeps running, so `clock_time` drops with `now` → overrun
  **frozen**. (Decided behaviour: a rugby penalty shot is genuine regulation progress.) This differs
  from the accumulator, which froze for *all* timeouts; the projection needs no timeout check.
- **Extra time (overtime/sudden-death and their breaks):** `remaining_regulation = 0`, `real_elapsed`
  grows → overrun climbs once past the clawback slack.
- **Clock edit of `+Δ`/`−Δ` during regulation:** `clock_time` shifts by `Δ` → `remaining_regulation`
  shifts by `Δ` → overrun (and the figure) shifts by `Δ` **immediately**.

### Equivalence with the current model (no behaviour regression off the edit path)

In any state reached without a clock edit, `remaining_regulation(now) = regulation_play −
regulation_played(now)`, so the projected overrun equals the accumulator's `real_elapsed −
regulation_played`. The two models are algebraically identical except when a clock edit (or a
clock-reading-vs-period-length mismatch) makes the live remaining differ from "budget minus
wall-time-played" — which is exactly the case we want to change.

## What is removed

The projection reads only `game_start_time`, the live `clock_state`/`clock_time`, `current_period`, and
config durations — all pre-existing. The accumulator becomes unused and is deleted:

- fields `regulation_played`, `regulation_mark`
- methods `in_regulation_progress`, `settle_regulation_played`, `regulation_played_now`
- the 16 `settle_regulation_played(now)` calls
- the `regulation_played`/`regulation_mark` reset in `start_game`

(The compiler enforces completeness: a missed `settle_regulation_played` call won't compile once the
method is gone.)

## What is unchanged

- Between-games branch (already a projection), display, the **Show Behind Schedule Time** setting,
  `inherited`, `slot_buffer`/clawback, `current_scheduled_start`, no `uwh-common`/wire-format change.

## Testing

- **New (the gap):** in-game clock-edit tests — editing the running game clock up by Δ raises the
  figure by Δ immediately; editing down by Δ lowers it by Δ immediately (sampled right after the edit,
  with the clock running). Drive it through the same `set_game_clock_time` path the app uses (stop →
  set → resume), matching `TimeEditComplete`.
- **Adjusted:** `test_behind_schedule_frozen_while_running_clock_exceeds_period_len` — the
  frozen-while-running assertion (`a == b`) stays; the `a == ZERO` assertion changes, because the
  projection correctly reports a nonzero, *frozen* figure for a clock set longer than its period (a
  40s clock in a 10s half projects ~30s of extra play). Re-derive the literal; keep `a == b`.
- **Unchanged (must still pass):** all other `test_behind_schedule_*` (stoppage growth, ref-timeout
  accrual, inherited lateness, single-period, half-time frozen, overtime climbs, continuity,
  recovery, between-games) — identical results off the edit path.

## Acceptance criteria (operator-observable)

1. Editing the running game clock **up** raises the delay figure immediately by the edited amount;
   editing it **down** lowers it immediately. (The break edit already behaves this way.)
2. The figure still freezes during regulation play (including half-time) and during a rugby penalty
   shot; still climbs while paused, during normal timeouts/penalty shots, and during overtime past the
   slack.
3. `just check` passes; new in-game-edit tests pass; all prior behind-schedule tests pass (with the one
   re-derived literal noted above).

## Blast radius & process

High — game-clock read-model. Heavy process: test-first, per-task verification. Net effect is *less*
code (one projection helper replaces the accumulator + 16 settle sites). Confined to
`refbox/src/tournament_manager/mod.rs`.
