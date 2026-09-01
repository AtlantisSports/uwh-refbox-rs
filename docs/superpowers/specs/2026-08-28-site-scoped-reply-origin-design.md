# Design — every site-scoped reply carries its origin

**Status:** approved in principle by Eric 2026-08-28. Not built. Own branch.
**Suggested branch:** `fix/refbox/site-scoped-reply-origin`
**Blocked on:** `feat/refbox/source-switch-confirmation` landing (it is the branch that makes the
fault reachable, and it touches every file this one would).

---

## The problem, in one paragraph

A request knows which site it is going to — the URL is built from the live client. **The reply does
not.** By the time an answer is a `Message` inside the app, the URL has been discarded and only an
`EventId` (or `TeamId`) remains. Each handler then infers the site by asking `self.source` — *which
source am I on right now* — and since the source buttons commit **and repoint at the tap**, that is
no longer necessarily the source that issued the request. A reply from the site the refbox has left
is therefore attributed to the site it has arrived at.

Two things previously kept this harmless, and both are gone:

1. **`EventStore` is source-separated**, so a lookup with the wrong bucket used to miss. It only
   misses while the two sites use *different* event numbering. **Eric confirmed 2026-08-28 that
   custom sites will be required to use portal-style numbering, with the same event id after
   `/api`** — so the ids match by specification and the lookup now hits.
2. **The client only moved on APPLY**, which is the premise stated verbatim in the comments on
   `RecvSchedule` and `RecvTeamsList`. `feat/refbox/source-switch-confirmation` retired it. Those
   two comments now record the gap rather than assert the premise (commit `1f4bdc62`).

## Scope

**In:** the five site-scoped reply messages and the code that issues them.

| Message | Carries today | Guard today |
|---|---|---|
| `RecvEventList(Vec<Event>)` | nothing | none — `set_portal_list` installs unconditionally |
| `RecvTeamsList(EventId, TeamList)` | event id | bucket from arrival-time `self.source` |
| `RecvSchedule(EventId, Schedule)` | event id | bucket from arrival-time `self.source` |
| `RecvTokenValid(EventId, bool)` | event id | `settings.current_event_id == event_id` only |
| `RecvTeamRoster(GameSource, TeamId, Vec<u8>)` | **source** | source equality (`bbe3e3bb`) |

**Out of scope, named so it is not forgotten:**

- **The persisted results queue.** `ItemId` is `{ event_id, game_number }` — **no site at all**
  (`portal_manager/mod.rs:306`). With mandated identical numbering, a queued result for portal event
  `1889-B` is indistinguishable from a custom site's `1889-B`. Same root cause, higher stakes (a
  game result posted to the wrong server), and it needs a `portal_queue.json` format migration plus
  a decision about items already on disk. Eric has it on the backlog as the queued-results item.
  **Do not fold it in here.**
- Anything about *which* site is correct to talk to. This design only ensures an answer is
  attributed to the site that gave it.

## The mechanism

One `site_generation: u64` on `RefBoxApp`, starting at 0.

- **Bumped in `repoint_client`, and only where it assigns `self.current_site`.** The early return
  when `build_site_client` fails must NOT bump — that path leaves the client where it was, so
  replies in flight are still valid. (This is the same asymmetry `1f4bdc62` fixed for the portal
  fetch; keep the two consistent.)
- Every request-issuing function reads it once, at the moment it issues, and moves the value into
  the reply.
- Every handler compares on arrival: equal → accept; different → `warn!` naming the team/event and
  both generations, then drop.

`RecvTeamRoster`'s `GameSource` tag **collapses into this** — delete it and use the stamp, so there
is one mechanism rather than two. That also closes the custom-site-to-custom-site case the source
tag structurally cannot reach, which is the whole reason for preferring a stamp.

### Why a counter and not the site address

A `String` address would let `portal → custom → portal` accept a reply issued to the first portal
visit, because the address matches again. That is *correct data*, so the address is strictly more
precise. A counter rejects it.

**Recommend the counter anyway:** it is one `u64` per message rather than a heap string on a path
that fires dozens of messages per event; the rejected case is a wasted fetch, not a wrong answer,
and a fresh fetch always follows a switch; and erring toward dropping is the right default for a
guard whose whole purpose is to refuse data of uncertain origin. Record the trade-off in the code so
the next reader does not mistake it for an oversight.

## Acceptance criteria

Observable, and each one fails before the change:

1. Switch source while a schedule fetch is in flight against a site using the same event numbering.
   The departed site's schedule must **not** appear in the new site's court or game pickers. Today
   it does.
2. Same for the team list.
3. With a portal token check in flight, switch to a custom site **that has no saved access key**.
   The ACCESS TOKEN row must **not** read "Connected". Today the portal's late success paints it
   green and un-greys COURT and GAME. (Note for whoever writes this up: the green is a *real*
   verification against the portal, not a fabricated one — the fault is attribution, not invention.
   Getting that wrong in a report cost two rounds of Eric's time on 2026-08-28.)
4. Switch from one custom site to a **different** custom site with a colliding team id, while
   roster fetches are in flight. The departed site's cap numbers must not seed the player-number
   grid. Today the source tag cannot catch this, because the source does not change.
5. A repoint that fails (`build_site_client` returns `None`) must **not** invalidate replies in
   flight — the client did not move.

## Testing

`RefBoxApp` has **no test harness** in this crate — every `#[cfg(test)]` module in `app/mod.rs`
tests free functions, and the app is only ever built by its real startup path. So the comparison
must be extracted as a **free function** to be testable, the way `source_tap_outcome` was:

```rust
fn reply_is_current(issued_at: u64, now: u64) -> bool
```

Trivial, but it is the seam that makes the rule assertable at all, and it keeps the four handlers
from drifting apart. The five acceptance criteria above are then walkthrough items, which needs two
sites configured with identical event numbering — see
[[reference_local_mock_portal_recipe]] and [[reference_overlay_test_server_mock_portal]].

## Risks

- **Four handlers, one rule.** The failure mode is fixing three. Extracting the comparison as one
  free function called from all four is the mitigation; a reviewer should grep for
  `Recv.*(EventId` and confirm no site-scoped reply is left unstamped.
- **Dropping too much.** If the stamp is bumped somewhere it should not be (e.g. on a failed
  repoint), pickers go empty and it will look like a network fault. Criterion 5 exists for this.
- **`RecvEventList` carries no id at all**, so it has nothing to fall back on — it is the one that
  is currently wholly unguarded and the one most likely to be missed.

---

## Follow-up — a sixth site-scoped reply the scope table missed

`RecvPortalToken`, a site's answer to a login attempt, is site-scoped and is not in the table
above, so it went unstamped when this design was built. It is the only reply in the group that
carries a **credential**: `request_uwhportal_token` issues against the live client, and the handler
both installed the returned key on that client and filed it by arrival-time `current_site.kind`. A
source switch during a login therefore handed the Portal's access key to a third-party server, and
the reverse direction overwrote the operator's real Portal login.

Found by the whole-branch review of `fix/refbox/site-scoped-reply-origin` on 2026-08-31 — read from
the code, never reproduced at the time — and fixed on its own branch,
`fix/refbox/portal-token-cross-site-leak`, with the same generation stamp. A late key is discarded
silently: a log line and nothing on screen, so the cost of switching mid-login is one re-login
rather than a credential on the wrong server.

With that stamped, the claim holds for the *replies to requests the refbox issues*: every one of
them carries its origin except `RecvEventList`, which is deliberately unstamped because it does not
use the live client at all. The reason is recorded on `request_event_list`.

It does **not** hold for the whole message group, and an audit reading the paragraph above as if it
did would stop too early. `Message::PortalEvent` still delivers `PortalEvent::TokenStatus` and
`PortalEvent::TokenUnreachable` unstamped — site-scoped token verdicts travelling as messages, not
merely as `portal_manager` state.

Still open, same family: the background health probe re-checks the token on a cadence through the
shared client and applies the verdict to `portal_manager` state, which carries no site
(`refbox/src/portal_manager/health.rs`).
