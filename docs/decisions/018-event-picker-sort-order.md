# 018 — Event Picker Sort Order

**Date:** 2026-05-12
**Status:** proposed
**Behavior definition required:** before any planning or implementation, the
operator must define the desired sort and display semantics. The Open Design
Questions section is a checklist of decisions needed.

## Context

The event picker in the refbox's settings flow shows the operator a scrollable
list of events to choose from. Today the ordering is incidental:

- [`refbox/src/app/view_builders/list_selector.rs:78`](../../refbox/src/app/view_builders/list_selector.rs)
  iterates `events.values().rev()`, where `events` is a `BTreeMap<EventId, Event>`
  sorted by `EventId` lexicographically.
- This puts events with higher EventIds (often, but not always, more recent ones)
  at the top of the displayed list.
- Operators have no signal in the picker of *when* each event takes place — only
  the event name.

Two specific symptoms result, both surfaced during ADR-009 Task 8 smoke testing:

1. **Wrong ordering for operator mental model.** Operators think about events
   by date proximity ("the event happening today / next week"), not by EventId.
   The picker forces them to scan or scroll past events that may be in the
   distant past or future to find the relevant one.

2. **Disappearing-event scroll bug.** [`refbox/src/app/mod.rs:1874-1883`](../../refbox/src/app/mod.rs)
   computes the initial scroll index against the *non-reversed* BTreeMap
   iteration, but the picker view reverses for display. The result: when
   `self.current_event_id` is at non-reversed position `i`, the picker
   `skip(i)` on the reversed list, hiding the first `i` events of the reversed
   display. The wider the index, the more events disappear from view at the
   top.

A band-aid fix exists for #2 alone (change the index computation to
`events.len() - 1 - i`), but it doesn't address #1 — and any band-aid will be
invalidated by the proximity-based sort the operator actually wants.

## Open design questions — to be answered before planning

1. **What is the sort key?**
   - Event start date (next-to-occur first)
   - First-game-start-time within the event
   - Some combination (events with no schedule yet ordered separately?)
2. **How are past events handled?**
   - Excluded from the picker entirely
   - Grouped at the end of the list, oldest last
   - Intermixed with future events but visually de-emphasized
   - Configurable / operator preference?
3. **Should event dates be displayed in the picker rows?**
   - Show date alongside event name (e.g., `"Mock NW Cup — Apr 19-21"`)
   - Show relative date (`"in 3 days"`, `"yesterday"`)
   - Don't show dates (current behaviour)
4. **What is the initial scroll position when the picker opens?**
   - Top of list (first/next-up event visible)
   - Centered on currently selected event (if any)
   - Some other heuristic
5. **Does the same sort apply to the game picker?**
   - The game picker also uses `BTreeMap` ordering today, but games within an
     event have explicit `start_time` fields. The operator should decide
     whether the game picker matches this ADR's choices or follows its own.
6. **What happens if an event has no date?**
   - Sorted to end as "undated"
   - Sorted by EventId as today (fallback)

## Decision

**TBD — pending operator-defined behavior for the questions above.**

## Sequencing

This ADR is a follow-up to ADR-009 Task 8, where the disappearing-event scroll
bug and the underlying sort-order question were surfaced. It overlaps with
ADR-017 (portal data lifecycle) — the loading-state question for the picker
appears in both. Recommended ordering:

1. Resolve the open questions above (operator input required).
2. Decide whether to address jointly with ADR 017 or as a separate concern.
3. Write the implementation plan as a separate document.
4. Land on a new branch — likely `feat/refbox/event-picker-sort` or similar.

## References

- [`refbox/src/app/view_builders/list_selector.rs`](../../refbox/src/app/view_builders/list_selector.rs)
  — `build_list_selector_page`, the `ListableParameter::Event` arm
  (`list.values().rev()`).
- [`refbox/src/app/mod.rs`](../../refbox/src/app/mod.rs) — `Message::SelectParameter`
  index-computation for `ListableParameter::Event` (around line 1874).
- ADR 009 Task 8 — surfaced both the sort-order question and the
  disappearing-event scroll bug.
- ADR 017 — portal data lifecycle (related loading-state concerns in the picker).
