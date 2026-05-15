# Event Picker Sort Order — Design Spec

**Date:** 2026-05-15
**ADR:** [018 — Event Picker Sort Order](../../decisions/018-event-picker-sort-order.md)
**Status:** Proposed (ready for implementation planning)

## Summary

Change the event picker in the refbox settings flow so that events appear
in chronological order (nearest-upcoming first) instead of in `EventId`
order. Apply a corresponding fix to the court picker so that the operator
no longer sees the picker land on a stale, applied-state-derived scroll
position.

Both pickers will use a single, simple rule: **always open scrolled to
the top of the list.** This eliminates the existing scroll-position bugs
in both pickers by construction — no index computation, no chance of
mismatch between view ordering and index ordering.

The game picker is intentionally out of scope; its current
centered-on-current behavior is correct and operationally valuable.

## Scope

In scope:
- Event picker in [refbox/src/app/view_builders/list_selector.rs](../../../refbox/src/app/view_builders/list_selector.rs)
- Court picker in the same file
- Initial scroll-index computation in [refbox/src/app/mod.rs](../../../refbox/src/app/mod.rs)
  (the `Message::SelectParameter` arms for `ListableParameter::Event` and
  `ListableParameter::Court`)

Out of scope:
- Game picker behavior (unchanged)
- `--all-events` CLI mode (separate retrospective tool; not part of the
  normal operator flow)
- Court ordering inside an event (currently lexicographic from
  `BTreeSet`; a separate concern from this ADR)
- Date display in picker rows (operator chose to keep current
  name-only rows)
- Loading state in the picker while events are still fetching (overlaps
  with ADR 017; out of scope here)

## Operator-Visible Behavior

After this change:

1. **Event picker order.** Events appear in ascending order by event
   start date. The event whose start date is nearest in the future
   appears at the top of the list. Past events do not appear (they are
   already filtered out at the portal API layer).

2. **Event picker scroll position.** Every time the operator opens the
   event picker, the list starts scrolled to the top. The top of the
   list is the next-upcoming event — the most likely target.

3. **Event picker rows.** Each row shows only the event name, as today.
   No date is shown in the row.

4. **Court picker scroll position.** Every time the operator opens the
   court picker, the list starts scrolled to the top. The court at the
   top of the (alphabetical) court list is always visible first.

5. **Court picker order.** Unchanged — alphabetical/lexicographic from
   the existing `BTreeSet` derivation.

6. **Game picker.** Unchanged. The game picker remains centered on the
   currently-selected game so the operator can see games before and
   after their current position during a tournament.

## Architectural Sketch

Two small, isolated changes:

### 1. Sort the event list before display

In [refbox/src/app/view_builders/list_selector.rs](../../../refbox/src/app/view_builders/list_selector.rs)
inside the `ListableParameter::Event` arm of `build_list_selector_page`:

- Replace the existing `events.values().rev()` iterator with one that
  sorts the events by `event.date_range.start` ascending, with
  tiebreakers on `event.date_range.end` ascending and then `event.id`
  for stability.
- This can be expressed as collecting into a `Vec<&Event>` and sorting
  with `sort_by` using a tuple key `(date_range.start, date_range.end,
  id)`, then iterating the sorted vec. The list is small (a handful of
  upcoming events per tournament), so allocation cost is negligible.
- The sort uses `date_range.start` directly. There is no fallback
  branch for "missing date" — every event from the portal has a
  guaranteed `DateRange` (the field is required, not `Option`). Do not
  add defensive code for an undated case; if the portal contract ever
  changes, it is a `uwh-common` concern handled at the API boundary,
  not in the picker.

### 2. Always set the initial scroll index to zero for event and court

In [refbox/src/app/mod.rs](../../../refbox/src/app/mod.rs) inside
`Message::SelectParameter`:

- For `ListableParameter::Event`: replace the existing
  `events.iter().enumerate().find(...).map(|(i, _)| i)` lookup with a
  constant `0`.
- For `ListableParameter::Court`: replace the existing
  `courts.iter().enumerate().find(...).map(|(i, _)| i)` lookup with a
  constant `0`.
- Leave the `ListableParameter::Game` arm unchanged.

This not only fixes the disappearing-scroll bug (event picker) and the
applied-vs-edited mix-up (court picker) — it also deletes the lookup
code that *causes* both bugs. There is nothing to keep in sync.

## Why "Scroll to Top" is the Right Choice for Both

Both buggy pickers fail in the same operator-facing way: the picker
lands on a scroll position that hides the row the operator expected to
see. The two underlying mechanisms are different (one is reversal
mismatch, one is applied-vs-edited mismatch), but the symptom and the
operator's correction (manual scrolling) are the same.

Setting the index to zero in both cases:

- Removes the buggy code entirely (fix by construction; nothing to
  regress).
- Gives the operator a predictable starting view every time, satisfying
  the project's "predictable UI over conditional UI" preference.
- Aligns with the new sort: the top of the event picker is always the
  most-relevant-event (next-upcoming), so the operator's typical
  selection is already in view.

The trade-off — operators with 5+ courts or many upcoming events need
to scroll down to reach a previously-selected entry — is acceptable
because:

- The court list is short in practice (1–6 entries; usually one screen).
- The event list at any one tournament is short (a handful of upcoming
  events; the operator's target is at the top).
- There is no visual highlight for the currently-selected entry in
  these pickers today, so "centered" provided no real confirmation cue
  anyway.

## What This Does *Not* Fix

These items are noted for transparency but are deliberately *not*
addressed in this ADR:

- **Court ordering.** Courts come from a `BTreeSet`, so "Court 1",
  "Court 10", "Court 2" sort lexicographically. Numeric court names
  appear in a counterintuitive order. Separate ADR territory.
- **No visual indicator of the current selection in the picker.** The
  picker shows names only; the operator has no in-picker cue for which
  row is currently selected. Separate UI concern.
- **Loading state.** What the picker shows while the event list is
  still being fetched is not in scope; this is the ADR-017 area.
- **`--all-events` mode.** That CLI flag swaps the portal filter to
  past events. It is a retrospective tool, not part of the operator's
  normal workflow, and this ADR does not change its behavior.

## Acceptance Criteria

The operator can verify the change worked by doing all of the
following at the running refbox:

1. **Event picker shows nearest-upcoming first.** Launch the refbox
   against the portal. Open the event picker from the settings flow.
   The first event in the list should be the event whose start date
   is closest to today (in the future). Subsequent events should be in
   ascending date order.

2. **Event picker always opens at the top.** Pick an event from
   somewhere other than the top. Return to settings. Reopen the event
   picker. The list should be back at the top, with the next-upcoming
   event visible at the first row — *not* scrolled to where the
   previously-picked event was.

3. **Event picker no longer hides rows.** Open the event picker, then
   pick an event from anywhere in the list. Return and reopen. No
   events should be "missing" from the top of the displayed list. (This
   verifies the disappearing-scroll bug is gone.)

4. **Court picker always opens at the top.** With an event that has 5
   or more courts loaded, open the court picker and pick a court near
   the end of the list. Return to settings. Reopen the court picker.
   The list should be back at the top of the court list, not at the
   previously-applied court.

5. **Court picker no longer hides the just-picked row.** With an event
   that has 5 or more courts, open the court picker, scroll down, pick
   a court that is not visible at the top. Return and reopen. The
   first courts in the list should be visible — none "gone."

6. **Game picker is unchanged.** Open the game picker after the
   schedule has loaded. The picker should center on the currently
   selected game number, as it does today.

## Risk and Reversal

- **Risk.** Low. Both changes are tiny (one sort call, two `0` literals)
  and localized to the refbox crate. No wire format changes, no
  `uwh-common` changes, no embedded changes, no state machine touches.
- **Reversal.** Either change can be reverted independently with a
  single small commit if a problem emerges.
- **Heavy process tier?** Borderline. The fix touches the
  `Message::SelectParameter` dispatch (state-machine adjacent) and
  changes user-visible scroll behavior, so per-task verification at the
  refbox build/test level is warranted, but full heavy ceremony
  (per-task code review, deviation tracking) is not required. Treat as
  lean process with explicit verification of each acceptance criterion
  before claiming completion.

## Sequencing

This ADR overlaps with ADR-016 (UWR mode portal routing), which is
currently being implemented on branch
`feat/refbox/uwr-mode-portal-routing`. Conflicts are unlikely (different
code paths) but the eventual merge order should be:

1. Land ADR-016 first (its changes touch event-fetching, not picker
   rendering).
2. Implement and land ADR-018 on a separate branch named
   `feat/refbox/event-picker-sort` (or similar).
3. If both are in flight at Final Integration, rebase ADR-018 onto
   master after ADR-016 lands.

## References

- ADR [018 — Event Picker Sort Order](../../decisions/018-event-picker-sort-order.md) (the document this spec answers)
- [refbox/src/app/view_builders/list_selector.rs](../../../refbox/src/app/view_builders/list_selector.rs) — `build_list_selector_page`
- [refbox/src/app/mod.rs](../../../refbox/src/app/mod.rs) — `Message::SelectParameter` (event/court/game arms around line 1478)
- [uwh-common/src/uwhportal/schedule.rs](../../../uwh-common/src/uwhportal/schedule.rs) — `Event` and `DateRange` definitions
- [uwh-common/src/uwhportal/mod.rs](../../../uwh-common/src/uwhportal/mod.rs) — `get_event_list` portal filter that excludes past events by default
