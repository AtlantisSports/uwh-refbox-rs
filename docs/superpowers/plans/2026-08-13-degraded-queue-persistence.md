# Degraded Queue Persistence — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Results recorded while the portal subsystem is dead must survive to the next healthy launch instead of being written where nothing will ever read them.

**Architecture:** `PortalManager::new_degraded` stops hard-coding `std::env::temp_dir()` and takes the real config directory. Critically, it also **loads any queue already there** — because `queue::save` overwrites the whole file, so without the load the first enqueue would destroy results already waiting. Both production call sites pass the config directory they already have in scope.

**Tech Stack:** Rust 2024, MSRV 1.85, `tokio`, `serde_json` (queue file).

## Global Constraints

- **Crate scope:** `refbox` only. Do NOT touch `uwh-common`, `overlay`, or `wireless-remote`.
- **Process:** lean, per `.claude/rules/plan-execution.md` — no per-task deviation commits, one code review at the end. Record deviations in the "Deviations" section at the bottom of this file.
- **Clippy:** `cargo clippy --workspace --all-features -- -D warnings` clean. No new `#[allow]`.
- **`refbox` is a BIN-ONLY crate.** `cargo test -p refbox --lib` FAILS with "no library targets found". Always use `cargo test -p refbox --bin refbox`.
- **A degraded manager must never fail to construct.** `new_degraded` must NOT return `Result`. Every I/O it does is best-effort — the whole point of degraded mode is that the game can still run.
- **No `unwrap()`/`expect()`** in non-test code. `unwrap_or_else` with a logged fallback is the required shape here.
- **Do NOT copy `sweep_expired()` from `new()`** into `new_degraded`. It writes; degraded construction stays otherwise read-only. See the spec.
- **Do NOT change what "stuck" means** (`is_item_stuck`). Eric ruled that a separate concern on 2026-08-13; it has its own backlog note.
- **Branch:** `fix/refbox/degraded-queue-persistence`, already created off `origin/master` (`1846c549`). Ask the human before every commit.

---

### Task 1: `new_degraded` takes a directory and adopts the queue already there

This is one atomic task: the signature change, the load, and both call sites must land together or the crate does not compile.

**Files:**
- Modify: `refbox/src/portal_manager/mod.rs` — `new_degraded` (~`:624-649`), tests (~`:1458`)
- Modify: `refbox/src/app/mod.rs` — the two `new_degraded()` call sites (~`:2404`, ~`:2433`)

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: `pub(crate) fn new_degraded(config_dir: &std::path::Path) -> (Self, mpsc::Receiver<PortalEvent>)`. Still infallible — no `Result`. Existing fields and `indicator_state()` are unchanged.

- [ ] **Step 1: Write the three failing tests**

Add to the `mod tests` block in `refbox/src/portal_manager/mod.rs`, next to the existing degraded-mode tests (~`:1458`). `NullIo` and `mk_stuck_item()` are existing test helpers in that module.

```rust
    #[tokio::test]
    async fn degraded_queue_is_found_by_a_later_healthy_start() {
        let tmp = tempfile::TempDir::new().unwrap();

        // A degraded session records a result.
        {
            let (mut degraded, _rx) = PortalManager::new_degraded(tmp.path());
            degraded
                .enqueue_game_end("event".into(), "G1".into(), 3, 2, "{}".into())
                .unwrap();
        }

        // Once the fault is fixed, the next healthy launch must find it.
        let (healthy, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();
        assert_eq!(
            healthy.queue.items.len(),
            1,
            "a result queued in degraded mode must survive into the next healthy session"
        );
        assert_eq!(healthy.queue.items[0].id.game_number, "G1");
    }

    #[tokio::test]
    async fn degraded_enqueue_does_not_clobber_results_already_queued() {
        // THE TRAP. `queue::save` rewrites the whole file, so a degraded
        // manager that starts from an empty queue would overwrite results
        // already waiting. This test fails on that naive version.
        let tmp = tempfile::TempDir::new().unwrap();

        // Two results are already waiting from an earlier healthy session.
        {
            let (mut healthy, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();
            healthy
                .enqueue_game_end("event".into(), "G1".into(), 1, 0, "{}".into())
                .unwrap();
            healthy
                .enqueue_game_end("event".into(), "G2".into(), 2, 0, "{}".into())
                .unwrap();
        }

        // The refbox restarts into degraded mode and a third game finishes.
        {
            let (mut degraded, _rx) = PortalManager::new_degraded(tmp.path());
            degraded
                .enqueue_game_end("event".into(), "G3".into(), 3, 0, "{}".into())
                .unwrap();
        }

        let on_disk = queue::load_or_empty(tmp.path()).unwrap();
        let mut games: Vec<&str> = on_disk
            .items
            .iter()
            .map(|i| i.id.game_number.as_str())
            .collect();
        games.sort_unstable();
        assert_eq!(
            games,
            vec!["G1", "G2", "G3"],
            "a degraded session must never destroy results already queued"
        );
    }

    #[test]
    fn degraded_construction_neither_invents_nor_replaces_a_queue() {
        // Degraded construction may now READ the queue — that is the change.
        // What it must not do is create a queue file where none existed, or
        // replace one that is already there.
        let empty_dir = tempfile::TempDir::new().unwrap();
        let (manager, _rx) = PortalManager::new_degraded(empty_dir.path());
        assert_eq!(manager.config_dir.as_path(), empty_dir.path());
        assert!(
            !empty_dir.path().join("portal_queue.json").exists(),
            "degraded construction must not create a queue file"
        );

        let seeded_dir = tempfile::TempDir::new().unwrap();
        queue::save(
            seeded_dir.path(),
            &QueueFile {
                version: 1,
                items: vec![mk_stuck_item()],
            },
        )
        .unwrap();
        let (adopted, _rx) = PortalManager::new_degraded(seeded_dir.path());
        assert_eq!(
            adopted.queue.items.len(),
            1,
            "degraded construction must adopt the queue that is already there"
        );
        assert_eq!(
            queue::load_or_empty(seeded_dir.path()).unwrap().items.len(),
            1,
            "and must leave it on disk untouched"
        );
    }
```

- [ ] **Step 2: Run the tests and verify they fail**

Run: `cargo test -p refbox --bin refbox degraded -- --nocapture`
Expected: **compile error** — `new_degraded` takes 0 arguments but 1 was supplied. A compile failure is a legitimate red here; do not weaken the tests to make them build.

- [ ] **Step 3: Change `new_degraded` to take a directory and adopt the queue**

Replace the signature, the doc comment, and the `queue`/`config_dir` fields (~`:624-649`). Everything else in the function stays as it is.

```rust
    /// Constructs a `PortalManager` with no background task, used when the
    /// portal subsystem cannot start: either the HTTP client failed to build,
    /// or the retry queue was unreadable in both the config dir and the
    /// system temp dir. The refbox's core game functions still work and the
    /// portal indicator shows Red so the operator sees the problem.
    ///
    /// No background task is spawned: there's nothing for it to do, and
    /// spawning one with `NullIo` would report success for every call, clear
    /// the red state and fake-resolve every queued game.
    ///
    /// `config_dir` is the real config directory, not a scratch path. Results
    /// are still queued in this mode (`enqueue_game_end` runs whenever an
    /// event is linked), so they must land where the next healthy launch will
    /// look for them — otherwise they are silently abandoned. Writing
    /// elsewhere is what this constructor used to do, back when its only
    /// caller was the both-directories-failed case.
    ///
    /// The returned receiver is a dummy that never emits events.
    pub(crate) fn new_degraded(
        config_dir: &std::path::Path,
    ) -> (Self, mpsc::Receiver<PortalEvent>) {
        // Build (sender, receiver) pairs where the senders go nowhere:
        // the event-channel sender is discarded so the returned receiver
        // never emits, and the command-channel sender is kept on the
        // manager only because its type demands it — no background task
        // exists to receive from it.
        let (_, rx) = mpsc::channel(1);
        let (command_tx, _command_rx) = mpsc::channel(1);

        // Adopt whatever is already queued, best-effort. This must never fail
        // the construction: one trigger for degraded mode IS an unreadable
        // queue file, and the game has to keep running regardless.
        //
        // Skipping this load would NOT be the safe option: `queue::save`
        // rewrites the whole file, so an empty starting queue means the first
        // enqueue overwrites — and destroys — results already waiting.
        let queue = queue::load_or_empty(config_dir).unwrap_or_else(|e| {
            log::warn!(
                "could not read the portal queue in degraded mode ({e}); starting with an empty queue"
            );
            QueueFile::empty()
        });

        let mut m = Self {
            queue,
            check_in_flight: false,
            // The portal subsystem never started. Red so the operator sees
            // the problem — but NOT `token_known_problem`: nothing here is
            // evidence the login expired.
            token_known_problem: false,
            connection_problem: false,
            startup_problem: true,
            indicator_state: PortalIndicatorState::default(),
            command_tx,
            config_dir: config_dir.to_path_buf(),
            recent_successes: VecDeque::new(),
        };
        m.recompute_indicator();
        (m, rx)
    }
```

Note there is deliberately **no** `sweep_expired()` call here, unlike `new()`. Sweeping writes; anything genuinely expired is swept by the next healthy start, which is the only session that could have uploaded it anyway.

- [ ] **Step 4: Update both call sites**

In `refbox/src/app/mod.rs`, both sites already have `config_dir` in scope. Change:

```rust
PortalManager::new_degraded()
```

to:

```rust
PortalManager::new_degraded(&config_dir)
```

at **both** ~`:2404` (the no-client branch) and ~`:2433` (the both-directories-failed branch).

Passing the config dir at the second site is intentional even though its I/O just failed: the load fails again, gets logged, and the session runs on an in-memory queue — which is exactly what happens today, minus the pointless write to a directory nothing reads.

- [ ] **Step 5: Update the four existing degraded tests — and give each its OWN temp directory**

PR #2319 left four tests that call `new_degraded()` with no argument, so they will not compile after Step 3. They are all `#[test]` (synchronous — they only construct and inspect, so they need no tokio runtime):

- `degraded_startup_is_red` (~`:1117`)
- `degraded_startup_does_not_report_token_expired` (~`:1133`)
- `degraded_startup_shows_startup_failed_row_not_token_expired` (~`:1142`)
- `new_degraded_is_red_with_no_spawned_task` (~`:1434`)

In each, add a fresh temp directory and pass it:

```rust
        let dir = tempfile::TempDir::new().unwrap();
        let (m, _rx) = PortalManager::new_degraded(dir.path());
```

(`new_degraded_is_red_with_no_spawned_task` binds `(manager, mut rx)` — keep its own binding names.)

**Do NOT pass `std::env::temp_dir()`, even though that is what the constructor used to hard-code.** The constructor now *reads* the queue file, so a shared directory makes these tests depend on whatever is lying around in the system temp directory. `new_degraded_is_red_with_no_spawned_task` asserts the queue is empty and would fail outright if a stale `portal_queue.json` were present. A per-test `TempDir` is required, not stylistic.

- [ ] **Step 6: Delete the old `new_degraded_does_not_touch_disk` test**

It asserts `manager.config_dir == std::env::temp_dir()` — the precise behaviour being removed — so it cannot be kept. `degraded_construction_neither_invents_nor_replaces_a_queue` from Step 1 replaces it and tests the real safety property rather than a proxy for it.

- [ ] **Step 7: Run the full refbox suite**

Run: `cargo test -p refbox --bin refbox`
Expected: PASS, with no remaining reference to `new_degraded_does_not_touch_disk`.

- [ ] **Step 8: Mutation-test the clobber guard**

Required, not optional — this is the test protecting against data destruction.

Temporarily replace the `let queue = queue::load_or_empty(...)` block with `let queue = QueueFile::empty();` (the naive fix). Run:

`cargo test -p refbox --bin refbox degraded_enqueue_does_not_clobber`

Expected: **FAIL**, reporting `["G3"]` instead of `["G1", "G2", "G3"]`. Then restore the load and confirm it passes again. If the test passes with the mutation applied, it is not protecting anything — fix the test before continuing.

- [ ] **Step 9: Check clippy and formatting**

Run: `cargo clippy -p refbox --all-features 2>&1 | grep -E "^(warning|error)"; cargo fmt --all --check`
Expected: no output from either.

- [ ] **Step 10: Commit** (ask the human first)

```bash
git add refbox/src/portal_manager/mod.rs refbox/src/app/mod.rs
git commit -m "fix(refbox): keep degraded-mode results where a healthy start will find them"
```

---

### Task 2: Documentation and final validation

**Files:**
- Already modified in the working tree (uncommitted, carried onto this branch):
  `docs/superpowers/specs/2026-08-13-degraded-portal-startup-message-design.md` (the merged spec's
  overstated-trigger correction), `docs/backlog/degraded-queue-written-to-temp-dir/NOTE.md`
  (superseded banner)
- Already created, untracked: `docs/superpowers/specs/2026-08-13-degraded-queue-persistence-design.md`,
  `docs/backlog/untried-result-labelled-as-send-error/NOTE.md`, this plan

- [ ] **Step 1: Run the whole gate**

Run: `just check`
Expected: EXIT CODE 0. Capture the exit code explicitly — do **not** pipe to `tail`, which masks it.

- [ ] **Step 2: Confirm no stray scaffolding**

Run: `grep -rn "SCRATCH\|REFBOX_DEMO" refbox/src/ || echo "clean"`
Expected: `clean`. (A previous branch used a temporary env-var switch to force degraded mode; make sure nothing like it was reintroduced.)

- [ ] **Step 3: Commit the docs** (ask the human first)

```bash
git add docs/superpowers/specs/2026-08-13-degraded-queue-persistence-design.md \
        docs/superpowers/specs/2026-08-13-degraded-portal-startup-message-design.md \
        docs/superpowers/plans/2026-08-13-degraded-queue-persistence.md \
        docs/backlog/degraded-queue-written-to-temp-dir/NOTE.md \
        docs/backlog/untried-result-labelled-as-send-error/NOTE.md
git commit -m "docs(refbox): spec, plan and backlog for degraded-queue persistence"
```

Add only those five paths. `docs/backlog/` and `docs/superpowers/plans/` hold many untracked notes from earlier sessions that must not be swept in.

- [ ] **Step 4: Code review**

Use `superpowers:requesting-code-review` once, now that the change is complete (lean process). Tell the reviewer specifically to attack the data-safety question: can any ordering of degraded and healthy sessions lose or duplicate a queued result?

---

## Acceptance criteria

1. A degraded manager given directory D queues a game; a fresh healthy `PortalManager::new(D, …)` finds it.
2. Directory D already holds two queued games; a degraded manager on D queues a third; all three survive on disk. **Fails on the naive fix.**
3. Degraded construction creates no queue file where none existed, and leaves an existing one on disk untouched.
4. PR #2319's behaviour is intact: still red, `token_expired` still false, startup-failure row still shown, still no background task.
5. `just check` exits 0.

## What this does NOT deliver

Recovered results do **not** upload unattended. Anything past the 30-minute stuck threshold is excluded from auto-retry (`health.rs:31-39`, *"Stuck items wait for operator action"*), and any real outage exceeds 30 minutes — so these arrive as red rows and go up on one RETRY ALL press. That is the difference between "recoverable in one tap" and "silently gone", and it is deliberately not more than that. See `docs/backlog/untried-result-labelled-as-send-error/NOTE.md`.

## Deviations

Record here rather than in standalone commits (lean process).

- **Branched off `origin/master` `1846c549`**, which already contains PR #2319 (all four commits confirmed in master via `git cherry`). The sequencing worry about stacking on an unmerged PR no longer applies.
