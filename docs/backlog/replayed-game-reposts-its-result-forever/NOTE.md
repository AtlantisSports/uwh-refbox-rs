# Backlog: a replayed game re-posts its result every 16 seconds, forever

**Status:** NOT FILED, not started. Local note only.
**Surfaced:** 2026-09-05, during the court-finished walkthrough, on branch
`fix/uwh-common/court-finished-behaviour`. Reproduced live against a local mock portal.
**Raised by:** Claude, from the app's own log and the mock's access log, after Eric noticed the
portal indicator had turned yellow.
**Almost certainly pre-existing — that branch does not touch the queue code.** The only lines in its
diff matching `score_sent`/`portal_queue`/`attempts` are documentation.

## The symptom

A game played a **second** time posts its result, the portal accepts it, the app logs
`portal post game scores successful` — and then posts the identical result again 16 seconds later,
indefinitely. It was still going after 15 posts when the app was stopped.

Games played once are unaffected: games 1 and 2 posted exactly once each, and game 3's **first** play
also posted once and resolved cleanly. Only the replay loops.

```
04:59:08  POST games/3/scores      <- game 3, first play. Resolved; no repeat.
05:25:21  POST games/3/scores      <- game 3, replayed. From here, every ~16s:
05:25:37  05:25:53  05:26:09  05:26:25  05:26:41  05:26:57  05:27:13 ... 05:28:32 ...
```

Both the score and the stats post report success each time:

```
[23:25:21] INFO uwh_common::uwhportal] portal post game scores successful
[23:25:21] INFO uwh_common::uwhportal] portal post game stats successful
[23:25:37] INFO uwh_common::uwhportal] portal post game scores successful
   ... repeating ...
```

## The queue entry never changes

`$XDG_CONFIG_HOME/refbox/portal_queue.json`, unchanged across all 15 attempts:

```json
{"version":1,"items":[{"event_id":"events/1-A","game_number":"3","black_score":0,"white_score":1,
 "stats":"[]","queued_at":"2026-09-05T05:25:19.688Z","attempts":0,"last_attempt_at":null,
 "force":false,"score_sent":false}]}
```

Note `attempts: 0` and `last_attempt_at: null`. Not merely "unsent" — **no bookkeeping happened at
all**, as though the main thread never learned an attempt was made. The retry gate at
`portal_manager/health.rs:174` is `if item.score_sent || !is_item_retry_eligible(item, now)`, so an
item that is never marked and never ages is eligible forever.

The main thread learns of outcomes through `PortalEvent::ItemResolved` / `ScoreSentStatsPending` /
`ItemAttempted` (handled in `app/mod.rs`, around the `Message::PortalEvent` arm). For this item none
of the three appears to arrive, or none matches the item.

## What was NOT established

**Whether a real portal reproduces it.** The mock returns `200 {}` to everything and never refuses.
A real portal would very likely reject a second result for a game that already has one, which is a
different response and possibly a different code path. This may be a genuine queue-bookkeeping bug,
or the app meeting an unexpected success where it expects a refusal.

Also not done: reproducing on `master`, and tracing which of the three `PortalEvent`s should fire.

## Why it matters

Replaying a game is a legitimate referee action, and one Eric's own ruling contemplates: a game that
has to be played later is played by going back to the original game and starting it there. If that
leaves the box posting a duplicate result to the live portal every 16 seconds for the rest of the
day, the portal sees a flood of writes against a real game.

The operator's only signal is the connection indicator turning **yellow**. Nothing says why.

## Scope when picked up

`refbox` only — `portal_manager/{queue,health}.rs` and the `PortalEvent` handling in `app/mod.rs`.
Needs a portal that refuses duplicates, or a mock taught to. Its own branch and its own walkthrough.

Related: `docs/backlog/event-list-never-refetched-after-offline-start/NOTE.md` — the other
pre-existing portal-resilience gap found the same night.
