# Re-ending a game stacks a duplicate queue entry — and the one-line fix is unsafe

**Found:** 2026-08-13, during the degraded-mode no-write-target redesign (PR #2367).
**Status:** attempted, reverted, needs its own design. Do NOT re-apply the one-line version.

## The original defect

`PortalManager::enqueue_game_end` appends unconditionally, so ending the same game twice queues two
entries for it. Two consequences:

- A session that cannot send never drains its queue, so the copies accumulate unbounded.
- `discard` and `on_item_resolved` remove by id with `retain(|it| it.id != id)`, which drops **every**
  match at once — so resolving one copy silently discards the other, unsent.

## Why the obvious fix was reverted

The obvious fix — replace the existing entry instead of appending — was implemented, tested green,
and reverted on review (commit `1239df99`, reverted by `4b0212e3`). It introduces three new failure
modes, all in the **healthy** path that every tournament uses, where the original defect only bites
the degraded path that has never been observed in the field. It also does not fix the loss it claims
to fix.

Any real design must answer all three.

### 1. It does not fix the lossy removal — only changes which copy is lost

The background task iterates its **own clone** of the queue (`health.rs:144`, `151`). So:

1. Task begins posting G1 with score 5-4 from its snapshot.
2. Operator re-ends G1 with a corrected 6-4; the entry is replaced in place.
3. The 5-4 post succeeds; `ItemResolved(G1)` arrives.
4. `on_item_resolved` runs `retain(|it| it.id != id)` (`mod.rs:1071`) and removes the **6-4** entry.
5. A green RecentSuccess row appears for G1.

The corrected score is discarded and the operator is shown a success. Resolution is keyed on id
alone, so it cannot tell which payload was actually sent. A fix likely needs the resolve to carry
what it sent (a generation counter, or the scores) so it only clears the entry it posted.

### 2. Replacement resets `score_sent`, so an already-accepted score is re-posted

`score_sent = true` records that the portal already accepted the game's **score** and only the stats
upload is outstanding. It gates three things: the auto-retry loop (`health.rs:153`), stuck/red
classification (`mod.rs:380`), and `is_stats_pending` (`mod.rs:776`).

Replacing the whole entry sets it back to `false`. So re-ending a stats-pending game re-enters the
auto-retry loop and re-posts a score the portal already holds. That is a conflict, and per ADR 011 a
conflict is indistinguishable from any other failure — so the row goes stuck/red permanently and
needs a manual Force. Meanwhile `is_stats_pending` now returns false, so the outstanding stats
upload stops being offered a retry and is untracked.

Note the old duplicate-stacking behaviour was *less* harmful here: `find`/`find_mut` return the first
match, so the stats-pending entry survived.

### 3. A re-ended game that already succeeded can never leave the queue

`on_item_resolved` opens with an idempotence guard (`mod.rs:1052`):

```rust
if self.recent_successes.iter().any(|rs| rs.id == id) {
    return;
}
```

Once G1 has succeeded, its id sits in the recent-successes ring (cap 5). Re-ending G1 queues it
again, but every subsequent resolve for it returns early before the `retain`. The item stays queued,
is re-posted every retry interval, crosses the 30-minute stuck threshold and paints the indicator
red, and only clears once five other games push G1 out of the ring — or at the 120-hour expiry.

This guard exists for a real reason (duplicate delivery would otherwise paint two green rows), so it
cannot simply be removed.

## Also worth folding in

- `self.queue.items.iter_mut().find(|it| it.id == item.id)` re-implements the existing `find_mut`
  helper (`mod.rs:617`). Use the helper.
- Whatever the design, it needs a test for re-ending a **stats-pending** game and one for re-ending
  an **already-resolved** game. Note that the naive "nothing is left behind after a discard" test
  passes on the *unfixed* code — `retain` removes both copies — so it proves nothing.

## Related

- Spec: `docs/superpowers/specs/2026-08-13-degraded-no-write-target-design.md` (records the dedupe as
  in scope; that judgement was reversed on review)
- Plan Deviations: `docs/superpowers/plans/2026-08-13-degraded-no-write-target.md`
- `docs/backlog/untried-result-labelled-as-send-error/NOTE.md` — also about `is_item_stuck` keying on
  age alone
