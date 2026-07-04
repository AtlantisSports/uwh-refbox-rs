# Unit 5 — Commit Ordering Anomaly Note

## The Apparent Problem

The task brief noted that commit `d72d643` (author-dated 2026-04-12) calls
`get_event_referee_name_map_from_referees`, but commit `353b476` (author-dated 2026-04-18) is
the one that *adds* that method to `UwhPortalClient` in `uwh-common`. At first glance, the
April 12 commit appears to use a function that doesn't exist until April 18 — a 6-day gap that
would mean the code couldn't compile at the April 12 point.

## What the Git Evidence Shows

Examining the actual topological (parent-child) order on master:

```
353b476  (author: 2026-04-18) — adds get_event_referee_name_map_from_referees
8d4a667  (author: 2026-04-18) — display team or individual referee assignments
996874a  (author: 2026-04-12) — correct referee role strings         ← TOPOLOGICALLY AFTER 8d4a667
d72d643  (author: 2026-04-12) — display real referee names           ← TOPOLOGICALLY AFTER 996874a
1bd4676  (author: 2026-04-18) — show "Unknown" for no display name
931d01d  (author: 2026-04-18) — display referee names on game-info page
```

Confirming via `git show 353b476^:uwh-common/src/uwhportal/mod.rs | grep "fn "`:
- `get_event_referee_name_map_from_referees` is ABSENT from 353b476's parent.
- It IS PRESENT in `996874a` (the commit with the April 12 author date).

This means `353b476` — despite carrying an April 18 author date — is the topological ancestor
of `996874a` (April 12 author date). The author dates are inverted relative to the commit graph.

## Characterization: Off-Master Development

This is the **Off-master development** pattern. The six commits were developed together on a
feature branch. The April 12 commits (`996874a`, `d72d643`) were written first (earlier author
date), then the April 18 commits were added on top. When the whole branch was rebased or
cherry-picked onto master, the commits landed in an order that preserved author dates but
inverted their relationship to the function they depend on: the function-definition commit
(`353b476`) ended up topologically before the commits that use it, which is correct for
compilation — the author dates just happen to be newer.

There is no stub-and-replace. There is no compilation window where the code was broken on
master. The order on master is compile-correct; the confusion arises entirely from author dates
being older than the commits that precede them topologically.

## Implication for B5.15 Catalog Entry (Task 3)

The catalog entry for B5.15 (referee name fetch in `request_schedule`) should note that
`get_event_referee_name_map_from_referees` was introduced in `353b476` and first used in
`d72d643`; both land on master in correct dependency order despite misleading author dates.
No ordering-related slop finding is warranted here — the anomaly is a process observation only.
