# Live Updates Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** While Live Updates is on and the refbox runs a Portal event in hockey mode, push the live game state to the
portal every 5 s and on every meaningful change, without ever touching the score/stats queue; plus the App Options
rearrangement and the new LAST 10 SECONDS COUNTDOWN page.

**Architecture:** The UI thread computes a `LiveFrame` (where to send + what to send) from app state after every
message and publishes it into a `tokio::sync::watch` channel. A separate tokio task (the live sender) owns all
networking: it sends a frame immediately when it differs meaningfully from the last one sent, otherwise every 5 s,
one request at a time with a short timeout, and drops every failure. `uwh-common`'s `UwhPortalClient` gains one
request builder that takes the access key as an argument, so the shared client's own credential (owned by the
background uploader) is never touched.

**Tech Stack:** Rust 2024 / MSRV 1.85, iced 0.13, tokio (`watch`, `time`, `net` for tests), reqwest 0.12, serde.

**Spec:** `docs/superpowers/specs/2026-10-19-live-updates-design.md` (approved by the PO 2026-10-19). Read it before
any task: its "Traps already paid for" section applies to every task. The wire contract it restates is the settled
agreement with the portal lane; nothing in this plan may change what goes on the wire.

## Global Constraints

- Crates: `uwh-common` (Task 1 only) and `refbox`. No third crate. **No new dependencies**, including dev-dependencies.
- Wire: `PUT {portal}/api/events/{partialEventId}/live-games/{gameNumber}`, header `Authorization: Bearer <key>`,
  JSON body with exactly the keys `period`, `clockSeconds`, `clockRunning`, `timeout`, `scores` (nulls serialised,
  never skipped). Any 2xx = success.
- Heartbeat 5 s. Request timeout 3 s. At most one live request in flight. Never retried, never queued.
- Default ON (`live_updates = true` for fresh and old configs).
- UI copy, literal (PO): tile label `LAST 10 SECONDS` newline `COUNTDOWN`; the page title is the same key. Toggle
  label `LIVE UPDATES`. Countdown rows `SHOW COUNTDOWN` and `AUDIBLE COUNTDOWN` (one line each).
- All 15 locales in the same commit as any key change (`translation_consistency.rs`); CJK/Thai wording must fit the
  subset font (the build hard-errors otherwise).
- No `unwrap`/`expect` in production code without a "why this cannot panic" comment. Clippy `-D warnings`.
- Helpers leave changes **uncommitted**; the main session reviews and commits each task
  (`type(scope): description`, ending with the `Co-Authored-By` line).
- Run refbox tests with `cargo test -p refbox -- --test-threads=1` if random ALSA tests fail (no sound card here).

## Review Focus

1. **Site switch mid-flight:** a frame built while on the Portal must never be sent after the published key store has
   moved to another site (custom site, UWR) — expect a skip, not a send. Pinned in Task 3 (`skips_when_site_changed`).
2. **Degraded start (no portal client):** no sender task, no panic, frames still computed harmlessly. Pinned in Task 4
   (`no_client_means_no_sender`).
3. **Switched off while the clock is stopped** (no ticks run then): the published frame must become `None` on the
   APPLY itself, not at the next tick. Pinned in Task 4 (`turning_off_clears_frame_without_a_tick`).
4. **Score changed while the clock is stopped:** the frame must change on that action, so the sender sends at once.
   Pinned in Task 4 (`score_while_stopped_updates_frame`).
5. **Endpoint failing all day:** one log line per distinct failure, not one per 5 s. Pinned in Task 3
   (`failure_log_reports_each_distinct_failure_once`).

---

### Task 1: `uwh-common` — live state types and the request (HEAVY process: `uwh-common`)

**Files:**
- Create: `uwh-common/src/uwhportal/live.rs` (types + `From<TimeoutSnapshot>`), `pub mod live;` in
  `uwh-common/src/uwhportal/mod.rs`
- Modify: `uwh-common/src/uwhportal/mod.rs` — two methods on `UwhPortalClient`, beside `post_game_scores`

**Interfaces:**
- Produces:
  ```rust
  // uwh_common::uwhportal::live
  #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
  #[serde(rename_all = "camelCase")]
  pub struct LiveGameState {
      pub period: GamePeriod,          // serialises as the bare variant name, e.g. "SecondHalf"
      pub clock_seconds: u32,
      pub clock_running: bool,
      pub timeout: Option<LiveTimeout>, // null when None — do NOT add skip_serializing_if
      pub scores: Option<LiveScores>,   // null when None
  }
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
  pub enum LiveTimeout { White, Black, Ref, PenaltyShot }
  impl From<TimeoutSnapshot> for LiveTimeout { /* drops the seconds */ }
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
  pub struct LiveScores { pub black: u8, pub white: u8 }

  impl UwhPortalClient {
      /// Builds the PUT without sending it. Uses `access_key`, never `self.auth_header`.
      pub fn live_game_request(&self, access_key: &str, event_id: &EventId, game_number: &GameNumber,
          state: &LiveGameState, timeout: Duration) -> Result<reqwest::Request, Box<dyn Error>>;
      /// Builds (synchronously, so a caller's lock can drop before awaiting) and returns the send.
      /// Ok(()) on any 2xx; Err carrying the status and body otherwise.
      pub fn put_live_game(&self, access_key: &str, event_id: &EventId, game_number: &GameNumber,
          state: &LiveGameState, timeout: Duration)
          -> Result<impl Future<Output = Result<(), Box<dyn Error>>> + use<>, Box<dyn Error>>;
  }
  ```
  The game number goes in as one URL path segment (`Url::path_segments_mut().push`), so it is escaped.
  `UnsendableAccessKey` from `build_auth_header` surfaces as the `Err` of the outer `Result`.

- [ ] **Step 1: Failing tests** in `uwh-common/src/uwhportal/live.rs` / `mod.rs` tests (no network, no tokio):
  - `in_play_body_is_exact`: `serde_json::to_string(&state)` ==
    `{"period":"SecondHalf","clockSeconds":462,"clockRunning":true,"timeout":null,"scores":{"black":1,"white":3}}`
  - `between_games_body_serialises_nulls`: `timeout: Some(LiveTimeout::Ref)`, `scores: None` →
    contains `"timeout":"Ref"` and `"scores":null`.
  - `timeout_kind_drops_seconds`: `LiveTimeout::from(TimeoutSnapshot::PenaltyShot(37)) == LiveTimeout::PenaltyShot`
    (one assert per variant).
  - `live_request_shape`: client built with base `https://api.example.test/` and **no** token; request for event
    `events/1889-B`, game `"SF 1"` → method `PUT`, url
    `https://api.example.test/api/events/1889-B/live-games/SF%201`, header `authorization: Bearer KEY-1`,
    `content-type: application/json`, `request.timeout() == Some(&Duration::from_secs(3))`.
  - `live_request_leaves_client_token_alone`: after building with `KEY-1`, `client.has_token()` is still `false`.
- [ ] **Step 2:** `cargo test -p uwh-common --features std uwhportal` → the new tests FAIL (not defined).
- [ ] **Step 3:** Implement the types and both methods. Mirror `post_game_scores`'s logging style (`info!` on success,
  `warn!` with status on failure) but at `debug!` for success — this runs every 5 s.
- [ ] **Step 4:** Same command → PASS. Then `cargo clippy -p uwh-common --all-targets --all-features -- -D warnings`
  and `cargo build -p uwh-common --no-default-features` (no_std must still build) → clean.
- [ ] **Step 5:** Main session reviews (heavy: per-task review) and commits
  `feat(uwh-common): add the live game state request`.

### Task 2: `refbox` — the frame rules (pure)

**Files:**
- Create: `refbox/src/live_updates/mod.rs` (`mod live_updates;` in `refbox/src/main.rs`)
- Modify: `refbox/src/config.rs` — `live_updates_possible` / `effective_live_updates` next to `effective_fouls_tracked`
- Modify: `refbox/src/tournament_manager/mod.rs` — one read-only accessor

**Interfaces:**
- Consumes: Task 1 types.
- Produces:
  ```rust
  // config.rs
  pub fn live_updates_possible(mode: Mode, source: GameSource) -> bool; // Portal && (Hockey6V6 | Hockey3V3)
  pub fn effective_live_updates(stored: bool, mode: Mode, source: GameSource) -> bool; // stored && possible
  // tournament_manager
  pub fn game_clock_running(&self) -> bool; // clock_state.is_running() && time_pause_confirmation.is_none()
                                            // — NOT clock_is_running(), which reports the timeout clock
  // live_updates
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct LiveFrame { pub site: String, pub event: EventId, pub game: GameNumber,
                         pub state: LiveGameState, pub heartbeat: bool }
  pub struct LiveInputs<'a> { pub stored_enabled: bool, pub mode: Mode, pub source: GameSource,
      pub site: &'a str, pub event: Option<&'a EventId>, pub snapshot: &'a GameSnapshot,
      pub game_clock_running: bool }
  pub fn live_frame(i: &LiveInputs) -> Option<LiveFrame>;
  pub fn needs_send_now(last_sent: Option<&LiveFrame>, next: &LiveFrame) -> bool;
  ```
- Rules `live_frame` encodes (from the spec): `None` unless `effective_live_updates` and `event.is_some()`. A number is
  "real" when non-blank and not `"0"`. Read the RAW fields `snapshot.game_number` / `snapshot.next_game_number`, never
  `GameSnapshot::game_number()`.
  - Period ≠ `BetweenGames`: game = current (None if not real); state from the snapshot; `scores: Some`;
    `heartbeat: true`.
  - `BetweenGames`, next real: game = next; `scores: None`; `heartbeat: true`.
  - `BetweenGames`, next not real, current real: game = current; state forced to `clock_seconds: 0`,
    `clock_running: false`, `timeout: None`, `scores: None`; `heartbeat: false` (one send).
  - Otherwise `None`.
- `needs_send_now` is true when `last_sent` is None, or site/event/game differ, or `period`, `timeout`,
  `clock_running` or `scores` differ, or (`!next.state.clock_running` and `clock_seconds` differ — a clock edit).
  A running clock's ticking `clock_seconds` alone is NOT a change.

- [ ] **Step 1: Failing tests** in `live_updates/mod.rs` (build `GameSnapshot` with `..Default::default()`):
  - `no_frame_when_off_manual_custom_rugby_beeptest_or_no_event` — one assert per condition.
  - `in_play_names_current_game_with_scores`; `sudden_death_seconds_pass_through` (snapshot 75 → 75).
  - `break_names_next_game_without_scores`; `break_after_reset_with_current_zero_names_next_game`
    (current `"0"`, next `"1"` → game `"1"`).
  - `finished_court_sends_one_forced_frame` (next `""`, current `"12"`, snapshot secs 40 → game `"12"`,
    `clock_seconds 0`, `clock_running false`, `heartbeat false`).
  - `nothing_real_means_no_frame` (`"0"` and `""`).
  - `clock_running_comes_from_input_not_timeout` (timeout `Some(Ref(5))`, `game_clock_running: false` →
    `clock_running false`, `timeout Some(Ref)`).
  - `ticking_clock_is_not_a_change`, `clock_edit_while_stopped_is_a_change`, `score_change_is_a_change`,
    `timeout_start_is_a_change`, `game_number_change_is_a_change`, `site_change_is_a_change`.
  - In `config.rs` tests: `live_updates_possible_only_for_portal_hockey` (all 4 modes × 3 sources).
  - In `tournament_manager` tests: `game_clock_running_ignores_timeout_clock` (start a ref timeout while the game
    clock is stopped → `clock_is_running()` true, `game_clock_running()` false) and
    `game_clock_running_false_during_score_confirmation` (open the confirmation pause at game end → false).
- [ ] **Step 2:** `cargo test -p refbox live_updates config tournament_manager::tests::game_clock` → FAIL.
- [ ] **Step 3:** Implement. No I/O in this module apart from types.
- [ ] **Step 4:** Same → PASS; `just lint` clean.
- [ ] **Step 5:** Main session commits `feat(refbox): decide what live update to send`.

### Task 3: `refbox` — the live sender task

**Files:**
- Create: `refbox/src/live_updates/sender.rs`
- Modify: `refbox/src/portal_manager/mod.rs` — `SiteAccessKeys::key_for` → `pub(crate)`, plus
  `pub(crate) fn site(&self) -> &str`

**Interfaces:**
- Consumes: Task 1 `put_live_game`; Task 2 `LiveFrame`, `needs_send_now`; `SharedAccessKeys`,
  `SharedUwhPortalClient` from `portal_manager`.
- Produces:
  ```rust
  pub struct LiveTiming { pub heartbeat: Duration, pub request_timeout: Duration }
  pub const LIVE_TIMING: LiveTiming = LiveTiming { heartbeat: Duration::from_secs(5),
                                                   request_timeout: Duration::from_secs(3) };
  pub fn spawn_live_sender(frames: watch::Receiver<Option<LiveFrame>>, client: SharedUwhPortalClient,
                           keys: SharedAccessKeys, timing: LiveTiming) -> tokio::task::JoinHandle<()>;
  ```
- Behaviour: one loop, sequential — each send is awaited before the next decision, which is what guarantees one in
  flight; the watch channel holds only the newest frame, which is what coalesces. Per iteration: read the current
  frame; `None` → forget `last_sent`, wait for a change. Otherwise send if `needs_send_now(last_sent, frame)`, or if
  `frame.heartbeat` and `timing.heartbeat` has passed since the last attempt; else wait for a change or the heartbeat
  deadline. Building a send: lock keys THEN client (the `request_for` order — never the reverse), skip with a reason
  if `keys.site() != frame.site` or there is no key for `frame.event`, call `put_live_game`, drop both guards, then
  await. A completed attempt (success or failure) updates `last_sent` and the attempt time — a failure is never
  retried early. Channel closed → task ends.
- Failure logging: a small `FailureLog` with `fn record(&mut self, outcome: Result<(), String>) -> Option<LogLine>`
  — logs a `warn!` the first time a failure text appears or changes, an `info!` once when sends succeed again,
  nothing otherwise.

- [ ] **Step 1: Failing tests** (`#[tokio::test]`, a `tokio::net::TcpListener` on `127.0.0.1:0` acting as the portal;
  read the raw HTTP request, answer `HTTP/1.1 204 No Content` unless the test says otherwise; client built with
  `require_https = false`; short timing, e.g. heartbeat 300 ms, timeout 150 ms):
  - `sends_put_with_body_and_key` — path, `authorization`, and JSON body match the frame.
  - `heartbeat_resends_unchanged_frame` — ≥2 requests within ~2.5 heartbeats.
  - `change_sends_without_waiting_for_heartbeat` — replace the frame with a new score; request arrives well before
    the heartbeat.
  - `stalled_server_is_abandoned_and_next_send_proceeds` — the server never answers the first request; the second
    arrives after ~timeout.
  - `never_two_in_flight` — the server holds each request for 2× heartbeat while counting open requests; max seen == 1.
  - `finished_court_sends_once` — a `heartbeat: false` frame produces exactly one request over 3 heartbeats.
  - `none_stops_sending`.
  - `skips_when_site_changed` — publish keys for a different site than `frame.site` → zero requests.
  - `skips_without_key` → zero requests.
  - `failure_log_reports_each_distinct_failure_once` (pure): 10× same error → 1 line; a different error → 1 more;
    success → 1 info; another success → none.
- [ ] **Step 2:** `cargo test -p refbox live_updates::sender -- --test-threads=1` → FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Same → PASS, and repeat the run 3 times to check the timing tests are stable; `just lint` clean.
- [ ] **Step 5:** Main session commits `feat(refbox): send live updates without touching the result queue`.

### Task 4: `refbox` — the setting and the app wiring

**Files:**
- Modify: `refbox/src/config.rs` — `Config.live_updates: bool` (`#[serde(default = …true)]`,
  `#[derivative(Default(value = "true"))]`), plus the migration read (`get_boolean_value(old, "live_updates", …)`)
- Modify: `refbox/src/app/mod.rs` — `EditableSettings.live_updates`; `PageEntrySnapshot::App` field; `revert_into`;
  commit in `apply_app_options` (where the apply succeeds — see the spec's settings trap); the `watch::Sender`
  field; spawning the sender; the per-message refresh
- Modify: `refbox/src/app/view_builders/configuration.rs` — `page_has_changes` App arm's `||` chain (one line; easy
  to miss, and nothing fails at compile time)

**Interfaces:**
- Consumes: Tasks 2–3.
- Produces: `RefBoxApp::refresh_live_frame(&mut self)` — builds `LiveInputs` from `self.config.live_updates`,
  `self.config.mode`, `self.source`, `self.current_site.base_url.expose()`, `self.current_event_id`, `self.snapshot`
  and `self.tm.lock().game_clock_running()`, and publishes `live_frame(..)` with `send_if_modified` (only when it
  differs). It is called after EVERY message: the existing `update` body moves into a private
  `fn handle_message(&mut self, message) -> Task<Message>` and `update` becomes
  `let task = self.handle_message(message); self.refresh_live_frame(); task`. This guards the whole class (ticks,
  operator actions while stopped, settings applies, source/event switches) instead of listing call sites.
- `refresh_live_frame` reads `self.snapshot`. If `score_while_stopped_updates_frame` shows that an operator action
  while stopped leaves `self.snapshot` stale, fix it by applying a fresh snapshot on that action path (as
  `apply_snapshot` already does elsewhere). Do not call `generate_snapshot` from the refresh: it has side effects.
- The sender is spawned once at startup where the portal client and `portal_access_keys` are built, only when a
  client exists (degraded start: no client → no sender; the `watch::Sender` still exists and its frames go nowhere).

- [ ] **Step 1: Failing tests:**
  - `config.rs`: `live_updates_defaults_true_when_absent` (old-format and current-format files without the key) and
    `live_updates_false_round_trips`.
  - `configuration.rs`: `app_page_has_changes_when_live_updates_toggled`.
  - `app/mod.rs` (construct the app the way existing app tests do; read the receiver side of the watch):
    `portal_hockey_linked_publishes_a_frame`, `turning_off_clears_frame_without_a_tick` (apply App settings with
    live updates off while the clock is stopped → `None` immediately), `score_while_stopped_updates_frame`,
    `custom_source_publishes_nothing`, `no_client_means_no_sender`, `cancel_leaves_live_updates_uncommitted`
    (toggle, then CANCEL / Back / `ConfigEditComplete` → `config.live_updates` unchanged).
- [ ] **Step 2:** `cargo test -p refbox live_updates config configuration app::tests -- --test-threads=1` → FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Same → PASS; `just check` green.
- [ ] **Step 5:** Main session commits `feat(refbox): add the live updates setting`.

### Task 5: `refbox` — App Options rearrangement and the LAST 10 SECONDS COUNTDOWN page

**Files:**
- Modify: `refbox/src/app/view_builders/configuration.rs` — `make_app_config_page`; new
  `make_countdown_edit_page` (built like the Half Length editor: `make_game_time_button`, centred title, two
  YES/NO selector rows in the TWO HALVES / ONE PERIOD style — `blue_selected_button` for the current choice,
  `light_gray_button` for the other, the current one pressing `Message::NoAction` — `vertical_space`, then
  CANCEL (`cancel_or_back_label`) | space | APPLY)
- Modify: `refbox/src/app/mod.rs` — `AppState::CountdownEditor { show_countdown: bool, audible_countdown: bool }`;
  `Message::EditCountdown` opens it, seeded from `edited_settings` (`show_countdown = !hide_time`); the existing
  `ToggleBoolParameter(HideTime | AudibleCountdown)` arms toggle the editor's staged values while in that state
  (the `SingleHalf` precedent); `ParameterEditComplete { canceled }` gets an arm that writes
  `hide_time = !show_countdown` and `audible_countdown` into `edited_settings` and returns to
  `AppState::EditGameConfig(ConfigPage::App)` **without** re-capturing the App page's entry snapshot (or APPLY never
  enables); both `unreachable!()` matches in that handler get the new arm
- Modify: `refbox/src/app/message.rs` — `Message::EditCountdown`; `BoolGameParameter::LiveUpdates`
- Modify: `refbox/translations/*/refbox.ftl` (15 files) — new keys `last-10-seconds-countdown` (en-US:
  `LAST 10 SECONDS` newline `COUNTDOWN`) and `live-updates` (`LIVE UPDATES`); change the VALUES of
  `show-countdown-for-last-10-seconds` → `SHOW COUNTDOWN` and `audible-countdown-for-last-10-seconds` →
  `AUDIBLE COUNTDOWN`, each one line, keys not renamed. Keep each locale's own words, dropping its "last 10 seconds"
  part; reword where it is woven in (de, ja, ko, zh, tr, th…). List every reworded locale in the task report.

Layout (right column; left column unchanged):
```
APP: …            | CONFIRM SCORE
TRACK CAP NUMBER  | LAST 10 SECONDS / COUNTDOWN  (make_tile_button, light_gray_button, → EditCountdown)
FORCE KEYPAD      | SHOW BEHIND TIME/DELAY
TRACK FOULS       | LIVE UPDATES: <effective YES/NO>  (greyed — message None — when !live_updates_possible(
                                                        settings.mode, settings.source))
```

- [ ] **Step 1: Failing tests** (`configuration.rs` / `app/mod.rs`):
  - `countdown_editor_apply_stages_into_app_page` — open, toggle AUDIBLE, APPLY → back on App page,
    `edited_settings.audible_countdown` flipped, `config` unchanged, App page `page_has_changes` true.
  - `countdown_editor_cancel_discards` — toggle, CANCEL → `edited_settings` unchanged.
  - `live_updates_tile_greyed_and_no_outside_portal_hockey` — test the value/enabled helper the view uses, for
    Manual, Custom, Rugby, BeepTest, and Portal+Hockey.
  - The translation tests in `translation_consistency.rs` stay green with the new keys.
- [ ] **Step 2:** `cargo test -p refbox configuration app::tests translation_consistency -- --test-threads=1` → FAIL.
- [ ] **Step 3:** Implement. Check that the subset font covers every new CJK/Thai character (a hard build error if not;
  choose wording from characters already in the font, never regenerate the font).
- [ ] **Step 4:** Same → PASS; `just check` green.
- [ ] **Step 5:** Main session commits `feat(refbox): move the countdown switches and add live updates to app options`.

### Task 6: Verification and disclosure (main session, no helper)

- [ ] `just check` green on the branch, rebased onto a fresh `origin/master` first.
- [ ] Data measurement: run the sender against a local listener that counts bytes in and out for 10 minutes of a
  running game (heartbeat + some changes); report KB/hour, and say it was measured locally (TLS not included). If the
  portal lane's endpoint is deployed on the dev portal by then, measure there instead (`UWH_PORTAL_URL_OVERRIDE`).
- [ ] Pre-PR check 1: `.claude/rules/pr-review.md` checklist plus the `code-review` skill against the branch diff
  (`git diff origin/master...HEAD`, run from this worktree).
- [ ] Pre-PR check 2: numbered walkthrough steps for the PO — App Options layout, the countdown page (CANCEL vs APPLY,
  then App APPLY), the greyed NO in Manual/Custom/Rugby, and (once deployed) a live game on the dev Portal's Schedule
  tab.
- [ ] Record any deviation in a "Deviations" section at the foot of this plan (no separate deviation commits).

## Deviations

Rulings made during execution (2026-10-19). Each names what changed against the text above.

1. **`BoolGameParameter::LiveUpdates` moved from Task 5 to Task 4.** Task 4's own tests needed the toggle; Task 5
   only renders the tile.
2. **Failed live sends log at `debug!` inside `uwh-common`.** Task 1's brief said `warn!` on every non-2xx; that
   would have logged every 5 s. All user-visible failure logging lives in the refbox sender.
3. **The failure log is rate-limited, not just de-duplicated.** Each failure kind is logged at most once per
   10 minutes, and the "getting through again" line at most once per 10 minutes, with a count of dropped sends.
   Remembering only the last kind would still have logged every 5 s on a flapping link.
4. **Intermediate commits failed `just lint` on dead code** until Task 5 rendered the tile. No `#[allow]` was added.
   The finished branch is lint-clean.
5. **Task 4's app-level tests were replaced by pure seams.** No test can construct `RefBoxApp` (`RefBoxApp::new`
   starts audio, writes the real settings file and opens sockets). Instead: `published_site` is shared by every
   key-store publish and the live frame, so the two site strings cannot differ; `live_inputs`, `publish_live_frame`
   and `spawn_live_sender_if_client` are tested on their own; and every operator action possible while the clock is
   stopped was code-read and found to refresh the snapshot. The `update` → `refresh_live_frame` wrapper itself is
   proven by review and by the walkthrough, not by a test.
6. **Final-review fixes:**
   - A frame is published only when the current site serves the current game source.
   - A missing access key or a site change is a send condition not met, not a failure, so it is logged at debug only.
   - With Live Updates off, the per-message refresh returns before any work.
   - The missing proof that a stalled live send does not delay a score post was added as a test.
7. **The countdown page title shows on two lines.** It reuses the tile's key (`LAST 10 SECONDS` / `COUNTDOWN`), while
   the approved mockup drew it on one line. Left for the PO's walkthrough.
8. **Translations.** de, nl, tr, ja, ko and zh were re-worded rather than trimmed. ko `생중계 업데이트` and zh `即时更新`
   were chosen because the subset font lacks the more usual `실시간` / `实时`. None of the 14 non-English locales has
   been checked by a native speaker.
