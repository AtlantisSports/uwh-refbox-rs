# 017 — Portal Data Lifecycle (Lazy Fetch and Refresh)

**Date:** 2026-05-12
**Status:** proposed
**Behavior definition required:** before any planning or implementation, the
operator must define the desired lifecycle semantics. The Open Design Questions
section is a checklist of decisions needed.

## Context

The refbox currently fetches the portal event list eagerly at app startup,
regardless of whether the operator intends to use the portal at all:

- [`refbox/src/app/mod.rs:927`](../../refbox/src/app/mod.rs) — `new()` returns
  `Task::batch([..., new.request_event_list(), ...])` unconditionally.
- [`refbox/src/app/mod.rs:267-285`](../../refbox/src/app/mod.rs) — `request_event_list`
  fires the HTTP request via the `UwhPortalClient` if one is constructed.
- When the event list arrives, [`Message::RecvEventList`](../../refbox/src/app/mod.rs)
  (around line 2447) iterates every event and fires a second wave of
  `request_teams_list(event_id)` HTTP requests, one per event.

For an operator running an offline tournament with `using_uwhportal=false`,
this means:

- An HTTP request goes out for the event list at every refbox launch.
- A burst of teams-list requests follows, one per event in the portal —
  potentially dozens of unused requests.
- If the portal URL is misconfigured (e.g. the production URL is left in the
  config but the operator is on a dev token), every launch leaks an authenticated
  request to the wrong tenant.
- The work is wasted because nothing in the UI surfaces this data until the
  operator sets `using_uwhportal=true`.

Additionally, the current lifecycle has secondary gaps that became visible
during ADR 009 Task 8 smoke testing:

- **Loading-state UI.** While teams data is in flight, the game picker renders
  rows as `"1 - Unknown vs Unknown"` instead of a deliberate
  "loading" affordance — the placeholder looks like real data and is misleading.
- **New-event-mid-session.** If a portal-side event is created after the initial
  `RecvEventList`, the refbox never re-fetches the event list. Its teams
  request is consequently never sent. The operator has no path to discover the
  new event without restarting refbox.
- **Refresh semantics.** `Message::RequestPortalRefresh` exists but refreshes
  only the *schedule* for the currently linked event. There is no operator-facing
  control for refreshing the event list, teams list, or token state.

This ADR exists because Task 8's manual smoke test surfaced both the wasted-
startup-fetch concern and the misleading-loading-state symptom, and because the
broader portal-lifecycle question is not addressable as a Task 8 deviation —
it crosses architectural boundaries and ADR-011 territory.

## Open design questions — to be answered before planning

1. **When should event-list fetch fire?**
   - At startup unconditionally (today)
   - At startup only when `config.using_uwhportal=true` (lazy from persisted
     config)
   - Only when the operator toggles `using_uwhportal` ON in settings (lazy from
     user action)
   - Some combination (e.g., fetch on toggle AND on every settings entry while
     toggle is on)
2. **When should teams-list fetch fire for each event?**
   - For all events when event list arrives (today)
   - Only for events the operator selects (saves N-1 unused fetches)
   - Some prefetch heuristic (e.g., events in next 30 days)
3. **How should mid-session changes on the portal be handled?**
   - Manual refresh button only
   - Periodic auto-refresh (interval? what triggers it?)
   - Push from portal (server-sent events / polling) — out of scope or future ADR?
4. **What does the operator see while data is loading?**
   - The current "1 - Unknown vs Unknown" placeholder (broken)
   - A "loading…" row in the picker
   - A spinner / disabled state on the entry button
   - Some combination
5. **How does this interact with ADR 011** (portal health indicator)?
   - Should lazy-fetch failures surface in the health tile?
   - Should the health tile drive refresh requests on click?
6. **What is the credential / privacy story for unauthenticated requests?**
   - If `using_uwhportal=false`, should we even construct `UwhPortalClient`?
   - Are there leak / fingerprint concerns from the eager teams-list burst?

## Decision

**TBD — pending operator-defined behavior for the questions above.**

## Sequencing

This ADR is a follow-up to ADR-009 Task 8, where the wasted-startup-fetch issue
was first surfaced. It also intersects with:

- ADR 011 — portal health indicator (status visibility for portal lifecycle).
- ADR 016 — UWR mode portal routing (URL selection lives in the same construction
  path as the lifecycle question).
- New event-picker UX ADR (currently proposed as 018) — the loading-state
  question overlaps with picker rendering.

Recommended ordering once behaviour is defined:

1. Resolve the open questions above (operator input required).
2. Write the implementation plan as a separate document.
3. Land on a new branch — likely `feat/refbox/portal-data-lifecycle` or
   `refactor/refbox/portal-data-lifecycle` depending on scope.

## References

- [`refbox/src/app/mod.rs`](../../refbox/src/app/mod.rs) — `request_event_list`,
  `request_teams_list`, `RecvEventList`, `RequestPortalRefresh`.
- ADR 009 Task 8 — surfaced the issue during smoke testing of the Game Options
  Cancel/Apply chrome.
- ADR 011 — portal health indicator; related portal lifecycle work.
- ADR 016 — UWR mode portal routing; another portal-construction-site concern.
