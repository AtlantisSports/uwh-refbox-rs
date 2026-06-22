# 017 — Portal Data Lifecycle (Lazy Fetch and Refresh)

**Date:** 2026-05-12 (proposed); accepted 2026-05-16
**Status:** accepted (verified by `feat/refbox/portal-subsystem-dormancy` 2026-05-16)

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

The portal subsystem is **dormant** whenever `self.using_uwhportal == false`.
Dormant means no new fetches are dispatched, the health indicator is hidden,
and no portal-side activity is observable to the operator. The toggle is the
single authoritative gate.

### Answers to the six open questions

1. **When should event-list fetch fire?**
   - **At startup,** only if the runtime `self.using_uwhportal` is true at the
     moment `RefBoxApp::new()` returns. The unconditional fetch in
     `RefBoxApp::new()` is replaced with a conditional push to the startup-task
     batch.
   - **On toggle ON,** the moment the operator taps the Using-UWH-Portal toggle
     button (handled in `BoolGameParameter::UsingUwhPortal` arm of
     `Message::ToggleBoolParameter`). The fetch fires immediately — operators
     do not wait for Apply to commit the toggle before seeing the picker populate.
   - **Never** while `using_uwhportal == false`. There is no other entry point
     for the event-list fetch.

2. **When should teams-list fetch fire for each event?**
   - **No structural change.** Teams-list fetches continue to fire in batch
     from the `RecvEventList` handler. Because the event-list fetch is now
     gated upstream by the dormancy contract, the teams-list burst only happens
     when the operator has opted in.
   - The "loading…" affordance for the picker, and the "fetch only for events
     the operator selects" optimisation, are out of scope for this ADR. They
     can land as follow-up branches if operator review surfaces a concrete need.

3. **How should mid-session changes on the portal be handled?**
   - **Manual refresh via `Message::RequestPortalRefresh` only.** No periodic
     auto-refresh, no server-sent events. The operator-facing refresh path
     is unchanged from today.
   - If the operator turns the toggle OFF and back ON in one session, the
     toggle-on path re-fires the event-list fetch (fresh data on each
     re-engagement). This is an intentional consequence of question 1's
     toggle-time fetch.

4. **What does the operator see while data is loading?**
   - **No change in this ADR.** The "1 - Unknown vs Unknown" placeholder
     issue is real but separable from the dormancy contract. Recommended
     follow-up branch:
     `feat/refbox/portal-loading-affordance` — replaces the placeholder
     with an explicit "loading…" row in the picker and disables Apply
     until teams data lands. Out of scope here.

5. **How does this interact with ADR 011 (portal health indicator)?**
   - **ADR 011 is amended.** A 2026-05-16 amendment to ADR 011 ties the
     indicator's visibility to `self.using_uwhportal && self.current_event_id.is_some()`,
     so the indicator is hidden whenever the subsystem is dormant. The
     previous 2026-04-23 amendment's "dormant until event linked" rule
     remains in force — both gates must pass for the tile to render.
   - Lazy-fetch failures continue to surface in the health tile the same
     way they do today (Yellow during retry, Red after escalation). No
     new failure surface is introduced.

6. **What is the credential / privacy story for unauthenticated requests?**
   - **`UwhPortalClient` is still constructed at startup** regardless of the
     toggle, because the client itself is cheap (no network I/O at
     construction time) and is referenced by other code paths (token
     verification, manual refresh). The dormancy contract gates *what
     requests fire*, not *whether the client exists*.
   - With this ADR's contract enforced, an operator running offline with
     `using_uwhportal == false` makes **no portal HTTP requests at any point
     in the session**, eliminating the leak/fingerprint concern raised in the
     original Context.

### Cached data on toggle transitions

- **OFF → ON:** fetch immediately (per Q1).
- **ON → OFF:** cached `self.events` / `self.schedule` / `current_event_id`
  are preserved in memory. The indicator hides immediately on the next
  snapshot. Toggling back to ON re-runs the event-list fetch (fresh data).
  No proactive clearing — keeps the toggle as a UI-level switch rather than
  a destructive state purge.
  **(SUPERSEDED 2026-06-23 — see the amendment below.)**

### Amendment 2026-06-23 — ON → OFF is now a clean wipe

The original ON → OFF decision ("no proactive clearing") is reversed. Switching the
portal off now returns the refbox to a fresh-manual-launch state: the loaded event,
court, game, and schedule are cleared, and the before-game clock is reset to the
nominal break (`TournamentManager::reset_to_manual_break`, which also resets the
game number to the fresh-launch default). The saved portal token is kept (this is
not a logout). Mid-game, the switch is gated by the `SwitchToManualFromApply`
confirmation (End game & apply / Keep game & apply), matching other mid-game
parameter changes. Rationale: a leftover portal-scheduled start time silently
driving the manual countdown is confusing; "switch to manual = clean slate" is more
predictable for the operator. The original network-cost rationale is unaffected — no
fetches fire while the portal is off. See
`docs/superpowers/specs/2026-06-22-portal-off-manual-reset-design.md`.

### What is not in scope for this ADR

- Persisting `using_uwhportal` across sessions in `config.toml` (currently
  hardcoded to `false` at startup in `RefBoxApp::new()`). If operator review
  later prefers session-persisted toggle state, that is a follow-up.
- Loading affordance for the picker (deferred per Q4).
- Per-event teams-list fetching (deferred per Q2).
- Push notifications / server-sent events (rejected as out-of-scope future work).

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
