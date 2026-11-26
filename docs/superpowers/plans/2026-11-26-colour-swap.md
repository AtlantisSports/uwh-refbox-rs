# Swap a Game's Colours (#152 Part 2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A SWAP COLOURS button on the score edit page that moves the two team names (and their cap-number lists) to the other colour, and tells the portal which team wore which colour before that game's final score is sent.

**Architecture:** The portal client in `uwh-common` gains one call (`post_game_colours`) and a plain function that sorts the site's answer into applied / refused / not accepted / retry. The refbox send queue gains a second list, for swaps. A game's final score waits in the queue while a swap for the same game is still there. The refbox keeps its own record of each saved swap and lays it over every schedule it receives, so its screens show the swap even before the portal has it.

**Tech Stack:** Rust 2024, iced 0.13, reqwest (via `uwh-common` `std` feature), serde_json, Fluent translations.

**Spec:** `docs/superpowers/specs/2026-11-02-forfeit-skip-and-colour-swap-design.md` (commit 4a922951, branch `docs/refbox/forfeit-skip-colour-swap-design`), section "Part 2 — swapping a game's colours" and the "Agreement" block. Part 1 (forfeit skip) and the R8 overlay re-read are NOT in this plan.

**PO rulings made while planning (2026-11-26, design lane chat):**
- P1. A permanent refusal from the site (named reason, e.g. `AlreadyScored`) is **logged only**: the operator is not told, the same as "site does not accept swaps". This replaces the spec's "the refbox shows the reason".
- P2. The SWAP COLOURS button is **hidden whenever it cannot work**: Manual mode (no schedule), no game number, or either side of the game without a real team id (placeholder such as "Winner of game 9").
- Earlier (handover 2026-11-26): build now, ahead of the portal. Unsupported = keep the local swap, log, stop sending; operator not told.

## Task 0 — dev portal probe (DONE 2026-11-26, before planning)

`POST https://api.dev.uwhportal.com/api/events/1889-B/schedule/games/13/colours`:

| Auth sent | Answer |
|---|---|
| none | 404, empty body |
| garbage bearer | 404, empty body |
| expired key (exp 2026-11-03) | 401 + `Token-Expired: true` |
| GET, none | 404 |

Portal source (`uwh-portal` `api/Program.cs` ~line 811) explains the 401: a middleware after `UseAuthorization` turns ANY request carrying an expired key into 401, route or no route. A valid key passes through to routing, so the un-built route gives the empty 404 → "does not accept swaps" → dropped → the held score is released. An expired key gives 401 → retried, but the score upload fails with the same expired key, so nothing extra is held. **Gate passed.**

## Global Constraints

- MSRV 1.85: **no let-chains** (`if let … && …`), even though the pinned 1.96 toolchain accepts them and every gate passes.
- `uwh-common` must still build with `--no-default-features`; everything new in it lives inside the existing `#[cfg(feature = "std")] pub mod uwhportal`.
- Clippy `-D warnings`; no `unwrap()`/`expect()` in production code without a comment proving it cannot panic; no new dependencies.
- Every new English key goes into all 15 `refbox/translations/*/refbox.ftl` files (an English-only key fails `just check`). Non-English text is Claude's own and is owed a native-speaker check.
- Exact copy: button `SWAP COLOURS` (British spelling, as in the spec). Queue rows: see Task 2.
- Every cargo job needs the refbox coordinator's GO first, runs with `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4`, one at a time. refbox has no lib target: `cargo test -p refbox --bins <filter>`.
- Keep edits to `refbox/src/app/mod.rs`, `refbox/src/config.rs` and `refbox/src/tournament_manager/mod.rs` small (held DRAFT #4348 also edits them). New logic goes in new files; `mod.rs` only calls it.
- Goals, fouls, warnings, penalties and the score never move between colours (spec anti-goal).
- The swap message is ABSOLUTE ("dark = X, light = Y"), never "swap".
- The live-game updates (`put_live_game*`) are not touched.

## Review Focus

1. **Today's portal (no colours route yet) must never hold a final score back for good.** Empty-body 404 → swap dropped → that game's score goes on the next tick. Test in Task 2 (`swap_not_accepted_releases_the_held_score`).
2. **Operator swaps, saves, then swaps back and saves again before delivery** (or while offline): only the latest assignment is sent. Test in Task 2 (`enqueue_colour_swap_replaces_the_queued_one`).
3. **End-of-game schedule download arrives before the portal has the swap** (offline, or the site does not accept swaps): names stay swapped on the refbox. Test in Task 3 (`undelivered_swap_survives_a_fresh_schedule`).
4. **Refbox restarted with a swap in `portal_queue.json`**: the swap is still sent before the score; a queue file written by today's refbox (no `swaps` key) still loads. Tests in Task 2 (`queue_file_without_swaps_loads`, `queue_round_trips_swaps`).
5. **A site switch / SWITCH ANYWAY / discard-all with a swap queued**: the swap is dropped, never sent to the new site. Test in Task 2 (`discard_all_drops_queued_swaps`).

---

### Task 1: Portal client — the colours call and how its answer is read (`uwh-common`)

**Files:**
- Modify: `uwh-common/src/uwhportal/mod.rs` (new method next to `post_game_scores` ~line 368; new enum + pure fn + tests)

**Interfaces:**
- Produces:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub enum ColourSwapOutcome {
      /// 2xx: applied, or already that way.
      Applied,
      /// A named reason, or any other 4xx not listed below. Never retried.
      Refused { status: u16, reason: String },
      /// 404/405 without a named reason: this site has no colours call.
      NotAccepted { status: u16 },
      /// 401, 403, 408, 429, 5xx, or anything unexpected. Retried like a score.
      Retry { status: u16 },
  }
  pub const COLOUR_SWAP_REASONS: [&str; 5] =
      ["EventNotFound", "GameNotFound", "TeamsDoNotMatch", "TeamsNotSet", "AlreadyScored"];
  pub fn classify_colour_swap_reply(status: u16, body: &str) -> ColourSwapOutcome;
  impl UwhPortalClient {
      pub fn post_game_colours(&self, event_id: &EventId, game_number: &GameNumber,
          dark: &TeamId, light: &TeamId)
          -> impl Future<Output = Result<ColourSwapOutcome, Box<dyn Error>>> + use<>;
  }
  ```
  `Err` from `post_game_colours` means the request never got an HTTP answer (connection, timeout, builder); the refbox treats it as retry.

- [ ] **Step 1: Write the failing tests** in the existing `#[cfg(test)]` module of `uwh-common/src/uwhportal/mod.rs`:
  - `colours_2xx_is_applied`: `classify_colour_swap_reply(200, "")` and `(204, "")` → `Applied`.
  - `colours_named_reason_is_refused`: `(409, r#"{"reason":"AlreadyScored"}"#)` → `Refused { status: 409, reason: "AlreadyScored" }`; `(404, r#"{"reason":"GameNotFound"}"#)` → `Refused { status: 404, reason: "GameNotFound" }`.
  - `colours_unnamed_404_405_is_not_accepted`: `(404, "")`, `(404, "<html>Not Found</html>")`, `(404, r#"{"reason":"Something"}"#)`, `(405, "")` → `NotAccepted`.
  - `colours_temporary_is_retry`: `401`, `403`, `408`, `429`, `500`, `503` (each with `""` and with `{"reason":"AlreadyScored"}`) → `Retry { status }`. (A 401 with a reason is still a login problem.)
  - `colours_other_4xx_is_refused`: `(400, "bad")` → `Refused { status: 400, reason: "bad" }`; `(409, r#"{"reason":"Other"}"#)` → `Refused { reason: "Other" }`; `(422, "")` → `Refused { reason: "" }`.
  - `colours_unexpected_status_is_retry`: `(302, "")` → `Retry`.
- [ ] **Step 2: Run them and see them fail** — `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 cargo test -p uwh-common colours_` → compile error (not defined).
- [ ] **Step 3: Implement `classify_colour_swap_reply`.** Order of checks: 2xx → Applied; 401/403/408/429/5xx → Retry; body parses as JSON with a string `reason` in `COLOUR_SWAP_REASONS` → Refused; 404/405 → NotAccepted; other 4xx → Refused (reason = the JSON `reason` if there is one, else the body, trimmed); anything else → Retry. Parse the body with `serde_json::from_str::<serde_json::Value>` (the same way the refbox's `portal_reason_is` reads `"reason"`).
- [ ] **Step 4: Implement `post_game_colours`** by copying `post_game_scores`'s shape: URL `format!("{}/api/events/{}/schedule/games/{game_number}/colours", self.base_url, event_id.partial())`, `authenticated_request(.., Method::POST, ..)`, JSON body `{"darkTeamId": dark.full(), "lightTeamId": light.full()}`, then `Ok(classify_colour_swap_reply(status.as_u16(), &response.text().await?))`. Never treat a non-200 as `Err`.
- [ ] **Step 5: Run the tests** — same command → PASS; then `cargo build -p uwh-common --no-default-features` → builds.
- [ ] **Step 6: Commit** — `feat(uwh-common): add the portal call that records a game's colours`

---

### Task 2: Send queue — swaps go first, and a game's score waits for its swap (`refbox/src/portal_manager/`)

**Files:**
- Modify: `refbox/src/portal_manager/queue.rs` (new `QueuedSwap`, `QueueFile.swaps`)
- Modify: `refbox/src/portal_manager/health.rs` (`PortalTaskIo::post_colours`, `run_task` loop, gate fn, events)
- Modify: `refbox/src/portal_manager/mod.rs` (`enqueue_colour_swap`, event handlers, indicator, `detail_rows`, `retry_all`, `discard_all_queued`, expiry sweep, site move, `UwhPortalIo::post_colours`)
- Modify: the view that renders `DetailRow` (find it with `git grep -n "DetailRow::" refbox/src/app`) and all 15 `refbox/translations/*/refbox.ftl`

**Interfaces:**
- Consumes: Task 1's `ColourSwapOutcome`, `UwhPortalClient::post_game_colours`.
- Produces:
  ```rust
  // queue.rs
  pub struct QueuedSwap {
      #[serde(flatten)] pub id: ItemId,          // same event + game as the score item
      pub dark_team_id: String,                  // TeamId::full(), e.g. "teams/12-A"
      pub light_team_id: String,
      pub queued_at: OffsetDateTime,
      pub attempts: u32,
      pub last_attempt_at: Option<OffsetDateTime>,
  }
  // QueueFile gains:
  #[serde(default, skip_serializing_if = "Vec::is_empty")] pub swaps: Vec<QueuedSwap>,
  // CURRENT_VERSION stays 1.

  // mod.rs
  impl PortalManager {
      pub fn enqueue_colour_swap(&mut self, event_id: String, game_number: String,
          dark_team_id: String, light_team_id: String) -> std::io::Result<()>;
      pub fn on_swap_attempted(&mut self, id: ItemId, attempts: u32, at: OffsetDateTime);
      pub fn on_swap_resolved(&mut self, id: ItemId);
  }
  pub fn is_swap_stuck(swap: &QueuedSwap, now: OffsetDateTime) -> bool; // same 30-min rule as is_item_stuck
  // PortalEvent gains:
  SwapAttempted { id: ItemId, attempts: u32, at: OffsetDateTime },
  SwapResolved { id: ItemId, delivered: bool },   // delivered = the site answered Applied
  // DetailRow gains (NOT tappable):
  SwapPending { game_number: String },
  SwapStuck { game_number: String },

  // health.rs
  async fn post_colours(&self, swap: &QueuedSwap, epoch: u64)
      -> Result<ColourSwapOutcome, PortalCallError>;   // new PortalTaskIo method
  pub(crate) fn score_waits_for_swap(item: &QueuedItem, swaps: &[QueuedSwap]) -> bool;
  pub(crate) enum SwapStep { Delivered, Dropped, Retry }
  pub(crate) fn swap_step(result: &Result<ColourSwapOutcome, PortalCallError>) -> SwapStep;
  ```
  The app (Task 4) handles `PortalEvent::SwapResolved` by calling `on_swap_resolved` and, if `delivered`, marking its local record delivered (Task 3's `mark_delivered`).

**Behaviour to build:**
- `enqueue_colour_swap` REPLACES any queued swap with the same `ItemId` (latest wins; attempts and `last_attempt_at` reset; `queued_at` = now), else pushes. Then `persist`, `recompute_indicator`, `push_queue_snapshot`, like `enqueue_game_end`.
- `run_task` tick: FIRST attempt each retry-eligible swap (same 15 s interval and 30-min stuck rule as scores), THEN the score loop, which skips any item where `score_waits_for_swap` is true. `swap_step`: `Ok(Applied)` → Delivered; `Ok(Refused{..})` and `Ok(NotAccepted{..})` → Dropped; `Ok(Retry{..})` and every `Err` → Retry. Delivered/Dropped send `SwapResolved`; Retry sends `SwapAttempted`. **Log every outcome**: Refused at `warn!` with the game, status and reason ("Game {n}: site refused the colour swap ({status} {reason}); kept on this refbox only"); NotAccepted at `info!` ("site does not accept colour swaps ({status}); kept on this refbox only"); Retry at `warn!` as scores do. No operator message (P1).
- `UwhPortalIo::post_colours` uses `request_for(&event_id, Some(epoch), |c| c.post_game_colours(..))` so the discard epoch and per-event key apply exactly as for scores; transport errors via `classify_error`. Team ids rebuilt with `TeamId::from_full`.
- Indicator: a stuck swap makes it Red; any queued swap makes it at least Yellow.
- `detail_rows`: `SwapStuck` sorts with the stuck rows, `SwapPending` with the pending rows. Copy (en-US), placed after `portal-row-stats-pending`:
  ```
  portal-row-swap-pending = Game { $game } Colour swap not sent yet
  portal-row-swap-stuck = Game { $game } Colour swap send error, press RETRY ALL
  ```
- `retry_all`: resets swaps exactly as it resets score items (attempts 0, `last_attempt_at` None, `queued_at` now).
- Every path that empties or re-points the queue also clears `swaps`: `discard_all_queued`, the 120-h expiry sweep (log each dropped swap; do not archive), and any site-move path. Grep `self.queue.items` in `mod.rs` and decide each site; the reviewer checks the list. `has_queued_items` (the results-waiting warning) stays scores-only: a swap is not a result.
- Every test `PortalTaskIo` implementation in the crate gains `post_colours` (default answer `Ok(Applied)`).

- [ ] **Step 1: Write the failing tests** (pure fns and the existing manager/test-IO harnesses):
  - `queue_file_without_swaps_loads`: today's JSON `{"version":1,"items":[]}` deserialises with `swaps` empty.
  - `queue_round_trips_swaps`: a `QueueFile` with one swap serialises and reads back equal; an empty `swaps` is not written (`!json.contains("swaps")`).
  - `score_waits_only_for_its_own_game`: a swap for game 12 holds item 12, not item 13, and not game 12 of a different event.
  - `swap_step_sorts_outcomes`: Applied → Delivered; Refused, NotAccepted → Dropped; Retry and `Err(PortalCallError::Unreachable(..))` → Retry.
  - `enqueue_colour_swap_replaces_the_queued_one`: two enqueues for game 12 leave ONE swap carrying the second ids, attempts 0.
  - `swap_not_accepted_releases_the_held_score` (task harness): queue = swap 12 + score 12, test IO answers `NotAccepted{404}` → `SwapResolved{delivered:false}` is sent, and on the next tick with the swap removed the score is posted.
  - `score_is_held_while_its_swap_retries` (task harness): IO answers `Retry{503}` → `post_scores` is never called for game 12; a score for game 13 IS posted in the same tick.
  - `discard_all_drops_queued_swaps`: after `discard_all_queued`, `swaps` is empty on disk.
  - `stuck_swap_turns_the_indicator_red` and `detail_rows_lists_swap_rows`.
- [ ] **Step 2: Run them and see them fail** — `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 cargo test -p refbox --bins portal_manager` → compile errors.
- [ ] **Step 3: Implement** the interfaces and behaviour above.
- [ ] **Step 4: Run the tests** — same command → all `portal_manager` tests pass (old and new).
- [ ] **Step 5: Commit** — `feat(refbox): send a game's colour swap before its final score`

---

### Task 3: The refbox's own record of a swap, laid over every schedule (`refbox/src/app/colour_swap.rs`)

**Files:**
- Create: `refbox/src/app/colour_swap.rs` (all logic + tests)
- Modify: `refbox/src/app/mod.rs`: `mod colour_swap;`, one field, three call sites (no logic)

**Interfaces:**
- Consumes: `uwh_common::uwhportal::schedule::{Schedule, Game, GameNumber, TeamId}`; `BlackWhiteBundle`.
- Produces:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub(super) struct SwapRecord { pub dark: TeamId, pub light: TeamId, pub delivered: bool }
  /// Keyed by (event `full()` id, game number). Session memory only.
  pub(super) type ColourSwaps = BTreeMap<(String, GameNumber), SwapRecord>;
  #[derive(Debug, PartialEq, Eq)]
  pub(super) enum SwapFit { AlreadyAgrees, Swapped, NoMatch }
  /// Swaps dark<->light AND dark_partner<->light_partner when the game has `light` on dark
  /// and `dark` on light; leaves the game alone otherwise.
  pub(super) fn apply_swap(game: &mut Game, dark: &TeamId, light: &TeamId) -> SwapFit;
  /// Applies every record for `event_id` to `schedule`; removes a record that is
  /// `delivered` AND `AlreadyAgrees` (the portal now shows it, so stop overriding).
  pub(super) fn apply_colour_swaps(schedule: &mut Schedule, event_id: &str, swaps: &mut ColourSwaps);
  /// Some(dark id, light id) only when the game exists and BOTH sides have an assigned team.
  pub(super) fn swappable_teams(schedule: Option<&Schedule>, game: &GameNumber) -> Option<BlackWhiteBundle<TeamId>>;
  pub(super) fn mark_delivered(swaps: &mut ColourSwaps, event_id: &str, game: &GameNumber);
  ```
  (`BlackWhiteBundle` black = dark, white = light, as everywhere in the refbox.)

**Wiring in `mod.rs`** (the whole of this task's `mod.rs` change):
- Field `colour_swaps: ColourSwaps` on `RefBoxApp`, empty at start.
- `Message::RecvSchedule`: call `apply_colour_swaps(&mut schedule, event_id.full(), &mut self.colour_swaps)` BEFORE the schedule is stored (`event.schedule = Some(schedule.clone())`, ~line 8110), so the event store, the edited-settings copy and `self.schedule` all carry it.
- `commit_link_selection` (~line 2598): apply to `schedule` (when it is `Some` and `event_id` is `Some`) before `self.schedule = schedule`.
- Clear `colour_swaps` wherever `team_rosters` is cleared on a site switch / portal off (~lines 2976 and 3627).

- [ ] **Step 1: Write the failing tests** in `colour_swap.rs`:
  - `apply_swap_moves_both_sides_and_partners`: game dark=A(+partner P), light=B(+partner Q); `apply_swap(game, B, A)` → `Swapped`, dark=B(+Q), light=A(+P).
  - `apply_swap_is_a_no_op_when_the_portal_already_agrees`: dark=B, light=A; `apply_swap(game, B, A)` → `AlreadyAgrees`, unchanged.
  - `apply_swap_never_invents_a_team`: dark=A, light=C (a different team) → `NoMatch`, unchanged.
  - `undelivered_swap_survives_a_fresh_schedule`: record {B,A,delivered:false}; a schedule with dark=A, light=B is swapped and the record stays; applying it again to a schedule that already agrees still keeps the undelivered record.
  - `delivered_swap_retires_once_the_portal_agrees`: record {B,A,delivered:true}; schedule already B/A → record removed. Schedule still A/B (an old download in flight) → swapped and record KEPT.
  - `records_apply_only_to_their_own_event`: record for event X game 12 leaves event Y's game 12 alone.
  - `swappable_teams_needs_two_real_teams`: both assigned → Some; one side `new_winner_of(..)` → None; unknown game → None; `None` schedule → None.
- [ ] **Step 2: Run and see them fail** — `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 cargo test -p refbox --bins colour_swap` → compile error.
- [ ] **Step 3: Implement** `colour_swap.rs` and the `mod.rs` wiring above.
- [ ] **Step 4: Run the tests** — same command → PASS.
- [ ] **Step 5: Commit** — `feat(refbox): keep a saved colour swap over every schedule download`

---

### Task 4: SWAP COLOURS on the score edit page, and saving it (`refbox/src/app/`)

**Files:**
- Modify: `refbox/src/app/view_builders/score_edit.rs` (names, button, change detection)
- Modify: `refbox/src/app/message.rs` (`Message::SwapColours` in the enum and the four lists that name every variant)
- Modify: `refbox/src/app/mod.rs` (`AppState::ScoreEdit` field; the `SwapColours` arm; a save call in `ScoreEditComplete`; `PortalEvent::SwapResolved/SwapAttempted` arms; `tap_outlived_its_screen` list; the `build_score_edit_view` call)
- Modify: `refbox/src/app/colour_swap.rs` (one more pure fn + test)
- Modify: all 15 `refbox/translations/*/refbox.ftl` (`swap-colours = SWAP COLOURS`)

**Interfaces:**
- Consumes: Task 2's `enqueue_colour_swap`, `PortalEvent::{SwapAttempted, SwapResolved}`, `on_swap_attempted`, `on_swap_resolved`; Task 3's `ColourSwaps`, `apply_colour_swaps`, `swappable_teams`, `mark_delivered`; `shared_elements::side_name`.
- Produces:
  ```rust
  AppState::ScoreEdit { scores: BlackWhiteBundle<u8>, is_confirmation: bool, colours_swapped: bool }
  Message::SwapColours            // toggles colours_swapped; valid only in ScoreEdit
  // colour_swap.rs
  /// The record to save: the teams as they will be AFTER the swap.
  pub(super) fn record_after_swap(current: &BlackWhiteBundle<TeamId>) -> SwapRecord; // dark=current.white, light=current.black, delivered=false
  // score_edit.rs
  pub(in super::super) fn build_score_edit_view<'a>(data: ViewData<'_, '_>, scores: BlackWhiteBundle<u8>,
      is_confirmation: bool, confirmation_time: Option<u32>, old_scores: BlackWhiteBundle<u8>,
      team_names: Option<BlackWhiteBundle<String>>, colours_swapped: bool) -> Element<'a, Message>;
  fn score_edit_has_changes(scores, old_scores, colours_swapped: bool) -> bool; // scores != old || colours_swapped
  ```

**Behaviour to build:**
- `team_names` is `Some` only when `swappable_teams(self.schedule.as_ref(), &self.snapshot.game_number)` is `Some` AND `uses_remote()`; names come from `side_name(&game.dark, game.dark_partner.as_ref(), teams, ..)` / light, with the same length limit the game-info table uses. `None` → the page looks exactly as today: no names, no button (P2).
- Names show under the BLACK / WHITE captions; while `colours_swapped` the two names show the other way round.
- The button: `make_chrome_button(fl!("swap-colours"))`, a neutral existing style from `theme/button.rs` that is neither the red (cancel) nor the green (confirm) one, centred between BACK/CANCEL and APPLY/DONE in BOTH forms; `on_press(Message::SwapColours)`. Not drawn when `team_names` is `None`.
- `score_edit_has_changes(.., colours_swapped)` drives APPLY's enabled state and the BACK → CANCEL label (R7). The end-of-game form keeps CANCEL permanently disabled and DONE always enabled (unchanged).
- Every place that builds `AppState::ScoreEdit` sets `colours_swapped: false`.
- `ScoreEditComplete { canceled: false }` with `colours_swapped == true`, in BOTH forms, runs this FIRST, before the engine is locked and before `end_confirm_pause` (so the swap is queued ahead of the score `handle_game_end` queues):
  1. `current = swappable_teams(..)`; if `None` (the schedule changed under the page) log `warn!` and skip the swap; the score change still saves.
  2. `record = record_after_swap(&current)`; insert into `self.colour_swaps` under (event full id, game number); `apply_colour_swaps` on `self.schedule`.
  3. Swap `self.game_rosters.black` and `.white`, so the cap-number lists follow the teams.
  4. `self.portal_manager.enqueue_colour_swap(event full id, game number, record.dark.full(), record.light.full())`; on `Err` log `error!` (the local swap still stands).
  Put steps 1-4 in ONE `fn save_colour_swap(&mut self)` in `mod.rs`; the arm calls it.
- `canceled: true` (BACK/CANCEL) discards the pending swap with the page. `AutoConfirmScores` loses an unsaved swap, exactly as it loses an unsaved score change (spec, accepted).
- `PortalEvent::SwapAttempted` → `portal_manager.on_swap_attempted(..)`. `PortalEvent::SwapResolved { id, delivered }` → `on_swap_resolved(id)`, and if `delivered`, `mark_delivered(&mut self.colour_swaps, &id.event_id, &id.game_number)`.

- [ ] **Step 1: Write the failing tests:**
  - `colour_swap.rs`: `record_after_swap_puts_light_on_dark`: current {black: A, white: B} → {dark: B, light: A, delivered: false}.
  - `score_edit.rs`: `a_swap_alone_counts_as_a_change`: `score_edit_has_changes(s, s, true)` is true; `(s, s, false)` false; `(s2, s, false)` true.
- [ ] **Step 2: Run and see them fail** — `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 cargo test -p refbox --bins -- colour_swap score_edit` → compile errors.
- [ ] **Step 3: Implement** everything in "Behaviour to build", plus the translation key in all 15 files. Non-English values (Claude's own, owed a native check): de-DE `FARBEN TAUSCHEN`, es `CAMBIAR COLORES`, fr `ÉCHANGER COULEURS`, id-ID `TUKAR WARNA`, it-IT `SCAMBIA COLORI`, ja-JP `色を入れ替え`, ko-KR `색상 교체`, ms-MY `TUKAR WARNA`, nl-NL `KLEUREN WISSELEN`, pt-PT `TROCAR CORES`, th-TH `สลับสี`, tl-PH `PAGPALITIN ANG KULAY`, tr-TR `RENKLERİ DEĞİŞTİR`, zh-CN `交换颜色`. Task 2's two row strings likewise get Claude's translations in all 14.
- [ ] **Step 4: Run the tests** — same command → PASS. Then the full gate, with the coordinator's GO: `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 just check` → green.
- [ ] **Step 5: Commit** — `feat(refbox): add SWAP COLOURS to the score edit page`

---

### Task 5: Third-party document + plan deviations (docs)

**Files:**
- Modify: `docs/third-party-integration.md` — a new optional call section: the URL, auth (the same key as the score upload), body, the answers table from the spec's Agreement, and the one line a custom site needs: "a site that does not offer this call answers 404 or 405, and the refbox simply stops sending it; scores are unaffected". Part 1's forfeit field is NOT added (not built).
- Modify: this plan's Deviations section.

- [ ] **Step 1:** Write the section, in the document's existing style.
- [ ] **Step 2: Commit** — `docs(refbox): describe the colour swap call for custom sites`

---

## Known gaps carried to the PR (not built here)

- **Pod games:** the body carries only the two primary team ids; partners move with them on the refbox. The portal's handling of a swap on a pod game is not in the agreement; raise via the coordinator.
- The classic stream overlay follows a swap only from the next game until the R8 follow-up PR (third crate, own PR).
- Part 1 (forfeit skip) waits on portal #1343/#1344.

## Verification (pre-PR, per `.claude/rules/pr-review.md`)

- Leg 1: `code-review` skill on the full diff + `just check` + `cargo audit`.
- Leg 2 (PO walkthrough): dev portal event, isolated `XDG_CONFIG_HOME`; swap during play → names and cap lists move, goals stay; swap + swap back → APPLY greyed again; end-of-game DONE with a swap; the log shows "does not accept colour swaps (404)" and the final score arrives on the dev portal; Manual mode → no button.
- Leg 3 (Claude-driven): optional; the queue ordering is pinned by tests.

## Deviations

Rulings made during execution (what the plan said → what was built → why):

- **Task boundaries (R1–R3).** The plan put the two `PortalEvent` arms in `app/mod.rs` and the two queue-row translations in Task 4 → Task 2 added the arms (calling the queue only) and wrote the row strings in all 15 languages; unused-code warnings between Tasks 2 and 4 were tolerated without `#[allow]` and cleared by Task 4 → each task had to compile, and an English-only key fails `just check`. No behaviour change.
- **R4 — a report only settles the swap it was about.** `SwapAttempted` / `SwapResolved` carried only the game → they also carry the dark and light team ids, and the queue acts only when the queued swap still has them → otherwise "swap, save, swap back" before the first send answered would delete the newer swap unsent.
- **R5 — Japanese, Korean and Chinese wording.** The plan's 色を入れ替え / 색상 교체 / 交换颜色 (and matching row strings) → "black/white swap" wording (黒/白の入れ替え, 검정/흰 교체, 黑白对换) → the refbox's CJK font has no 色, 색 or 颜 and draws a missing character blank. Owed a native-speaker check.
- **R6 — a stuck swap stops holding its score.** The plan said a game's score waits for its swap → it waits only until the swap has been stuck 30 minutes; the swap then stays queued, red and retryable with RETRY ALL → the PO's intent that a final score is never held back for good outranks the literal rule. Trade-off: after 30 minutes of a failing colours call the site may record the score first and then refuse the swap (`AlreadyScored`), which is only logged.
- **R7 — the local record checks which swap was delivered.** `mark_delivered(event, game)` → `mark_delivered(event, game, sent teams)`, marking the record delivered only when it matches what was sent → an older send's success could otherwise retire the newer record, and a later download would show the opposite of the operator's last choice.

Accepted implementer choices that change behaviour:

- RETRY ALL is enabled by swap rows too (the stuck row tells the operator to press it).
- A queued swap whose ids cannot be read is dropped (logged), not retried, since it never could succeed.
- The SWAP COLOURS button (and the team names on the score edit page) also require the refbox to be linked to an event, matching when a swap can be saved and sent.
