# Degraded portal mode: no write target — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make it structurally impossible for a refbox session that could not read its portal queue
to write one, by replacing `config_dir: PathBuf` with `store: Option<QueueStore>` whose only
constructor requires a successful read.

**Architecture:** `QueueStore::open(dir)` returns the store *together with* the `QueueFile` it
loaded, so a write target cannot exist for a directory nobody read. `PortalManager` holds
`Option<QueueStore>`; the temp-directory fallback and `queue_dir()` are deleted. Three existing
guards survive because they are about *not being able to send*, which is a different fact from
*having nowhere to write*.

**Tech Stack:** Rust 2024, MSRV 1.85, `tokio` for the async tests, `tempfile` for test dirs,
`serde_json` for the on-disk format. No new dependencies.

**Spec:** `docs/superpowers/specs/2026-08-13-degraded-no-write-target-design.md` — read it first;
this plan argues from it.

## Global Constraints

- `refbox` crate only. Do not touch `uwh-common`, `overlay`, `wireless-remote`, or any file under
  `refbox/src/app/view_builders/`, and change no translation file.
- No new dependencies, no new on-disk file format, no UI changes. The inert detail rows keep
  exactly the appearance they have on master.
- Rust edition 2024, MSRV 1.85. `cargo clippy --workspace --all-targets --all-features -- -D
  warnings` must stay clean.
- No `unwrap()` / `expect()` in non-test code without a comment justifying it.
- **Tests must never write to the shared system temp directory.** `std::env::temp_dir()` is the
  healthy path's second-choice queue directory (`app/mod.rs:2424`), so a file planted there can be
  adopted and uploaded by a real session. Every test uses `tempfile::TempDir`.
- Heavy process applies (persistence of tournament results): every task ends with its own test run
  and commit; run `just check` before the final commit.
- Keep these three guards. Deleting any of them reintroduces the #2349 regression, because in the
  no-client route the disk works and the store exists:
  - `retry_all`'s `if self.startup_problem { … return Ok(()) }`
  - the `!self.startup_problem` gate around `sweep_expired` in `ui_tick`
  - the `has_startup_problem()` early return in `Message::PortalRowTapped` (`app/mod.rs:3492`)

---

## File Structure

| File | Responsibility after this work |
|---|---|
| `refbox/src/portal_manager/queue.rs` | The on-disk format **and** `QueueStore` — the only thing that can write it. Free read/write functions become `pub(super)`. |
| `refbox/src/portal_manager/mod.rs` | `PortalManager` holds `store: Option<QueueStore>`; all ten write sites go through one `persist()` funnel; owns the tenant-switch flush. |
| `refbox/src/app/mod.rs` | Constructs the manager; calls the flush by name instead of reaching for a directory. |

No files are created. No file is split — `queue.rs` is ~230 lines and gains ~40.

---

## Task 1: `QueueStore` — a write target you can only get by reading

**Files:**
- Modify: `refbox/src/portal_manager/queue.rs` (add after `append_to_archive`, ~line 183)
- Test: `refbox/src/portal_manager/queue.rs` (the existing `#[cfg(test)] mod tests`, ~line 185)

**Interfaces:**
- Consumes: existing private `queue_path`, `tmp_path`, `write_atomic`, `load_or_empty`,
  `append_to_archive`, `save` in this file.
- Produces: `pub(super) struct QueueStore` with
  `open(dir: &Path) -> std::io::Result<(Self, QueueFile)>`,
  `save(&self, q: &QueueFile) -> std::io::Result<()>`,
  `append_to_archive(&self, items: &[QueuedItem]) -> std::io::Result<()>`,
  `flush(&self) -> std::io::Result<()>`.

- [ ] **Step 1: Write the failing tests**

Add to the existing nested `mod load_save_tests` in `queue.rs` (~line 224) — it already has
`use super::*;` and `use tempfile::TempDir;`. Add this helper at the top of that module, since the
existing tests write the literal out longhand three times:

```rust
        fn one_item_queue(game: &str, black: u8, white: u8) -> QueueFile {
            QueueFile {
                version: 1,
                items: vec![QueuedItem {
                    id: ItemId {
                        event_id: "event".into(),
                        game_number: game.into(),
                    },
                    black_score: black,
                    white_score: white,
                    stats: "{}".into(),
                    queued_at: datetime!(2026-08-13 10:00 UTC),
                    attempts: 0,
                    last_attempt_at: None,
                    force: false,
                    score_sent: false,
                }],
            }
        }
```

Then the tests:

```rust
        #[test]
        fn store_open_hands_back_the_queue_it_loaded() {
            let tmp = TempDir::new().unwrap();
            let q = one_item_queue("G1", 3, 2);
            save(tmp.path(), &q).unwrap();

            let (_store, loaded) = QueueStore::open(tmp.path()).unwrap();
            assert_eq!(loaded, q, "open must return the queue it read, not an empty one");
        }

        #[test]
        fn store_open_on_a_missing_file_is_an_empty_queue() {
            let tmp = TempDir::new().unwrap();
            let (_store, loaded) = QueueStore::open(tmp.path()).unwrap();
            assert!(loaded.items.is_empty());
        }

        #[test]
        fn store_round_trips_and_flushes() {
            let tmp = TempDir::new().unwrap();
            let (store, _) = QueueStore::open(tmp.path()).unwrap();
            let q = one_item_queue("G2", 1, 0);

            store.save(&q).unwrap();
            assert_eq!(load_or_empty(tmp.path()).unwrap(), q);

            store.flush().unwrap();
            assert!(
                load_or_empty(tmp.path()).unwrap().items.is_empty(),
                "flush must leave an empty queue on disk"
            );
        }
```

And the one that pins the safety property (note `datetime!` is already imported by the parent
`mod tests`, which `use super::*` re-exports into this module):

```rust
    #[cfg(unix)]
    #[test]
    fn no_store_exists_for_a_queue_that_cannot_be_read() {
        // THE safety property: a directory whose queue file we cannot read
        // yields no write target at all, so nothing can overwrite it.
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::TempDir::new().unwrap();
        save(tmp.path(), &QueueFile::empty()).unwrap();
        let path = queue_path(tmp.path());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();

        let result = QueueStore::open(tmp.path());

        // Restore before asserting so a failure cannot leave an unreadable
        // file behind for the rest of the suite.
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(
            result.is_err(),
            "an unreadable queue file must not produce a write target"
        );
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p refbox --bins queue::tests::store 2>&1 | tail -20`
Expected: FAIL — `cannot find struct QueueStore in this scope`.

- [ ] **Step 3: Implement `QueueStore`**

Add to `queue.rs` after `append_to_archive`:

```rust
/// A directory this session successfully read the portal queue from, and is
/// therefore allowed to write back to.
///
/// `open` is the only constructor, and it returns the loaded queue along with
/// the store — so a `QueueStore` cannot exist for a directory we could not
/// read. That is the entire safety property: a session with no readable queue
/// holds no store, and a store is the only route to a queue write, so it
/// cannot destroy a file it never saw. `queue::save` renames over the target,
/// and a rename needs write permission on the *directory*, not the file, so an
/// unreadable-but-replaceable queue is exactly the case this prevents.
///
/// See `docs/superpowers/specs/2026-08-13-degraded-no-write-target-design.md`.
#[derive(Debug)]
pub(super) struct QueueStore {
    dir: PathBuf,
}

impl QueueStore {
    /// Read `dir`'s queue and, on success, return the write target for it.
    /// A missing file is a successful read of an empty queue (a first run);
    /// a corrupt one is rotated aside by `load_or_empty` and also succeeds.
    /// Only an I/O or permission failure yields `Err` — and therefore no
    /// write target.
    pub(super) fn open(dir: &Path) -> std::io::Result<(Self, QueueFile)> {
        let queue = load_or_empty(dir)?;
        Ok((
            Self {
                dir: dir.to_path_buf(),
            },
            queue,
        ))
    }

    pub(super) fn save(&self, q: &QueueFile) -> std::io::Result<()> {
        save(&self.dir, q)
    }

    pub(super) fn append_to_archive(&self, items: &[QueuedItem]) -> std::io::Result<()> {
        append_to_archive(&self.dir, items)
    }

    /// Clear the on-disk queue. Used by the portal-tenant switch, where items
    /// queued for the old tenant cannot be delivered to the new one.
    pub(super) fn flush(&self) -> std::io::Result<()> {
        self.save(&QueueFile::empty())
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p refbox --bins queue:: 2>&1 | tail -10`
Expected: PASS, all queue tests green (the four new ones plus the existing round-trip tests).

- [ ] **Step 5: Commit**

```bash
git add refbox/src/portal_manager/queue.rs
git commit -m "refactor(refbox): add a queue write target you can only get by reading"
```

---

## Task 2: `PortalManager` holds `Option<QueueStore>`; the temp-dir fallback goes

This is the task that changes behaviour. It must be atomic — the field cannot be half-migrated.

**Files:**
- Modify: `refbox/src/portal_manager/mod.rs` — field at ~425, `new_for_test` ~438, `sweep_expired`
  ~524, `new` ~610, `new_degraded` ~658, and the ten write sites at 536, 539, 796, 814, 869, 894,
  905, 947, 967, 1042
- Modify: `refbox/src/app/mod.rs:1955` and `:5887` — keep compiling via `queue_dir()`'s new
  `Option` return; Task 3 replaces them properly
- Test: `refbox/src/portal_manager/mod.rs` (`mod tests`)

**Interfaces:**
- Consumes: `QueueStore::{open, save, append_to_archive}` from Task 1.
- Produces: private `fn persist(&self) -> std::io::Result<()>` on `PortalManager`; field
  `store: Option<QueueStore>`; `fn queue_dir(&self) -> Option<&std::path::Path>` (temporary —
  deleted in Task 3).

- [ ] **Step 1: Write the failing tests**

Replace the whole body of the existing `degraded_never_overwrites_a_queue_it_could_not_read`
(~line 1694) with this, and rename it:

```rust
    #[cfg(unix)]
    #[tokio::test]
    async fn a_session_that_could_not_read_its_queue_writes_nothing_anywhere() {
        // The write target is the directory we read from, or there is none.
        // This session cannot read, so it must leave the file byte-identical,
        // create nothing beside it, and touch no other directory either.
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::TempDir::new().unwrap();
        {
            let (mut healthy, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();
            healthy
                .enqueue_game_end("event".into(), "G1".into(), 1, 0, "{}".into())
                .unwrap();
            healthy
                .enqueue_game_end("event".into(), "G2".into(), 2, 0, "{}".into())
                .unwrap();
        }
        let path = tmp.path().join("portal_queue.json");
        let before = std::fs::read(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();

        // The shared temp dir is the healthy path's SECOND-choice queue dir,
        // so prove we do not fall back to it. `None` vs `Some` also covers
        // "there was no file and there still isn't".
        let shared = std::env::temp_dir().join("portal_queue.json");
        let shared_before = std::fs::read(&shared).ok();

        {
            let (mut degraded, _rx) = PortalManager::new_degraded(tmp.path());
            assert!(
                degraded.store.is_none(),
                "a queue we could not read must leave this session with no write target"
            );
            let _ = degraded.enqueue_game_end("event".into(), "G3".into(), 3, 0, "{}".into());
            assert_eq!(
                degraded.queue.items.len(),
                1,
                "the result is still held in memory for the operator to read off the screen"
            );
        }

        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(
            std::fs::read(&path).unwrap(),
            before,
            "the unreadable queue file must be left byte-identical"
        );
        assert_eq!(
            std::fs::read(&shared).ok(),
            shared_before,
            "the shared system temp dir must not be used as a fallback"
        );
        let mut names: Vec<_> = std::fs::read_dir(tmp.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        names.sort();
        assert_eq!(
            names.len(),
            1,
            "no new file may appear beside the queue we could not read, got {names:?}"
        );
    }
```

Then add, to pin the other half of the rule:

```rust
    #[tokio::test]
    async fn a_session_that_cannot_send_still_saves_when_the_disk_is_fine() {
        // The no-client route: nothing can be uploaded, but the disk is
        // perfect, so results MUST still be recorded where a later healthy
        // start finds them. "Cannot send" is not "cannot save".
        let tmp = tempfile::TempDir::new().unwrap();
        {
            let (mut degraded, _rx) = PortalManager::new_degraded(tmp.path());
            assert!(
                degraded.store.is_some(),
                "a readable queue must still give this session a write target"
            );
            degraded
                .enqueue_game_end("event".into(), "G7".into(), 5, 4, "{}".into())
                .unwrap();
        }

        let (later, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();
        assert_eq!(later.queue.items.len(), 1, "a later healthy start must find it");
        assert_eq!(later.queue.items[0].id.game_number, "G7");
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p refbox --bins a_session_that 2>&1 | tail -20`
Expected: FAIL — `no field 'store' on type 'PortalManager'`.

- [ ] **Step 3: Migrate the field and all ten write sites**

In `refbox/src/portal_manager/mod.rs`:

1. Replace the field (~425):

```rust
    /// Where this session may write the queue — or `None` when it may not.
    ///
    /// A `QueueStore` can only be obtained by successfully reading a
    /// directory (`QueueStore::open`), so `None` means "we could not read the
    /// queue file, therefore we must never write over it". This absence is
    /// the safety property; it replaces six years of per-method guards. See
    /// the 2026-08-13 spec.
    store: Option<queue::QueueStore>,
```

Delete the `config_dir: PathBuf` field.

2. Add the funnel, next to `has_startup_problem`:

```rust
    /// Write the queue back, if this session has anywhere to write it.
    ///
    /// NOT a guard: a session without a store holds no directory at all, so
    /// there is nothing to guard against. This is only the shared funnel for
    /// the ten call sites — the safety comes from `Option<QueueStore>`,
    /// because the sole way to obtain a store is a successful read. A future
    /// mutator cannot bypass this the way it could bypass a boolean check:
    /// there is no path to write without a store in hand.
    fn persist(&self) -> std::io::Result<()> {
        match &self.store {
            Some(store) => store.save(&self.queue),
            None => Ok(()),
        }
    }
```

3. Replace every `queue::save(&self.config_dir, &self.queue)?` with `self.persist()?`, and each
   `if let Err(e) = queue::save(&self.config_dir, &self.queue) {` with
   `if let Err(e) = self.persist() {`. That is the sites at 796, 814, 869, 894, 905, 947, 967, 1042.

4. `sweep_expired` (~524) needs the store for the archive too. Replace its body's write section:

```rust
        // Archive BEFORE removing so a failed write never loses a score. With
        // no write target there is nothing to archive to and nothing on disk
        // to shrink, so leave the queue alone entirely: dropping items from
        // memory would lose them with no on-disk copy anywhere.
        let Some(store) = &self.store else {
            return Ok(());
        };
        store.append_to_archive(&expired)?;
        let n = expired.len();
        self.queue.items = kept;
        store.save(&self.queue)?;
```

5. `new` (~614): replace `let queue = queue::load_or_empty(config_dir)?;` with

```rust
        let (store, queue) = queue::QueueStore::open(config_dir)?;
```

and set `store: Some(store),` in the struct literal.

6. `new_degraded`: delete `new_degraded_with_fallback` entirely and make `new_degraded` the only
   constructor again, with this in place of the old load-and-fallback block:

```rust
        // Adopt whatever is already queued, and take a write target only if
        // that read succeeded. A failed read leaves this session with none:
        // `queue::save` renames over the file and a rename needs permission
        // on the directory rather than the file, so an unreadable-but-
        // replaceable queue would otherwise be destroyed by the first
        // enqueue. There is deliberately no second-choice directory — the
        // temp dir is the healthy path's own fallback, and writing a
        // fabricated queue there is how a real session ends up adopting one.
        let (queue, store) = match queue::QueueStore::open(config_dir) {
            Ok((store, queue)) => (queue, Some(store)),
            Err(e) => {
                log::warn!(
                    "could not read the portal queue ({e}); this session has no write target, so \
                     the existing file is left untouched and any result recorded now is held in \
                     memory only"
                );
                (QueueFile::empty(), None)
            }
        };
```

7. `new_for_test` (~438): replace the `config_dir` line with `store: None,` and this comment:

```rust
            // No write target: these managers are for indicator and row-
            // ordering assertions, none of which persists. A test that needs a
            // save must build its own via `QueueStore::open` over a `TempDir`
            // — never a path in the shared system temp dir, which is the
            // healthy path's second-choice queue directory.
            store: None,
```

8. `queue_dir` (~474) becomes temporary and honest until Task 3:

```rust
    pub fn queue_dir(&self) -> Option<&std::path::Path> {
        self.store.as_ref().map(|s| s.dir())
    }
```

Add to `QueueStore` in `queue.rs`:

```rust
    pub(super) fn dir(&self) -> &Path {
        &self.dir
    }
```

9. In `app/mod.rs`, wrap both flush sites (1954 and 5886) so they compile against the `Option`:

```rust
                if old_source == GameSource::Portal {
                    if let Some(dir) = self.portal_manager.queue_dir() {
                        if let Err(e) = crate::portal_manager::queue::save(
                            dir,
                            &crate::portal_manager::queue::QueueFile::empty(),
                        ) {
                            error!("Failed to flush portal queue before restart: {e}");
                        }
                    }
                }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p refbox --bins 2>&1 | tail -5`
Expected: PASS — all tests green, including the untouched `degraded_retry_all_is_inert_and_does_not_rewrite_ageing`, `degraded_ui_tick_does_not_archive_expired_items`, `degraded_construction_neither_invents_nor_replaces_a_queue`, `degraded_enqueue_does_not_clobber_results_already_queued` and `degraded_queue_is_found_by_a_later_healthy_start`. If any of those five fail, STOP: the redesign has changed a behaviour it was supposed to preserve.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/portal_manager/mod.rs refbox/src/portal_manager/queue.rs refbox/src/app/mod.rs
git commit -m "refactor(refbox): a session with no readable queue gets no write target"
```

---

## Task 3: the tenant-switch flush becomes a method; `queue_dir()` is deleted

**Files:**
- Modify: `refbox/src/portal_manager/mod.rs` — delete `queue_dir`, add
  `flush_queue_for_tenant_switch`
- Modify: `refbox/src/app/mod.rs:1954` and `:5886`
- Test: `refbox/src/portal_manager/mod.rs` (`mod tests`)

**Interfaces:**
- Consumes: `QueueStore::flush` (Task 1), `store` field (Task 2).
- Produces: `pub fn flush_queue_for_tenant_switch(&mut self) -> std::io::Result<()>`. `queue_dir`
  and `QueueStore::dir` no longer exist.

- [ ] **Step 1: Write the failing tests**

```rust
    #[tokio::test]
    async fn tenant_switch_flush_clears_a_queue_this_session_owns() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();
        m.enqueue_game_end("old-tenant".into(), "G1".into(), 1, 0, "{}".into())
            .unwrap();

        m.flush_queue_for_tenant_switch().unwrap();

        assert!(m.queue.items.is_empty(), "in memory too, not just on disk");
        let (_store, on_disk) = queue::QueueStore::open(tmp.path()).unwrap();
        assert!(on_disk.items.is_empty(), "the on-disk queue must be cleared");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn tenant_switch_flush_writes_nothing_without_a_write_target() {
        // The old code flushed through `queue_dir()`, bypassing every guard —
        // in the fallback state it wrote to the shared temp dir, clearing a
        // queue belonging to a different session and a different tenant.
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::TempDir::new().unwrap();
        {
            let (mut healthy, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();
            healthy
                .enqueue_game_end("event".into(), "G1".into(), 1, 0, "{}".into())
                .unwrap();
        }
        let path = tmp.path().join("portal_queue.json");
        let before = std::fs::read(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
        let shared = std::env::temp_dir().join("portal_queue.json");
        let shared_before = std::fs::read(&shared).ok();

        {
            let (mut degraded, _rx) = PortalManager::new_degraded(tmp.path());
            degraded.flush_queue_for_tenant_switch().unwrap();
        }

        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(
            std::fs::read(&path).unwrap(),
            before,
            "a session with no write target must not clear a queue it never owned"
        );
        assert_eq!(std::fs::read(&shared).ok(), shared_before);
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p refbox --bins tenant_switch_flush 2>&1 | tail -20`
Expected: FAIL — `no method named 'flush_queue_for_tenant_switch'`.

- [ ] **Step 3: Implement the method and delete `queue_dir`**

In `mod.rs`, delete `queue_dir` and add:

```rust
    /// Clear the retry queue for a portal-tenant switch: results queued under
    /// the old tenant cannot be delivered to the new one, so the restarted app
    /// must not carry them over. Only ever called for the built-in portal —
    /// a custom site is one address the operator typed, and results queued for
    /// it stay deliverable to exactly that site after a restart.
    ///
    /// A session with no write target does nothing: it never owned a queue
    /// file, so there is nothing of its own to clear and clearing someone
    /// else's would destroy real results.
    pub fn flush_queue_for_tenant_switch(&mut self) -> std::io::Result<()> {
        self.queue.items.clear();
        if let Some(store) = &self.store {
            store.flush()?;
        }
        self.recompute_indicator();
        Ok(())
    }
```

Delete `QueueStore::dir` from `queue.rs` — nothing needs it now.

In `app/mod.rs`, replace both flush blocks (the one added in Task 2 at ~1954, and ~5886) with:

```rust
                if old_source == GameSource::Portal {
                    if let Err(e) = self.portal_manager.flush_queue_for_tenant_switch() {
                        error!("Failed to flush portal queue before restart: {e}");
                        // Continue with restart — the operator pressed Restart and we
                        // must not block. The queue will be treated as stale items for
                        // the new tenant, which the retry logic will eventually discard.
                    }
                }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p refbox --bins 2>&1 | tail -5`
Expected: PASS, whole suite green.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/portal_manager/mod.rs refbox/src/portal_manager/queue.rs refbox/src/app/mod.rs
git commit -m "refactor(refbox): flush the queue through the manager, not a borrowed path"
```

---

## Task 4: re-ending a game replaces its queued result

**Files:**
- Modify: `refbox/src/portal_manager/mod.rs` — `enqueue_game_end` (~773, the `push` at ~795)
- Test: `refbox/src/portal_manager/mod.rs` (`mod tests`)

**Interfaces:**
- Consumes: nothing new.
- Produces: no signature change — `enqueue_game_end` keeps
  `(&mut self, String, String, u8, u8, String) -> std::io::Result<()>`.

- [ ] **Step 1: Write the failing tests**

```rust
    #[tokio::test]
    async fn re_ending_a_game_replaces_its_queued_result() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();
        m.enqueue_game_end("event".into(), "G1".into(), 1, 0, "{}".into())
            .unwrap();
        m.enqueue_game_end("event".into(), "G1".into(), 2, 1, "{}".into())
            .unwrap();

        assert_eq!(
            m.queue.items.len(),
            1,
            "a re-ended game must replace its entry, not stack a second one"
        );
        assert_eq!(
            (m.queue.items[0].black_score, m.queue.items[0].white_score),
            (2, 1),
            "the later result wins — it is the corrected score"
        );
    }

    #[tokio::test]
    async fn resolving_a_re_ended_game_leaves_nothing_behind() {
        // `discard`/`on_item_resolved` remove by id with `retain`, which drops
        // EVERY match — so with duplicates queued, resolving one silently
        // discarded the other, unsent.
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();
        m.enqueue_game_end("event".into(), "G1".into(), 1, 0, "{}".into())
            .unwrap();
        m.enqueue_game_end("event".into(), "G1".into(), 2, 1, "{}".into())
            .unwrap();

        let id = m.queue.items[0].id.clone();
        m.discard(&id).unwrap();

        assert!(
            m.queue.items.is_empty(),
            "no orphaned duplicate may survive the removal"
        );
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p refbox --bins re_ending_a_game 2>&1 | tail -20`
Expected: FAIL — `assertion failed: left: 2, right: 1` (two entries queued).

- [ ] **Step 3: Replace instead of push**

In `enqueue_game_end`, replace `self.queue.items.push(item);` with:

```rust
        // A game can legitimately be re-ended: a corrected score, or an
        // operator re-running the same game number. Replace the existing entry
        // rather than stacking a second one. Two reasons this matters: a
        // session that cannot send never drains its queue, so duplicates
        // accumulate unbounded; and `discard`/`on_item_resolved` remove by id
        // with `retain`, which drops every match at once — so resolving one
        // copy silently discarded the other, unsent. The later result wins.
        if let Some(existing) = self.queue.items.iter_mut().find(|it| it.id == item.id) {
            *existing = item;
        } else {
            self.queue.items.push(item);
        }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p refbox --bins 2>&1 | tail -5`
Expected: PASS, whole suite green.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/portal_manager/mod.rs
git commit -m "fix(refbox): re-ending a game replaces its queued result"
```

---

## Task 5: seal the module and prove the shared temp dir stays clean

**Files:**
- Modify: `refbox/src/portal_manager/mod.rs:9` (`pub mod queue;`)
- Modify: `refbox/src/portal_manager/queue.rs` — visibility of `load_or_empty`, `save`,
  `append_to_archive`, `load_archive_or_empty`

**Interfaces:**
- Consumes: everything above.
- Produces: nothing new. `queue` is no longer reachable from `app`.

- [ ] **Step 1: Seal it**

`app/mod.rs` no longer references `crate::portal_manager::queue` at all after Task 3 — verify
first, then narrow the visibility so it cannot come back:

```bash
grep -rn "portal_manager::queue" refbox/src/ --include=*.rs | grep -v "^refbox/src/portal_manager/"
```

Expected: no output. Then in `mod.rs:9` change `pub mod queue;` to `mod queue;`, and in `queue.rs`
change `pub fn load_or_empty`, `pub fn save`, `pub fn append_to_archive` and
`pub fn load_archive_or_empty` to `pub(super) fn …`. Leave `QueueFile`, `QueuedItem` and their
fields `pub` — the background task and the snapshot channel use them inside this module tree.

- [ ] **Step 2: Verify it compiles and the suite is green**

Run: `cargo test -p refbox --bins 2>&1 | tail -5`
Expected: PASS. A compile error here means something outside `portal_manager` still reaches for the
queue — find it rather than widening the visibility back.

- [ ] **Step 3: Prove no test writes to the shared temp directory**

Both directions, exactly as #2349 established — a delete-guard is not an overwrite-guard, so check
content, not existence:

```bash
printf '{"sentinel":1}' > /tmp/portal_queue.json
md5sum /tmp/portal_queue.json
cargo test -p refbox --bins 2>&1 | tail -3
md5sum /tmp/portal_queue.json
rm -f /tmp/portal_queue.json
cargo test -p refbox --bins 2>&1 | tail -3
ls -la /tmp/portal_queue.json /tmp/portal_queue.json.tmp /tmp/portal_queue.expired.json
```

Expected: the two md5sums identical, and the final `ls` reports all three as missing.

- [ ] **Step 4: Full gate**

Run: `just check`
Expected: exit 0. Fix any clippy finding rather than allowing it.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/portal_manager/mod.rs refbox/src/portal_manager/queue.rs
git commit -m "refactor(refbox): seal the queue module inside portal_manager"
```

---

## Acceptance criteria check (from the spec)

Map each spec criterion to where it is proven, and confirm before opening the PR:

| # | Criterion | Proven by |
|---|---|---|
| 1 | Unreadable-queue session creates no file anywhere | `a_session_that_could_not_read_its_queue_writes_nothing_anywhere` + Task 5 Step 3 |
| 2 | No-client session still saves; later healthy start finds it | `a_session_that_cannot_send_still_saves_when_the_disk_is_fine`, plus the preserved `degraded_queue_is_found_by_a_later_healthy_start` |
| 3 | The unreadable file is byte-identical | same test, `assert_eq!(read(&path), before)` |
| 4 | Tenant switch writes nothing without a target; still clears normally | `tenant_switch_flush_writes_nothing_without_a_write_target`, `tenant_switch_flush_clears_a_queue_this_session_owns` |
| 5 | Re-ending leaves one entry; removal leaves nothing | `re_ending_a_game_replaces_its_queued_result`, `resolving_a_re_ended_game_leaves_nothing_behind` |
| 6 | The three send-ability guards still hold | existing `degraded_retry_all_is_inert_and_does_not_rewrite_ageing`, `degraded_ui_tick_does_not_archive_expired_items`, unchanged `PortalRowTapped` gate |
| 7 | `just check` exit 0 | Task 5 Step 4 |
| 8 | No view_builder or translation file touched | `git diff --stat origin/master` before the PR |

## Deviations

Record anything that diverged from this plan here rather than in separate commits, per
`.claude/rules/plan-execution.md`.
