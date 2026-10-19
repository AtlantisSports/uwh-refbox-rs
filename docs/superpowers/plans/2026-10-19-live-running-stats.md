# Live Running Stats Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** While a game is in play, the refbox PUTs its running list of goals and penalties to `…/live-games/{n}/stats`. It sends whenever the list changes, retries every 15 s until the send is accepted, and drops a pending retry once the game ends. None of this touches the final stats upload, its queue or the operator's portal light.

**Architecture:** The engine builds the list from the same event objects the final upload already uses (`GameStats`' private `Event`), adding the penalties still running. It also returns a change key: the list with `occurredOn` removed, because `occurredOn` is re-derived from the wall clock on every build and so jitters. The app reads the list in the same engine lock as the live frame and publishes a `StatsFrame` on its own `watch` channel. A new sender task sends the newest frame whenever its key differs from the last one accepted, and retries on failure.

**Tech Stack:** Rust 2024, MSRV 1.85, serde_json, tokio, reqwest (all already in use).

**Spec:** `/home/estraily/projects/uwh-portal-live-schedule/docs/superpowers/specs/2026-10-19-live-schedule-refbox-live-updates-agreement.md`, section "Running stats during play" (items 9-11 SETTLED). READ ONLY: it belongs to uwh-portal. Base: branch `feat/uwh-common/live-updates-v3` @ 5ebcace0 (v3, itself stacked on v2 PR #3633).

**Scope Card**
Task: Build the in-play running stats send: during play the refbox PUTs its goals + penalties list to `…/live-games/{n}/stats` on change, retried every 15 s until accepted, per the agreement's "Running stats during play".
Explicitly not doing: any change to the final stats upload, its queue or the operator's portal light; sending fouls or saving the list to disk; anything in uwh-portal or the v3 live fields.

## Global Constraints

- MSRV 1.85, edition 2024; clippy `-D warnings`; no new dependencies; no `unwrap`/`expect` in production code without a "why this cannot panic" comment; no `#[allow]` without discussion.
- Body: a JSON array ordered by `occurredOn`, using EXACTLY the final upload's event objects. Goal: `{"$type":"goal","playerCapNumber","side":"dark"|"light","gamePeriod","periodTime","occurredOn"}`. Penalty: the same plus `"duration"` (seconds, or null if and only if `"isTotalDismissal"` is true). No fouls, ever. `[]` before anything has happened.
- Contents: every goal so far, and every penalty GIVEN so far, served or still running. A running penalty is in the list from the moment it is given. A goal taken back by a score edit stays in the list (as in the final upload). An edited or deleted running penalty changes or disappears in the next list.
- Sent only under the v2 conditions (Live Updates on, source Portal, Hockey 6v6/3v3, an event, a key for it, a site that serves the source, a real game number) AND only while the period is not `BetweenGames`. The game number is always the game IN PLAY (the raw `game_number`).
- Sent once whenever the list changes, NOT on a timer. At most one send in flight. Request timeout 3 s. Only the newest list is kept.
- A failed send is retried every **15 s** until it is accepted (any 2xx; the portal answers `204`, including for a late list after final stats), a newer list replaces it, or the game ends. Failures are logged quietly (once per kind per 10 min, as v2 does) and never shown to the operator. Nothing is saved to disk.
- When the game ends, or the conditions stop holding, the pending retry is dropped. If the conditions return mid-game, the current list is sent again.
- Never touch `GameSnapshot`, `PenaltySnapshot`, the score/stats queue, `portal_manager/health.rs` or the final `post_game_stats` path.
- Build and test only in `/home/estraily/projects/uwh-refbox-rs-live-stats`, using absolute paths. Leave the walkthrough refbox (pids 884863/884864) alone. Refbox tests use `--test-threads=1` (no sound card here).

## Review Focus

1. **Wall-clock jitter.** Building the list twice with nothing changed must give the same key even though `occurredOn` differs by microseconds, or the refbox sends on every message. Pinned in Task 2.
2. **A penalty being culled** (moved from the running list into `GameStats` at a period start) must keep the key unchanged: same content, no extra send. Pinned in Task 2.
3. **No duplicates.** A penalty must be in the list exactly once, whether running, served-but-not-culled or culled. Check that no path re-enters play after `end_game` (which copies running penalties into `GameStats`) without going through `start_game`'s reset. Pinned in Task 2, with the premise verified against the code and stated in the report.
4. **An old portal without the endpoint** answers 404. The send is retried every 15 s all game, logged quietly, with the live sends and the final upload unaffected. Pinned in Task 3 (404 is retried and logged at most once per window).
5. **A list change during a failing retry.** The next attempt carries the NEWEST list, never the stale one. Pinned in Task 3.

---

### Task 1: the stats PUT (uwh-common)

**Files:** Modify `uwh-common/src/uwhportal/mod.rs` (next to `live_game_request` / `put_live_game`, plus the `live_request_tests` module).

**Interfaces — Produces:**
```rust
impl UwhPortalClient {
    /// `PUT {base}/api/events/{partial}/live-games/{gameNumber}/stats`, game number pushed as one escaped segment,
    /// then the literal segment `stats`. Body: `events_json` as-is, `Content-Type: application/json`. Auth as live_game_request.
    pub fn live_game_stats_request(&self, access_key: &str, event_id: &EventId, game_number: &GameNumber,
        events_json: &str, timeout: Duration) -> Result<reqwest::Request, Box<dyn Error>>;
    /// Same contract as `put_live_game`: built before returning; Ok(()) on any 2xx; else "{status}: {body}".
    pub fn put_live_game_stats(&self, access_key: &str, event_id: &EventId, game_number: &GameNumber,
        events_json: &str, timeout: Duration)
        -> Result<impl Future<Output = Result<(), Box<dyn Error>>> + use<>, Box<dyn Error>>;
}
```
Share the URL-building and the response handling with the live-games pair: no verbatim copy of either block.

- [ ] **Step 1:** Test `live_stats_request_shape`: method PUT; URL `https://api.example.test/api/events/1889-B/live-games/SF%201/stats`; `Bearer KEY-1`; `application/json`; timeout 3 s; body bytes equal `[]` exactly. Test `live_stats_request_leaves_client_token_alone` and `live_stats_request_refuses_an_unsendable_key` (mirroring the live ones).
- [ ] **Step 2:** `cargo test -p uwh-common live_` → FAIL. **Step 3:** implement. **Step 4:** `cargo test -p uwh-common` → PASS (existing live tests unchanged).
- [ ] **Step 5: Commit** `feat(uwh-common): add the in-play stats request`

### Task 2: the running list and its change key (refbox engine)

**Files:** Modify `refbox/src/tournament_manager/game_stats.rs`, `refbox/src/tournament_manager/mod.rs`; Create nothing else.

**Interfaces — Produces:**
```rust
// tournament_manager, re-exported
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunningStats {
    /// The PUT body: JSON array, ordered by occurredOn.
    pub body: String,
    /// The same list with every `occurredOn` removed, serialised: equal keys = same list.
    pub key: String,
}
impl TournamentManager { pub fn running_stats(&self) -> RunningStats; } // read-only
// game_stats.rs
impl GameStats { pub(crate) fn running_list(&self, running: &BlackWhiteBundle<Vec<Penalty>>) -> RunningStats; }
```
`running_list` takes the Goal and Penalty events already in `GameStats` (never Foul), adds a Penalty event for each running penalty, built by the SAME code `add_penalty` uses (extract it into one function that returns the `Event`), sorts by `occurred_on` as `as_json` does, and serialises. The key is the list as `serde_json::Value`s with `occurredOn` removed, then serialised.

- [ ] **Step 1: Failing tests** in `tournament_manager/mod.rs` tests (real engine, in FirstHalf):
  - `running_stats_lists_goals_and_given_penalties_without_fouls`: a goal, a TwoMinute that is served and culled, a running OneMinute, a TotalDismissal and a foul. The body has 4 events: the `$type`s, caps, sides, `duration` 120/60/null and `isTotalDismissal` false/false/true are as given, it is in `occurredOn` order, and there is no `foul`.
  - `running_stats_empty_list_at_game_start`: body `[]`.
  - `running_stats_key_ignores_wall_clock_jitter`: two calls with nothing changed give an equal key (the bodies may differ) (Review Focus 1).
  - `running_stats_key_unchanged_across_cull`: a served penalty, key before vs after the period start culls it (Review Focus 2).
  - `running_stats_key_changes_on_goal_penalty_edit_delete`: each of those four changes the key.
  - `running_stats_penalty_listed_once`: a penalty appears once while running, once after cull, and the next game's list after `start_game` holds none of the previous game's events (Review Focus 3). The report states which code path guarantees no re-entry into play after `end_game` without a reset.
- [ ] **Step 2:** `cargo test -p refbox running_stats -- --test-threads=1` → FAIL. **Step 3:** implement. **Step 4:** `cargo test -p refbox tournament_manager -- --test-threads=1` → PASS, including the existing `end_game_*` / `as_json` tests unchanged.
- [ ] **Step 5: Commit** `feat(refbox): build the in-play stats list`

### Task 3: the stats frame and its sender (refbox live_updates)

**Files:**
- Modify `refbox/src/live_updates/mod.rs`: `StatsFrame`, `stats_frame`.
- Create `refbox/src/live_updates/stats_sender.rs`: the loop.
- Create `refbox/src/live_updates/test_portal.rs` (`#[cfg(test)]`): MOVE the fake portal (`Reply`, `Recorded`, `Portal`, `portal()`, `serve()`, `shared()`, `answer_at_once`) out of `sender.rs`'s tests so both senders' tests use it. A pure move: `sender.rs` tests keep passing unchanged.
- Modify `refbox/src/live_updates/sender.rs`: extract the keys-then-client lock step of `build_send` into `pub(super) fn keyed_send<F, T>(client, keys, site, event, f: F) -> Result<T, Failure>` that both senders call, so the lock order is written once. Give `FailureLog` a wording: `FailureLog::new(LogWording)`, `LogWording::LIVE` (today's exact text: v2 tests unchanged) and `LogWording::STATS`. STATS text: "in-play stats to the portal failed, and will be retried: …", recovery "in-play stats to the portal are getting through again ({n} failed attempts)", skip "in-play stats to the portal not sent: …".

**Interfaces — Produces:**
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatsFrame { pub site: String, pub event: EventId, pub game: GameNumber, pub list: RunningStats }
impl StatsFrame { pub fn same_list(&self, other: &StatsFrame) -> bool; } // site, event, game, list.key, never list.body
/// `Some` only under live_frame's v2 conditions AND period != BetweenGames AND a real current game number.
pub fn stats_frame(i: &LiveInputs, stats: Option<&RunningStats>) -> Option<StatsFrame>; // None when stats is None
pub struct StatsTiming { pub retry: Duration, pub request_timeout: Duration }
pub const STATS_TIMING: StatsTiming = StatsTiming { retry: Duration::from_secs(15), request_timeout: Duration::from_secs(3) };
pub fn spawn_stats_sender(frames: watch::Receiver<Option<StatsFrame>>, client: SharedUwhPortalClient,
    keys: SharedAccessKeys, timing: StatsTiming) -> tokio::task::JoinHandle<()>;
```
The loop keeps `accepted: Option<StatsFrame>`. It reads the newest frame. On `None`, it clears `accepted` and waits for a change. When the frame is the `same_list` as `accepted`, it waits for a change. Otherwise it sends once (each send awaited, so at most one in flight). On 2xx, `accepted` is set. On any failure, including no key or a changed site, it logs through `FailureLog::new(LogWording::STATS)` and then waits for a change OR `retry`, whichever comes first, before reading the newest frame again. It ends when the channel closes.

- [ ] **Step 1: Failing tests.** In mod.rs: `stats_frame_none_when_off_manual_custom_rugby_beeptest_or_no_event`, `stats_frame_none_between_games`, `stats_frame_none_for_unreal_game_number`, `stats_frame_names_current_game`, and `same_list_ignores_body` (same key with different bodies is the same; a different key, game, event or site is not). In stats_sender.rs, using the fake portal: `sends_put_to_stats_path_with_body`, `unchanged_list_not_resent`, `changed_list_sent_once`, `failure_retried_after_retry_interval` (retry 100 ms; 500 then 204 gives exactly 2 PUTs), `retry_carries_newest_list` (Review Focus 5), `none_drops_pending_retry` (no PUT after None), `accepted_list_stops_retries`, `stalled_send_abandoned_by_timeout`, `never_two_in_flight`, `not_found_is_retried_and_logged_once_per_window` (Review Focus 4, at the `FailureLog` level), `conditions_return_resends_current_list` (Some → None → the same Some is sent again). In sender.rs: the existing log tests pass unchanged, plus `stats_wording_says_retried`.
- [ ] **Step 2:** `cargo test -p refbox live_updates -- --test-threads=1` → the new tests FAIL. **Step 3:** implement. **Step 4:** the same command → PASS (all v2/v3 sender tests included).
- [ ] **Step 5: Commit** `feat(refbox): send the in-play stats list with retries`

### Task 4: wire it into the app

**Files:** Modify `refbox/src/app/mod.rs` (`LiveEngineRead`, `live_engine_read`, `refresh_live_frame`, the app struct field next to `live_frames`, the spawn next to `spawn_live_sender_if_client`, tests). Add a short note to `docs/superpowers/specs/2026-10-19-live-updates-design.md` under the v3 note, pointing at the agreement's "Running stats during play".

**Interfaces:** `LiveEngineRead` gains `running_stats: RunningStats`, read in the same lock and at the same `now`. Its fallback accessor (the snapshot falling back to the app's) yields `None` for the stats as well as for the details. `refresh_live_frame` publishes both frames from one read through `live_frame_for_site`'s site guard (the stats frame must pass the same `site_serves` check). It publishes `None` to both when Live Updates is off. Publishing compares with `same_list`, never `==`, so a jittering body never wakes the sender. On a degraded start (no client), no stats sender runs, as for the live sender.

- [ ] **Step 1: Failing tests:** `stats_frame_from_engine_carries_goal_and_running_penalty`, `fallback_snapshot_drops_running_stats`, `publish_stats_frame_ignores_body_only_change`, `stats_frame_blocked_when_site_does_not_serve_source`.
- [ ] **Step 2:** `cargo test -p refbox app:: -- --test-threads=1` → FAIL. **Step 3:** implement plus the doc note. **Step 4:** `just check` in the worktree → green.
- [ ] **Step 5: Commit** `feat(refbox): send in-play stats while a game is live`

---

## Acceptance

- `just check` green. Each Global Constraint and Review Focus line is pinned by a named test.
- **Leg 3 (Claude-driven, needs Eric's go-ahead):** the local portal (slot 3) has no `/stats` endpoint yet (portal PR #1135 is v2). A game against it with a goal and penalties must show: stats PUTs at each change, a 404 retried every 15 s and logged quietly, every live-games PUT still accepted, the operator's portal light unaffected, and the final score and stats still uploaded at game end.
- **Leg 2:** nothing new is visible until the portal builds its in-play Stats. The walkthrough is a regression check: the live score and the end-of-game result upload behave exactly as before.

## Deviations

- Task 1 shares the live-games URL and reply code through private helpers (`live_game_url`, `live_put`, `send_live_put`). The live-games request is unchanged apart from checking the URL before the key.
- Task 2: the plan's Review Focus 3 premise is FALSE on one path. A same-game sleep catch-up (`place_at_schedule_position`) re-enters play without a reset and lists a penalty twice. It is pre-existing (the final upload double-records too), was left for a separate branch, and is recorded in the refbox known-bugs list. The PR must name it.
- Task 3 also extracted `keyed_send` and `write_log`, gave `FailureLog` a `LogWording` (LIVE text unchanged), moved the fake portal to `live_updates/test_portal.rs`, and routed the v2 send conditions through one `send_event` helper.
- Task 4: on a FALLBACK engine read the stats frame is left as it was, not set to `None` (a ruling: `None` would drop a pending retry and resend an accepted list). The plan's test `fallback_snapshot_drops_running_stats` became `fallback_read_leaves_the_stats_frame_alone`. The site guard is a separate `stats_frame_for_site`. The Live-Updates-off `None` publish is read-verified only and is covered by a leg-3 step.
- Tasks 2 and 3 commits carry clippy `dead_code` until Task 4 adds the callers. The branch tip is clean. No `#[allow]`.
- Final review: added the goal-taken-back test, made the running list's sort stable, and corrected the design note's retry clause.
