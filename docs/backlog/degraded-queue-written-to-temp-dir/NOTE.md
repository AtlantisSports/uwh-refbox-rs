# Backlog: results queued in degraded mode are written somewhere nothing reads

**Status:** NOT FILED, not started. Local note only.
**Surfaced:** 2026-08-13, by the code review of
`fix/refbox/degraded-portal-startup-message`. **Pre-existing — identical before and after that
branch**, and squarely inside that spec's stated non-goals, so it was deliberately left alone.
**Verified by reading the call chain, not assumed.**

## The gap

A degraded-mode `PortalManager` sets `config_dir` to `std::env::temp_dir()`
(`refbox/src/portal_manager/mod.rs`, in `new_degraded`). Game results are still queued in that mode:
`enqueue_game_end` is called at game end whenever an event is linked, with **no check for a portal
client** (`refbox/src/app/mod.rs:1389-1392`) — and a degraded session can absolutely have an event
linked, either restored from a portal link note or adopted from a custom site's URL.

So the queue file is written to the system temp directory. The next healthy launch constructs the
manager with the **real** config dir and loads the queue from there, so it never sees that file. The
queued results are effectively abandoned.

Meanwhile the end-of-game banner tells the operator the opposite:

> *"Connection issue detected. Score will still be queued — find an admin to resolve."*

In degraded mode the first half of that promise is technically true and practically false: it is
queued, to a location nothing will ever read.

## Why this is not a lost game, but is still a real problem

The score itself is not gone from the machine — the tournament manager still holds the game record
and the scoresheet is unaffected. What is lost is the **portal submission**, silently, after the
operator was told it was safely queued. Nobody learns that the result needs entering by hand.

Note that the new startup-failure row added by the 2026-08-13 branch —
*"Connection unavailable — results will not upload"* — is now the **more accurate** of the two
messages on this path. The game-end banner is the one that overpromises.

## The ask

Either make the promise true, or stop making it. Options to weigh with the human:

1. Keep degraded mode's queue in memory only and say so, rather than writing a file that is never
   read again.
2. Have a healthy startup also check the temp dir and adopt any queue file it finds there.
3. Change the game-end banner wording so it does not promise queueing when the subsystem never
   started.

Option 3 is the smallest; option 2 is the only one that actually preserves the results.

## Scope when picked up

- `refbox/src/portal_manager/mod.rs` (`new_degraded`'s `config_dir`, and `queue::load_or_empty`).
- Possibly `refbox/translations/` if the banner wording changes (15 locales, and the wording must
  stay source-neutral — see the 2026-08-13 spec's "wording must be source-neutral" section).
- **Check first** whether a queue file left in the temp dir by an earlier degraded run could be
  picked up by an *unrelated* refbox install or a different user account on a shared machine. Not
  traced — verify before choosing option 2.

## Explicitly NOT part of this

- Not the degraded-startup message (shipped on `fix/refbox/degraded-portal-startup-message`).
- Not the silent portal login with no client — that is
  `docs/backlog/portal-login-silent-when-no-client/NOTE.md`.
