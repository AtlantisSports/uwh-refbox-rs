# Backlog: a refbox that starts offline never recovers its event list, and REFRESH goes dead

**Status:** NOT FILED, not started. Local note only.
**Surfaced:** 2026-09-04, during the court-finished walkthrough (criteria 4 and 5), on branch
`fix/uwh-common/court-finished-behaviour`. Reproduced live against a local mock portal.
**Raised by:** Claude, from the running app's own log. **Pre-existing on master — not caused by
that branch.**

## The gap

If the refbox starts while the portal is unreachable, it never gets the event list. When the portal
comes back, the token revalidates and the connection dot turns **green** — but the event list is
still missing, and from that point every schedule the app receives is discarded. REFRESH fetches
the schedule, the portal serves it, and the refbox throws it away. The operator sees a healthy-
looking box whose REFRESH button does nothing, with no on-screen explanation.

Observed, in order, in one run:

```
21:25:44  Failed to get event list: error sending request for url (.../api/events?...)
21:25:46  portal health check could not reach the portal            (repeats every ~16s)
21:27:22  portal token validation successful                        <- portal is back, dot goes green
21:30:09  Got schedule
21:30:09  Received schedule for event_id events/1-A, but there is no event list yet
21:30:18  Got schedule
21:30:18  Received schedule for event_id events/1-A, but there is no event list yet
```

The mock portal's own access log confirms it served both `GET /api/events/1-A/schedule` requests, so
REFRESH is working end to end; the discard is on the refbox side.

Restarting the app while the portal is reachable clears it completely — the event list loads and
REFRESH behaves normally. That was how the walkthrough proceeded.

## Where it is in the code

Described by **shape** rather than line number: this note has already been invalidated once by a
rebase, and the surrounding code moves often.

Both the schedule reply (`Message::RecvSchedule`) and the teams reply (`Message::RecvTeamsList`) in
`refbox/src/app/mod.rs` do their work inside

```rust
if let Some(event) = self.events.get_mut(source, &event_id) {
    // ... store the schedule / teams on the event ...
} else if source == GameSource::Portal && !self.events.portal_list_loaded() {
    error!("Received ... but there is no event list yet", ...);
} else {
    error!("Received ... it is not in the event list", ...);
}
```

With no event list, `get_mut` misses, and **both** remaining arms only choose which error message to
log. `portal_list_loaded()` is not a guard that changes behaviour — the reply is dropped either way,
with no retry and nothing on screen. (At the time of writing: the teams guard near line 6459, the
schedule guard near 6683. Treat those as hints, not anchors.)

Both came in with `51c7a61e` *"Switch to fully using uwhportal"*, which is on `origin/master`.

**Re-verified 2026-09-05** against master at `1229c396`: still present, still unfixed, and master's
roster-before-kickoff work did not touch it.

## Why it matters

A flaky or slow network while the equipment is being set up poolside is an ordinary tournament
situation, not an edge case. The failure is silent and self-perpetuating: the one control an
operator would reach for — REFRESH — is exactly the one that cannot work, and nothing on screen says
so. The recovery (restart the app) is not discoverable.

## The ask

A refbox whose event list never loaded should either fetch it when the portal becomes reachable
again, or say plainly that it cannot act on the schedule until it is restarted. What it must not do
is keep accepting schedules and discarding them while looking connected.

## Scope when picked up

`refbox` only — the portal reply handling in `refbox/src/app/mod.rs` and whatever owns event-list
fetching. No `uwh-common` change is implied. Deserves its own branch and its own walkthrough, since
reproducing it means starting the app with the portal down.
