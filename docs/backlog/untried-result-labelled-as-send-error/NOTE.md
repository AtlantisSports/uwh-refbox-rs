# Backlog: a result that was never attempted is labelled "Score send error"

**Status:** NOT FILED, not started. Local note only.
**Surfaced:** 2026-08-13, while designing
`docs/superpowers/specs/2026-08-13-degraded-queue-persistence-design.md`.
**Eric's decision, 2026-08-13: keep this separate from that branch** — "stuck" is shared by every
queued game on every path, so changing what it means is a different and riskier change deserving its
own design and tests. One concern per branch.

## The gap

Whether a queued result is "stuck" is decided **purely by age**:

```rust
!item.score_sent && (now - item.queued_at) >= STUCK_THRESHOLD   // 30 minutes
```

It never asks whether the result was ever actually *attempted*. A stuck item is then excluded from
auto-retry — *"Stuck items wait for operator action"* (`refbox/src/portal_manager/health.rs:31-39`) —
and is rendered as **"Game { $game } Score send error, tap to fix"**
(`portal-row-stuck`).

For a game that failed to send several times, that is right: the portal is rejecting something and
the operator must decide (Retry / Force / Discard).

For a game that has **zero attempts** it is wrong twice over:

1. It has not errored. Nothing was ever sent, so calling it a "send error" misreports the cause.
2. It is excluded from auto-retry as though a decision were needed, when in fact nobody has tried
   once.

The clearest way to reach this is the degraded-mode path: the portal subsystem never started, so no
attempt is possible, and any outage lasting over 30 minutes leaves every game from it looking like a
send error on the next healthy start.

## Why it matters

It is the difference between results resuming on their own and results waiting on the operator to
notice red rows and press RETRY ALL. The 2026-08-13 queue-persistence branch makes those results
*recoverable in one tap*; closing this gap would make them recover with **no** operator action.

## The ask

Distinguish "never attempted" from "tried and failing". A first cut: treat an item with
`attempts == 0` (equivalently `last_attempt_at == None`) as not stuck, so it stays in the auto-retry
pool and is shown as pending rather than as an error.

## Scope when picked up — and the risk

- `refbox/src/portal_manager/mod.rs` (`is_item_stuck`) and `health.rs`
  (`is_item_retry_eligible`).
- **`is_item_stuck` is a `pub fn` used by more than the detail page** — it feeds
  `needs_attention()`, and therefore the red indicator, and `is_stuck()` routes row taps to the
  attention-action page. Enumerate every caller before changing it; this is not a display-only
  predicate.
- Consider the escalation this protects: an item that genuinely cannot be delivered must still reach
  red eventually rather than retrying forever. Exempting `attempts == 0` should delay escalation, not
  remove it — an item that stays unattempted forever (which is exactly the degraded case) must not
  become invisible.
- Likely needs a translation change too if a new row wording is introduced (15 locales, and health
  wording must stay source-neutral — see the source-neutral section of
  `docs/superpowers/specs/2026-08-13-degraded-portal-startup-message-design.md`).

## Explicitly NOT part of this

- Not the queue-persistence fix (2026-08-13 spec above) — that ships first and independently.
- Not the 30-minute threshold value itself, which ADR 011 sets deliberately.
