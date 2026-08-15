# 012 — Time-to-Next-Game Log Level

**Date:** 2026-04-19
**Status:** accepted

## Context

Inside the tournament manager there is a helper called
`calc_time_to_next_game` (`refbox/src/tournament_manager/mod.rs:1001-1035`).
It runs whenever the refbox needs to know how far away the *next*
game's scheduled start is — for example when the current game ends
and the between-games countdown is about to begin.

Most of the time this calculation is straightforward. The helper
takes the next game's scheduled start time, subtracts the current
wall-clock time, and converts the resulting duration into a
monotonic clock value. All of the routine inputs and outputs are
logged at `info` level (lines 1002, 1006, 1007, and 1010).

There is one case where the conversion cannot proceed: when the
scheduled start time is *in the past*, producing a negative
duration. A negative duration cannot be converted into the standard
library duration type used by the clock. Today the helper catches
that conversion failure and writes:

```
error: Failed to calculate time to next game start: ...
```

It then falls back to `Instant::now()` — i.e. "use the current time
as the anchor, as if the next game starts now." This fallback is
benign and matches what the operator would want in practice.

The problem is the `error!` level and the phrasing. A negative
duration is not an error. It is a meaningful operational signal:
**the tournament is running behind schedule.** The scheduled start
for the next game has already passed. This is extremely common at
real tournaments and should not be recorded as a failure.

### How this came up

This ADR was prompted by the 2026-04-18 tournament log review that
also produced ADRs 011 and 013. During that review the log line was
initially treated as a likely bug trail because it reads as an
error. Following the trail confirmed the helper's fallback is
correct and has been in place since 2022-06-20 — the log line has
always been cosmetic. Retaining `error!` creates an ongoing trap:
every future log investigator will chase this line the same way,
and waste time reaching the same conclusion.

## Decision

Two changes to the single log call on `mod.rs:1017`:

1. **Demote the level.** Change `error!` to `warn!`. A warning
   communicates "this is unusual and worth noting, but nothing
   failed," which matches what is actually happening.
2. **Rephrase the message.** Replace *"Failed to calculate time to
   next game start: {e}"* with wording that describes the
   situation, for example *"Next game start time is in the past
   ({e}); using current time as anchor."* The conversion-error
   value `{e}` is still preserved, because it carries the negative
   duration's magnitude — which is directly useful diagnostic
   information (how far behind schedule the tournament is running).

### Why `warn`, not `info`

The surrounding lines in `calc_time_to_next_game` are at `info`
level and report routine inputs (the next-game info, the current
time, the start time, the computed duration). Using `info` here
would blend the "behind schedule" outcome into that routine
stream. `warn` keeps the outcome visible as a distinct signal
without dressing it up as a failure. Both levels are shown to the
operator by default (the default log-level threshold is `info`), so
visibility is not the deciding factor — severity accuracy is.

### Why not remove the log line entirely

The conversion failure carries diagnostic value. The `{e}` value
contains the negative duration (i.e. *how far* behind schedule the
tournament is). Keeping it at `warn` preserves that information for
later log reviews without polluting the error stream.

## What is not changing

- The fallback behaviour. `Instant::now()` remains the anchor when
  the conversion fails. No game-clock behaviour changes.
- The helper's signature, return type, or call sites. This is a
  single-line log-level-and-message change.
- The default log-level filter in `refbox/src/main.rs`. The
  existing default (`info` for `refbox` and `uwh_common`, `error`
  for the rest) is unaffected.
- The other `info!` calls on lines 1002, 1006, 1007, and 1010. They
  describe the routine inputs and computation and remain at
  `info` level.
- The overflow guard on the successful conversion arm
  (`mod.rs:1013-1015`), added after this ADR was first drafted. Its
  comment cross-references "the conversion-error arm below"; that
  remains accurate, because both arms still fall back to `now`.

## Consequences

**Becomes easier:**

- Log reviews after a tournament can distinguish real errors from
  "tournament ran behind schedule" at a glance. The line no longer
  masquerades as a failure.
- Anyone who searches the log for "error" during an incident
  investigation will not be pulled off-course by this line.
- The new message text makes the condition self-describing — a
  reader does not need to know the helper's internal structure to
  understand what happened.

**Becomes harder / constrained:**

- Nothing. The fallback, semantics, and downstream behaviour are
  identical.

**Does *not* address:**

- The broader question of whether the refbox should *visibly*
  indicate to the operator that the tournament is running behind
  schedule. That is a UX concern that belongs to the Game Block
  work (ADR 008), which already proposes a main-screen overrun
  indicator. ADR 012 is only about the log signal.

**Scope:**

- `refbox` — one file, one log call (`tournament_manager/mod.rs:1017`).
- No other crate is touched. No test is affected; the behaviour
  under test is identical. Clippy and formatting checks are
  unaffected.

## References

- `refbox/src/tournament_manager/mod.rs:1001-1035` —
  `calc_time_to_next_game`, the helper containing the log call.
- `refbox/src/main.rs:373-446` — log-level configuration. Establishes
  that `info`, `warn`, and `error` are all visible to the operator by
  default; `debug`/`trace` require the `-v` / `-vv` flags.
- ADR 008 — Game Block. The operator-visible "tournament running
  behind" indicator is owned by ADR 008's main-screen overrun
  feature, not by this log change.
- ADRs 011 and 013 — sibling ADRs from the same 04-18 tournament-log
  investigation. Independent remedies; shared context.
