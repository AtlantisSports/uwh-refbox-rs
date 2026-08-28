# Overlay-bridge game feed (`GET /game`) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `GET /game` to `overlay-bridge`, serving one typed JSON object carrying the game
information the NDI renderer needs, so the renderer can consume the bridge over HTTP instead of
connecting to a refbox directly.

**Architecture:** A new `game_feed` module builds a typed payload from a `Display`, mirroring the
role `tables` already plays for the vMix tables — pure building, no state access, so it is testable
without a server. `server.rs` gains one thin handler and one route, matching the five existing table
handlers exactly. The five vMix tables are not modified; a new guard test freezes their column names.

**Tech Stack:** Rust 2024, axum 0.8, serde + serde_json (all already dependencies of this crate —
this plan adds none).

**Spec:** `docs/superpowers/specs/2026-08-28-overlay-bridge-renderer-feed-design.md` (committed
`83a4beb7`). Read it before Task 2; §4, §4a, §4b, §6 and §7 are the contract this plan implements.

## Global Constraints

- **MSRV 1.85, edition 2024.** No language or std features newer than 1.85.
- **`cargo clippy --workspace --all-targets --all-features -- -D warnings` must pass clean.**
- **No new dependencies.** `serde` (with `derive`), `serde_json` and `axum` are already in
  `overlay-bridge/Cargo.toml`.
- **No `unwrap()` or `expect()` in non-test code** without a comment explaining why it cannot panic.
- **All changes confined to `overlay-bridge/`.** No edits to `refbox/`, `uwh-common/`, `overlay/`, or
  any workspace `Cargo.toml`.
- **The five vMix tables' served output must not change.** `/scorebug`, `/penalties`, `/fouls`,
  `/warnings`, `/nextgame` keep exactly the columns and values they have today. Task 1 makes this
  enforceable; Tasks 2-4 must keep it green.
- **Never `skip_serializing_if` on any payload field.** Spec §7 requires every key to be present
  with a `null` value when there is nothing to report; skipping a key violates the contract in a way
  no test in this plan would notice unless it checks key presence explicitly.
- **Never render an `EventId` with `Display`/`to_string()`.** `impl fmt::Display for EventId` writes
  `"Event ID events/1889-B"` (`uwh-common/src/uwhportal/schedule.rs:758`) — a human label, not an
  id. Use `.full()`.

---

## File Structure

- **Create `overlay-bridge/src/game_feed.rs`** — the typed payload structs, `SCHEMA_VERSION`, the
  `game_feed()` builder, and the credential-stripping helper. Owns the whole `/game` contract, so
  the contract can be read in one file. Its own `#[cfg(test)] mod tests`.
- **Modify `overlay-bridge/src/lib.rs`** — one line adding `pub mod game_feed;`.
- **Modify `overlay-bridge/src/server.rs`** — one route, one handler. Nothing else.
- **Modify `overlay-bridge/src/tables.rs`** — the column-freeze guard test (Task 1), and three
  private helpers widened to `pub(crate)` (Task 2) so `game_feed` reuses the same vocabulary rather
  than defining a second one.

---

### Task 1: Freeze the vMix tables' column names

The guard goes in first, so every later task is done underneath it. It is worth having on its own
merits: it is the only thing that would catch the alphabetical-shift hazard of spec §3.

**Files:**
- Modify: `overlay-bridge/src/tables.rs` (test module, which begins at line 584)

**Interfaces:**
- Consumes: `tables::{scorebug, next_game, penalties, fouls, warnings}`, and the test module's
  existing `display_with(snapshot)` helper (`tables.rs:589`).
- Produces: nothing other tasks depend on.

- [ ] **Step 1: Write the failing test**

Add to the end of `tables.rs`'s `mod tests`:

```rust
/// The exact column set every vMix table serves. **Pinned deliberately, and a failure here is a
/// question rather than a list to update.**
///
/// Rows are `BTreeMap`s, so columns serialise in alphabetical order, and a vMix title left on
/// positional fallback reads whichever column now occupies its position rather than the one it was
/// bound to. That has already happened once for real: a title expecting `blackTeam` received
/// `blackFouls`. So adding, removing or renaming any column below silently repoints every
/// positionally-bound title after it, on air, with no error anywhere -- a vMix title has no logic
/// and therefore no error path. See the design spec's §3.
#[test]
fn the_vmix_tables_column_names_are_frozen() {
    let display = display_with(GameSnapshot::default());
    let rosters = Rosters::default();

    let columns = |rows: &[BTreeMap<String, String>]| -> Vec<String> {
        rows.first()
            .expect("every table serves at least one row")
            .keys()
            .cloned()
            .collect()
    };

    assert_eq!(
        columns(&scorebug(&display, None, true)),
        vec![
            "blackFouls",
            "blackScore",
            "blackTeam",
            "blackWarnings",
            "clock",
            "clockSeconds",
            "connected",
            "equalFouls",
            "period",
            "timeout",
            "timeoutClock",
            "timeoutClockSeconds",
            "whiteFouls",
            "whiteScore",
            "whiteTeam",
            "whiteWarnings",
        ],
        "/scorebug columns"
    );

    assert_eq!(
        columns(&next_game(None, true)),
        vec!["blackTeam", "connected", "court", "startTime", "whiteTeam"],
        "/nextgame columns"
    );

    assert_eq!(
        columns(&penalties(&display, &rosters, true)),
        vec![
            "connected",
            "infraction",
            "number",
            "player",
            "team",
            "time",
            "timeSeconds",
        ],
        "/penalties columns"
    );

    for (name, rows) in [
        ("/fouls", fouls(&display, &rosters, true)),
        ("/warnings", warnings(&display, &rosters, true)),
    ] {
        assert_eq!(
            columns(&rows),
            vec!["connected", "infraction", "number", "player", "team"],
            "{name} columns"
        );
    }
}
```

- [ ] **Step 2: Run it and confirm it passes**

```bash
cd /home/estraily/projects/refbox-overlay-delivery
cargo test -p overlay-bridge the_vmix_tables_column_names_are_frozen -- --nocapture
```

Expected: PASS. This test documents current behaviour, so it passes immediately — that is correct
for a guard, and it is why Step 3 exists.

- [ ] **Step 3: Prove the guard actually guards**

A test never seen failing is not a test. Temporarily add a column to `scorebug`, immediately before
its `finish_table(vec![row], connected)` call:

```rust
    row.insert("aaaTemporary".to_string(), String::new());
```

Run the test again. It MUST fail, and the failure must show `aaaTemporary` sorted to the **front**
of `/scorebug`'s list — which is the hazard itself: an innocuous-looking addition displacing every
column after it. Then remove the line and confirm the test passes again.

- [ ] **Step 4: Commit**

```bash
cd /home/estraily/projects/refbox-overlay-delivery
git add overlay-bridge/src/tables.rs
git commit -m "test(overlay-bridge): freeze the vMix tables' column names"
```

---

### Task 2: `/game` serving the scalars, `connected` and `schemaVersion`

**Files:**
- Create: `overlay-bridge/src/game_feed.rs`
- Modify: `overlay-bridge/src/lib.rs` (add `pub mod game_feed;` between `feed` and `portal`)
- Modify: `overlay-bridge/src/tables.rs` (widen three helpers to `pub(crate)`)
- Modify: `overlay-bridge/src/server.rs` (route in `router()` at line ~340, handler beside
  `get_scorebug` at line ~355)

**Interfaces:**
- Consumes: `state::Display`, `portal::TeamNames`, `tables::{Rosters, color_code, timeout_label,
  timeout_seconds}`, `server::{current_display, names_for_game, current_rosters, is_connected}`.
  **Note the paths — they are not where you would guess:** `TeamNames` is in `crate::portal`, not
  `tables`; `Color` is `uwh_common::color::Color`, not `game_snapshot`; `BlackWhiteBundle` is
  `uwh_common::bundles::BlackWhiteBundle`. Copy them from `tables.rs:117-128`.
- Produces:
  - `game_feed::SCHEMA_VERSION: u32`
  - `game_feed::GameFeed` (serde `rename_all = "camelCase"`)
  - `game_feed::game_feed(display: &Display, names: Option<&TeamNames>, rosters: &Rosters, connected: bool) -> GameFeed`
  - `game_feed::Timeout`, `game_feed::Goal`
  - Task 3 adds the `penalties` field's contents; Task 4 adds `event_id` / `portal_base_url`. Both
    fields are declared in this task so the payload shape is settled once.

- [ ] **Step 1: Widen the three shared helpers in `tables.rs`**

Spec §4 requires `period` and `timeout.kind` to carry the same strings the vMix tables serve, so
these are reused rather than reimplemented. Change three signatures only — no bodies, no behaviour:

```rust
pub(crate) fn timeout_label(timeout: TimeoutSnapshot) -> &'static str {
pub(crate) fn timeout_seconds(timeout: TimeoutSnapshot) -> u16 {
pub(crate) fn color_code(color: Color) -> &'static str {
```

Task 1's guard test proves this changed no served output.

- [ ] **Step 2: Write the failing tests**

Create `overlay-bridge/src/game_feed.rs` containing only this test module for now:

```rust
//! The typed game feed served at `GET /game`, for a consumer that can read values rather than the
//! display strings the vMix tables carry (see `tables`' module doc).
//!
//! **This is a published contract with a version on it.** Adding a field is free; removing,
//! renaming, or changing the meaning of one requires bumping [`SCHEMA_VERSION`]. See the design
//! spec `docs/superpowers/specs/2026-08-28-overlay-bridge-renderer-feed-design.md` §6.

#[cfg(test)]
mod tests {
    use uwh_common::game_snapshot::{GamePeriod, GameSnapshot, TimeoutSnapshot};

    use super::*;

    fn display_with(snapshot: GameSnapshot) -> Display {
        Display { snapshot }
    }

    fn live_snapshot() -> GameSnapshot {
        GameSnapshot {
            current_period: GamePeriod::FirstHalf,
            secs_in_period: 431,
            scores: BlackWhiteBundle { black: 3, white: 2 },
            game_number: "10".to_string(),
            next_game_number: "20".to_string(),
            is_old_game: false,
            recent_goal: Some((Color::Black, 7)),
            next_period_len_secs: Some(180),
            timeout: Some(TimeoutSnapshot::Black(45)),
            ..Default::default()
        }
    }

    #[test]
    fn a_connected_feed_carries_the_snapshot_values() {
        let feed = game_feed(
            &display_with(live_snapshot()),
            None,
            &Rosters::default(),
            true,
        );

        assert_eq!(feed.schema_version, SCHEMA_VERSION);
        assert!(feed.connected);
        assert_eq!(feed.secs_in_period, Some(431));
        assert_eq!(feed.black_score, Some(3));
        assert_eq!(feed.white_score, Some(2));
        assert_eq!(feed.game_number.as_deref(), Some("10"));
        assert_eq!(feed.next_game_number.as_deref(), Some("20"));
        assert_eq!(feed.is_old_game, Some(false));
        assert_eq!(feed.next_period_len_secs, Some(180));
    }

    /// `period` and `timeout.kind` must be the same strings the vMix tables serve -- one vocabulary
    /// for both consumers, per spec §4. Asserted against `tables`' own helpers rather than against
    /// hardcoded text, so the two cannot drift apart silently.
    #[test]
    fn period_and_timeout_use_the_same_words_as_the_vmix_tables() {
        let snapshot = live_snapshot();
        let feed = game_feed(&display_with(snapshot.clone()), None, &Rosters::default(), true);

        assert_eq!(
            feed.period.as_deref(),
            Some(snapshot.current_period.to_string().as_str())
        );

        let timeout = feed.timeout.expect("a timeout was set on the snapshot");
        assert_eq!(timeout.kind, crate::tables::timeout_label(TimeoutSnapshot::Black(45)));
        assert_eq!(timeout.secs_remaining, 45);
    }

    #[test]
    fn a_goal_is_carried_as_the_refbox_sent_it() {
        let feed = game_feed(&display_with(live_snapshot()), None, &Rosters::default(), true);
        let goal = feed.recent_goal.expect("a recent goal was set");
        assert_eq!(goal.team, "BLACK");
        assert_eq!(goal.player, 7);
    }

    /// Spec §5: the team names come from the bridge's own portal lookup, which is what stops a
    /// consumer resolving an id against a portal of its own and getting real names for the wrong
    /// tournament. Unresolved serves `null`, never a placeholder.
    #[test]
    fn team_names_come_from_the_bridge_not_from_an_id() {
        let names = TeamNames {
            dark: Some("Dark Team".to_string()),
            light: Some("Light Team".to_string()),
            court: None,
            start_time: None,
        };
        let feed = game_feed(
            &display_with(live_snapshot()),
            Some(&names),
            &Rosters::default(),
            true,
        );
        assert_eq!(feed.black_team.as_deref(), Some("Dark Team"));
        assert_eq!(feed.white_team.as_deref(), Some("Light Team"));

        let unresolved =
            game_feed(&display_with(live_snapshot()), None, &Rosters::default(), true);
        assert_eq!(
            unresolved.black_team, None,
            "an unresolved name serves null, never a placeholder"
        );
        assert_eq!(unresolved.white_team, None);
    }

    /// Spec §7. Every game value reads `null`, `connected` is `false`, and -- the part a key-count
    /// check would miss -- every key is still present, so a consumer indexing by name finds a null
    /// rather than nothing.
    #[test]
    fn a_disconnected_feed_nulls_every_game_value_and_keeps_every_key() {
        let feed = game_feed(
            &display_with(live_snapshot()),
            None,
            &Rosters::default(),
            false,
        );

        assert!(!feed.connected);
        assert_eq!(
            feed.schema_version, SCHEMA_VERSION,
            "the version must survive a disconnect -- that is exactly when a consumer needs it"
        );
        assert_eq!(feed.period, None);
        assert_eq!(feed.secs_in_period, None);
        assert_eq!(feed.black_score, None);
        assert_eq!(feed.white_score, None);
        assert_eq!(feed.black_team, None);
        assert_eq!(feed.white_team, None);
        assert_eq!(feed.timeout, None);
        assert_eq!(feed.game_number, None);
        assert_eq!(feed.next_game_number, None);
        assert_eq!(feed.is_old_game, None);
        assert_eq!(feed.recent_goal, None);
        assert_eq!(feed.next_period_len_secs, None);
        assert_eq!(feed.penalties, None);

        let json = serde_json::to_value(&feed).expect("the feed should serialise");
        for key in [
            "schemaVersion", "connected", "period", "secsInPeriod", "blackScore", "whiteScore",
            "blackTeam", "whiteTeam", "timeout", "gameNumber", "nextGameNumber", "isOldGame",
            "recentGoal", "nextPeriodLenSecs", "penalties", "eventId", "portalBaseUrl",
        ] {
            assert!(
                json.get(key).is_some(),
                "key {key} must be present even when null -- an absent key and a null are \
                 different things to a consumer"
            );
        }
    }

    /// A blanked score must never read as a real one. `0` is the specific danger: it is a plausible
    /// value, and plausible values invented during an outage are what produced the phantom 0-0
    /// result bug.
    #[test]
    fn a_disconnected_score_is_null_not_zero() {
        let feed = game_feed(&display_with(live_snapshot()), None, &Rosters::default(), false);
        let json = serde_json::to_value(&feed).expect("the feed should serialise");
        assert!(json["blackScore"].is_null());
        assert!(json["whiteScore"].is_null());
        assert!(json["secsInPeriod"].is_null());
    }
}
```

- [ ] **Step 3: Run the tests and confirm they fail to compile**

```bash
cd /home/estraily/projects/refbox-overlay-delivery
cargo test -p overlay-bridge --lib game_feed
```

Expected: compile errors — `game_feed`, `GameFeed`, `SCHEMA_VERSION`, `Display`, `Rosters` not
found. That is the correct starting point.

- [ ] **Step 4: Write the implementation**

Add above the test module in `game_feed.rs`:

```rust
use serde::Serialize;

use crate::{
    portal::TeamNames,
    state::Display,
    tables::{Rosters, color_code, timeout_label, timeout_seconds},
};

/// The version of the `/game` contract this bridge serves.
///
/// **Bumped only when a field is removed, renamed, or changes meaning. Adding a field never bumps
/// it** (spec §6). Bumping is expected to stop a consumer, so bumping for a change that could not
/// have broken anything would take a graphic off a live stream for nothing. Note that `period` and
/// `timeout.kind` are `Display` implementations written for people, so renaming a period or timeout
/// label for display reasons *is* a meaning change and does require a bump.
pub const SCHEMA_VERSION: u32 = 1;

/// One goal, exactly as the refbox reported it.
///
/// **No identity is added here, deliberately.** The refbox's `recent_goal` is a single slot with no
/// goal id, so two goals by the same player inside the retention window are byte-identical and
/// indistinguishable. Spec §2 records that defect as real and explicitly *not* fixed by this work;
/// adding a sequence number here would change what a viewer sees, which is out of scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Goal {
    /// `"BLACK"` or `"WHITE"` -- the same identifiers every vMix table's `team` column uses.
    pub team: String,
    pub player: u8,
}

/// The running timeout, if any.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Timeout {
    /// The same label `/scorebug`'s `timeout` column carries, from [`timeout_label`].
    pub kind: String,
    /// The timeout's own countdown -- not `secs_in_period`.
    pub secs_remaining: u32,
}

/// Everything `GET /game` serves.
///
/// **Every field is always serialised, including as `null`.** No field may gain
/// `skip_serializing_if`: to a consumer an absent key and a null are different things, and only one
/// of them is safe to read blind (the same reasoning `tables::blank_row` already records for the
/// vMix tables). `schema_version` and `connected` are the only fields that are never null.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameFeed {
    pub schema_version: u32,
    /// Whether the refbox is alive. **The only field that answers that question** -- `recent_goal`,
    /// `timeout` and `next_game_number` are all legitimately null during normal play, so this is
    /// what separates "nothing to report" from "nobody is reporting". Never inferred from timing:
    /// the refbox goes silent for ~25s whenever the clock is stopped.
    pub connected: bool,
    pub period: Option<String>,
    pub secs_in_period: Option<u32>,
    pub black_score: Option<u8>,
    pub white_score: Option<u8>,
    pub black_team: Option<String>,
    pub white_team: Option<String>,
    pub timeout: Option<Timeout>,
    pub game_number: Option<String>,
    pub next_game_number: Option<String>,
    pub is_old_game: Option<bool>,
    pub recent_goal: Option<Goal>,
    pub next_period_len_secs: Option<u32>,
    /// Every penalty on the snapshot -- neither padded nor truncated, unlike `/penalties`. `None`
    /// only when disconnected; an empty list means there are no penalties. Populated in Task 3.
    pub penalties: Option<Vec<Penalty>>,
    /// Paired with `portal_base_url`: both or neither. Populated in Task 4.
    pub event_id: Option<String>,
    /// Paired with `event_id`, and served credential-stripped. Populated in Task 4.
    pub portal_base_url: Option<String>,
}

/// One penalty. Mirrors the columns `tables::penalty_row` produces, so both consumers describe a
/// penalty the same way -- with one difference: where the vMix table encodes a dismissal as the
/// literal string `"TD"` with an empty seconds column, this carries a boolean and a null. A typed
/// consumer should not have to recognise `"TD"`.
///
/// Populated by Task 3; declared here because [`GameFeed`] names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Penalty {
    /// `"BLACK"` or `"WHITE"`.
    pub team: String,
    /// The player's cap number, as the refbox reports it.
    pub number: u8,
    /// The player's name from the roster, or `None` when that cap number is not on it -- never a
    /// placeholder, matching `tables`' own contract.
    pub player: Option<String>,
    /// Remaining seconds, or `None` for a total dismissal.
    pub secs_remaining: Option<u32>,
    pub total_dismissal: bool,
    pub infraction: String,
}

/// A blanked feed: `connected: false` and every game value `null`.
fn blanked() -> GameFeed {
    GameFeed {
        schema_version: SCHEMA_VERSION,
        connected: false,
        period: None,
        secs_in_period: None,
        black_score: None,
        white_score: None,
        black_team: None,
        white_team: None,
        timeout: None,
        game_number: None,
        next_game_number: None,
        is_old_game: None,
        recent_goal: None,
        next_period_len_secs: None,
        penalties: None,
        event_id: None,
        portal_base_url: None,
    }
}

/// Builds the `/game` payload.
///
/// `names` and `rosters` are resolved by the caller, exactly as they are for `tables`' own
/// builders -- this module never reaches into portal state.
///
/// When `connected` is false the snapshot is ignored entirely and every game value is blanked. That
/// is the same rule `tables::finish_table` applies to every vMix table, so both consumers agree on
/// what a dropped refbox looks like.
pub fn game_feed(
    display: &Display,
    names: Option<&TeamNames>,
    rosters: &Rosters,
    connected: bool,
) -> GameFeed {
    if !connected {
        return blanked();
    }

    let snapshot = &display.snapshot;
    let _ = rosters; // Task 3 uses this to resolve penalty player names.

    GameFeed {
        schema_version: SCHEMA_VERSION,
        connected: true,
        period: Some(snapshot.current_period.to_string()),
        secs_in_period: Some(snapshot.secs_in_period),
        black_score: Some(snapshot.scores.black),
        white_score: Some(snapshot.scores.white),
        black_team: names.and_then(|n| n.dark.clone()),
        white_team: names.and_then(|n| n.light.clone()),
        timeout: snapshot.timeout.map(|timeout| Timeout {
            kind: timeout_label(timeout).to_string(),
            secs_remaining: u32::from(timeout_seconds(timeout)),
        }),
        game_number: Some(snapshot.game_number().to_string()),
        next_game_number: snapshot.next_game_number().map(ToString::to_string),
        is_old_game: Some(snapshot.is_old_game),
        recent_goal: snapshot.recent_goal.map(|(color, player)| Goal {
            team: color_code(color).to_string(),
            player,
        }),
        next_period_len_secs: snapshot.next_period_len_secs,
        penalties: Some(Vec::new()),
        event_id: None,
        portal_base_url: None,
    }
}
```

Note the two deliberate stubs: `penalties: Some(Vec::new())` and the two `None` id fields. Task 3
and Task 4 replace them. The `let _ = rosters;` line keeps the parameter in the signature so later
tasks do not change it — remove that line in Task 3.

Add `pub mod game_feed;` to `lib.rs`, between `pub mod feed;` and `pub mod portal;`.

- [ ] **Step 5: Run the tests and confirm they pass**

```bash
cd /home/estraily/projects/refbox-overlay-delivery
cargo test -p overlay-bridge --lib game_feed
```

Expected: all eight tests PASS.

- [ ] **Step 6: Wire up the route and handler**

In `server.rs`'s `router()`, add after the `/nextgame` line:

```rust
        .route("/game", get(get_game))
```

And beside the other handlers:

```rust
/// The typed feed for a consumer that reads values rather than display strings -- see
/// `game_feed`'s module doc. Deliberately a separate route rather than extra columns on
/// `/scorebug`: a vMix title bound by position reads whichever column occupies its position, so
/// adding one to an existing table silently repoints every title after it (design spec §3).
async fn get_game(State(state): State<Arc<AppState>>) -> Json<game_feed::GameFeed> {
    let display = current_display(&state);
    let names = names_for_game(&state, display.snapshot.game_number());
    let rosters = current_rosters(&state, &display.snapshot);
    Json(game_feed::game_feed(
        &display,
        names.as_ref(),
        &rosters,
        is_connected(&state),
    ))
}
```

Add `game_feed` to `server.rs`'s existing `use crate::{...}` list.

- [ ] **Step 7: Add the end-to-end route test**

In `server.rs`'s `mod tests`:

```rust
#[tokio::test]
async fn the_game_route_serves_a_typed_object_not_a_table() {
    let state = Arc::new(AppState::new(config::Resolved::default()));
    mark_connected(&state);
    *write_lock(&state.live) = LiveState::new(
        GameSnapshot {
            current_period: GamePeriod::FirstHalf,
            secs_in_period: 431,
            scores: BlackWhiteBundle { black: 3, white: 2 },
            game_number: "10".to_string(),
            ..Default::default()
        },
        Instant::now(),
    );
    let addr = spawn_test_server(Arc::clone(&state)).await;

    let body = get_json(addr, "/game").await;

    // An object, not the array every vMix table serves -- there is no positional consumer here to
    // protect, so there is no table shape to preserve.
    assert!(body.is_object(), "/game should serve one object");
    assert_eq!(body["connected"].as_bool(), Some(true));
    assert_eq!(body["blackScore"].as_u64(), Some(3));
    assert_eq!(body["secsInPeriod"].as_u64(), Some(431));
    assert_eq!(body["schemaVersion"].as_u64(), Some(1));
}

/// A bridge that has never connected serves nulls, not zeros -- the same case
/// `a_never_connected_bridge_serves_connected_false_and_blank_tables` covers for the vMix tables.
#[tokio::test]
async fn a_never_connected_bridge_serves_a_null_game_feed() {
    let state = Arc::new(AppState::new(config::Resolved::default()));
    let addr = spawn_test_server(state).await;

    let body = get_json(addr, "/game").await;

    assert_eq!(body["connected"].as_bool(), Some(false));
    assert_eq!(body["schemaVersion"].as_u64(), Some(1));
    assert!(body["blackScore"].is_null(), "a blank score must not read as 0");
    assert!(body["period"].is_null());
    assert!(body["penalties"].is_null());
}
```

- [ ] **Step 8: Run the full crate suite**

```bash
cd /home/estraily/projects/refbox-overlay-delivery
cargo test -p overlay-bridge
cargo clippy -p overlay-bridge --all-targets --all-features -- -D warnings
cargo fmt --all
```

Expected: all tests pass, including Task 1's frozen-columns guard, and clippy is silent.

- [ ] **Step 9: Commit**

```bash
cd /home/estraily/projects/refbox-overlay-delivery
git add overlay-bridge/src/game_feed.rs overlay-bridge/src/lib.rs overlay-bridge/src/server.rs overlay-bridge/src/tables.rs
git commit -m "feat(overlay-bridge): serve the typed game feed at /game"
```

---

### Task 3: Penalties

**Files:**
- Modify: `overlay-bridge/src/game_feed.rs`

**Interfaces:**
- Consumes: `tables::Rosters`, `uwh_common::game_snapshot::{PenaltySnapshot, PenaltyTime}`.
- Produces: `game_feed::Penalty`, and `GameFeed::penalties` populated for real.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn penalties_are_neither_padded_nor_truncated() {
    // /penalties pads up to ten rows and takes only the first ten, because a vMix title needs a
    // fixed row count to bind to. An array needs neither, and the renderer is better served by
    // the truth -- the same reasoning /scorebug already applies to its untruncated foul counts.
    let empty = game_feed(
        &display_with(GameSnapshot::default()),
        None,
        &Rosters::default(),
        true,
    );
    assert_eq!(
        empty.penalties.as_deref(),
        Some(&[][..]),
        "no penalties must serve an empty array, not ten blank rows"
    );

    let mut snapshot = GameSnapshot::default();
    snapshot.penalties.black = (1..=12)
        .map(|n| PenaltySnapshot {
            player_number: n,
            time: PenaltyTime::Seconds(u16::from(n) * 10),
            infraction: Infraction::Unknown,
        })
        .collect();
    let many = game_feed(&display_with(snapshot), None, &Rosters::default(), true);
    assert_eq!(
        many.penalties.expect("connected").len(),
        12,
        "all twelve must be served, not the first ten"
    );
}

#[test]
fn a_total_dismissal_is_a_flag_and_a_null_never_td_or_zero() {
    let mut snapshot = GameSnapshot::default();
    snapshot.penalties.white = vec![PenaltySnapshot {
        player_number: 4,
        time: PenaltyTime::TotalDismissal,
        infraction: Infraction::Unknown,
    }];
    let feed = game_feed(&display_with(snapshot), None, &Rosters::default(), true);
    let penalty = &feed.penalties.expect("connected")[0];

    assert_eq!(penalty.team, "WHITE");
    assert_eq!(penalty.number, 4);
    assert!(penalty.total_dismissal);
    assert_eq!(
        penalty.secs_remaining, None,
        "a dismissal has no countdown -- 0 would read as about to expire"
    );

    let json = serde_json::to_value(&feed).expect("serialise");
    assert!(json["penalties"][0]["secsRemaining"].is_null());
    assert_eq!(json["penalties"][0]["totalDismissal"].as_bool(), Some(true));
}

#[test]
fn a_penalty_carries_the_roster_name_when_the_cap_number_is_known() {
    let mut snapshot = GameSnapshot::default();
    snapshot.penalties.black = vec![
        PenaltySnapshot {
            player_number: 7,
            time: PenaltyTime::Seconds(60),
            infraction: Infraction::Unknown,
        },
        PenaltySnapshot {
            player_number: 9,
            time: PenaltyTime::Seconds(30),
            infraction: Infraction::Unknown,
        },
    ];
    let mut rosters = Rosters::default();
    rosters.black.insert(7, "Known Player".to_string());

    let feed = game_feed(&display_with(snapshot), None, &rosters, true);
    let served = feed.penalties.expect("connected");

    assert_eq!(served[0].player.as_deref(), Some("Known Player"));
    assert_eq!(
        served[1].player, None,
        "an unknown cap number serves null, never a placeholder"
    );
}
```

Add `Infraction`, `PenaltySnapshot` and `PenaltyTime` to the test module's `use` list.

- [ ] **Step 2: Run and confirm failure**

```bash
cd /home/estraily/projects/refbox-overlay-delivery
cargo test -p overlay-bridge --lib game_feed
```

Expected: FAIL — `penalties` is the Task 2 stub (`Some(Vec::new())`), so the twelve-penalty and
dismissal assertions fail.

- [ ] **Step 3: Implement**

`Penalty` already exists from Task 2. First add the imports the builder needs — these are this
module's first `uwh_common` imports, and they belong here rather than in Task 2 because an unused
import fails the `-D warnings` gate:

```rust
use uwh_common::{
    color::Color,
    game_snapshot::{GameSnapshot, PenaltySnapshot, PenaltyTime},
};
```

Then add the builder:

```rust
/// Every penalty on the snapshot, ordered exactly as `tables::penalties` orders it so both
/// consumers agree on which penalty is first. **Neither padded nor truncated** -- see `Penalty`.
fn penalties_of(snapshot: &GameSnapshot, rosters: &Rosters) -> Vec<Penalty> {
    let mut entries: Vec<(Color, &PenaltySnapshot)> = snapshot
        .penalties
        .black
        .iter()
        .map(|penalty| (Color::Black, penalty))
        .chain(
            snapshot
                .penalties
                .white
                .iter()
                .map(|penalty| (Color::White, penalty)),
        )
        .collect();
    entries.sort_by_key(|(_, penalty)| std::cmp::Reverse(penalty.time));

    entries
        .into_iter()
        .map(|(color, penalty)| Penalty {
            team: color_code(color).to_string(),
            number: penalty.player_number,
            player: rosters[color].get(&penalty.player_number).cloned(),
            secs_remaining: match penalty.time {
                PenaltyTime::Seconds(secs) => Some(u32::from(secs)),
                PenaltyTime::TotalDismissal => None,
            },
            total_dismissal: matches!(penalty.time, PenaltyTime::TotalDismissal),
            infraction: penalty.infraction.short_name().to_string(),
        })
        .collect()
}
```

In `game_feed()`, remove the `let _ = rosters;` line and replace the stub:

```rust
        penalties: Some(penalties_of(snapshot, rosters)),
```

Add `PenaltySnapshot` and `PenaltyTime` to the module's `use uwh_common::game_snapshot::{...}`.

- [ ] **Step 4: Run and confirm pass**

```bash
cd /home/estraily/projects/refbox-overlay-delivery
cargo test -p overlay-bridge
cargo clippy -p overlay-bridge --all-targets --all-features -- -D warnings
```

Expected: all pass, guard test still green.

- [ ] **Step 5: Commit**

```bash
cd /home/estraily/projects/refbox-overlay-delivery
git add overlay-bridge/src/game_feed.rs
git commit -m "feat(overlay-bridge): serve every penalty on the game feed"
```

---

### Task 4: The event id and portal address, paired and credential-stripped

**Files:**
- Modify: `overlay-bridge/src/game_feed.rs`

**Interfaces:**
- Consumes: `GameSnapshot::{event_id, portal_base_url}`.
- Produces: `GameFeed::{event_id, portal_base_url}` populated; `without_credentials()` (private).

- [ ] **Step 1: Write the failing tests**

```rust
/// Spec §4b. An event id is useless and unsafe on its own: ids are not unique across portal
/// environments, so `1889-B` is one tournament on the development portal and a different one on
/// production. A consumer must be able to see which portal the id belongs to, so the two travel
/// together -- both or neither, as `server.rs` already treats them.
#[test]
fn the_event_id_and_portal_address_are_both_or_neither() {
    let paired = GameSnapshot {
        event_id: Some(EventId::from_partial("1889-B")),
        portal_base_url: Some("https://api.dev.uwhportal.com".to_string()),
        ..Default::default()
    };
    let feed = game_feed(&display_with(paired), None, &Rosters::default(), true);
    assert_eq!(feed.event_id.as_deref(), Some("events/1889-B"));
    assert_eq!(
        feed.portal_base_url.as_deref(),
        Some("https://api.dev.uwhportal.com")
    );

    let id_only = GameSnapshot {
        event_id: Some(EventId::from_partial("1889-B")),
        portal_base_url: None,
        ..Default::default()
    };
    let feed = game_feed(&display_with(id_only), None, &Rosters::default(), true);
    assert_eq!(feed.event_id, None, "an id with no portal must not be served");
    assert_eq!(feed.portal_base_url, None);

    let url_only = GameSnapshot {
        event_id: None,
        portal_base_url: Some("https://api.dev.uwhportal.com".to_string()),
        ..Default::default()
    };
    let feed = game_feed(&display_with(url_only), None, &Rosters::default(), true);
    assert_eq!(feed.event_id, None);
    assert_eq!(feed.portal_base_url, None);
}

/// The id must never be rendered through `Display`, which writes "Event ID events/1889-B" -- a
/// human label, not an id.
#[test]
fn the_event_id_is_not_its_display_label() {
    let snapshot = GameSnapshot {
        event_id: Some(EventId::from_partial("1889-B")),
        portal_base_url: Some("https://api.dev.uwhportal.com".to_string()),
        ..Default::default()
    };
    let feed = game_feed(&display_with(snapshot), None, &Rosters::default(), true);
    let served = feed.event_id.expect("paired, so served");
    assert!(
        !served.contains("Event ID"),
        "served {served:?} -- Display was used instead of .full()"
    );
}

/// `/game` is readable by anything on the network, and the portal address is normalised only by
/// trimming a trailing slash -- nothing strips a `user:password@` prefix. Serving it raw would put
/// a credential an operator typed into a custom site address onto the network.
#[test]
fn a_credential_in_the_portal_address_is_not_served() {
    for (raw, expected) in [
        (
            "https://someone:s3cret@api.dev.uwhportal.com",
            "https://api.dev.uwhportal.com",
        ),
        (
            "https://someone:p%40ss@api.dev.uwhportal.com/base",
            "https://api.dev.uwhportal.com/base",
        ),
        (
            "https://someone:has@at@api.dev.uwhportal.com/base",
            "https://api.dev.uwhportal.com/base",
        ),
        (
            "https://api.dev.uwhportal.com/base",
            "https://api.dev.uwhportal.com/base",
        ),
    ] {
        let snapshot = GameSnapshot {
            event_id: Some(EventId::from_partial("1889-B")),
            portal_base_url: Some(raw.to_string()),
            ..Default::default()
        };
        let feed = game_feed(&display_with(snapshot), None, &Rosters::default(), true);
        assert_eq!(
            feed.portal_base_url.as_deref(),
            Some(expected),
            "stripping {raw:?}"
        );
    }
}
```

Add `use uwh_common::uwhportal::schedule::EventId;` to the test module.

- [ ] **Step 2: Run and confirm failure**

```bash
cd /home/estraily/projects/refbox-overlay-delivery
cargo test -p overlay-bridge --lib game_feed
```

Expected: FAIL — both fields are still the Task 2 `None` stubs.

- [ ] **Step 3: Implement**

```rust
/// The address with any `user:password@` prefix removed, leaving scheme, host and path.
///
/// `base_url` is a plain `String` normalised only by trimming a trailing slash
/// (`uwh-common/src/uwhportal/mod.rs:179`), and nothing anywhere strips credentials from it. That
/// has never mattered while the value was only used to build requests, but this one is served over
/// HTTP to anything on the network.
///
/// Splits on the **last** `@` in the authority, so a password containing `@` cannot leave part of
/// itself behind. Anything without `://` is returned unchanged -- the refbox refuses non-http
/// schemes upstream, and inventing a repair here would be guessing.
fn without_credentials(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_string();
    };
    let (authority, path) = match rest.find('/') {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, ""),
    };
    let authority = match authority.rsplit_once('@') {
        Some((_credentials, host)) => host,
        None => authority,
    };
    format!("{scheme}://{authority}{path}")
}
```

In `game_feed()`, replace the two stub fields:

```rust
        // Both or neither -- see `without_credentials` and the design spec's §4b. `server.rs`'s
        // own `LastSeen` treats these two as a unit for the same reason.
        event_id: paired_ids.clone().map(|(id, _)| id),
        portal_base_url: paired_ids.map(|(_, url)| url),
```

and compute it just above the struct literal:

```rust
    let paired_ids = match (
        snapshot.event_id.as_ref(),
        snapshot.portal_base_url.as_deref(),
    ) {
        (Some(id), Some(url)) => Some((id.full().to_string(), without_credentials(url))),
        _ => None,
    };
```

- [ ] **Step 4: Run and confirm pass**

```bash
cd /home/estraily/projects/refbox-overlay-delivery
cargo test -p overlay-bridge
cargo clippy -p overlay-bridge --all-targets --all-features -- -D warnings
cargo fmt --all
```

- [ ] **Step 5: Run the full workspace gate**

```bash
cd /home/estraily/projects/refbox-overlay-delivery
just check
```

Expected: fmt, lint, tests and audit all clean. This is the gate the project requires before a PR.

- [ ] **Step 6: Commit**

```bash
cd /home/estraily/projects/refbox-overlay-delivery
git add overlay-bridge/src/game_feed.rs
git commit -m "feat(overlay-bridge): pair the event id with its portal address on the game feed"
```

---

## After the plan

The spec's §10 records one open item that does not gate this work: the field list was derived from
what the current overlay reads, not from the renderer effort's own requirements. Once `/game` is
running, the useful next step is to have Eric's colleague confirm the list against what their
renderer needs. Under §6's additive-safe rule, anything missing is a one-line addition and does not
bump `SCHEMA_VERSION`.

Not covered here, deliberately: typed `fouls` and `warnings` lists (spec §9), goal identity
(spec §2), and anything about rendering.
