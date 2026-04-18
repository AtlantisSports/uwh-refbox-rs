# 008 — Game Block

**Date:** 2026-04-18
**Status:** proposed

## Context

The tournament schedule for a pool runs on a fixed cadence — games start
every 30 minutes, or maybe every 23 minutes, this is variable depending 
on the event. This cadence is what lets organizers lay out a full day of 
play with predictable slot boundaries. But **the refbox has no name for 
this concept today, and no parameter that captures it authoritatively.**

Today's state:

- `nominal_break` (on `GameConfig`) is the only break-related setting
  the operator can tune. It is break-only — the full time between games
  is `2·half + half_time + nominal_break`, but the break itself is what
  the field actually stores.
- `nominal_break` is **only used at runtime when the refbox is not on a
  portal schedule.** When a portal schedule is present, explicit
  per-game `start_time` values drive scheduling
  (`refbox/src/tournament_manager/mod.rs:886`). `nominal_break` falls
  back to being (a) a default-filler in the portal timing-rule decoder
  (`uwh-common/src/uwhportal/schedule.rs:316`), (b) the last-resort
  scheduling cadence if the portal omits a `start_time` for some game,
  and (c) a still-editable button on the Configuration screen even
  though that button has no effect on portal-driven tournaments.
- The equivalent math minimum (`2·half_play + half_time + minimum_break`)
  is already computed — unnamed — inside schedule-processor at
  `schedule-processor/src/schedule_checks.rs:707` as an internal
  validation helper. Nothing else in the workspace names or exposes it.
- Searched `refbox/`, `uwh-common/`, `schedule-processor/`, and
  `/home/estraily/projects/uwh-portal/`: no `gameBlock`, `GameBreak`, or
  `nominalGameBreak` concept exists in either repo.

The absence of an authoritative "this tournament runs on X-minute
blocks" value makes the math less teachable, leaves the Configuration
screen with a parameter that lies about its effect, and denies the
refbox any built-in check that a given block size even fits the rules
configured.

## Decision

Introduce **Game Block** as the authoritative name for a tournament's
game-to-game slot duration.

### Definition

> **Game Block** — the start-to-start duration of one game's slot in a
> tournament schedule. A new game begins every Game Block minutes.

Game Block replaces `nominal_break` as the user-facing scheduling
parameter on the refbox. `nominal_break` remains as an internal field
to support backwards compatibility only (see below).

### Math minimum

The math minimum is the smallest Game Block that physically fits a game
plus its minimum break. It depends on the game format:

- **Two-period format:** `2·half_play + half_time + minimum_break`
- **Single-period format:** `period_play + minimum_break`

A Game Block below the minimum is a schedule contradiction: games
cannot complete inside their slot.

### Data model and backwards compatibility

Two modes, chosen by the input the refbox is reading:

- **Old-world mode** — the portal sends timing rules in the current
  shape (no Game Block field), **or** a local config file has not yet
  been migrated. The refbox behaves exactly as today: the Configuration
  screen shows "Nominal Break"; the main screen shows no block
  indicator; runtime scheduling uses `nominal_break` as it does now.
  Nothing changes for users in this mode.
- **New-world mode** — the portal sends timing rules that include a
  Game Block field, **or** the refbox is running standalone with no
  portal schedule. The Configuration screen shows "Game Block"; the
  runtime code uses Game Block directly as the scheduling cadence; the
  main screen surfaces the overrun indicator described below.

The portal's `TimingRule` gains an optional `game_block` field. The
refbox accepts both shapes during a transition period with no declared
end date. When both shapes coexist at a tournament the operator may
see different Configuration screens depending on which tournament they
are running; this cost is accepted.

### Local config migration

`GameConfig` gains a `game_block` field. On upgrade, an existing
config file written by an older refbox version does not have this
field. The existing migration path
(`refbox/src/main.rs:353-381` → `Config::migrate` in
`uwh-common/src/config.rs`) translates such files on first read:

```
game_block = 2·half_play + half_time + nominal_break
           (or period_play + nominal_break for single-period)
```

The migrated config is rewritten to disk via `confy::store`. The
operator sees their prior scheduling cadence pre-filled under the new
name, with no manual action required.

`nominal_break` remains on the `GameConfig` struct indefinitely. In
old-world mode it is the user-editable parameter. In new-world mode
it is derived from Game Block and not exposed on the Configuration
screen.

### Configuration-screen validation (new-world)

The Game Block button and its editor use warning-only colouring: the
button keeps the standard gray appearance of every other timing
button on the Configuration screen when the value is healthy, and
changes colour only in problem states.

- **Red, Done button disabled.** Game Block is below the math minimum.
  The value cannot be saved; an invalid Game Block is never written.
- **Yellow, Done button enabled.** Game Block meets the minimum but
  the buffer (Game Block minus math minimum) is insufficient to absorb
  the timeouts the current rule set permits. Value can be saved but
  the operator is warned that timeouts will push games past their
  slot.
- **No colour (default gray).** Game Block has adequate buffer for the
  allowed timeouts. Visually indistinguishable from the other
  timing-parameter buttons on the page — no positive "green" signal.

The same rules apply inside the value editor so the colour updates as
the operator types.

### Main-screen runtime indicator (new-world)

On the main game screen, the right side of the time bar carries a
silent-by-default indicator. It appears **only when the current game
has accumulated more wall-time overrun than the buffer can absorb**.
The overrun is computed from actual wall time elapsed minus expected
game time — i.e., the time already lost to timeouts and stoppages in
this game, with no projection of future timeouts.

- While `accumulated_overrun ≤ buffer`: no indicator is shown.
- When `accumulated_overrun > buffer`: the indicator shows
  `-M:SS` in red, where `M:SS = accumulated_overrun − buffer`. This
  is how far the current game has cut into the minimum-break cushion.

The indicator gives the operator a quiet-until-it-matters warning
that this game's slot is being squeezed.

## Open design questions (to resolve during implementation)

- **Yellow threshold formula.** "Buffer insufficient to absorb allowed
  timeouts" is unambiguous in concept, but the exact expression
  depends on how timeouts are configured (number per team × duration
  each, plus any ref-timeout assumption). Implementation must settle
  on a single formula and document it.
- **Portal/rule disagreement handling.** If a new-shape portal
  payload arrives with `game_block` below its own math minimum, the
  refbox should surface the error clearly rather than silently
  sanitising. Decide whether to refuse the schedule, warn loudly, or
  cap at minimum.
- **Format of the runtime indicator.** `-M:SS` matches the style of
  the main clock; but an operator at a distance might parse a rounded
  `-M` more quickly. Test under actual lighting conditions.

## Consequences

**Becomes easier:**

- Operators and organizers have a named parameter for the tournament's
  cadence — a concept everyone already talks about informally.
- The Configuration screen refuses impossible values before they can
  propagate into a game.
- Runtime scheduling math simplifies: `sched_start + game_block`
  replaces the current
  `sched_start + 2·half + half_time + nominal_break` expression in
  `refbox/src/tournament_manager/mod.rs:1040-1044` and `:1970-1974`.
- Existing refboxes upgrade silently: the auto-migration preserves the
  user's prior cadence.

**Becomes harder / constrained:**

- Adding `game_block` to the portal's `TimingRule` is a cross-system
  change; until the portal ships new-shape rules, all portal-driven
  refboxes stay in old-world mode and see no benefit from the feature.
- The refbox code carries both old-world and new-world paths until
  old-shape portal payloads are retired. No sunset date is declared.
- `nominal_break` remains on `GameConfig` indefinitely. Removing it
  requires a separate decision once old-world mode is retired.

**Scope:**

- `uwh-common` — new `game_block` field on `GameConfig`; update to
  `Config::migrate`; optional `game_block` field on portal-side
  `TimingRule` decoder accepting both shapes.
- `refbox` — Configuration screen (new-world only): new button, new
  editor with colour validation; `tournament_manager` runtime switched
  to use `game_block` directly in new-world; main-screen time bar
  gains the overrun indicator in new-world.
- `schedule-processor` — no change strictly required. A follow-up
  could replace the unnamed `occupied_time` at
  `schedule-processor/src/schedule_checks.rs:707` with a shared
  `game_block_minimum()` helper, but is not part of this decision.
- **Portal (out of workspace)** — add `game_block` to the TimingRule
  shape. Coordination item; prerequisite for any refbox to enter
  new-world mode via portal data.

## References

- `refbox/src/tournament_manager/mod.rs:886` — portal `start_time`
  drives scheduling when present; `nominal_break` is the fallback.
- `refbox/src/tournament_manager/mod.rs:1040-1044`, `:1970-1974` —
  the `2·half + half_time + nominal_break` expression that becomes
  `game_block` in new-world mode.
- `refbox/src/app/view_builders/game_info.rs:233` — existing
  `!using_uwhportal` gate hides "Nominal Break" on portal schedules.
- `refbox/src/app/view_builders/configuration.rs:436-441` — the
  always-visible "Nominal Break" button on the Configuration screen
  that new-world mode replaces.
- `refbox/src/main.rs:353-381` and `uwh-common/src/config.rs`
  `Config::migrate` — existing migration machinery that will handle
  the `nominal_break` → `game_block` transition.
- `uwh-common/src/uwhportal/schedule.rs:314-338` — portal timing-rule
  decoder where old-shape vs new-shape acceptance will live.
- `schedule-processor/src/schedule_checks.rs:707` — unnamed
  `occupied_time` computation equivalent to the Game Block math
  minimum.
