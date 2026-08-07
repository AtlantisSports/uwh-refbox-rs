# Penalties: make the infraction optional

**Date:** 2026-08-07
**Branch:** `fix/refbox/optional-penalty-infraction` (off `origin/master` `1900f4ad`)
**Crate:** `refbox` only

## Problem

On the penalty page, the DONE button required a player number *and* — whenever "track fouls
and warnings" was on — an infraction. Poolside the operator may need the exclusion clock
running before the reason has been settled, and the gate blocked that.

## Decision

DONE requires **only a player number**. The infraction picker is unchanged and still shown
whenever tracking is on; choosing an infraction is simply no longer a precondition for saving.

Applies to both adding a new penalty and editing an existing one — the same page serves both.

## The asymmetry, and why it is correct

Fouls and warnings still **require** an infraction (`foul_add_can_commit`,
`warning_add_can_commit`). A foul or warning is nothing but its infraction, so a blank one
records no information at all. A penalty's essential content is the player and the duration;
the reason is useful but secondary.

This pairs with the sibling change on `fix/refbox/infraction-picker-prompt-label`, which makes
the foul/warning picker bar read "Infraction: Make selection" precisely because it *is*
required there. The penalty page never displayed that bar
(`make_penalty_dropdown(infraction, false)`), so the two changes do not interact visually.

## Why this is low-risk

An infraction-less penalty is not a new state:

- It is exactly what this page has always produced with tracking **off**.
- Tracking is a settings toggle that can be flipped at any time, so "penalty with no
  infraction, tracking on" was already reachable before this change.
- `Infraction::Unknown` is the enum's `#[derivative(Default)]` variant and is handled
  everywhere downstream — scoresheets print "Unknown", the LED panel and portal serialize it
  like any other variant. Nothing special-cases it as impossible.
- The penalty list rows do not render the infraction at all (they show player number, time,
  and kind), so there is no new display case.

## Implementation

`penalty_edit_can_commit` in `refbox/src/app/view_builders/keypad_pages/penalty_edit.rs`
narrows its rule to `player_num > 0`.

It keeps — and ignores — the team, infraction, and tracking flag:

```rust
fn penalty_edit_can_commit(
    _color: GameColor,
    _infraction: Infraction,
    _track_fouls_and_warnings: bool,
    player_num: u32,
) -> bool {
    player_num > 0
}
```

Taking the whole of the page state that could plausibly gate saving, rather than only what
the current rule reads, is what makes the *irrelevance* of the other three assertable. Without
them the guarantee "an infraction never blocks saving" would be untestable, and a future
change could quietly re-block the operator.

`make_penalty_edit_page` still uses `track_fouls_and_warnings` to decide whether the picker is
drawn and `infraction` to drive its selected tile.

No translation changes.

## Note on the button's name

The button gated here is the penalty page's **DONE** button. The **APPLY** button on the
penalties *overview* page is a different control — it commits the whole list of pending
changes and is not affected.

## Acceptance criteria

1. Tracking **on**: pick a team, enter a player number, pick a duration, leave the infraction
   grid untouched → DONE is enabled and the penalty saves.
2. Tracking **on**, no player number → DONE stays greyed out.
3. Tracking **off** → behaviour unchanged.
4. Add Foul / Add Warning with no infraction → still refuse to save.
5. `just check` clean.

## Testing

`penalty_gate_depends_only_on_the_player_number` sweeps the full cross-product of team
(Black/White) × every `Infraction` variant × tracking on/off, asserting that a player number
is sufficient in all 48 combinations and that its absence blocks in all 48. It replaces the
three narrower tests that encoded the old rule.

Mutation-checked: restoring the old condition fails the test with
`a player number must be enough: Black, Unknown, tracking=true`.

A team cannot be deselected on this page, so "team is required" is an invariant rather than a
gate; the sweep covers both teams to pin that either one suffices.
