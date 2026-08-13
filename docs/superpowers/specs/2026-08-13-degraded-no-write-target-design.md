# Degraded portal mode: no write target by construction

**Date:** 2026-08-13
**Crate:** `refbox` only
**Status:** approved design, not yet implemented
**Follows:** PR #2319 (startup message), #2336 (degraded queue persistence), #2348 (closed
unmerged — redesign chosen over a seventh guard), #2349 (RETRY ALL false-success regression)

---

## Why this exists

Four review rounds in this area have each found the same class of defect in the previous round's
own fix: a session whose portal subsystem failed to start could write — and destroy — queue data.
Each round added another per-method guard. Four guards now exist in four different shapes across
two layers — `if !self.startup_problem { … }` wrapping the body in `ui_tick`, an early
`return Ok(())` in `retry_all`, an early return in the app's `PortalRowTapped` handler, and a
bool passed into the view — while ten write sites and several mutators have none. The fourth
round found a hole inside the third round's own fix.

The decision recorded here is to stop guarding methods and remove the capability instead: a
session that could not read a queue file has **nowhere to write**, enforced by the type system
rather than by a check each new method must remember to repeat.

## What was wrong with "degraded mode" as a concept

The existing code treats one flag, `startup_problem`, as meaning both "cannot send" and "cannot
save". They are different failures with different correct behaviours, and conflating them is the
root of the bug class.

| Route into degraded mode | What actually broke | Disk usable? |
|---|---|---|
| No portal client (`app/mod.rs:2404`) | `UwhPortalClient::new` failed — realistically a TLS/certificate-store problem | **Yes, fully** |
| Both directories failed (`app/mod.rs:2433`) | Queue unreadable in the config dir *and* the temp dir | No |
| Queue load failed inside `new_degraded` | The file we would be replacing could not be read | Not safely |

Two consequences of reading this properly:

- In the no-client route the disk is perfect. That session should persist results exactly as a
  healthy one does; only the uploading is missing. This is what #2336 established and it stays.
- An unreadable config-dir queue **with** a working client does not reach degraded mode at all: it
  falls to the temp dir, where there is normally no queue file, so it starts *healthy* pointed at
  the temp dir. Reaching degraded mode with a failed load therefore requires two independent
  faults at once.

## Reachability, recorded so nobody over-invests

Degraded mode has never been observed in the field. Corruption — the plausible cause — is already
handled elsewhere: `queue::load_or_empty` renames a corrupt or wrong-version file to
`portal_queue.corrupt.<ts>.json` and returns an empty queue as **success**. So the "unreadable"
path needs a permission or hardware I/O error on the refbox's own config directory, coinciding
with a failure that reaches degraded mode in the first place.

This work is insurance against silently losing tournament results, not a response to an incident.
Keep it proportionate: no new UI, no new on-disk format, no new dependencies.

---

## The design

### The invariant as a type

```rust
pub(crate) struct QueueStore { dir: PathBuf }

impl QueueStore {
    /// The ONLY constructor. Returns the store together with the queue it
    /// loaded, so a store cannot exist unless this directory was read.
    pub(crate) fn open(dir: &Path) -> std::io::Result<(Self, QueueFile)>;

    pub(crate) fn save(&self, q: &QueueFile) -> std::io::Result<()>;
    pub(crate) fn append_to_archive(&self, items: &[QueuedItem]) -> std::io::Result<()>;
    /// Writes an empty queue. For the portal-tenant switch.
    pub(crate) fn flush(&self) -> std::io::Result<()>;
}
```

`PortalManager` holds `store: Option<QueueStore>` in place of `config_dir: PathBuf`. Because
`open` is the only constructor and it hands back the loaded queue, "only ever write to the
directory you successfully read from" is true of the program's structure, not of its discipline.

`queue.rs`'s free functions `save`, `load_or_empty`, `append_to_archive` and
`load_archive_or_empty` become private to the `portal_manager` module. `QueueFile`, `QueuedItem`
and `ItemId` stay public — the background task and the app snapshot them.

### What this removes

- **The temp-directory fallback, entirely.** With it goes `new_degraded_with_fallback`, the
  `new_for_test` non-existent-path guard, and the possibility of any session or test writing to
  the shared temp dir at all.
- **`queue_dir()`**, and with it the tenant-switch flush's ability to bypass the manager
  (`app/mod.rs:1955` and `:5887`). The flush becomes `store.flush()`, so a store-less session
  writes nothing and no session can flush a directory it does not own.
- **The need for a guard on every mutator that never got one.** `token_refreshed` was identified
  in review round 3 as an ungated mutator reachable in the client-present route, and the two
  background-task callbacks (`on_score_sent_stats_pending`, `on_item_resolved`) write unguarded as
  well. None of them needs a guard after this: with no store they cannot write, and with no
  background task the callbacks never fire. This is the hole the next review round would have
  found.

### What this deliberately keeps

Three guards survive, all justified by the *same* single reason — nothing can be sent — rather
than six ad-hoc ones:

| Guard | Why it is not about writing |
|---|---|
| `retry_all` | Flips red rows to yellow "attempt 0" and rewrites every result's age, reporting a success no background task can deliver |
| the expiry sweep in `ui_tick` | Archives results into a file the UI never shows, when the session was never able to send them |
| `Message::PortalRowTapped` | Offers a Retry that cannot run and a Discard that deletes permanently |

These matter *most* in the no-client route, where the store exists and writes succeed. Deleting
them would reintroduce #2349 exactly.

### The duplicate-game defect

`enqueue_game_end` appends unconditionally. In a session that never drains its queue, re-ending
the same game stacks duplicate entries; `discard` and `on_item_resolved` then use
`retain(|it| it.id != id)`, which removes **every** copy — so resolving one silently drops the
other, unsent.

Fix: enqueueing a game that is already queued replaces the existing entry. The later result wins,
which is correct for a game re-ended with a corrected score.

### Accepted behaviour change

Today an unreadable-queue session writes to the temp directory, where a later healthy start might
adopt the results because the temp dir is its second choice. After this change that session writes
nothing, and that recovery path is gone.

Accepted deliberately: #2349 established the same write can destroy a *different* session's real
queue, and adopting a stranger's temp-dir queue is itself a hazard. A theoretical recovery is
traded for a real destruction risk. Approved by Eric 2026-08-13.

---

## Acceptance criteria

Observable or runnable, not internal:

1. A session that cannot read its queue file creates **no file anywhere** — not in the config
   directory, not in the system temp directory. Verifiable by planting a sentinel at
   `$TMPDIR/portal_queue.json` and confirming it is untouched, and by listing the config dir.
2. A no-client session with a working disk still records finished games to the real queue file,
   and a subsequent healthy start finds and uploads them. (Unchanged from #2336 — this must not
   regress.)
3. The unreadable queue file is left byte-identical.
4. A portal-tenant switch in a store-less session writes nothing; in a normal session it still
   clears the queue as it does today.
5. Re-ending the same game leaves one entry holding the later score; removing that game leaves
   nothing behind.
6. RETRY ALL, the expiry sweep, and row taps remain inert whenever nothing can be sent — the
   #2349 guarantees, still proven by their existing tests.
7. `just check` exit 0, all existing tests green.
8. No file under `refbox/src/app/view_builders/` is modified, and no translation file changes.

## Out of scope

- Any UI change, including the inert rows' appearance. They keep exactly the styling they have on
  master. If that is revisited it gets its own decision — the `red_button_armed` /
  `yellow_button_armed` option is recorded in `docs/backlog/` and in memory, not here.
- `uwh-common`, the wire format, the LED panel, the overlay, the wireless remote.
- The healthy path's directory selection (config dir, then temp dir). Only the *degraded*
  fallback is removed.
- `is_item_stuck` keying purely on age without asking whether an item was ever attempted —
  already backlogged at `docs/backlog/untried-result-labelled-as-send-error/NOTE.md`.
- Concurrent refbox processes sharing a config dir. One-refbox-at-a-time remains the standing
  rule.

## Testing approach

The type-level facts cannot be unit-tested — being compile errors is the point. Tests cover the
behavioural consequences, one per acceptance criterion above, plus:

- A test that a store-less manager's mutators leave the config directory empty, driven through
  `enqueue_game_end` (the one mutator genuinely reachable in that state).
- Tests must never write to the shared system temp directory. `QueueStore::open` over a
  `TempDir` is the only pattern; `new_for_test` gets `store: None` or a `TempDir`-backed store,
  never a path in the shared temp dir. A full `cargo test` must leave a planted sentinel at
  `$TMPDIR/portal_queue.json` byte-identical, and create nothing when none is present.

## Files expected to change

| File | Change |
|---|---|
| `refbox/src/portal_manager/queue.rs` | Add `QueueStore`; make the free read/write functions module-private |
| `refbox/src/portal_manager/mod.rs` | `config_dir: PathBuf` → `store: Option<QueueStore>`; route all ten write sites through it; delete the temp-dir fallback, `new_degraded_with_fallback`, `queue_dir()`; dedupe `enqueue_game_end` |
| `refbox/src/app/mod.rs` | Construction at ~2397–2439; the two flush sites at ~1955 and ~5887 |

No other files. `refbox` is a binary crate, so all of this is internal — nothing outside it can
observe the type change.
