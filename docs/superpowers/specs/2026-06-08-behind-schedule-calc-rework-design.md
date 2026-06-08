# Design: Rework the behind-schedule calculation (direct measurement, overtime- and edit-aware)

Date: 2026-06-08
Crates: `refbox` only (`src/tournament_manager/mod.rs` — the timing read-model + tests).
Status: approved (brainstorm) — pending written-spec review.
Reworks: the calculation in `docs/superpowers/specs/2026-06-06-behind-schedule-indicator-design.md`
(sections 2–4). The display, the **Show Behind Schedule Time** setting, the `-M:SS` format, and the
scope boundaries of that spec are unchanged and are not restated here.

## Problem

The behind-schedule figure misbehaves in the running app. Two bugs were confirmed live:

1. **It climbs while the game clock is running.** Observed on screen: the play clock counted
   `0:41 → 0:38` (down 3s) while the delay went `-49:28 → -49:31` (up 3s) — climbing in lockstep
   with real time when it should have been frozen.
2. **Between-games time doesn't feed the figure.** Pausing or overrunning the break does not move
   the delay; it sits frozen at the value captured when the last game ended.

Both stem from one design weakness: the figure is **reconstructed** indirectly rather than
**measured** directly.

- Bug 1's root cause: `accumulated_overrun` computes "scheduled game time consumed" as
  `period_length − clock_reading`. When the clock reading exceeds the configured period length
  (a time-edit, or a config change between games — the live case had Half Length `0:10` but a clock
  reading of `0:41`), that subtraction saturates to zero, so the code believes *no* game time is
  being consumed even though the clock is visibly running, and the figure climbs with real time.
- Bug 2's root cause: the between-games branch returns a frozen snapshot
  (`behind_at_game_end`, added in commit `d0833c54`) instead of reacting to the live break clock.

## The unified rule

> The figure **freezes** whenever the run is moving through **scheduled** time at the scheduled
> pace, and **climbs** whenever it is not. Time edits to any clock flow straight through.

"Moving through scheduled time" means exactly:

- A **regulation** period's clock is counting down — **first half, half-time, or second half** (or
  the single half). These are the periods the scheduled game length budgets for.
- The **between-games break** is counting down normally. The schedule budgets a gap between games,
  so a normally-running break is scheduled progress.

Everything else makes the figure climb:

- The clock is **stopped** for any reason — pause, team timeout, ref timeout, penalty shot, time
  edit, score-confirmation pause.
- The game is in **any extra time beyond regulation** — pre-overtime break, overtime halves,
  overtime half-time, pre-sudden-death break, sudden death. The schedule never budgeted for these,
  so the game is running long even though the clock is running. (This is gated by clawback — see
  below — so a little overtime within the slack shows nothing, then the figure begins to climb once
  the slack is used up.)
- A break is **paused**, has **counted to zero** and is sitting unstarted, or is **edited**.

**Clawback (unchanged from the 2026-06-06 spec):** each game's slot has slack equal to
`game_block − regulation_play − minimum_break` (= `game_block_buffer`; portal mode derives the
equivalent from the printed gap). Overrun and stoppage eat this slack before the figure grows, and a
break can compress down to (never below) the minimum break to claw a delay back down. Unused team
timeouts need no special handling — not taking a timeout simply means less stopped time.

## Behaviour table (operator-observable)

| What's happening | Figure |
|---|---|
| Regulation clock counting down — either half **or half-time** | frozen |
| Clock paused | climbs |
| Team timeout / ref timeout / penalty shot | climbs |
| Time edit / score-confirmation pause | climbs |
| Any extra time — pre-OT break, overtime, OT half-time, pre-sudden-death, sudden death | climbs (after clawback slack is used up) |
| Between games, break counting down normally | frozen |
| Between games, break paused, or counted to zero and sitting unstarted | climbs |
| Between games, break time edited | adjusts by the edit (extend the break → figure up; trim it → figure down) |

## Calculation

`behind_schedule(now) -> Duration` keeps its two branches, but both become direct measurements.

### During a live game

```
behind = inherited + saturating(overrun(now) − clawback)
```

- `inherited = saturating(game_start_time − current_scheduled_start)` — lateness carried in at this
  game's start (unchanged).
- `clawback = (sched_next − current_scheduled_start) − regulation_play − minimum_break`, saturating
  (manual mode = `game_block_buffer`) — unchanged.
- `overrun(now) = saturating(real_elapsed_since_game_start − regulation_played(now))` — **replaces**
  the old `accumulated_overrun`.

`regulation_played(now)` is the new directly-measured quantity:

- The total **wall-clock time the game has spent counting down a regulation period** (first half,
  half-time, second half) with **no timeout active** — i.e. while `current_period` is a regulation
  period **and** the main game clock is running.
- **Capped at `regulation_play`** (it can never report more scheduled play than the game actually
  budgets).
- **Reset to zero** when a game starts.

Because it counts real wall-time spent in scheduled play — not `period_length − clock_reading` — it
is immune to the clock-reading-exceeds-period-length case (fixes Bug 1), to mid-game config changes,
and to count-up periods. Its consequences:

- During regulation play (clock running): `real_elapsed` and `regulation_played` advance together →
  `overrun` frozen → figure frozen.
- While stopped: `real_elapsed` advances, `regulation_played` does not → `overrun` climbs.
- During overtime/sudden-death: `regulation_played` is capped at `regulation_play`, `real_elapsed`
  keeps advancing → `overrun` climbs (fixes the overtime gap).

### Between games

```
behind = saturating(projected_next_start − sched_next)
projected_next_start = now + remaining_break_clock
```

- `remaining_break_clock` is read from the live break countdown (the existing `clock_state` /
  `clock_time(now)` machinery), so:
  - counting down normally → `projected_next_start` holds steady → frozen;
  - paused → `projected_next_start` slides later → climbs;
  - counted to zero and sitting → `remaining = 0`, `projected_next_start = now` → climbs;
  - edited → `projected_next_start` moves by exactly the edit → figure follows (fixes Bug 2).
- `sched_next` is the next game's scheduled start (the same source the countdown already uses).

### Continuity

The two branches agree at the instant a game ends. Substituting the game-end values
(`remaining_break_clock = calc_time_to_next_game = max(sched_next − game_end, minimum_break)`,
`now = game_end`) into the between-games formula yields `saturating(game_end + minimum_break −
sched_next)`, which equals the in-game `inherited + overrun − clawback` evaluated at game end. No
jump on the transition. (This must be covered by a test.)

## What is removed

- `accumulated_overrun(now)` — its only non-test caller is `behind_schedule`; replaced by
  `regulation_played` + `overrun`. Its tests are replaced.
- The `behind_at_game_end` field and the `end_game` assignment / `reset` clear that maintain it —
  the between-games branch no longer reads a snapshot. (Removes the `d0833c54` freeze.)

## What is unchanged

- The `inherited` and `clawback` definitions, `current_scheduled_start`, `next_game_scheduled_start`,
  portal-vs-manual scheduled-start derivation.
- The display (`-M:SS` red figure, placement), the **Show Behind Schedule Time** setting, and the
  per-render computation in the view layer (`refbox/src/app/mod.rs`).
- No `uwh-common`, wire-format, `GameSnapshot`, scheduling-math, or between-games-countdown change.

## Implementation sketch

- Add a small amount of recorded state to the tournament manager to accumulate `regulation_played`
  directly. The robust pattern: a `Duration` accumulator plus a "last settled at" `Instant`, settled
  whenever the game-clock running/stopped state or the period changes, and on read. The
  implementation plan enumerates every clock-state transition site that must settle it (clock
  start/stop, timeout start/end, period change, game start/end) — this is the delicate part and is
  where the per-task verification effort goes.
- Point `overrun` at `regulation_played`; rewrite the between-games branch to the projected formula;
  delete `accumulated_overrun` and `behind_at_game_end`.

## Testing

Tournament-manager unit tests (this is state-machine code → heavy process, test-first):

1. **Reproduce-the-bugs first** (must fail before the change, pass after):
   - Clock running in first half with a clock reading **greater than** the configured half length →
     `behind_schedule` does **not** change across time. (Bug 1.)
   - Between games, advancing `now` while the break is paused / past zero → `behind_schedule`
     **grows**; editing the break value changes it. (Bug 2.)
2. **Model coverage:** frozen during regulation play and half-time; climbs while stopped; climbs
   during each extra-time period (overtime, sudden death) once past clawback; clawback recovery at a
   compressible break; zero before the first game and when on-time/ahead; continuity across the
   game-end boundary; robustness to a mid-game half-length change and to a time-edit-up.
3. `just check` green; refbox bin builds; downstream crates unaffected (none touched).

## Acceptance criteria (operator-observable)

1. With the clock **running** in regulation (including half-time), the figure does **not** climb,
   regardless of the clock reading.
2. A **paused** clock, any **timeout**, a **penalty shot**, and a **time edit** each make the figure
   climb second-by-second while active.
3. A game going into **overtime/sudden-death** shows the figure begin to climb once the slot's
   clawback slack is used up.
4. **Between games:** a normally-running break keeps the figure frozen; pausing the break or sitting
   past a finished break makes it climb; **editing** the break time moves the figure by that amount.
5. `just check` passes; the two reproduce-the-bug tests fail before and pass after; downstream crates
   build.

## Blast radius & process

High — this is the game-clock state machine. Per `.claude/rules/plan-execution.md`, **heavy
process**: test-first, per-task verification with real unit tests, careful enumeration of every
clock-state transition that must update `regulation_played`. Confined to
`refbox/src/tournament_manager/mod.rs`; no wire-format or `uwh-common` change.
