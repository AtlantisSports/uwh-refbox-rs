# Portal Stats Best-Effort (Decouple Score & Stats Uploads) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make a finished game's portal score the thing that drives its queue status; a stats-only upload failure keeps the dot green and parks the game as a tappable "Stats not sent, tap to retry" row that is never auto-retried.

**Architecture:** Add a persisted `score_sent` flag to each queued item. The background retry loop only auto-attempts `score_sent == false` items (score-pending). When a score posts but stats fail, the loop emits a new `ScoreSentStatsPending` event that flips the item to `score_sent == true` (stats-pending); from then on the item is invisible to the auto-retry loop, the stuck-escalation, and the indicator's yellow/red logic, and is re-sent only via a one-shot `PortalCommand::RetryStats` triggered by tapping its row or RETRY ALL.

**Tech Stack:** Rust 2024 (MSRV 1.85), `iced` 0.13, `tokio` async, `serde`/`serde_json` for the on-disk queue, `time` crate (RFC 3339), `i18n-embed-fl` (`fl!` macro, compile-time-checked against en-US Fluent files).

## Global Constraints

- **Crate scope:** `refbox` only. No `uwh-common`, no wire-format change, no portal-client change, no `Cargo.toml` change.
- **Process:** HEAVY — this changes the portal retry-queue state machine. Per-task tests, per-task commits, `just check` at the end.
- **No version bump:** `QueueFile::CURRENT_VERSION` stays `1`. The new field uses `#[serde(default)]` so existing `portal_queue.json` files load unchanged.
- **MSRV 1.85 / Edition 2024.** No `unwrap()`/`expect()` in non-test code without a justifying comment. Clippy `-D warnings`, all platforms.
- **Translations:** the new key `portal-row-stats-pending` must exist in **all 15 locales** (`de-DE, en-US, es, fr, id-ID, it-IT, ja-JP, ko-KR, ms-MY, nl-NL, pt-PT, th-TH, tl-PH, tr-TR, zh-CN`). Best-guess translations, no English placeholders; native review is a later follow-up.
- **Literal en-US copy:** `portal-row-stats-pending = Game { $game } Stats not sent, tap to retry` (no em-dash). The row is rendered **without** the `(attempt N)` suffix — unlike the auto-retried score-pending row, a stats-pending row is one-shot, so a counter would wrongly imply background retrying. (Operator decision, 2026-06-25.)
- **Branch:** `fix/refbox/portal-stats-best-effort`, cut from fresh `origin/master`. Create an isolated worktree via `superpowers:using-git-worktrees` before Task 1.

---

## Background: how the queue works today (read before starting)

- `refbox/src/portal_manager/queue.rs` — `QueuedItem` (the on-disk record) and `QueueFile` (envelope, `version: 1`). Atomic load/save.
- `refbox/src/portal_manager/health.rs` — the background `run_task` tokio loop. Every `POLL_INTERVAL` (2s) it iterates an **immutable clone** of the queue (`queue_snapshot`) and, for each retry-eligible item, calls `attempt_item` (posts scores then stats). It also fires `verify_token` on a cadence. It receives a fresh snapshot via `PortalCommand::QueueUpdated`. **The loop never mutates items** — `attempts`/`last_attempt_at` are only ever reset by `PortalManager` methods on the main thread.
- `refbox/src/portal_manager/mod.rs` — `PortalManager` (main-thread owner of the authoritative queue), the `PortalEvent` enum (task→UI), the `DetailRow` enum (rendered rows), `is_item_stuck`, the indicator recompute, `detail_rows`, `retry_all`, `force_immediate_retry`, `on_item_resolved`, and `push_queue_snapshot` (clones the queue and sends `QueueUpdated`).
- `refbox/src/app/mod.rs` (~2595–2660) — maps `PortalEvent` and the portal `Message`s (`PortalRowTapped`, `PortalRetryAll`, …) onto `PortalManager` calls.
- `refbox/src/app/view_builders/portal_detail.rs` — renders the detail page; `has_unsent` gates the RETRY ALL button; `render_row` matches each `DetailRow`.

**Consequence that shapes this design:** because the loop reads a clone it can never clear a per-item "please retry" flag — so the one-shot stats retry is a *command* (`RetryStats`), consumed once from the channel, not a flag the loop polls. Stats-pending items are gated out of the loop entirely.

---

## File Structure

| File | Change |
|------|--------|
| `refbox/src/portal_manager/queue.rs` | Add `score_sent: bool` to `QueuedItem` (Task 1) |
| `refbox/src/portal_manager/mod.rs` | `is_item_stuck` (Task 2); indicator helper (Task 3); `PortalEvent` variant + `on_score_sent_stats_pending` (Task 4); `send_stats_retry`/`request_stats_retry`/`is_stats_pending` (Task 5); `retry_all` (Task 6); `DetailRow` variant + `detail_rows` (Task 8) |
| `refbox/src/portal_manager/health.rs` | `attempt_item` + loop gating (Task 4); `PortalCommand::RetryStats` + run_task arm (Task 5) |
| `refbox/src/app/mod.rs` | `PortalEvent::ScoreSentStatsPending` arm (Task 4); `PortalRowTapped` routing (Task 5) |
| `refbox/src/app/view_builders/portal_detail.rs` | `has_unsent` + `render_row` arm (Task 8) |
| `refbox/translations/*/refbox.ftl` | `portal-row-stats-pending` ×15 (Task 7) |

---

## Task 1: Add `score_sent` field to `QueuedItem`

**Files:**
- Modify: `refbox/src/portal_manager/queue.rs` (struct + test literals)
- Modify: `refbox/src/portal_manager/mod.rs:499-511` (`enqueue_game_end` literal) and test literals (`mk_young_item`)
- Modify: `refbox/src/portal_manager/health.rs:221-235` (`mk_queue_item` test literal)

**Interfaces:**
- Produces: `QueuedItem.score_sent: bool` — `false` means score not yet accepted by the portal (score-pending); `true` means score accepted but stats still outstanding (stats-pending). Defaults to `false` for items loaded from a pre-existing `portal_queue.json`.

- [ ] **Step 1: Write the failing tests** (append to the `tests` module in `queue.rs`):

```rust
    #[test]
    fn score_sent_round_trips_true() {
        let item = QueuedItem {
            id: ItemId {
                event_id: "e1".into(),
                game_number: "G1".into(),
            },
            black_score: 1,
            white_score: 0,
            stats: "{}".into(),
            queued_at: datetime!(2026-06-25 12:00:00 UTC),
            attempts: 0,
            last_attempt_at: None,
            force: false,
            score_sent: true,
        };
        let s = serde_json::to_string(&item).unwrap();
        let back: QueuedItem = serde_json::from_str(&s).unwrap();
        assert!(back.score_sent);
        assert_eq!(item, back);
    }

    #[test]
    fn missing_score_sent_field_defaults_to_false() {
        // Simulate an old portal_queue.json written before this field existed.
        let item = QueuedItem {
            id: ItemId {
                event_id: "e1".into(),
                game_number: "G1".into(),
            },
            black_score: 0,
            white_score: 0,
            stats: "{}".into(),
            queued_at: datetime!(2026-06-25 12:00:00 UTC),
            attempts: 0,
            last_attempt_at: None,
            force: false,
            score_sent: true,
        };
        let mut v = serde_json::to_value(&item).unwrap();
        v.as_object_mut().unwrap().remove("score_sent");
        let back: QueuedItem = serde_json::from_value(v).unwrap();
        assert!(
            !back.score_sent,
            "an item with no score_sent field must load as score-pending (false)"
        );
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p refbox --bin refbox portal_manager::queue 2>&1 | tail -20`
Expected: FAIL — `missing field score_sent` / `QueuedItem` has no field `score_sent`.

- [ ] **Step 3: Add the field to the struct** (`queue.rs`, inside `QueuedItem`, after the `force` field at line ~59):

```rust
    /// Whether the portal has already accepted this game's **score**.
    /// `false` = score-pending (the normal queued state: auto-retried,
    /// can go stuck/red). `true` = stats-pending (score is up, only the
    /// stats upload is outstanding) — excluded from the auto-retry loop,
    /// the stuck escalation, and the yellow/red indicator; re-sent only
    /// by a one-shot `RetryStats` command. `#[serde(default)]` so old
    /// `portal_queue.json` files (written before this field existed)
    /// load as score-pending.
    #[serde(default)]
    pub score_sent: bool,
```

- [ ] **Step 4: Fix every `QueuedItem` literal so the crate compiles.** Add `score_sent: false,` to each construction site:
  - `queue.rs` test literals: `round_trips_queue_with_items` (~line 144) and `save_then_load_round_trip` (~line 180).
  - `mod.rs` production: `enqueue_game_end` (~line 499) — add `score_sent: false,` (a brand-new game's score is not yet sent).
  - `mod.rs` test: `mk_young_item` (~line 737).
  - `health.rs` test: `mk_queue_item` (~line 221).

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p refbox --bin refbox portal_manager 2>&1 | tail -20`
Expected: PASS (all existing portal_manager tests + the two new ones).

- [ ] **Step 6: Commit**

```bash
git add refbox/src/portal_manager/queue.rs refbox/src/portal_manager/mod.rs refbox/src/portal_manager/health.rs
git commit -m "feat(refbox): add score_sent flag to queued portal items"
```

---

## Task 2: `is_item_stuck` ignores stats-pending items

**Files:**
- Modify: `refbox/src/portal_manager/mod.rs:298-300` (`is_item_stuck`)
- Test: same file's `tests` module

**Interfaces:**
- Consumes: `QueuedItem.score_sent` (Task 1).
- Produces: `is_item_stuck` now returns `false` for any `score_sent == true` item regardless of age. This single change makes `is_stuck`, `has_stuck_items`, and `is_item_retry_eligible` all treat stats-pending items as never-stuck.

- [ ] **Step 1: Write the failing test** (append to `mod.rs` tests):

```rust
    #[test]
    fn stats_pending_item_is_never_stuck_even_when_old() {
        let mut it = mk_young_item();
        it.score_sent = true;
        it.queued_at = OffsetDateTime::now_utc() - TimeDuration::minutes(120);
        assert!(
            !is_item_stuck(&it, OffsetDateTime::now_utc()),
            "a stats-pending item must never be classified as stuck"
        );
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p refbox --bin refbox stats_pending_item_is_never_stuck 2>&1 | tail -20`
Expected: FAIL — currently returns `true` (2-hour-old item).

- [ ] **Step 3: Implement** (replace the body of `is_item_stuck`):

```rust
pub fn is_item_stuck(item: &QueuedItem, now: OffsetDateTime) -> bool {
    // Stats-pending items (score already accepted) never go stuck: a
    // missing stat must not nag the operator or escalate to red.
    !item.score_sent && (now - item.queued_at) >= STUCK_THRESHOLD
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p refbox --bin refbox portal_manager 2>&1 | tail -20`
Expected: PASS (new test + existing `stuck_item_is_red`, `is_stuck_classifies_items_...` still pass — those use `score_sent == false` items).

- [ ] **Step 5: Commit**

```bash
git add refbox/src/portal_manager/mod.rs
git commit -m "feat(refbox): exclude stats-pending items from stuck escalation"
```

---

## Task 3: Indicator dot ignores stats-pending items

**Files:**
- Modify: `refbox/src/portal_manager/mod.rs` — replace `has_any_queue_items` (line ~368) with `has_score_pending_items`; update `recompute_indicator` (line ~376-386)
- Test: same file's `tests` module

**Interfaces:**
- Consumes: `QueuedItem.score_sent`.
- Produces: `fn has_score_pending_items(&self) -> bool` — true iff any queued item has `score_sent == false`. `recompute_indicator` drives Yellow off this (was `has_any_queue_items`), so a queue containing only stats-pending items stays Green.

- [ ] **Step 1: Write the failing test** (append to `mod.rs` tests):

```rust
    #[tokio::test]
    async fn queue_with_only_stats_pending_item_is_green() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();
        m.enqueue_game_end("e".into(), "G1".into(), 3, 2, "{}".into())
            .unwrap();
        // Mark it stats-pending and age it well past the stuck threshold.
        m.queue.items[0].score_sent = true;
        m.queue.items[0].queued_at = OffsetDateTime::now_utc() - TimeDuration::minutes(120);
        m.recompute_indicator();
        assert_eq!(
            m.indicator_state().health,
            HealthState::Green,
            "a queue holding only stats-pending items must keep the dot green"
        );
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p refbox --bin refbox queue_with_only_stats_pending_item_is_green 2>&1 | tail -20`
Expected: FAIL — currently Yellow (`has_any_queue_items` is true).

- [ ] **Step 3: Implement.** Replace the `has_any_queue_items` method:

```rust
    fn has_score_pending_items(&self) -> bool {
        self.queue.items.iter().any(|it| !it.score_sent)
    }
```

And in `recompute_indicator`, change the Yellow condition:

```rust
    fn recompute_indicator(&mut self) {
        let health = if self.needs_attention() {
            HealthState::Red
        } else if self.check_in_flight || self.has_score_pending_items() {
            HealthState::Yellow
        } else {
            HealthState::Green
        };

        self.indicator_state = PortalIndicatorState { health };
    }
```

(`needs_attention` already calls `has_stuck_items`, which after Task 2 ignores stats-pending items — so the Red path is correct with no change.)

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p refbox --bin refbox portal_manager 2>&1 | tail -20`
Expected: PASS (new test + existing `young_pending_item_is_yellow`, `stuck_item_is_red`, `empty_queue_and_no_problems_is_green` — all use `score_sent == false`).

- [ ] **Step 5: Commit**

```bash
git add refbox/src/portal_manager/mod.rs
git commit -m "feat(refbox): keep portal indicator green for stats-pending items"
```

---

## Task 4: Score-OK / stats-fail marks the item stats-pending

This task introduces the transition. It touches the emit site (`health.rs`), the event enum + handler method (`mod.rs`), and the UI dispatch (`app/mod.rs`) together, because adding a `PortalEvent` variant makes the `app/mod.rs` match non-exhaustive (compile error) until handled.

**Files:**
- Modify: `refbox/src/portal_manager/mod.rs` — add `PortalEvent::ScoreSentStatsPending(ItemId)` (enum at line ~239); add `on_score_sent_stats_pending` method
- Modify: `refbox/src/portal_manager/health.rs` — `attempt_item` stats-fail branch (line ~209); loop gating (line ~137)
- Modify: `refbox/src/app/mod.rs:2604-2614` — new match arm
- Test: `health.rs` and `mod.rs` test modules

**Interfaces:**
- Consumes: `QueuedItem.score_sent`.
- Produces:
  - `PortalEvent::ScoreSentStatsPending(ItemId)` — emitted when a score posts but stats fail.
  - `PortalManager::on_score_sent_stats_pending(&mut self, id: ItemId)` — sets the item's `score_sent = true`, persists, recomputes the indicator, and pushes a fresh snapshot so the loop stops attempting it. Idempotent (no-op if already `score_sent` or id unknown).
  - `attempt_item` now returns `true` on score-OK/stats-fail (portal is reachable → suppress the cadence `verify_token`).
  - The background loop attempts only `score_sent == false` items.

- [ ] **Step 1: Write the failing tests.**

In `health.rs` tests, add a test that score-OK + stats-fail emits `ScoreSentStatsPending` (uses the existing `FakeIo`/`spawn`/`drain_events` harness; mirror `eligible_item_triggers_scores_then_stats_and_emits_resolved_on_success`):

```rust
    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn score_ok_stats_fail_emits_score_sent_stats_pending() {
        let scores_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let stats_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let io = FakeIo {
            verify_results: Mutex::new(vec![Ok(())]),
            verify_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_results: Mutex::new(vec![Ok(())]),
            scores_count: scores_count.clone(),
            stats_results: Mutex::new(vec![Err(PortalCallError::Failed("no unique caps".into()))]),
            stats_count: stats_count.clone(),
        };
        let mut handle = spawn(io);

        let queue = queue_with_one_eligible_item();
        let expected_id = queue.items[0].id.clone();
        handle
            .command_tx
            .send(PortalCommand::QueueUpdated(queue))
            .await
            .unwrap();

        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(3)).await;
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(500)).await;
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        assert_eq!(scores_count.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(stats_count.load(std::sync::atomic::Ordering::SeqCst), 1);

        let events = drain_events(&mut handle.event_rx);
        assert!(
            events.iter().any(|ev| matches!(
                ev,
                PortalEvent::ScoreSentStatsPending(id) if id == &expected_id
            )),
            "expected ScoreSentStatsPending for {expected_id:?}, got {events:?}"
        );
        assert!(
            !events.iter().any(|ev| matches!(ev, PortalEvent::ItemResolved(_))),
            "score-ok/stats-fail must NOT resolve the item"
        );
        drop(handle);
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn stats_pending_item_is_not_auto_attempted_by_loop() {
        let scores_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let stats_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let io = FakeIo {
            verify_results: Mutex::new(vec![Ok(())]),
            verify_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_results: Mutex::new(vec![Ok(())]),
            scores_count: scores_count.clone(),
            stats_results: Mutex::new(vec![Ok(())]),
            stats_count: stats_count.clone(),
        };
        let handle = spawn(io);

        // A single item already in the stats-pending state.
        let mut item = mk_queue_item(0);
        item.score_sent = true;
        let queue = QueueFile {
            version: QueueFile::CURRENT_VERSION,
            items: vec![item],
        };
        handle
            .command_tx
            .send(PortalCommand::QueueUpdated(queue))
            .await
            .unwrap();

        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(3)).await;
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(500)).await;
        tokio::task::yield_now().await;

        assert_eq!(
            scores_count.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "the auto-retry loop must not re-post scores for a stats-pending item"
        );
        assert_eq!(
            stats_count.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "the auto-retry loop must not auto-attempt stats for a stats-pending item"
        );
        drop(handle);
    }
```

In `mod.rs` tests, add:

```rust
    #[tokio::test]
    async fn on_score_sent_stats_pending_marks_item_and_stays_green() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();
        m.enqueue_game_end("e".into(), "G1".into(), 3, 2, "{}".into())
            .unwrap();
        let id = m.queue.items[0].id.clone();
        assert_eq!(m.indicator_state().health, HealthState::Yellow);

        m.on_score_sent_stats_pending(id);

        assert!(m.queue.items[0].score_sent, "item must be marked stats-pending");
        assert_eq!(
            m.indicator_state().health,
            HealthState::Green,
            "once the score is sent the dot must be green even though stats are outstanding"
        );
        // Persisted: a restart must reload it as stats-pending.
        let reloaded = queue::load_or_empty(tmp.path()).unwrap();
        assert!(reloaded.items[0].score_sent);
    }
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p refbox --bin refbox portal_manager 2>&1 | tail -25`
Expected: FAIL to compile — `ScoreSentStatsPending` and `on_score_sent_stats_pending` do not exist yet.

- [ ] **Step 3: Add the event variant** (`mod.rs`, in `PortalEvent`, after `ItemUpdated`):

```rust
    /// The score posted successfully but the stats upload failed. The
    /// main thread flips the item to stats-pending (`score_sent = true`)
    /// so the auto-retry loop, the stuck escalation, and the indicator
    /// all stop treating it as outstanding. Carries the item id.
    ScoreSentStatsPending(ItemId),
```

- [ ] **Step 4: Change `attempt_item`** (`health.rs`, the stats `Err` branch) and update its doc comment. The score-success branch's `Err` arm becomes:

```rust
    match io.post_stats(item).await {
        Ok(()) => {
            let _ = event_tx
                .send(PortalEvent::ItemResolved(item.id.clone()))
                .await;
            true
        }
        Err(_) => {
            // Score is up but stats failed (e.g. an event that does not
            // require unique cap numbers rejects all stats). Mark the
            // item stats-pending; the portal is reachable, so return
            // `true` to suppress the cadence `verify_token`.
            let _ = event_tx
                .send(PortalEvent::ScoreSentStatsPending(item.id.clone()))
                .await;
            true
        }
    }
```

Update the `attempt_item` doc comment's "Only a full success … emits `ItemResolved`" / "Returns `true` iff both portal calls succeeded" lines to describe the new behavior (score-fail → `ItemUpdated`/`false`; stats-fail → `ScoreSentStatsPending`/`true`; both-ok → `ItemResolved`/`true`).

- [ ] **Step 5: Gate the loop** (`health.rs` `run_task`, the per-item loop at line ~137):

```rust
                for item in &queue_snapshot.items {
                    if !item.score_sent
                        && is_item_retry_eligible(item, now)
                        && attempt_item(&io, item, &event_tx).await
                    {
                        last_success = Some(TokioInstant::now());
                    }
                }
```

- [ ] **Step 6: Add the manager method** (`mod.rs`, near `on_item_resolved`):

```rust
    /// Background task reported a score-OK / stats-fail outcome: the score
    /// is up but the stats upload was rejected. Flip the item to
    /// stats-pending so it leaves the auto-retry loop and the yellow/red
    /// indicator, persist, and push a fresh snapshot so the background
    /// task stops attempting it. Idempotent: a duplicate event (or an
    /// unknown id) is a silent no-op.
    pub fn on_score_sent_stats_pending(&mut self, id: ItemId) {
        let Some(item) = self.find_mut(&id) else {
            return;
        };
        if item.score_sent {
            return; // Already stats-pending; nothing to do.
        }
        item.score_sent = true;
        if let Err(e) = queue::save(&self.config_dir, &self.queue) {
            log::warn!("portal queue save after score-sent/stats-pending failed: {e}");
        }
        self.recompute_indicator();
        self.push_queue_snapshot();
    }
```

- [ ] **Step 7: Handle the event in the UI dispatch** (`app/mod.rs`, the `match ev` block at line ~2604):

```rust
                match ev {
                    PortalEvent::ItemResolved(id) => {
                        self.portal_manager.on_item_resolved(id);
                    }
                    PortalEvent::ScoreSentStatsPending(id) => {
                        self.portal_manager.on_score_sent_stats_pending(id);
                    }
                    PortalEvent::HealthChanged | PortalEvent::ItemUpdated => {
                        self.portal_manager.ui_tick();
                    }
                    PortalEvent::TokenStatus(valid) => {
                        self.portal_manager.on_token_status(valid);
                    }
                }
```

- [ ] **Step 8: Run to verify they pass**

Run: `cargo test -p refbox --bin refbox portal_manager 2>&1 | tail -25`
Expected: PASS (new tests + all existing; `scores_failure_emits_item_updated_and_skips_stats` and `eligible_item_triggers_..._resolved_on_success` are unchanged).

- [ ] **Step 9: Commit**

```bash
git add refbox/src/portal_manager/health.rs refbox/src/portal_manager/mod.rs refbox/src/app/mod.rs
git commit -m "feat(refbox): mark game stats-pending when score posts but stats fail"
```

---

## Task 5: One-shot stats retry (tap a stats-pending row)

**Files:**
- Modify: `refbox/src/portal_manager/health.rs` — `PortalCommand::RetryStats(QueuedItem)` (enum at line ~67); `run_task` command arm (line ~168)
- Modify: `refbox/src/portal_manager/mod.rs` — `send_stats_retry` (private), `request_stats_retry`, `is_stats_pending`
- Modify: `refbox/src/app/mod.rs:2631-2645` — `PortalRowTapped` routing
- Test: `health.rs` and `mod.rs` test modules

**Interfaces:**
- Consumes: `QueuedItem.score_sent`, `PortalManager::find`, `command_tx`.
- Produces:
  - `PortalCommand::RetryStats(QueuedItem)` — a one-shot: the background task posts stats exactly once. `Ok` → `ItemResolved(id)`; `Err` → `ItemUpdated` (item stays stats-pending, no escalation).
  - `PortalManager::request_stats_retry(&self, id: &ItemId)` — sends one `RetryStats` for the named item if it exists.
  - `PortalManager::is_stats_pending(&self, id: &ItemId) -> bool` — true iff the queued item has `score_sent == true`.

- [ ] **Step 1: Write the failing tests.**

In `health.rs` tests (one-shot via the real loop):

```rust
    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn retry_stats_command_posts_stats_once_and_resolves_on_success() {
        let scores_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let stats_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let io = FakeIo {
            verify_results: Mutex::new(vec![Ok(())]),
            verify_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_results: Mutex::new(vec![]),
            scores_count: scores_count.clone(),
            stats_results: Mutex::new(vec![Ok(())]),
            stats_count: stats_count.clone(),
        };
        let mut handle = spawn(io);

        let mut item = mk_queue_item(0);
        item.score_sent = true;
        let expected_id = item.id.clone();
        handle
            .command_tx
            .send(PortalCommand::RetryStats(item))
            .await
            .unwrap();

        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        assert_eq!(
            stats_count.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "RetryStats must post stats exactly once"
        );
        assert_eq!(
            scores_count.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "RetryStats must never re-post the score"
        );
        let events = drain_events(&mut handle.event_rx);
        assert!(
            events.iter().any(|ev| matches!(
                ev, PortalEvent::ItemResolved(id) if id == &expected_id
            )),
            "successful RetryStats must resolve the item, got {events:?}"
        );
        drop(handle);
    }
```

In `mod.rs` tests:

```rust
    #[tokio::test]
    async fn is_stats_pending_reflects_score_sent_flag() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();
        m.enqueue_game_end("e".into(), "G1".into(), 0, 0, "{}".into())
            .unwrap();
        let id = m.queue.items[0].id.clone();
        assert!(!m.is_stats_pending(&id), "fresh item is score-pending");

        m.queue.items[0].score_sent = true;
        assert!(m.is_stats_pending(&id), "score_sent item is stats-pending");

        let unknown = ItemId { event_id: "e".into(), game_number: "GX".into() };
        assert!(!m.is_stats_pending(&unknown), "unknown id must be false, not panic");
    }
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p refbox --bin refbox portal_manager 2>&1 | tail -25`
Expected: FAIL to compile — `PortalCommand::RetryStats` and `is_stats_pending` do not exist.

- [ ] **Step 3: Add the command variant** (`health.rs`, in `PortalCommand`):

```rust
    /// One-shot request to (re)post the stats for a single stats-pending
    /// item. Triggered by the operator tapping the item's row or RETRY
    /// ALL. The task attempts stats exactly once and never schedules a
    /// follow-up — stats-pending items are not on the auto-retry cadence.
    RetryStats(QueuedItem),
```

- [ ] **Step 4: Handle the command** (`health.rs` `run_task`, the `command_rx` arm — extend the inner match):

```rust
                    Some(PortalCommand::QueueUpdated(new_queue)) => {
                        queue_snapshot = new_queue;
                    }
                    Some(PortalCommand::RetryStats(item)) => {
                        match io.post_stats(&item).await {
                            Ok(()) => {
                                let _ = event_tx
                                    .send(PortalEvent::ItemResolved(item.id.clone()))
                                    .await;
                            }
                            Err(_) => {
                                // Stays stats-pending; no escalation, no loop.
                                let _ = event_tx.send(PortalEvent::ItemUpdated).await;
                            }
                        }
                    }
```

- [ ] **Step 5: Add the manager methods** (`mod.rs`):

```rust
    /// Send a one-shot `RetryStats` command to the background task for
    /// the given item. The command carries a clone of the item, so the
    /// task can attempt it without holding the UI lock. Spawned like
    /// `push_queue_snapshot` because `update()` is synchronous.
    fn send_stats_retry(&self, item: &QueuedItem) {
        let tx = self.command_tx.clone();
        let item = item.clone();
        tokio::spawn(async move {
            let _ = tx.send(health::PortalCommand::RetryStats(item)).await;
        });
    }

    /// Operator asked to retry the stats for one stats-pending item
    /// (tapped its row). Sends exactly one `RetryStats`. No-op if the id
    /// is not in the queue.
    pub fn request_stats_retry(&self, id: &ItemId) {
        if let Some(item) = self.find(id) {
            self.send_stats_retry(item);
        }
    }

    /// True iff the queued item exists and its score has been sent but
    /// stats are still outstanding (stats-pending). False for score-
    /// pending items and for unknown ids.
    pub fn is_stats_pending(&self, id: &ItemId) -> bool {
        self.find(id).is_some_and(|it| it.score_sent)
    }
```

- [ ] **Step 6: Route the row tap** (`app/mod.rs`, `Message::PortalRowTapped`):

```rust
            Message::PortalRowTapped(id) => {
                if self.portal_manager.is_stuck(&id) {
                    self.app_state = AppState::PortalAttentionAction {
                        item_id: id,
                        discard_armed: false,
                    };
                    trace!("AppState changed to {:?}", self.app_state);
                } else if self.portal_manager.is_stats_pending(&id) {
                    // Stats-pending row: fire one stats attempt. No
                    // background loop, no escalation.
                    self.portal_manager.request_stats_retry(&id);
                } else {
                    // Young score-pending row tapped — force an immediate retry.
                    if let Err(e) = self.portal_manager.force_immediate_retry(&id) {
                        error!("force_immediate_retry failed: {e}");
                    }
                }
                Task::none()
            }
```

- [ ] **Step 7: Run to verify they pass**

Run: `cargo test -p refbox --bin refbox portal_manager 2>&1 | tail -25`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add refbox/src/portal_manager/health.rs refbox/src/portal_manager/mod.rs refbox/src/app/mod.rs
git commit -m "feat(refbox): one-shot stats retry on tapping a stats-pending row"
```

---

## Task 6: RETRY ALL sweeps stats-pending items

**Files:**
- Modify: `refbox/src/portal_manager/mod.rs` — `retry_all` (line ~554)
- Test: same file's `tests` module

**Interfaces:**
- Consumes: `send_stats_retry` (Task 5), `QueuedItem.score_sent`.
- Produces: `retry_all` additionally fires one `RetryStats` per stats-pending item. Score-pending items are reset exactly as before; `score_sent` and `force` are left untouched.

- [ ] **Step 1: Write the failing test** (append to `mod.rs` tests):

```rust
    #[tokio::test]
    async fn retry_all_preserves_score_sent_and_resets_score_pending() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();
        // One stats-pending game and one score-pending game.
        m.enqueue_game_end("e".into(), "G_STATS".into(), 1, 0, "{}".into())
            .unwrap();
        m.queue.items[0].score_sent = true;
        m.enqueue_game_end("e".into(), "G_SCORE".into(), 2, 1, "{}".into())
            .unwrap();
        m.queue.items[1].attempts = 3;

        m.retry_all().unwrap();

        // Stats-pending item keeps score_sent and is not forced.
        assert!(m.queue.items[0].score_sent, "retry_all must not clear score_sent");
        assert!(!m.queue.items[0].force);
        // Score-pending item is reset for immediate auto-retry.
        assert_eq!(m.queue.items[1].attempts, 0);
        assert!(m.queue.items[1].last_attempt_at.is_none());
        assert!(!m.queue.items[1].force);
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p refbox --bin refbox retry_all_preserves_score_sent 2>&1 | tail -20`
Expected: It may already pass for the assertions on `score_sent`/reset (since `retry_all` does not touch `score_sent`). If it passes, that is acceptable — this task's behavioral change (firing `RetryStats`) is exercised by Task 5's `health.rs` command test. Still add the sweep so RETRY ALL actually retries stats. If it fails, proceed to Step 3.

> Note (observability): the `RetryStats` send goes through an internal `mpsc::Sender` that is not observable from a `PortalManager` unit test (same limitation documented on `force_immediate_retry_pushes_queue_snapshot`). The one-shot command path is covered in `health.rs`; here we assert only the observable state.

- [ ] **Step 3: Implement.** Replace `retry_all`'s body, keeping the existing reset loop and adding a stats sweep:

```rust
    pub fn retry_all(&mut self) -> std::io::Result<()> {
        let now = OffsetDateTime::now_utc();
        for item in &mut self.queue.items {
            item.attempts = 0;
            item.last_attempt_at = None;
            item.queued_at = now;
        }
        queue::save(&self.config_dir, &self.queue)?;
        self.recompute_indicator();
        self.push_queue_snapshot();
        // Stats-pending games are not on the auto-retry cadence; give
        // each a single fresh stats attempt so RETRY ALL sweeps them too.
        for item in &self.queue.items {
            if item.score_sent {
                self.send_stats_retry(item);
            }
        }
        Ok(())
    }
```

(Resetting `queued_at`/`attempts` on a stats-pending item is harmless — those fields are unused for `score_sent` items — and keeps the loop a single pass.)

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p refbox --bin refbox portal_manager 2>&1 | tail -20`
Expected: PASS (new test + existing `retry_all_unsticks_and_resets_every_item_without_forcing`).

- [ ] **Step 5: Commit**

```bash
git add refbox/src/portal_manager/mod.rs
git commit -m "feat(refbox): RETRY ALL also re-sends stats for stats-pending games"
```

---

## Task 7: Add `portal-row-stats-pending` to all 15 locales

This comes before the UI task because `fl!("portal-row-stats-pending")` is checked against the en-US Fluent file at compile time. Adding the key first (en-US enables compilation; an unused key is harmless) keeps every task compiling.

**Files:**
- Modify: `refbox/translations/{de-DE,en-US,es,fr,id-ID,it-IT,ja-JP,ko-KR,ms-MY,nl-NL,pt-PT,th-TH,tl-PH,tr-TR,zh-CN}/refbox.ftl`

**Interfaces:**
- Produces: Fluent message `portal-row-stats-pending` with one `$game` argument, in every locale. Mirrors each locale's existing `portal-row-pending` wording, swapping the "score" noun for "stats/statistics".

- [ ] **Step 1: Add the key directly under `portal-row-pending` in each file.** Use these exact values (best-guess, mirroring each locale's `portal-row-pending`):

```
en-US: portal-row-stats-pending = Game { $game } Stats not sent, tap to retry
de-DE: portal-row-stats-pending = Spiel { $game } Statistik nicht gesendet, zum erneuten Versuch tippen
es:    portal-row-stats-pending = Juego { $game } · Estadísticas no enviadas, toca para reintentar
fr:    portal-row-stats-pending = Match { $game } · Statistiques non envoyées, touchez pour réessayer
id-ID: portal-row-stats-pending = Pertandingan { $game } Statistik belum terkirim, ketuk untuk coba lagi
it-IT: portal-row-stats-pending = Partita { $game } Statistiche non inviate, tocca per riprovare
ja-JP: portal-row-stats-pending = 試合 { $game } の統計が未送信、タップして再試行
ko-KR: portal-row-stats-pending = 경기 { $game } 통계 전송 안 됨, 탭하여 재시도
ms-MY: portal-row-stats-pending = Perlawanan { $game } Statistik belum dihantar, ketik untuk cuba semula
nl-NL: portal-row-stats-pending = Wedstrijd { $game } Statistieken niet verzonden, tik om opnieuw te proberen
pt-PT: portal-row-stats-pending = Jogo { $game } Estatísticas não enviadas, toque para tentar novamente
th-TH: portal-row-stats-pending = เกม { $game } ยังไม่ส่งสถิติ แตะเพื่อลองอีกครั้ง
tl-PH: portal-row-stats-pending = Laro { $game } Hindi naipadala ang estadistika, pindutin para subukan muli
tr-TR: portal-row-stats-pending = Oyun { $game } İstatistikler gönderilmedi, tekrar denemek için dokunun
zh-CN: portal-row-stats-pending = 比赛 { $game } 统计未发送，点击重试
```

- [ ] **Step 2: Verify the key exists in all 15 locales**

Run: `for d in refbox/translations/*/; do grep -q "portal-row-stats-pending" "$d"refbox.ftl || echo "MISSING in $d"; done`
Expected: no output (present everywhere).

- [ ] **Step 3: Confirm the crate still builds** (unused key is fine)

Run: `cargo build -p refbox 2>&1 | tail -10`
Expected: builds clean.

- [ ] **Step 4: Commit**

```bash
git add refbox/translations
git commit -m "feat(refbox): add portal-row-stats-pending label in all locales"
```

---

## Task 8: `DetailRow::StatsPending` — render the row + enable RETRY ALL

**Files:**
- Modify: `refbox/src/portal_manager/mod.rs` — add `DetailRow::StatsPending` (enum at line ~256); update `detail_rows` (line ~686)
- Modify: `refbox/src/app/view_builders/portal_detail.rs` — `has_unsent` (line ~52); `render_row` new arm (line ~129)
- Test: `mod.rs` test module

**Interfaces:**
- Consumes: `QueuedItem.score_sent`, the `portal-row-stats-pending` key (Task 7).
- Produces:
  - `DetailRow::StatsPending { id: ItemId, game_number: String }` (no `attempts` field — the row shows no counter).
  - `detail_rows` emits `StatsPending` rows after `Stuck`/`Pending` and before `RecentSuccess`.
  - The detail page renders `StatsPending` as a yellow tap-to-retry row (tap → `Message::PortalRowTapped(id)`); `has_unsent` (RETRY ALL enabled) includes it.

- [ ] **Step 1: Write the failing test** (append to `mod.rs` tests):

```rust
    #[tokio::test]
    async fn detail_rows_places_stats_pending_after_pending_before_recent() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();

        // A score-pending (young) game and a stats-pending game.
        m.enqueue_game_end("e".into(), "G_SCORE".into(), 0, 0, "{}".into())
            .unwrap();
        m.queue.items[0].queued_at = OffsetDateTime::now_utc() - TimeDuration::minutes(2);
        m.enqueue_game_end("e".into(), "G_STATS".into(), 1, 0, "{}".into())
            .unwrap();
        m.queue.items[1].score_sent = true;

        // And one recent success.
        m.enqueue_game_end("e".into(), "G_DONE".into(), 2, 1, "{}".into())
            .unwrap();
        let done_id = m.queue.items[2].id.clone();
        m.on_item_resolved(done_id);

        let rows = m.detail_rows();
        assert!(
            matches!(&rows[0], DetailRow::Pending { game_number, .. } if game_number == "G_SCORE"),
            "row0 should be the score-pending game, got {:?}", rows[0]
        );
        assert!(
            matches!(&rows[1], DetailRow::StatsPending { game_number, .. } if game_number == "G_STATS"),
            "row1 should be the stats-pending game, got {:?}", rows[1]
        );
        assert!(
            matches!(&rows[2], DetailRow::RecentSuccess { game_number, .. } if game_number == "G_DONE"),
            "row2 should be the recent success, got {:?}", rows[2]
        );
        assert_eq!(rows.len(), 3);
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p refbox --bin refbox detail_rows_places_stats_pending 2>&1 | tail -20`
Expected: FAIL to compile — `DetailRow::StatsPending` does not exist (and currently the stats-pending item would wrongly become a `Pending` row).

- [ ] **Step 3: Add the variant** (`mod.rs`, in `DetailRow`, after `Pending`):

```rust
    /// A game whose score is sent but whose stats upload is still
    /// outstanding (stats-pending). Rendered yellow like `Pending`, but
    /// it never escalates and is not auto-retried — tapping fires one
    /// stats attempt. No attempt counter: the row is one-shot, so a
    /// counter would wrongly imply background retrying.
    StatsPending { id: ItemId, game_number: String },
```

- [ ] **Step 4: Update `detail_rows`.** Guard the existing pending pass so it skips stats-pending items, and add a stats-pending pass before the recent-successes loop:

```rust
        for it in &items {
            if !it.score_sent && !is_item_stuck(it, now) {
                out.push(DetailRow::Pending {
                    id: it.id.clone(),
                    game_number: it.id.game_number.clone(),
                    attempts: it.attempts,
                });
            }
        }
        for it in &items {
            if it.score_sent {
                out.push(DetailRow::StatsPending {
                    id: it.id.clone(),
                    game_number: it.id.game_number.clone(),
                });
            }
        }
```

(The `Stuck` pass above is unchanged: `is_item_stuck` already returns `false` for `score_sent` items after Task 2.)

- [ ] **Step 5: Render the row** (`portal_detail.rs`, add an arm in `render_row` — yellow and tappable like `Pending`, but no attempt suffix):

```rust
        DetailRow::StatsPending { id, game_number } => {
            button(row_text_centered(fl!(
                "portal-row-stats-pending",
                game = game_number
            )))
            .on_press(Message::PortalRowTapped(id))
            .style(yellow_button)
            .padding(PADDING)
            .width(Length::Fill)
            .height(Length::Fixed(MIN_BUTTON_SIZE))
            .into()
        }
```

- [ ] **Step 6: Widen `has_unsent`** (`portal_detail.rs`, line ~52) so RETRY ALL stays active while stats are pending:

```rust
    let has_unsent = rows.iter().any(|r| {
        matches!(
            r,
            DetailRow::Stuck { .. } | DetailRow::Pending { .. } | DetailRow::StatsPending { .. }
        )
    });
```

- [ ] **Step 7: Run to verify it passes + build**

Run: `cargo test -p refbox --bin refbox portal_manager 2>&1 | tail -20 && cargo build -p refbox 2>&1 | tail -5`
Expected: PASS + clean build (the `render_row` match is now exhaustive).

- [ ] **Step 8: Commit**

```bash
git add refbox/src/portal_manager/mod.rs refbox/src/app/view_builders/portal_detail.rs
git commit -m "feat(refbox): render stats-pending detail rows and keep RETRY ALL active"
```

---

## Task 9: Full verification + walkthrough

**Files:** none (verification only).

- [ ] **Step 1: Full check**

Run: `just check`
Expected: fmt, clippy (`-D warnings`, all targets/features), all tests, audit — clean.

- [ ] **Step 2: Build the runnable binary** (clippy/test build a different binary; rebuild before a live walkthrough)

Run: `cargo build -p refbox`

- [ ] **Step 3: Acceptance walkthrough** — confirm each spec acceptance criterion against the dev portal (event `1825-C`, which rejects stats with `400 "does not require unique cap numbers"`). Use `UWH_PORTAL_URL_OVERRIDE=https://api.dev.uwhportal.com`; launch with `WAYLAND_DISPLAY=` and `dangerouslyDisableSandbox: true`. Verify:
  1. Finish a game on `1825-C` (score posts `200`, stats `400`): the portal **dot stays green**, and the detail page shows **"Game N Stats not sent, tap to retry"** — it does **not** go yellow/red, does **not** escalate after 30 min, and the background does **not** keep re-posting.
  2. Tapping that row, or **RETRY ALL**, makes **one** fresh stats attempt; on `1825-C` it fails again and the row stays "Stats not sent" (no loop).
  3. A **score** failure still behaves exactly as today (yellow → red at 30 min, dot reflects it, background auto-retries).
  4. Restart with a stats-pending game in `portal_queue.json`: it reloads as stats-pending (green dot, "Stats not sent" row), still retriable.

- [ ] **Step 4: Code review** — run `superpowers:requesting-code-review` on the full branch diff, then open the PR per `.claude/rules/pr-review.md` (plain-language What/Why/Scope/How-to-verify). **Get the human's approval before creating the branch push / PR.**

---

## Self-Review (completed during planning)

- **Spec coverage:** Problem & root cause → addressed by the whole branch. Approved behavior outcomes 1/2/3 → Tasks 4 (transition), 2+3 (no nag/no escalation/green dot), unchanged score path (Task 4 loop gating leaves `score_sent==false` behavior intact). Architecture §1 (`score_sent`) → Task 1; §2 (`attempt_item`, loop gating, stuck) → Tasks 2,4; §3 (indicator, detail_rows, retry_all, has_unsent) → Tasks 3,6,8; §4 (one-shot mechanism) → Task 5 (resolved as `PortalCommand::RetryStats`, *not* a flag); §5 (rows + translations) → Tasks 7,8. Acceptance criteria 1–4 → Task 9. Testing list → covered across Tasks 1,3,4,5,6,8.
- **Open detail resolved:** §4's "transient flag vs dedicated path" → **dedicated command path** (`RetryStats`), because the loop reads an immutable clone and cannot consume a flag without re-firing.
- **Spec deviation (copy):** spec §5 said "reuse `portal-row-attempt-suffix`" on the stats row. Per operator decision (2026-06-25) the stats-pending row is rendered **without** the attempt suffix (it is one-shot, not auto-retried, so a counter would mislead). `DetailRow::StatsPending` therefore carries no `attempts` field.
- **Type consistency:** `score_sent: bool` (Task 1) used identically everywhere; `ScoreSentStatsPending(ItemId)` emitted (health) + handled (app) + acted on via `on_score_sent_stats_pending` (mod); `PortalCommand::RetryStats(QueuedItem)` sent via `send_stats_retry`/`request_stats_retry` and matched in `run_task`; `DetailRow::StatsPending { id, game_number, attempts }` produced in `detail_rows` and consumed in `render_row` + `has_unsent`.
- **No placeholders:** every code step shows real code; every test step shows the assertion and the run command.

## Deviations

(none yet — append here during execution if reality diverges)
