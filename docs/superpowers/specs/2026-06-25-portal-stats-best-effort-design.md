# Design: Decouple portal score & stats uploads (stats best-effort)

**Date:** 2026-06-25
**Status:** Approved in brainstorming; pending spec review → writing-plans
**Crate scope:** `refbox` only (no `uwh-common`, no wire-format changes)
**Process:** HEAVY (changes the portal retry-queue state machine) — own branch, per-task tests.

---

## Problem (confirmed root cause)

When a game ends, the refbox uploads two things to the UWH Portal, separately: the **score**
(`POST /api/events/<ev>/schedule/games/<n>/scores`) and the **stats**
(`POST /api/admin/events/stats?...`). Today, `attempt_item` in
`refbox/src/portal_manager/health.rs` only marks a game **resolved** when **both** succeed.

For an event the portal considers as **not requiring unique cap numbers**, the portal **rejects
every stats upload** with `400 {"reason":"This event does not require unique cap numbers."}` —
confirmed live against dev event `1825-C` (the score posts `200`, the stats `400`, even with a
valid cap number in the payload, so it is purely the portal's per-event decision). Because the
refbox requires stats to succeed, such a game **never resolves**: it sits yellow, escalates to
red after 30 min, and **re-posts the score endlessly**. The web refbox treats stats as optional
and completes the game on the score alone; the Rust refbox does not.

(Background: surfaced while testing RETRY ALL, PR #1424. The score-only games we saw came from
using **Edit Scores** with the refbox's "track player cap number" off, but that was a red
herring — the portal rejects stats for this event regardless of payload content.)

## Goal

Make the **score** the thing that drives a game's queue status. A **stats** failure must not
block the game or alarm the operator, but the stats must remain re-sendable.

## Non-goals

- The score-upload path and its existing behavior (yellow → red "stuck" at 30 min, background
  auto-retry) — unchanged.
- The RETRY ALL button / feature (PR #1424) beyond having it also sweep stats-pending games.
- How the refbox records stats or the "track player cap number" setting.
- The portal's event configuration. `uwh-common` (the portal client). No wire-format change.

---

## Approved behavior

Three outcomes when a finished game is processed from the queue:

1. **Score upload fails** → unchanged from today:
   - Row: **"Game N — Score not sent, tap to retry"** (yellow); after 30 min → red "stuck".
   - Portal **dot** goes yellow/red; background **auto-retries** on cadence.

2. **Score succeeds, stats fail** → new behavior:
   - Game is treated as essentially sent. Portal **dot stays green** (no alarm).
   - It **stays in the status list** as its own row: **"Game N — Stats not sent, tap to retry"**
     — same **yellow** "tap to retry" styling as score rows (only the wording differs).
   - **Not** auto-retried in the background; **never** escalates to red. It just waits.
   - Re-sent only when the operator **taps that row** or taps **RETRY ALL** (a one-shot stats
     attempt; if it fails again it returns to waiting).

3. **Both succeed** → fully resolved; drops into the green "recently sent" area (as today).

Plain terms: a missing **score** nags (yellow/red dot, auto-retry); a missing **stat** sits
quietly as a tappable "Stats not sent" line (dot green, nothing loops in the background).

---

## Architecture sketch (refbox only)

### 1. `portal_manager/queue.rs` — per-item state
- Add `#[serde(default)] score_sent: bool` to `QueuedItem` (default `false` → old
  `portal_queue.json` files load as score-not-sent; **no `QueueFile` version bump needed**).
- An item is now in one of: **score-pending** (`score_sent == false`) or **stats-pending**
  (`score_sent == true`, still in the queue because stats haven't succeeded).

### 2. `portal_manager/health.rs` — `attempt_item` + auto-retry gating
- `attempt_item`:
  - If `!score_sent`: post score. On `Err` → `ItemUpdated`, return (score-pending, as today).
    On `Ok` → post stats; on `Ok` → `ItemResolved`; on `Err` → emit a **new event**
    `PortalEvent::ScoreSentStatsPending(id)` (score is up; mark the item stats-pending).
  - If `score_sent` (stats-pending, reached only via a manual one-shot request): post stats only;
    on `Ok` → `ItemResolved`; on `Err` → `ItemUpdated` (stays stats-pending, no escalation).
- **Auto-retry gating:** the cadence loop only attempts `score_sent == false` items. Stats-pending
  items are **not** auto-retried (Option 2). They are attempted only when a manual one-shot retry
  is requested (see §4).
- `is_item_stuck` / the 30-min escalation applies **only** to `score_sent == false` items.

### 3. `portal_manager/mod.rs` — state transition + indicator + rows
- Handle `ScoreSentStatsPending(id)`: set `item.score_sent = true`, persist, recompute indicator.
- **Indicator dot:** stays **green** when the only outstanding items are stats-pending. Yellow/red
  is driven solely by score-pending items (and the existing token-problem path). `needs_attention`
  / `recompute_indicator` must ignore stats-pending items.
- **`detail_rows`:** emit a new row variant for stats-pending items
  (`DetailRow::StatsPending { id, game_number, attempts }`) ordered after stuck/score-pending and
  before recent successes. Score-pending rows unchanged.
- `retry_all` (PR #1424): already iterates all `queue.items`; ensure it also requests a one-shot
  stats attempt for stats-pending items (i.e., its reset makes stats-pending items eligible for a
  single attempt). **RETRY ALL enabled-state must be widened:** today `has_unsent` in
  `portal_detail.rs` is `rows.any(Stuck | Pending)`; add the new `StatsPending` variant so the
  button stays active while stats are pending — matching "available for the retry all still."

### 4. Manual one-shot stats retry
- Tapping a stats-pending row (`PortalRowTapped` for a `score_sent` item) and `retry_all` must
  trigger **one** stats attempt without enabling cadence auto-retry. Implementation options for
  the plan to choose: a transient `retry_requested` flag on the item that the loop consumes, or a
  dedicated one-shot path. (The plan resolves the exact mechanism; the requirement is: manual →
  exactly one attempt, no background loop.)

### 5. UI rows + translations
- `view_builders/portal_detail.rs`: render `DetailRow::StatsPending` as a yellow `tap-to-retry`
  row using a new key, e.g. `portal-row-stats-pending = Game { $game } Stats not sent, tap to retry`
  (mirror `portal-row-pending`). Reuse `portal-row-attempt-suffix`.
- Add `portal-row-stats-pending` to **all 15 locales** (best-guess; native review later) — no
  English placeholders.

---

## Acceptance criteria (operator-observable)

1. Finish a game on an event that doesn't track stats (score posts, stats `400`): the portal
   **dot stays green**, and the status list shows **"Game N — Stats not sent, tap to retry"** (it
   does **not** go yellow/red, does **not** escalate to red after 30 min, and the background does
   **not** keep hammering).
2. Tapping that row, or **RETRY ALL**, makes **one** fresh stats attempt; on this event it fails
   again and the row stays "Stats not sent" (no loop). On an event that does accept stats, the
   attempt succeeds and the game drops to "recently sent".
3. A **score** failure still behaves exactly as today (yellow → red at 30 min, dot reflects it,
   background auto-retries).
4. Restart with a stats-pending game in `portal_queue.json`: it reloads as stats-pending (green
   dot, "Stats not sent" row), retriable.

## Testing

- `attempt_item`: score `Ok` + stats `Err` → emits `ScoreSentStatsPending`, item not resolved.
- `attempt_item`: stats-pending item (`score_sent==true`) + stats `Ok` → `ItemResolved`.
- Auto-retry gating: a `score_sent==true` item is **not** picked up by the cadence loop, but a
  `score_sent==false` item is.
- Indicator: a queue containing only a stats-pending item → dot **green** (not yellow/red), and
  `is_item_stuck` never fires for it regardless of age.
- `detail_rows`: a stats-pending item produces a `StatsPending` row in the right order.
- `score_sent` round-trips through `queue.json` (serde default keeps old files loading).
- `just check` green; new translation key present in all 15 locales.

## Open implementation detail (for the plan)

The exact one-shot-stats-retry mechanism (§4) — transient `retry_requested` flag vs a dedicated
path — is left to the implementation plan. Everything else is specified above.
