# 015 — Refbox Stats-Endpoint Posting and 400 Handling

**Date:** 2026-04-23
**Status:** proposed

## Context

At game-end the refbox posts to two portal endpoints via the retry
queue introduced in ADR 011:

1. `post_game_scores` — `/api/events/{event}/schedule/games/{gameNumber}/scores`
   — the aggregate team score (black vs. white).
2. `post_game_stats` — `/api/admin/events/stats` — per-player game
   events (which player scored each goal, which player received each
   foul or warning, with cap-number attribution).

The first endpoint accepts scores unconditionally when the caller is
authorised for the event. The second has a precondition in the
portal's `AdminController.EventStats.PushGameEvents` handler
(`api/Controllers/AdminController.EventStats.cs`):

```csharp
if (@event.ProfilePropertyConfiguration.CapNumber[ParticipantRole.Player] != Level.Required
    || @event.ProfilePropertyConfiguration.CapNumberConstraints[ParticipantRole.Player] == CapNumberRequirement.None)
{
    return BadRequest(new { Reason = "This event does not require unique cap numbers." });
}
```

The cap-number check runs **before** the body's `items` array is
inspected. Consequently the portal will 400 a stats submission
against any event that does not require unique cap numbers — even
when the refbox is posting an empty-items payload because the
operator did not track any per-player events during the game.

This became observable during Task 22 walkthrough prep on 2026-04-23
against a minimally-configured dev-portal test event (no cap-number
requirement). Every `post_game_stats` attempt returned 400; every
`post_game_scores` succeeded. Under the ADR 011 retry policy, both
calls must succeed for `ItemResolved` to fire — so the UI showed the
item as permanently Pending while the scores were already visible
on the portal web UI. Before ADR 011, this partial-failure mode
existed but was silent; the health indicator is the first thing
that surfaces it.

Three outcomes share the same observable symptom (stats always
failing) but differ in cause and fix:

- **Operator did not track per-player events.** The refbox sends an
  empty `items` array. The cap-number check rejects it before the
  empty-array is inspected.
- **Event is misconfigured.** The operator *did* track events and
  they are valid, but the event lacks cap-number requirements.
- **Payload is malformed.** Non-GUID identifiers, duplicates, or
  schema violations — the current implementation conflates all
  four reasons into one retry-forever behaviour.

## Open design question

What should the refbox do when `post_game_stats` fails with a 400
— particularly when the stats payload is empty?

### Option A — Skip the stats post when there is nothing to submit

Before calling `post_game_stats`, inspect the stats payload.
If it is empty (no goals, fouls, warnings, or other per-player
events recorded for this game), skip the network call entirely
and treat the score-submit alone as a success. `ItemResolved`
fires after `post_game_scores` returns 200. The green-checkmark
overlay behaves as spec'd.

- **Pro:** No wasted traffic. No 400-noise from the portal on
  games the operator did not instrument. Matches operator intent
  — if nothing was tracked, nothing needs posting.
- **Con:** If an operator intended to track events but their input
  was silently dropped (UI bug, button miss), the stats simply do
  not land on the portal and no indicator ever fires. Silent-loss
  mode in a different shape than ADR 011 was trying to fix.
- **Code surface:** ~10 lines in `refbox/src/portal_manager/mod.rs`'s
  stats-posting path or in `attempt_item` in
  `refbox/src/portal_manager/health.rs`.

### Option B — Tolerate a 400 from the stats endpoint and resolve the item anyway

If `post_game_scores` succeeds but `post_game_stats` returns a 400,
treat the item as resolved (green checkmark fires) and log a
warning. The queue no longer holds the item.

- **Pro:** No more infinite retry storm against a misconfigured
  dev/staging event. Operator sees a clean checkmark when at least
  the score landed.
- **Con:** Masks genuinely bad payloads (malformed data) as silent
  successes. Loses the "something is wrong" signal the feature was
  designed to surface. Harder to distinguish "operator didn't track
  events" from "we sent a broken payload" from "portal changed its
  validation".

### Option C — Distinguish the 400 reason and branch per case

Parse the 400 response body (the portal returns a JSON
`{ "Reason": ... }` envelope) and classify:

- `"This event does not require unique cap numbers"` → treat as
  "stats not wanted by this event"; resolve the item.
- `"Identifiers are generated automatically..."` → treat as
  "malformed payload"; escalate immediately (do not retry forever;
  put into a distinct error state that surfaces in the detail page).
- Any other 400 → today's behaviour (retry as Pending).

- **Pro:** Most expressive. Tells operator exactly what is wrong.
- **Con:** Requires refbox to parse portal-specific error reasons,
  creating a contract surface the `uwh-common` portal client has
  historically avoided (ADR 011 amendment 2026-04-21 explicitly
  chose to collapse all errors into one opaque variant rather than
  modify `uwh-common`). Reversing that decision has broader
  implications.

## What this ADR does *not* decide

This ADR captures the problem and the three reasonable options for
further discussion. The chosen option may be implemented after the
portal health indicator PR (feat/refbox/portal-health-indicator)
merges, on a dedicated `fix/refbox/stats-empty-skip` or
`refactor/refbox/stats-post-policy` branch — whichever matches the
option chosen.

The walkthrough blocker for Task 22 on 2026-04-23 is resolved
separately by enabling the cap-number requirement on the dev-portal
test event (a portal-admin configuration change, not a code change).
That unblocks the manual verification without requiring any of
A/B/C above to be picked first.

## References

- `docs/decisions/011-portal-health-indicator.md` — the retry
  queue and `ItemResolved` semantics that make this choice visible.
- `refbox/src/portal_manager/health.rs` — `attempt_item` contains
  the `post_scores` + `post_stats` sequence whose partial-failure
  mode this ADR addresses.
- `refbox/src/portal_manager/mod.rs` — `UwhPortalIo::post_stats`
  forwards to `uwh-common`'s `UwhPortalClient::post_game_stats`
  which builds the JSON body.
- `api/Controllers/AdminController.EventStats.cs:72-79` (in the
  `uwh-portal` repo) — the cap-number pre-check that surfaces the
  symptom.
