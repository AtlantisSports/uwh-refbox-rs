# The bridge follows refbox's portal — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** refbox reports the portal address it is actually using on every game update, and the
bridge resolves team and player names from that address and nothing else — so the bridge can never
be looking at a different portal than refbox.

**Architecture:** One new optional field on the shared `GameSnapshot`, filled from the value refbox
already uses (`current_site.base_url`) rather than recomputed. The bridge remembers the last known
real address beside the last known real event id, and rebuilds its `Directory` when that *pair*
differs from the pair the running `Directory` was built from. `--portal-url` is deleted.

**Tech Stack:** Rust 2024, serde/serde_json, tokio, axum, clap. No new dependencies.

**Spec:** `docs/superpowers/specs/2026-08-27-bridge-follows-refbox-portal-design.md` — read it
first; this plan argues from it.

## Global Constraints

- MSRV 1.85, edition 2024. No APIs newer than 1.85.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` must be clean. An
  argument left unused after a change is a hard error, not a warning.
- **No new dependencies.** Everything needed is already present.
- `uwh-common` must still compile without the standard library. `GameSnapshot` is already
  `#[cfg(feature = "std")]`, so a `String` field is allowed there — but any new *test* touching it
  must be gated so `--no-default-features` still builds.
- **Heavy process** (`.claude/rules/plan-execution.md`): `uwh-common` is in scope, so every task
  ends with its own verification and review before the next begins.
- The literal field name is `portal_base_url`. It appears on the wire, so it is not up for
  improvement mid-execution.
- Nothing is added to the operator status page. Eric's call: "just properly used."
- `GameSnapshotNoHeap` (the LED panel's compact form) is NOT touched.

---

## File Structure

| File | Change | Responsibility |
|---|---|---|
| `uwh-common/src/game_snapshot.rs` | Modify (~line 54) | Declares `portal_base_url` on `GameSnapshot`; owns the two compatibility tests |
| `refbox/src/app/mod.rs` | Modify (~line 742) | Stamps refbox's own `current_site.base_url` onto the game path |
| `refbox/src/app/update_sender.rs` | Modify (tests only) | Existing full-snapshot test carries a populated address |
| `overlay-bridge/src/server.rs` | Modify (`LastSeen`, `consume_snapshots`, `start`) | Remembers the pair; owns the rebuild rule |
| `overlay-bridge/src/portal.rs` | Modify (`Directory`) | Exposes the pair it was built from |
| `overlay-bridge/src/main.rs` | Modify (`Cli`) | Loses `--portal-url`; rejects it if typed |
| Every other file with a `GameSnapshot { .. }` literal | Modify | Mechanical: `portal_base_url: None` |

Literal sites, by file (65 total; those ending in `..Default::default()` need nothing):
`overlay-bridge/src/{discovery.rs:5, feed.rs:2, server.rs:19, state.rs:4, tables.rs:17}`,
`overlay/src/main.rs:1`, `refbox/src/app/{mod.rs:1, update_sender.rs:4,
view_builders/game_info_table.rs:7}`, `refbox/src/tournament_manager/{mod.rs:1, golden/mod.rs:2}`,
`uwh-common/src/game_snapshot.rs:2`.

---

## Task 1: The field, and everything that stops compiling

**Files:**
- Modify: `uwh-common/src/game_snapshot.rs` (struct at line 42; test module at line 623)
- Modify: every file listed above that builds a `GameSnapshot` without `..Default::default()`

**Interfaces:**
- Consumes: nothing.
- Produces: `GameSnapshot::portal_base_url: Option<String>` — read by Tasks 2 and 3.

- [ ] **Step 1: Add the field**

In `uwh-common/src/game_snapshot.rs`, immediately after `pub event_id: Option<EventId>,`:

```rust
    /// The `base_url` of the uwhportal client the refbox that sent this snapshot is using: the
    /// official Portal (honouring `UWH_PORTAL_URL_OVERRIDE`), the UWR portal in Rugby mode, or a
    /// hand-typed custom site. `None` from a refbox too old to report it, and on snapshots
    /// synthesized outside a game (the beep test), where no portal call exists to describe.
    ///
    /// A consumer that resolves names from a portal must use this and not an address of its own.
    /// Event ids are not unique across portal environments -- `1889-B` is one tournament on the
    /// development portal and a different one on production -- so the same id looked up on the
    /// wrong portal returns real names for the wrong event, with no error anywhere.
    pub portal_base_url: Option<String>,
```

- [ ] **Step 2: Write the two failing compatibility tests**

In the existing `#[cfg(test)] mod` (line 623) of the same file. Both are gated so
`--no-default-features` still builds:

```rust
    #[test]
    #[cfg(feature = "std")]
    fn a_snapshot_line_without_a_portal_address_deserializes_as_absent() {
        // The older-refbox case: a refbox built before this field existed sends a line with no
        // such key. It must arrive absent -- never substituted with a plausible default, which
        // would be the production portal and would silently reintroduce the wrong-tournament bug.
        // Built by removing the key from a real serialization rather than hand-typing JSON, so
        // this cannot drift out of step with the struct.
        let mut value = serde_json::to_value(GameSnapshot::default()).expect("serialize");
        value
            .as_object_mut()
            .expect("a snapshot serializes as a JSON object")
            .remove("portal_base_url")
            .expect("the key should have been there to remove");

        let snapshot: GameSnapshot =
            serde_json::from_value(value).expect("a line predating the field must still parse");

        assert_eq!(snapshot.portal_base_url, None);
    }

    #[test]
    #[cfg(feature = "std")]
    fn a_snapshot_line_with_an_unknown_extra_field_still_deserializes() {
        // The other direction: an older consumer (a v0.5.0 stream overlay) reading a newer
        // refbox. Guards against anyone adding `serde(deny_unknown_fields)` later, which would
        // turn every future field addition into a hard break for deployed software.
        let mut value = serde_json::to_value(GameSnapshot::default()).expect("serialize");
        value
            .as_object_mut()
            .expect("a snapshot serializes as a JSON object")
            .insert("a_field_from_a_future_refbox".to_string(), 1.into());

        let snapshot: GameSnapshot =
            serde_json::from_value(value).expect("an unknown field must be ignored, not fatal");

        assert_eq!(snapshot, GameSnapshot::default());
    }
```

- [ ] **Step 3: Run them and watch them fail for the right reason**

Run: `cargo test -p uwh-common a_snapshot_line`
Expected: FAIL — the whole crate fails to compile first, because the two `GameSnapshot` literals
in this same file do not yet name the new field. That is the expected first failure; fix those two,
re-run, and both tests must then pass.

- [ ] **Step 4: Make the workspace compile again**

Run: `cargo check --workspace --all-targets`

Add `portal_base_url: None,` to every literal the compiler names. **The value is `None` at every
one of these sites** — they are existing tests and fixtures whose subject is not the portal — with
one deliberate exception in Step 5.

Do not restructure any literal, do not convert one to `..Default::default()`, and do not touch a
literal the compiler did not name.

- [ ] **Step 5: Give the one test that checks encoded bytes a populated address**

In `refbox/src/app/update_sender.rs`, the full-snapshot test that builds `json_expected` and
`binary_expected` sets every field explicitly. Give it a real value rather than `None`:

```rust
            portal_base_url: Some("https://api.dev.uwhportal.com".to_string()),
```

Why: that test compares the exact bytes read off both sockets, so a populated string field keeps
the length accounting on the JSON path covered. With `None` it would only ever prove that `null`
encodes.

- [ ] **Step 6: Verify**

Run: `cargo test -p uwh-common && cargo test -p refbox && cargo test -p overlay-bridge && cargo check -p uwh-common --no-default-features && cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: all green. The `--no-default-features` check is the no_std guard and is not optional.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat(uwh-common): carry the refbox's portal address on the snapshot"
```

---

## Task 2: refbox reports the address it is using

**Files:**
- Modify: `refbox/src/app/mod.rs` (in `apply_snapshot`, at the `new_snapshot.event_id` line, ~742)

**Interfaces:**
- Consumes: `GameSnapshot::portal_base_url` (Task 1).
- Produces: a live refbox whose JSON feed carries its portal address. Task 3 consumes it.

**There is deliberately no unit test in this task.** The stamp is a field read inside a method on
`RefBoxApp`, which no test constructs — it owns the portal client, the update sender and the sound
controller. Extracting a helper would buy a test proving that a function copies its argument, while
the thing that can actually go wrong (reading the wrong source) would stay in the untested caller.
Task 4 step 2 is this task's guard, and it is a real one: it fails if the stamp is missing or stale.
Do not invent a mock `RefBoxApp` to close this gap.

- [ ] **Step 1: Stamp it on the game path**

In `refbox/src/app/mod.rs`, directly below the existing line in `apply_snapshot`:

```rust
        new_snapshot.event_id = self.current_event_id.clone();
        // The address this refbox's own portal client is pointed at, so a consumer resolving names
        // from a portal looks them up where refbox looks them up. `current_site` is the value the
        // client is built from -- already accounting for the override env var, Rugby mode's
        // separate portal, and a custom site -- so this reports rather than re-derives, and the
        // two cannot disagree.
        new_snapshot.portal_base_url = Some(self.current_site.base_url.clone());
```

- [ ] **Step 2: Confirm the beep-test path is left alone**

Read `refbox/src/app/mod.rs` around line 5474 — the second place a snapshot reaches the update
sender. It synthesizes a `GameSnapshot` with `..Default::default()` to drive the LED panel.

Verify by reading that it still carries no address, and **change nothing there**. During a beep test
there is no game, no event and no portal call, and no overlay or bridge is in use (Eric,
2026-08-28), so `None` is the honest value. Confirming this is the step; editing it is a defect.

- [ ] **Step 3: Verify**

Run: `cargo clippy -p refbox --all-targets --all-features -- -D warnings && cargo test -p refbox`
Expected: green. No test asserts the new line — see the note above.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(refbox): report the portal address on the game feed"
```

---

## Task 3: The bridge follows the feed, and loses its flag

This is one task because it cannot be split without leaving the crate uncompilable or, worse,
briefly implementing the two-sources-of-truth structure the whole change exists to remove.

**Files:**
- Modify: `overlay-bridge/src/portal.rs` (`Directory`)
- Modify: `overlay-bridge/src/server.rs` (`LastSeen`, `consume_snapshots`, `start`)
- Modify: `overlay-bridge/src/main.rs` (`Cli`)

**Interfaces:**
- Consumes: `GameSnapshot::portal_base_url` (Task 1); a refbox that fills it (Task 2).
- Produces: `Directory::identity(&self) -> (&str, &EventId)`;
  `LastSeen::identity(&self) -> Option<(String, EventId)>`;
  `server::start(settings: config::Resolved) -> Bridge` (one argument, not two).

- [ ] **Step 1: Write the failing tests**

Add to `overlay-bridge/src/server.rs`'s test module, beside the existing
`a_new_event_id_creates_a_fresh_directory_replacing_the_previous_one`. They follow that test's
harness exactly (`AppState::new(config::Resolved::default())`, an unbounded channel,
`from_chosen_refbox`, a 50 ms settle, `read_lock(&state.directory)`), minus the `portal_url`
argument `consume_snapshots` no longer takes:

```rust
    /// The address the feed reports is the one the directory is built for. This is the incident of
    /// 2026-08-26 as a regression guard: refbox on the development portal, the bridge previously
    /// on its own production default, the same event id resolving on both.
    ///
    /// Asserting on the directory's identity rather than on an outgoing HTTP request is
    /// deliberate and sufficient: `portal.rs`'s own tests already prove a `Directory` fetches
    /// from the address it was built with (`refresh_schedule` formats its URL from
    /// `self.portal_url`, and those tests drive it against a local listener). The two compose to
    /// "the request goes where the feed said", without standing up a mock portal here.
    #[tokio::test]
    async fn the_directory_is_built_for_the_address_the_feed_reports() {
        let state = Arc::new(AppState::new(config::Resolved::default()));
        let (tx, rx) = mpsc::unbounded_channel();
        let refresh_notify = Arc::new(Notify::new());
        let consumer = tokio::spawn(consume_snapshots(
            Arc::clone(&state),
            rx,
            Client::new(),
            refresh_notify,
        ));

        tx.send(from_chosen_refbox(
            &state,
            GameSnapshot {
                current_period: GamePeriod::FirstHalf,
                event_id: Some(EventId::from_partial("1889-B")),
                portal_base_url: Some("https://api.dev.uwhportal.com".to_string()),
                game_number: "1".to_string(),
                ..Default::default()
            },
        ))
        .expect("channel should accept the snapshot");
        tokio::time::sleep(Duration::from_millis(50)).await;

        let directory = read_lock(&state.directory)
            .clone()
            .expect("a directory should exist once both halves are known");
        let (base_url, event_id) = directory.identity();
        assert_eq!(base_url, "https://api.dev.uwhportal.com");
        assert_eq!(*event_id, EventId::from_partial("1889-B"));

        consumer.abort();
    }

    /// Without an address there is nowhere legitimate to look, so nothing is looked up. The
    /// failure this forbids is falling back to production, which is what produced real names for
    /// the wrong tournament.
    #[tokio::test]
    async fn an_event_with_no_address_builds_no_directory_at_all() {
        let state = Arc::new(AppState::new(config::Resolved::default()));
        let (tx, rx) = mpsc::unbounded_channel();
        let refresh_notify = Arc::new(Notify::new());
        let consumer = tokio::spawn(consume_snapshots(
            Arc::clone(&state),
            rx,
            Client::new(),
            refresh_notify,
        ));

        tx.send(from_chosen_refbox(
            &state,
            GameSnapshot {
                current_period: GamePeriod::FirstHalf,
                event_id: Some(EventId::from_partial("1889-B")),
                portal_base_url: None,
                game_number: "1".to_string(),
                ..Default::default()
            },
        ))
        .expect("channel should accept the snapshot");
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert!(
            read_lock(&state.directory).is_none(),
            "with no address reported the bridge must fetch nothing, not guess a portal"
        );

        consumer.abort();
    }

    /// Same event, different portal: a different tournament, so the cache must not survive.
    #[tokio::test]
    async fn the_same_event_on_a_different_address_replaces_the_directory() {
        let state = Arc::new(AppState::new(config::Resolved::default()));
        let (tx, rx) = mpsc::unbounded_channel();
        let refresh_notify = Arc::new(Notify::new());
        let consumer = tokio::spawn(consume_snapshots(
            Arc::clone(&state),
            rx,
            Client::new(),
            refresh_notify,
        ));

        let snapshot_at = |address: &str| GameSnapshot {
            current_period: GamePeriod::FirstHalf,
            event_id: Some(EventId::from_partial("1889-B")),
            portal_base_url: Some(address.to_string()),
            game_number: "1".to_string(),
            ..Default::default()
        };

        tx.send(from_chosen_refbox(
            &state,
            snapshot_at("https://api.dev.uwhportal.com"),
        ))
        .expect("channel should accept the first snapshot");
        tokio::time::sleep(Duration::from_millis(50)).await;
        let first = read_lock(&state.directory)
            .clone()
            .expect("a directory should exist after the first snapshot");

        tx.send(from_chosen_refbox(
            &state,
            snapshot_at("https://api.uwhportal.com"),
        ))
        .expect("channel should accept the second snapshot");
        tokio::time::sleep(Duration::from_millis(50)).await;
        let second = read_lock(&state.directory)
            .clone()
            .expect("a directory should still exist after the second snapshot");

        assert!(
            !Arc::ptr_eq(&first, &second),
            "the same event id on a different portal is a different tournament -- the cached \
             names must not carry over"
        );
        assert_eq!(second.identity().0, "https://api.uwhportal.com");

        consumer.abort();
    }

    /// The other half of the rule the event id already has: an absent value is a gap in reporting,
    /// never news. If it rebuilt, every cached team name would be thrown away for nothing.
    #[tokio::test]
    async fn a_momentary_missing_address_does_not_rebuild_the_directory() {
        let state = Arc::new(AppState::new(config::Resolved::default()));
        let (tx, rx) = mpsc::unbounded_channel();
        let refresh_notify = Arc::new(Notify::new());
        let consumer = tokio::spawn(consume_snapshots(
            Arc::clone(&state),
            rx,
            Client::new(),
            refresh_notify,
        ));

        let with_address = GameSnapshot {
            current_period: GamePeriod::FirstHalf,
            event_id: Some(EventId::from_partial("1889-B")),
            portal_base_url: Some("https://api.dev.uwhportal.com".to_string()),
            game_number: "1".to_string(),
            ..Default::default()
        };
        tx.send(from_chosen_refbox(&state, with_address.clone()))
            .expect("channel should accept the first snapshot");
        tokio::time::sleep(Duration::from_millis(50)).await;
        let before = read_lock(&state.directory)
            .clone()
            .expect("a directory should exist after the first snapshot");

        tx.send(from_chosen_refbox(
            &state,
            GameSnapshot {
                portal_base_url: None,
                ..with_address.clone()
            },
        ))
        .expect("channel should accept the gap snapshot");
        tokio::time::sleep(Duration::from_millis(50)).await;
        let after = read_lock(&state.directory)
            .clone()
            .expect("the directory must survive a snapshot that reported no address");

        assert!(
            Arc::ptr_eq(&before, &after),
            "an absent address must not overwrite the remembered one, or the next snapshot \
             carrying it back would look like news and discard every cached name"
        );

        consumer.abort();
    }
```

And in `overlay-bridge/src/main.rs`'s test module, extend the existing removed-flags test rather
than adding a second one — it exists for exactly this:

```rust
        assert!(
            Cli::try_parse_from(["overlay-bridge", "--portal-url", "https://api.uwhportal.com"])
                .is_err(),
            "--portal-url was removed; the bridge reads the portal address from refbox's feed"
        );
```

Delete `the_portal_url_defaults_to_the_production_portal` outright. The default it pins is the
thing being removed; leaving it would fail, and "fixing" it would re-document the defect.

- [ ] **Step 2: Run them and confirm they fail**

Run: `cargo test -p overlay-bridge`
Expected: compile failure — `identity` does not exist, `consume_snapshots` still takes four
arguments, `portal_base_url` is not a field the tests can set on a snapshot the bridge builds. This
is the expected shape of failure at this step.

- [ ] **Step 3: Expose the pair a `Directory` was built from**

In `overlay-bridge/src/portal.rs`, on `impl Directory`:

```rust
    /// The pair this directory was built from. It is its own record of that pair, so nothing has
    /// to track "what did we build the current one for" alongside it and keep the two in step.
    pub fn identity(&self) -> (&str, &EventId) {
        (&self.portal_url, &self.event_id)
    }
```

- [ ] **Step 4: Remember the address beside the event id**

In `overlay-bridge/src/server.rs`, extend `LastSeen` (line ~183) and give it the pair accessor:

```rust
/// The last event, portal address and game `consume_snapshots` saw, so it can tell a change from a
/// repeat. See [`AppState::last_seen`].
#[derive(Debug, Default)]
struct LastSeen {
    event_id: Option<EventId>,
    /// The last address the feed actually reported. Remembered under the same rule as `event_id`:
    /// an absent value on the wire never overwrites it.
    portal_base_url: Option<String>,
    game_number: Option<GameNumber>,
}

impl LastSeen {
    /// The pair a [`Directory`] is built from, once both halves are known. `None` while either is
    /// still unknown -- and while it is `None` nothing is fetched, because there is no address to
    /// fetch from and inventing one is what served the wrong tournament's names.
    fn identity(&self) -> Option<(String, EventId)> {
        Some((self.portal_base_url.clone()?, self.event_id.clone()?))
    }
}
```

`AppState::forget_game` needs no change: it already resets `LastSeen::default()` and clears the
directory, so the new half is forgotten with the old one.

- [ ] **Step 5: Replace the rebuild rule with the pair comparison**

In `consume_snapshots`, replace everything from `let event_id = snapshot.event_id.clone();` through
the end of the `last_seen` block (server.rs ~890-930) with:

```rust
        let event_id = snapshot.event_id.clone();
        let portal_base_url = snapshot.portal_base_url.clone();
        let game_number = snapshot.game_number().clone();

        write_lock(&state.live).apply(snapshot, now);

        // Remember only what was actually reported. A `None` on the wire is a gap in reporting --
        // a snapshot arriving before the refbox has attached an event, or one synthesized outside
        // a game -- and must never overwrite a known value: if it did, the very next snapshot
        // carrying that same value back would look new, rebuilding the directory and throwing away
        // every team name and roster cached for something nothing has actually left.
        let (identity, game_changed) = {
            let mut last_seen = write_lock(&state.last_seen);
            if event_id.is_some() {
                last_seen.event_id = event_id;
            }
            if portal_base_url.is_some() {
                last_seen.portal_base_url = portal_base_url;
            }
            let game_changed = last_seen.game_number.as_ref() != Some(&game_number);
            last_seen.game_number = Some(game_number);
            (last_seen.identity(), game_changed)
        };

        // One comparison, against the running directory's own record of what it was built from.
        // This covers the first address arriving, the address changing and the event changing; and
        // because it reads the remembered pair rather than the raw wire values, it cannot fire on
        // a gap. Until both halves are known there is no directory at all -- names stay blank
        // rather than being looked up somewhere plausible.
        let rebuilt = match identity {
            Some((base_url, id)) => {
                let matches_current = read_lock(&state.directory)
                    .as_ref()
                    .is_some_and(|directory| directory.identity() == (base_url.as_str(), &id));
                if matches_current {
                    false
                } else {
                    *write_lock(&state.directory) =
                        Some(Arc::new(Directory::new(client.clone(), base_url, id)));
                    true
                }
            }
            None => false,
        };

        if rebuilt || game_changed {
            refresh_notify.notify_one();
        }
```

The `read_lock` guard above is a temporary inside the `is_some_and` expression and is dropped
before the `write_lock` on the next statement. Do not hoist it into a `let` binding that outlives
the branch: these are `std::sync::RwLock`s, and holding the read guard across the write would
deadlock the consumer permanently — the bridge would freeze with no error.

- [ ] **Step 6: Delete the flag and its plumbing**

Three signature changes, in this order:

1. `consume_snapshots` — drop the `portal_url: String` parameter (server.rs ~871).
2. `pub fn start(settings: config::Resolved, portal_url: String) -> Bridge` becomes
   `pub fn start(settings: config::Resolved) -> Bridge`; drop `portal_url` from the
   `consume_snapshots` call.
3. `overlay-bridge/src/main.rs` — delete the `portal_url` field and its doc comment from `Cli`, and
   call `server::start(settings)`.

Then fix the existing tests the compiler names: every `consume_snapshots(...)` call site loses its
`"http://portal.invalid".to_string()` argument.

- [ ] **Step 7: Update the existing tests whose expectations genuinely changed**

**Read this before "fixing" any failure.** Several existing tests send snapshots carrying an event
id and then assert a directory exists. Under the new rule a directory cannot exist without an
address, so those tests now fail — correctly. Give their snapshots a
`portal_base_url: Some("https://api.dev.uwhportal.com".to_string())` alongside the event id they
already set. Known members of that class:

- `a_new_event_id_creates_a_fresh_directory_replacing_the_previous_one`
- `a_momentary_missing_event_id_does_not_rebuild_the_directory_for_the_same_event`
- `returning_to_an_event_after_a_switch_rebuilds_its_directory_rather_than_reusing_it`
  (server.rs:2957) — this one is also what covers the spec's "choosing a different refbox
  re-establishes the pair" requirement, so no new test is needed for it; keeping it passing *is*
  that coverage.
- `choosing_a_different_refbox_makes_the_tables_serve_that_refbox_s_game` (server.rs:2301), if it
  asserts on resolved names rather than only on game data.

The compiler will not find these — they fail at run time, not build time. Run the crate's whole
test suite, not just the new tests.

A test that asserts a directory exists is asserting the bridge knows where to look. Adding the
address is restoring its premise, not weakening it. If a failing test cannot be fixed this way,
stop and report it rather than relaxing the assertion.

- [ ] **Step 8: Verify**

Run: `cargo test -p overlay-bridge && cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: all green, including the four new tests and the extended flag-rejection test.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "feat(overlay-bridge): resolve names from the portal refbox reports"
```

---

## Task 4: Prove it end to end

**Files:** none. This task builds and runs the real programs.

The only guard for Task 2 lives here, so it is not optional and it is not a formality.

- [ ] **Step 1: Full gate**

Run: `just check`
Expected: fmt, lint, tests, audit all clean. Note that `just lint` is not the `--all-targets` form;
step 8 of Task 3 already ran the stricter one.

- [ ] **Step 2: Build the binaries the walkthrough uses**

Run: `cargo build -p refbox -p overlay-bridge`

`just check` builds a *test* binary, not the one a walkthrough runs — build explicitly or the
walkthrough silently exercises an older binary.

- [ ] **Step 3: The walkthrough (hand these steps to Eric; he drives refbox)**

1. Start refbox against the **development** portal and select an event with teams:
   `WAYLAND_DISPLAY= UWH_PORTAL_URL_OVERRIDE=https://api.dev.uwhportal.com ./target/debug/refbox --allow-http`
2. Start the bridge with **no portal flag at all**:
   `./target/debug/overlay-bridge --port 8123 --refbox-host 127.0.0.1 --refbox-port 8000`
3. Confirm the team names the bridge serves are the development portal's — the tournament actually
   selected in refbox, not another one with the same event id.
4. Confirm `./target/debug/overlay-bridge --portal-url https://api.uwhportal.com` is refused with a
   message naming the flag.
5. **The stopped-clock test.** With the clock stopped 30+ seconds, the status page's dot stays
   green and every value holds. A wrong build passes every other check here and fails only this
   one.

- [ ] **Step 4: Record the outcome**

Append what was and was not walked to the plan's Deviations section below — including anything
skipped and why. Do not report the feature complete on the strength of green tests alone: Task 2
has no unit test by design, and step 3 is the only thing that exercises it.

- [ ] **Step 5: Commit anything the walkthrough changed**

If nothing changed, commit nothing.

---

## Deviations

(Record execution deviations here rather than in standalone commits, per
`.claude/rules/plan-execution.md`.)
