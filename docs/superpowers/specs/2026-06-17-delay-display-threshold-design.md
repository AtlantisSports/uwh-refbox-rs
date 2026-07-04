# Delay Display Threshold — Design Spec

**Date:** 2026-06-17
**Scope:** `refbox` only (display behaviour). No `uwh-common`, `overlay`, wire-format, or
timing-engine math changes.
**Status:** Approved design, pre-implementation.

---

## Problem

The behind-schedule **DELAY** figure is a raw deviation tally: it compares real time elapsed
against play time elapsed, second-for-second. Because a team timeout (and any stoppage) stops
the main game clock, the tally climbs for the whole timeout. The figure is hidden *during* a
timeout (a 2026-06-08 layout decision) but **reappears with the timeout's time baked in the
moment the timeout ends** — so a routine timeout, fully within the slot's spare time, surfaces a
DELAY that isn't really delay.

This was confirmed (2026-06-17) to be the *intended* behaviour of the raw-tally model, not a
code bug: the slot's spare time ("clawback") is realised at the break (the figure steps down by
the spare time when a game ends), not during the game. See
`backlog_delay_timer_immediate_during_timeout` and `project_behind_schedule_model_final`.

The user wants the **engine to stay raw** (so it keeps honestly tracking deviation and the
cross-game clawback math, golden traces, and 200+ tests are untouched) but the **operator-facing
figure to read as genuine delay** — i.e. how late the next game will *actually* start.

## Goal

Add a display-only rule so DELAY shows the **genuine, unrecoverable delay** — the raw tally with
the slot's spare time discounted — instead of the raw tally. The raw figure
(`TournamentManager::behind_schedule`) is unchanged.

## Decisions (locked with the user)

1. **Excess only.** Once accrued delay crosses the slot's spare-time line, show only the
   unrecoverable part (`behind − spare time`), not the full raw total. DELAY = "how late the
   next game will genuinely start." (Alternative — show the full raw total once gated — was
   rejected: it would jump to a big, mostly-recoverable number.)
2. **Uniform / stay-blank.** A late-starting game whose slot can recover it shows **blank**,
   exactly like a recoverable stoppage. One rule for everything: `displayed = max(0, behind −
   spare time)`. (Alternative — always flag a late start while only discounting stoppages — was
   rejected as a second, more complex rule.)

## The display rule

The "spare time" is the slot slack already defined in shared config:
`Game::game_block_buffer()` = `game_block − regulation_play − minimum_break` (saturating).

```
displayed_behind(now) =
    if current_period == BetweenGames:
        behind_schedule(now)                          // unchanged
    else:
        saturating_sub(behind_schedule(now), game_block_buffer())   // preview the break step-down
```

**Why between-games is left raw (no double-discount):** the engine *already* subtracts the slot
slack at the break — when a game ends the figure steps down by exactly `game_block_buffer()`
(see `test_behind_schedule_steps_down_by_slack_at_game_end`). Subtracting again between games
would count the spare time twice.

**Smoothness property (a benefit, not a coincidence):** because the in-game rule previews the
exact step-down the engine applies at the break, the displayed figure is continuous across the
end of a game. A game 10:00 behind with 7:00 spare shows **-3:00** during the game and still
**-3:00** the instant it ends (today: -10:00 then a snap to -3:00). This is an acceptance test.

*Caveat:* this strict continuity holds only at the **minimum-break** boundary, where the
engine's step-down equals `game_block_buffer()`. In portal mode with a long scheduled gap before
the next game, the engine steps the between-games figure down by *more* than the slack (possibly
to zero), so the displayed figure can drop *further* at the boundary. That step is always
downward (the figure only ever gets smaller, never a false spike), so it is benign — but it is
not strictly continuous.

## Architecture

**Chosen approach (B): a read-only derived accessor on the tournament manager.**

- Add `TournamentManager::behind_schedule_shown(&self, now: Instant) -> Duration` in
  `refbox/src/tournament_manager/mod.rs`, implementing the rule above. It calls the existing
  `behind_schedule(now)` and applies the buffer preview using `self.config.game_block_buffer()`
  and `self.current_period`. Pure read; no state mutation.
- `behind_schedule` itself is **not modified** — all existing `test_behind_schedule_*` tests and
  the golden-trace guard remain valid (the guard watches time/score state, not this derived
  read).
- Swap the single UI call site: `refbox/src/app/mod.rs:4292` changes
  `…behind_schedule(Instant::now())` → `…behind_schedule_shown(Instant::now())`. The
  `show_behind_schedule_time` config gate around it is unchanged.

Approaches considered and rejected:
- **A — compute it in the view layer** (`main_view.rs` / the `mod.rs` call site): puts
  branch-on-period logic in the UI and is awkward to unit-test next to the engine tests.
- **C — fold the discount into `behind_schedule`**: disturbs the raw engine and its tests/golden
  traces for no benefit.

## What stays the same

- DELAY block look, position, red colour, `-M:SS` format (`shared_elements.rs` /
  `make_game_time_button`) — untouched.
- Still hidden during a timeout, reappears after (`shared_elements.rs:627-633`) — now reappears
  blank when the slot absorbed the stoppage.
- `show_behind_schedule_time` setting keeps its meaning: on = show the new excess-only figure;
  off = hide entirely. **No new setting.**
- Slot with **no spare time** (`game_block_buffer() == 0`): the rule discounts nothing, so DELAY
  behaves exactly like today (every stoppage shows immediately).
- Extra time (overtime / sudden death): `behind_schedule` already treats `remaining_regulation`
  as zero there; the uniform in-game discount applies, no special-casing.

## Acceptance criteria (operator-observable)

1. On-time game, take a team timeout shorter than the slot's spare time → DELAY stays **blank**
   during and after the timeout.
2. Stoppages totalling more than the slot's spare time → DELAY shows only the **excess**
   (`behind − spare time`).
3. A game that starts late but whose slot can recover it → DELAY stays **blank**.
4. DELAY does **not jump** at the end of a game (figure is continuous across the boundary).
5. With spare time = 0, behaviour is identical to today.

## Testing

New unit tests in the `tournament_manager` test module, mirroring the existing `behind_schedule`
tests but asserting `behind_schedule_shown`:
- on-time game + team timeout within buffer → `shown == 0` while `behind_schedule > 0`
- stoppage exceeding buffer → `shown == behind_schedule − game_block_buffer`
- late start within buffer → `shown == 0`
- continuity across game end → `shown` just-before == `shown` just-after
- `game_block_buffer() == 0` → `shown == behind_schedule`
- between games → `shown == behind_schedule` (no double-discount)

Existing `behind_schedule` tests and golden traces unchanged. Run `just test` (always, after any
`tournament_manager` change) and `just check` before PR.

## Blast radius / process

Low. Refbox-only, display-only, additive read-only accessor; raw engine math, wire format, and
golden traces untouched. It lives in the critical `tournament_manager` file, so it gets full
unit-test coverage, but it is not a state-machine change.

## Out of scope

- Changing the raw `behind_schedule` math, inherited-lateness carry, or between-games clawback.
- Reopening the full "Absorbing" model (engine-level buffer subtraction).
- LED panel / overlay rendering.
- Any new config setting.

## Suggested branch (on approval, later)

`feat/refbox/delay-display-threshold`
