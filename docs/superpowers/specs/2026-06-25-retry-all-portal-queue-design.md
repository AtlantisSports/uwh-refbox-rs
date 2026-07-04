# Design: "RETRY ALL" for the portal retry queue

**Date:** 2026-06-25
**Status:** Approved in brainstorming, pending spec review
**Crate scope:** `refbox` only (no `uwh-common`, no wire-format changes)

---

## Goal

Give the operator a single **RETRY ALL** button on the portal status screen that
re-sends every game still waiting to reach the portal, so they no longer have to
tap each stuck game and Force it one-by-one after a wifi outage.

## Problem (why this is needed)

The refbox queues every finished game's result to disk (`portal_queue.json`) and a
background task auto-retries each one (wakes every 2 s; ≥15 s between attempts per
game). But a game that has been waiting **more than 30 minutes** (`STUCK_THRESHOLD`)
is marked **stuck** and is deliberately removed from auto-retry
(`is_item_retry_eligible` returns `false`). Stuck games then require manual action:
tap the row → attention page → **FORCE THIS GAME RESULT**.

After a multi-hour wifi outage spanning many games, the oldest games are all stuck,
so the operator must individually tap-and-force each one. RETRY ALL collapses that
into one tap.

## Non-goals (explicitly out of scope)

- Changing the automatic background-retry cadence or the 30-minute stuck threshold.
- A bulk "Discard All".
- Changing the token-expired re-login flow, or the per-game Force/Discard flow.
- Any change to `uwh-common`, the wire format, or the game state machine.

---

## Decisions (settled in brainstorming)

| Question | Decision |
|----------|----------|
| What it does to each game | **Safe resend** — re-attempt normally, **never** force-overwrite. |
| Which games it acts on | **All unsent games** — both stuck (red) and pending (yellow). |
| How games get sent | **Hand to the existing background sender** — the button only resets each game's timers; the proven retry loop does the actual sending over the next few seconds. |
| Placement | **Footer, far right**, opposite BACK (reuses the currently-blank space). |
| Color | **Blue** (`blue_button`) — a safe "action", not an alarm. |
| Visibility | **Always shown** in the footer; **grayed/disabled** when there are no unsent games. |
| Confirmation | **None** — it is non-destructive (cannot overwrite or delete). |
| Label | **"RETRY ALL"**, translated into all 15 locales. |

### Why "safe resend" and not "force"

The portal API collapses all failure causes (409 conflict, 401 token, 5xx, network)
into one opaque error (ADR 011), so the refbox cannot tell a game that *never got
through* (wifi) from one the portal is *rejecting because a result already exists*
(conflict). Forcing the whole batch could silently overwrite a real conflicting
result. Safe resend fixes the wifi case completely; any genuine conflict simply
re-fails and re-surfaces as stuck for the operator to Force or Discard individually.

---

## Behavior

When the operator taps **RETRY ALL**:

1. For **every** item in the queue (stuck and pending alike), reset its retry timers:
   - `queued_at = now` — this clears the "stuck" status, returning it to the
     auto-retry pool.
   - `last_attempt_at = None` — this makes it immediately eligible (no 15 s wait).
   - `force` is **left unchanged** (normally `false`; only ever `true` if the
     operator previously chose Force on that specific game — RETRY ALL does not
     introduce forcing).
2. Persist the queue once, recompute the status indicator, and push one fresh
   snapshot to the background task.
3. The background task, on its next tick (≤2 s), attempts each game. Successful ones
   turn green and drop off the list; genuine conflicts re-fail and, after 30 min,
   re-appear as stuck.

Observable result: after one tap, the rows resolve over the next few seconds without
any further operator interaction, except for true conflicts which return to the
"needs attention" state on their own.

### Edge cases

- **No unsent games** (list empty or only recent successes): button is grayed and
  does nothing.
- **Token expired** (red token row present): RETRY ALL still resets and re-attempts;
  attempts will keep failing until the operator re-logs in via the token row. This is
  acceptable — RETRY ALL is not responsible for fixing auth. (We do **not** special-
  case hiding it when the token is expired; predictable placement wins.)
- **Disk write failure**: best-effort, mirroring `enqueue_game_end`/`force_submit` —
  the in-memory reset still happens and is logged; the next successful mutation
  re-persists.

---

## Implementation sketch

All changes are in `refbox`.

### 1. `portal_manager/mod.rs` — new method
```text
pub fn retry_all(&mut self) -> std::io::Result<()>
```
- Iterate `self.queue.items`; for each set `queued_at = now`, `last_attempt_at = None`
  (leave `force` as-is).
- `queue::save(...)` once, then `recompute_indicator()` and `push_queue_snapshot()`
  once (not per item).
- No-op safe when the queue is empty.
- Add a small helper (or reuse existing counting) to report whether any unsent items
  exist, for the button's enabled state — e.g. `has_unsent_items(&self) -> bool`.

### 2. `app/message.rs` — new message
- `Message::PortalRetryAll` (no payload — it acts on the whole queue).

### 3. `app/mod.rs` — handle the message
- On `PortalRetryAll`: call `self.portal_manager.retry_all()`, log any error
  (plain-English `error!`), stay on the detail page. Pattern mirrors the existing
  `PortalForceSubmit` arm.

### 4. `app/view_builders/portal_detail.rs` — the button
- Compute `has_unsent` from the `rows` already passed in:
  `rows.iter().any(|r| matches!(r, DetailRow::Stuck { .. } | DetailRow::Pending { .. }))`.
- Build a blue button labeled `fl!("portal-retry-all")`, enabled via
  `on_press_maybe(has_unsent.then_some(Message::PortalRetryAll))` so it grays out
  when there's nothing to send (mirrors the Apply-button gray-when-unchanged pattern).
- Footer row becomes `row![back, horizontal_space(), retry_all]` (BACK left third,
  blank middle third, RETRY ALL right third) — exactly the Option A layout.

### 5. Translations — new key `portal-retry-all`
- Add `portal-retry-all = RETRY ALL` (best-guess translation) to all 15 locale
  `.ftl` files: de-DE, en-US, es, fr, id-ID, it-IT, ja-JP, ko-KR, ms-MY, nl-NL,
  pt-PT, th-TH, tl-PH, tr-TR, zh-CN. Native review to follow, per project convention.

---

## Testing

- **`retry_all` unit test:** seed the queue with one stuck item (queued 31 min ago)
  and one young pending item; call `retry_all`; assert both now have
  `last_attempt_at == None` and `queued_at` within seconds of now, `force` unchanged,
  and that the queue was persisted (reload round-trip).
- **Stuck → eligible:** assert the previously-stuck item is now
  `is_item_retry_eligible == true`.
- **Button enabled-state:** assert `has_unsent_items` is `true` with a stuck or
  pending row present and `false` with only recent successes / empty queue.
- `just check` (fmt, clippy `-D warnings`, tests) green before PR.

## Acceptance criteria (operator-observable)

1. With one or more games not yet on the portal, a blue **RETRY ALL** button is
   visible in the bottom-right of the portal status screen.
2. Tapping it makes the waiting games (including stuck red ones) send over the next
   few seconds without any further taps; resolved games turn green and leave the list.
3. With no unsent games, the button is visibly grayed and inert.
4. A genuine conflict is not silently overwritten — it returns to "needs attention".

## Process / blast radius

Lean process (refbox UI + portal manager; no wire format, no game state machine, no
`uwh-common`). One feature branch, e.g. `feat/refbox/portal-retry-all`. Design doc is
a local working doc and is **not** committed to the feature branch (project
convention). Branch creation, commits, and PR await explicit approval.
