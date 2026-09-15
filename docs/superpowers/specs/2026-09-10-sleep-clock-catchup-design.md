# Design — refbox catches up its clocks after the machine sleeps

**Status:** approved in conversation 2026-09-10. Not built.

**Precondition — this design is contingent.** The root cause below is a hypothesis with a
confirmed mechanism but no confirmed instance. Before any of this is built, the diagnostic in
"Confirming the cause" must come back positive. If it does not, this document is void and the
investigation reopens.

---

## The problem

A Mac running refbox showed the countdown to game 1 as ~146 minutes when the game was ~20
minutes away. A Windows machine on the same Portal schedule showed the correct time.

### It is not a timezone fault

Portal game times arrive as an absolute moment with the offset attached (`startsOn`) and are
parsed identically on every platform — `uwh-common/src/uwhportal/schedule.rs:230`. The countdown
is computed as scheduled-start minus current-time-in-UTC —
`refbox/src/tournament_manager/mod.rs:1102-1106`. Neither side of that subtraction consults the
machine's timezone. refbox never asks for a local timezone anywhere, and the only `cfg(target_os)`
in the whole crate is the Linux sound driver.

### What it actually is

The between-games countdown is converted from wall-clock time into the machine's monotonic
stopwatch **once**, and then free-runs:

```rust
self.clock_state = ClockState::CountingDown {
    start_time: now,                 // an Instant — a stopwatch reading
    time_remaining_at_start,
};
```
— `refbox/src/tournament_manager/mod.rs:1173-1176`

On Apple targets, Rust's `Instant` is `CLOCK_UPTIME_RAW`. From Rust's own std source:

> `CLOCK_UPTIME_RAW` clock that increments monotonically, in the same manner as
> `CLOCK_MONOTONIC_RAW`, but **that does not increment while the system is asleep**.

Every minute the lid is shut is a minute the countdown does not lose. 146 − 20 = **126 minutes**
of sleep. Linux uses `CLOCK_MONOTONIC`, which behaves the same way under suspend; Windows uses
`QueryPerformanceCounter`. The bug is therefore **not macOS-specific in principle** — it is a
monotonic-clock-versus-wall-clock divergence that a sleeping Mac makes easy to hit.

### Why it does not heal itself

`apply_next_game_start` — the only thing that re-derives the countdown from the wall clock — is
reached from operator actions only: the Settings APPLY path (`refbox/src/app/mod.rs:1750`), its
confirmation variant (`:1898`), and a startup link restore (`:5121`).

**REFRESH does not fix it.** `RequestPortalRefresh` fetches the schedule and stores the fresh
start time, but the re-anchor at `refbox/src/app/mod.rs:5121` sits behind `if restore_num
.is_some()`, and `restore_num` comes from `pending_restore_game`, which is set only at startup
(`:2594`) and consumed on the first schedule received. On a manual REFRESH it is always empty.
The result is a correct stored start time behind a stale displayed countdown — which reads, to an
operator, as though pressing the button did something.

Ending a game *does* re-derive from the wall clock (`:1231-1233`), so the error clears itself at
the end of each game and then re-accumulates across the next sleep.

### Confirming the cause

On the affected Mac, before anything else: Settings → APPLY.

- Countdown snaps to the true value → confirmed, build this.
- Countdown unchanged → not this bug. Check the machine's clock and confirm both machines are on
  the same event **and the same court**, then read
  `~/Library/Application Support/uwh-refbox-logs/refbox-log.txt` for the
  `Current time is:` / `Start time is:` / `Calculated time to next game:` lines, which are
  emitted at default verbosity (`refbox/src/tournament_manager/mod.rs:1103-1107`).

---

## The principle

> "One of the main purposes of this app is to keep time accurately." — Eric, 2026-09-10

On waking, refbox shows where the schedule truly is. It does not show where it would have been
had time stopped.

---

## Rulings (Eric, 2026-09-10)

| Question | Ruling |
|---|---|
| Correct the countdown on waking? | Yes — it should simply read correctly, no operator action |
| Correct a game clock too, not just the break? | Yes. "If it wakes up and the proper time is in the middle of a game then that's where it should be" |
| Stop at the end of the current game? | **No** — overruled Claude's proposal. Go to the true position wherever it falls, across multiple games if need be |
| Upload 0-0 for games slept through? | No |
| Upload the score held for the game that was in progress? | No — treat it as skipped too. "That way it is easy to know which games were impacted" |
| Tell the operator on screen what happened? | No. "This will be obvious and I don't want to introduce new UI" |

### Why "0-0 for skipped games" was rejected

Eric raised it and supplied the objection himself. A 0-0 only ends a game where draws are
permitted — `end_second_half`, `refbox/src/tournament_manager/mod.rs:1656-1676`. In a knockout
game a 0-0 goes to overtime, then sudden death, and sudden death counts **up** with no finish
condition. A fast-forward through a skipped knockout game does not record a questionable result;
it **wedges there**, clock counting up, and the catch-up cannot proceed. The rule would fail
precisely on finals day.

Secondary, and Eric's to weigh: a 0-0 on the Portal is not a blank. It is a claim the game was
played and drawn, and it moves points, standings and goal difference.

### Why fast-forwarding was rejected as the mechanism

Beyond the wedge above, walking the engine through games triggers the result upload for each one
— `handle_game_end` → `enqueue_game_end`, `refbox/src/app/mod.rs:1451`, fired off the period
transition at `:738`. And after the first walked game the engine has no schedule info left
(`start_game` consumes `next_game.take()`), so `next_game_number()` falls back to **incrementing
the number** (`:1479-1483`). On a court-filtered schedule consecutive numbers are not the next
games on that court, so the fabricated results would land on other games' numbers.

Catching the clock up is not the same as playing the games. Once those are separated, the games
slept through are simply not played and nothing is uploaded.

---

## Behaviour

On each tick refbox compares its stopwatch against the wall clock. Where the wall clock has moved
further, the difference is time the stopwatch lost, and refbox re-places itself.

| Where the true time falls | Result on waking |
|---|---|
| Before the first game | Countdown reads the true time to kickoff |
| Inside the game that was running | Clock at the true position in that half |
| Inside a later game | That game loaded and running at its true position, with its own number and timing rule |
| In a break between games | Break countdown reading its true remaining time |
| Past the last game on the schedule | Sits after the final game; nothing started |
| Clock stopped, or paused for score confirmation | Nothing — see below |

**Games passed over are not played.** No result, no stats, nothing enqueued for the Portal.
"Passed over" means the catch-up landed **past that game's end**. A game the catch-up lands
*inside* has not been passed over: it continues, and the score already recorded for it stands.
A game that was in progress and is passed over is treated like any other — its recorded score is
discarded rather than published.

**A stopped clock is left alone, deliberately.** A stopped clock stores a plain number rather
than a stopwatch anchor, so it cannot drift — but the schedule around it has still moved on, and
re-placing would be defensible. It is excluded anyway: a stopped clock is an operator holding the
game on purpose, and overriding that hold because a screen went to sleep would take the game out
of their hands. The same applies to the score-confirmation pause.

**No schedule loaded (manual mode).** There is no true position to jump to. refbox takes the lost
time off the running clock and stops there; it never starts a game on its own.

**No new UI.** The correction is written to the log and nothing else.

---

## Mechanism

### 1. Detection

Each tick, note both clocks and compare movement since the previous tick. Tick-to-tick rather
than against a fixed anchor, so ordinary clock adjustments do not accumulate into a false alarm.

- **Threshold: 5 minutes.** Raised from the 10 seconds this document originally proposed —
  Eric's ruling during implementation, 2026-09-10. Ten seconds was not merely conservative, it
  was unsafe: a tournament running late has its schedule *ahead* of the live game, so a routine
  few-second time-server nudge was read as a sleep and moved the game forward to the scheduled
  position, abandoning a game in progress. Eric chose to remove the trigger rather than
  compensate for the effect, which keeps this document's plain "go to the true schedule
  position" intact. The value lives in one named constant, `TIME_JUMP_THRESHOLD`.
  **Residual, accepted:** a sleep longer than five minutes, while running more than about one
  game-slot late, will still move to the schedule position and abandon the live game.
- **Never wind backwards.** A wall clock that jumps back is logged and otherwise ignored.
- A large forward step from a corrected system clock is treated the same as a sleep. This is
  deliberate: in both cases the true time has moved and the schedule position should follow.

### 2. Placement calculation

Given the true time, the schedule and the current court: which game covers that moment, and how
far into it, expressed as a period plus a time within that period. Pure arithmetic over data
already in memory, using the period lengths from `GamePeriod::duration`
(`uwh-common/src/game_snapshot.rs:187`). No I/O, no clock, no mocking — fully unit-testable.

Overtime and sudden death lengths depend on scores. A game nobody played has none, so the
calculation assumes regulation only.

### 3. Placing the engine

The one genuinely new capability. Today the engine reaches a position only by playing through to
it: there is no public period setter, and `set_game_clock_time`
(`refbox/src/tournament_manager/mod.rs:2050`) works only on a stopped clock and forces
`ClockState::Stopped`. This needs a new method setting game number, timing rule, period and clock
together in one step, alongside `start_game` / `apply_next_game_start`. **This is where the review
effort belongs.**

Penalties need no special handling: they are tracked against period and time-within-period
(`refbox/src/tournament_manager/penalty.rs:52`), not against the stopwatch, so they follow the
game clock automatically.

### Surface

- `refbox/src/tournament_manager/` — the new placement method, the detection hook
- a new self-contained module beside it for the placement calculation
- `refbox/src/app/` — wiring that supplies the schedule and current court

**`uwh-common` is not touched.** The period lengths already exist there. Blast radius stays
inside `refbox`.

---

## Out of scope

- How Portal schedule times are fetched or parsed — that path is already correct on both platforms
- Preventing the machine from sleeping (OS-level keep-awake assertions)
- Any change to the score-confirmation pause or its duration
- Any new operator-facing screen

---

## Testing

- Detection and placement calculation: ordinary unit tests, both pure.
- Engine placement: the engine takes the current time as a parameter throughout, so a test
  simulates a two-hour sleep by handing it a jumped time. No sleeping required.
- Golden traces: a scenario covering a catch-up across a game boundary.

**What cannot be tested here or in CI:** an actual lid-shut sleep. There is no Mac in the
development environment and CI cannot suspend a machine. The only real proof is somebody closing
a laptop lid and opening it again — a walkthrough on Eric's side, required before this is trusted
at a tournament.

---

## Risks

- Game-engine work, which `.claude/rules/plan-execution.md` puts in the heavy-process bucket:
  per-task review, golden-trace coverage, strict deviation tracking.
- It lands near transitions that already carry known bugs (phantom-game POST, replayed game
  re-posting, stale restore note). A new jump path could interact with these.
- The threshold is a judgement call. Too low and a busy machine or a clock correction re-places
  the schedule spuriously; too high and a short sleep goes uncorrected. **This risk was
  realised**: the 10 seconds originally proposed above did exactly the first of those, and the
  figure is now 5 minutes. See the Detection section.

---

## Acceptance criteria

1. A simulated two-hour gap before game 1 leaves the countdown reading the true time to kickoff.
2. A simulated gap landing inside a later game loads that game, with its own number and timing
   rule, at the true position within it.
3. No Portal submission is enqueued for any game the catch-up lands past the end of, including
   one that was in progress. A game the catch-up lands inside keeps its recorded score.
4. A backwards wall-clock step changes no clock.
5. A gap below the threshold changes no clock.
6. With no schedule loaded, a gap takes the lost time off the running clock and starts no game.
7. `just check` clean.
8. A real lid-shut sleep on a Mac leaves the countdown correct on waking (Eric, on hardware).
