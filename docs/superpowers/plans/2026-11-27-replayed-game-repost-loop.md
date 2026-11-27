# Replayed Game Re-post Loop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A replayed game's result leaves the portal queue once the portal accepts it, instead of being re-sent every ~16 s for 30 minutes and then shown as a red "stuck" row.

**Architecture:** `PortalManager::on_item_resolved` (`refbox/src/portal_manager/mod.rs`) returns early when the game's id is already in the recent-successes list (the last 5 results sent, kept in memory). A replay re-uses that id (event + game number), so the replay's queue item is never removed. Drop that early return: the "is this item still on the queue?" check right after it already makes a duplicate resolution a no-op. To keep one green row per game on the detail page (the early return's stated purpose), remove an older row for the same id before adding the new one.

**Tech Stack:** Rust, refbox crate, tokio tests.

**Spec:** this plan; cause recorded in `docs/backlog/replayed-game-reposts-its-result-forever/NOTE.md` (master) and memory `project_replayed_game_reposts_forever.md`. PO approved "fix it now, small fix" 2026-11-27.

## Global Constraints

- MSRV 1.85, edition 2024; clippy `-D warnings`; no new dependencies; no `unwrap()` outside tests.
- Scope: `refbox/src/portal_manager/mod.rs` only, plus the backlog NOTE and this plan file. No change to `ItemId`, the queue file format, `health.rs`, or `app/mod.rs`.
- Every cargo job is a HEAVY job: coordinator GO first, `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4`.
- Branch: `fix/refbox/replayed-game-repost-loop`, own worktree off freshly fetched `origin/master`.

## Review Focus

1. **A late duplicate `ItemResolved` after the item is gone** must still be a no-op with no second green row. Already pinned by the existing test `on_item_resolved_with_unknown_id_is_noop_and_does_not_duplicate`, which must keep passing unchanged.
2. **Replay, then replay again** (three plays of one game): the queue empties each time and there is still exactly one green row for that game. Pinned in Task 1's test.
3. **The replayed game's row moves to the top** of the green list (newest first) rather than keeping the first play's position. Pinned in Task 1's test.
4. **The cap of 5 still holds** when a replay replaces a row: the list never exceeds `RECENT_SUCCESS_CAP` and no unrelated game is evicted by a replay. Pinned by `on_item_resolved_replay_at_the_cap_evicts_no_other_game` (added after the pre-PR review; see Deviations).
5. **Known and left open (PO chose the small fix):** an offline game replayed with a different score while BOTH results wait in the queue. Resolving the first removes both (`retain` matches by id), so the replay's result is silently lost. Not fixed here; record it in the backlog NOTE (Task 1, Step 6).

---

### Task 1: Resolve a replayed game's queue item

**Files:**
- Modify: `refbox/src/portal_manager/mod.rs` (`on_item_resolved`, ~line 1282; tests module)
- Modify: `docs/backlog/replayed-game-reposts-its-result-forever/NOTE.md` (status line + the open offline edge)

**Interfaces:** none new. `on_item_resolved(&mut self, id: ItemId)` keeps its signature.

- [ ] **Step 1: Write the failing test** in the tests module, next to `on_item_resolved_removes_queue_item_and_adds_recent_success`:

```rust
#[tokio::test]
async fn on_item_resolved_removes_a_replayed_games_item() {
    let tmp = tempfile::TempDir::new().unwrap();
    let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo, DiscardEpoch::default()).unwrap();

    // Game 3 played and sent, then another game sent in between.
    m.enqueue_game_end("e".into(), "3".into(), 0, 1, "[]".into()).unwrap();
    let id3 = m.queue.items[0].id.clone();
    m.on_item_resolved(id3.clone());
    m.enqueue_game_end("e".into(), "4".into(), 2, 2, "[]".into()).unwrap();
    let id4 = m.queue.items[0].id.clone();
    m.on_item_resolved(id4.clone());

    // Game 3 replayed twice; each replay's result is accepted.
    for _ in 0..2 {
        m.enqueue_game_end("e".into(), "3".into(), 1, 1, "[]".into()).unwrap();
        m.on_item_resolved(id3.clone());
        assert!(m.queue.items.is_empty(), "a replayed game's item must leave the queue once sent");
    }

    // One green row per game, the replay newest, nothing else evicted.
    assert_eq!(m.recent_successes.len(), 2);
    assert_eq!(m.recent_successes[0].id, id3);
    assert_eq!(m.recent_successes[1].id, id4);
    assert_eq!(m.indicator_state().health, HealthState::Green);
    // Persisted: a restart must not reload and re-send it.
    assert!(queue::load_or_empty(tmp.path()).unwrap().items.is_empty());
}
```

- [ ] **Step 2: Run it and watch it fail** (coordinator GO first)

Run: `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 cargo test -p refbox on_item_resolved_removes_a_replayed_games_item`
Expected: FAIL at "a replayed game's item must leave the queue once sent". This failure is the proof of the cause.

- [ ] **Step 3: Fix `on_item_resolved`.** Delete the early return on `self.recent_successes.iter().any(|rs| rs.id == id)` and its comment block. Keep the "not on the queue → return" check, and update its comment to say it also covers a duplicate resolution. Before `push_front`, add `self.recent_successes.retain(|rs| rs.id != id);` with a one-line comment: one green row per game, and a replay shows as the newest.

- [ ] **Step 4: Run the portal_manager tests**

Run: `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 cargo test -p refbox portal_manager`
Expected: PASS, including the new test and `on_item_resolved_with_unknown_id_is_noop_and_does_not_duplicate` unchanged.

- [ ] **Step 5: `just check`** (coordinator GO; `--test-threads=1` if ALSA tests flake, per memory). Expected: green.

- [ ] **Step 6: Update the backlog NOTE.** Status: fixed on this branch (cause: the recent-successes guard in `on_item_resolved`). Add the open offline-replay edge from Review Focus 5 as "known, not fixed". Also say the old `attempts: 0` clue was a red herring: attempt counts are deliberately never written to disk.

- [ ] **Step 7: Commit**

```bash
git add refbox/src/portal_manager/mod.rs docs/backlog/replayed-game-reposts-its-result-forever/NOTE.md docs/superpowers/plans/2026-11-27-replayed-game-repost-loop.md
git commit -m "fix(refbox): stop re-sending a replayed game's result"
```

## Verification (after the task)

- **Code review (check 1):** `code-review` skill on the branch diff, after rebasing on fresh master, plus `cargo audit`.
- **Manual walkthrough (check 2, PO):** refbox launched with its own `XDG_CONFIG_HOME` and `UWH_PORTAL_URL_OVERRIDE` pointing at the local mock portal (`docs/backlog/court-finished-panel-state/mock-portal/server.py`, copied into the worktree's scratch area). Play game 3, then return to game 3 and play it again. Watch that the portal dot goes back to green within ~20 s, and that the mock's post log shows ONE post for the replay, not one every 16 s. Book the refbox app with the coordinator first.
- **Claude-driven check (check 3):** not recommended. The mock's post log in the walk already shows the outcome, and the unit test pins the logic.

## Deviations

- **Pre-PR review (`code-review`, 2026-11-27) findings applied:** Review Focus 4 was claimed as pinned but Task 1's test never filled the list to the cap, so a second test now does. The plan's scope line and commit step now name this plan file. The backlog NOTE's "known, not fixed" edge is wider than Review Focus 5 says: it covers any moment a first play and its replay are both queued, including a first play stuck at "stats not sent" replayed online, plus a rare out-of-order queue-snapshot route. Same root cause, same remedy (a per-result identity), still not fixed here.
