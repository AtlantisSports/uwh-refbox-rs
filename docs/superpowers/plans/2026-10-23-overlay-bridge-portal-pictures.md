# Overlay bridge mode: portal pictures Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** In bridge mode (`--features bridge`), the overlay shows everything the direct (Pi) mode shows from the portal — team logos, player photos, event logo, sponsor logo, referees, court and start time — fetched by the overlay itself.

**Architecture:** The bridge's `/game` feed already carries `eventId` + `portalBaseUrl`, which `build_snapshot` puts on the `GameSnapshot`. `bridge_network.rs` uses them to fetch the portal's `GameData` and `EventLogos` once per game/event, in background tasks that retry, and merges the pictures into the names-only `GameData` it already builds every tick. Names stay the bridge's. `network.rs` gains fallible (never-panicking) twins of its fetchers; the direct path keeps calling the existing ones, unchanged.

**Tech Stack:** Rust 2024, tokio, reqwest, serde_json, macroquad (renderer).

**Spec:** this conversation's approved description (PO, 2026-10-23), restated in full here:
- Bridge mode shows team logos, player photos, event + sponsor logos, referees, court, start time — the same screens as the Pi.
- Only the `overlay` crate changes. No change to refbox, overlay-bridge, uwh-common, or the Pi/direct path's behaviour.
- Names come from the bridge, never swapped for the portal's.
- A player's photo is shown only when the portal's name for that cap number matches the bridge's name for it; otherwise no photo.
- No portal event on the feed → no pictures (as today). Non-portal pictures may come later; not now.
- Portal unreachable → game keeps running; pictures fill in once it is reachable (retry).
- Pictures are fetched once per game (event logos once per event), not on every 250 ms poll, and not re-sent to the renderer every poll.

## Global Constraints

- MSRV Rust 1.85, edition 2024. No new dependencies.
- No `unwrap()`/`expect()` added in non-test code without a comment proving it cannot panic.
- `clippy -D warnings` must pass **with the bridge feature on**: `just check` never compiles `bridge_network.rs` (it is `#[cfg(feature = "bridge")]`, and no Justfile/CI gate passes `--features bridge`). Every task runs the feature-on commands below explicitly.
- Do not touch `overlay-bridge/`, `refbox/`, `uwh-common/`, CI files, or `network.rs::networking_thread` / `fetch_game_data` / `TeamInfoRaw::new` / `EventLogos::new` behaviour.
- Branch: `fix/overlay/bridge-mode-portal-pictures`, from current `origin/master`.

Feature-on gates (run from the worktree root, absolute worktree path — never the main checkout):
- `cargo test -p overlay --features bridge`
- `cargo clippy -p overlay --features bridge --all-targets -- -D warnings`
- `cargo clippy -p overlay --all-targets -- -D warnings` (feature off: new `network.rs` items must not be dead code there — gate them `#[cfg(feature = "bridge")]` if they are only used by bridge mode)

## Review Focus

1. **Portal unreachable at start-up, reachable later** — pictures must appear without restarting the overlay. Pinned by Task 1's "unreachable returns Err, never panics" tests; the retry loop itself is walked by the PO.
2. **The game changes while a fetch is still running** — the old game's pictures must never be applied to the new game. Pinned by Task 2's `result_is_for_current_target` test.
3. **A local-file roster whose names differ from the portal's** — no photo, never a wrong face. Pinned by Task 2's mismatch test.
4. **The HTTP client is never initialised in bridge mode today** (`CLIENT_CELL` is only set in `network::networking_thread`) — every fetch would panic inside its task and silently show nothing. Pinned by Task 2 calling `network::init_http_client()` first and Task 1's tests running without the direct thread.
5. **Re-sending pictures every 250 ms** — each `GameData` makes the renderer rebuild every texture. Pinned by Task 2's `needs_send` tests.

---

### Task 1: Never-panicking portal fetchers in `network.rs`

**Files:**
- Modify: `overlay/src/network.rs` (`TeamInfoRaw::new` ~L80-158, `EventLogos::new` ~L167-211, near `fetch_game_data` ~L355; tests module ~L697)

**Interfaces:**
- Produces:
  - `pub fn init_http_client()` — `CLIENT_CELL.get_or_init(...)` with the same builder (`connect_timeout(20 s)`) the direct thread uses. Safe to call more than once. (The direct thread's own `.set(...).unwrap()` stays as is.)
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum PortalFetchError { Unreachable, NotInSchedule }` — `Unreachable`: network error, or a body that is not the expected JSON (retry later). `NotInSchedule`: the schedule loaded but has no game with that number (do not retry).
  - `pub async fn try_fetch_game_data(uwhportal_url: &str, event_id: &EventId, game_number: &GameNumber) -> Result<GameData, PortalFetchError>` — one attempt, same requests and same result as `fetch_game_data` (schedule → `parse_game_display_info` → referees + both teams joined). Referee failure → empty list (as the direct path). Either team's fetch failing → `Unreachable`. A team with no `TeamId` → `TeamInfoRaw::default()` (as the direct path). `event_id: Some(event_id.clone())`.
  - `TeamInfoRaw::try_new(uwhportal_url: &str, team_id: Option<&TeamId>, team_color: Color) -> Result<Self, PortalFetchError>`
  - `EventLogos::try_new(uwhportal_url: &str, event_id: EventId) -> Result<Self, PortalFetchError>`
  - Add `PartialEq` to the derives of `MemberRaw`, `TeamInfoRaw`, `GameData` (Task 2 compares them).
- Refactor so the fallible and panicking versions share one body: extract `async fn team_from_json(data: &Value, team_color: Color) -> TeamInfoRaw` (everything after the request in `new`) and `async fn logos_from_json(data: &Value, event_id: EventId) -> EventLogos`. `new` keeps its exact current requests, panics and log lines, then calls the shared body.

- [ ] **Step 1: Write the failing tests** (in the existing `#[cfg(test)]` module)
  - `team_from_json_reads_names_numbers_roles_and_uppercases_the_team_name`: call `init_http_client()`; JSON `{"name":" Team A ","roster":[{"rosterName":" Ann ","capNumber":3,"roles":["Player","Captain"]},{"rosterName":"Coach Bob","roles":["Coach"]}]}` with no photo/logo URLs → `team_name == "TEAM A"`, members `("Ann", Some(3), Some("Captain"))` and `("Coach Bob", None, Some("Coach"))`, `flag == None`, all pictures `None`. Use `#[tokio::test]` (tokio is already a dependency; add the `macros`/`rt` features only if they are not already enabled — check `overlay/Cargo.toml` first and stop and report if a feature change is needed).
  - `try_fetch_game_data_reports_unreachable_instead_of_panicking`: `init_http_client()`; URL `http://127.0.0.1:1` → `Err(PortalFetchError::Unreachable)`.
  - `team_try_new_reports_unreachable_instead_of_panicking`: same URL, `Some(&TeamId::from_full("teams/2529-B").unwrap())` → `Err(Unreachable)`.
  - `team_try_new_with_no_team_id_is_the_default_team`: → `Ok(TeamInfoRaw::default())`.
  - `event_logos_try_new_reports_unreachable_instead_of_panicking`: → `Err(Unreachable)`.
- [ ] **Step 2: Run** `cargo test -p overlay --features bridge network::` — expect compile failure (items not defined).
- [ ] **Step 3: Implement** the Interfaces above.
- [ ] **Step 4: Run** the three feature-on gates (Global Constraints). Expect all green, and the existing `network.rs` tests still pass.
- [ ] **Step 5: Leave changes uncommitted** and report.

### Task 2: Bridge mode fetches, merges and sends the pictures

**Files:**
- Modify: `overlay/src/bridge_network.rs` (module doc L13-18 — replace the "does not yet call" TODO text with what now happens; `networking_thread` ~L357-449; new helpers; new `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes (Task 1): `network::{init_http_client, try_fetch_game_data, PortalFetchError, EventLogos, GameData, TeamInfoRaw, MemberRaw}`, `EventLogos::try_new`, `PartialEq` on `GameData`.
- Produces (module-private):
  - `#[derive(Debug, Clone, PartialEq)] struct PortalTarget { portal_url: String, event_id: EventId, game_number: GameNumber }`
  - `fn portal_target(snapshot: &GameSnapshot) -> Option<PortalTarget>` — `Some` only when both `snapshot.event_id` and `snapshot.portal_base_url` are `Some`; `portal_url` has trailing `/` trimmed; `game_number` = `snapshot.game_number().clone()` (the period-aware accessor, same one `build_game_data` uses).
  - `fn names_match(a: &str, b: &str) -> bool` — equal after `trim()`, ignoring case.
  - `fn with_portal_pictures(bridge: GameData, portal: &GameData) -> GameData` — returns `bridge` with: each team's `flag` from the portal team of the same colour; each member's `picture`/`geared_picture` from the portal member with the same `number` **only when** `names_match`, else left `None`; `referees`, `pool`, `start_time` from `portal`. Team names, member names/numbers/roles, member list, `game_number`, `event_id` all stay the bridge's. Portal members not in the bridge list are not added.
  - `fn result_is_for_current_target(current: Option<&PortalTarget>, result_for: &PortalTarget) -> bool`
  - `fn needs_send(last_sent: Option<&(GameData, u64)>, bridge_data: &GameData, portal_generation: u64) -> bool` — true unless `last_sent == Some(&(bridge_data.clone(), portal_generation))` (compare without cloning if convenient). `bridge_data` is the names-only `GameData` from `build_game_data`, so the comparison never touches image bytes.

**Loop behaviour** (in `networking_thread`):
- Call `network::init_http_client()` before the loop.
- After `build_snapshot` succeeds: `let target = portal_target(&snapshot)`. If it differs from `current_target`: abort the old game-data task (`JoinHandle::abort`), drop the held portal `GameData`, bump `portal_generation`, and if `Some`, spawn a task that loops `try_fetch_game_data` — `Ok` → send `(target, data)` on an unbounded mpsc channel and end; `Err(NotInSchedule)` → `warn!` once and end; `Err(Unreachable)` → `warn!` and sleep 5 s, then retry.
- Event logos: keyed by `(portal_url, event_id)`; when that key changes, abort the old logos task and spawn one that retries `EventLogos::try_new` every 5 s on `Unreachable`, then sends `StateUpdate::EventLogos(event_id, logos)` straight on a clone of `state_tx`. (The renderer already discards logos for a different event and clears them on an event change.)
- Each tick, drain the result channel with `try_recv`; keep a result only when `result_is_for_current_target`, and bump `portal_generation` when one is kept.
- Send order stays: snapshot first, then game data. Build `bridge_data` with the existing `build_game_data`. If `needs_send(...)`: send `with_portal_pictures(bridge_data.clone(), portal)` when a portal `GameData` is held, else `bridge_data`; record `(bridge_data, portal_generation)` as last sent.
- `connected: false`, unreachable bridge and schema mismatch keep their current "send nothing" behaviour.

- [ ] **Step 1: Write the failing tests** (`#[cfg(test)] mod tests` in `bridge_network.rs`; build `GameData`/`MemberRaw` literals with small fake images like `(1, 1, vec![0, 0, 0, 255])`)
  - `a_photo_is_copied_when_cap_and_name_match` — bridge `("Ann", 3)`, portal `("Ann", 3, picture P, geared G)` → merged member has `P`, `G`.
  - `no_photo_when_the_portal_name_for_that_cap_differs` — bridge `("Ann", 3)`, portal `("Bea", 3, P)` → `picture == None`, `geared_picture == None`, name still `"Ann"`.
  - `names_match_ignores_case_and_surrounding_spaces` — `names_match(" ann ", "ANN")` true; `names_match("Ann", "Anna")` false.
  - `team_logo_referees_court_and_start_come_from_the_portal_names_stay_the_bridges` — portal team name `"PORTAL NAME"`, flag F, referees `[Ref]`, pool `"COURT: 1"`, start `"START: 10:00"`; bridge team name `"BRIDGE NAME"` → merged has bridge name, flag F, the portal's referees/pool/start.
  - `portal_only_players_are_not_added` — portal has cap 7 the bridge lacks → merged member list length equals the bridge's.
  - `portal_target_needs_both_event_and_address_and_trims_the_slash` — snapshot with both (`"https://portal.example/"`) → `portal_url == "https://portal.example"`; with either missing → `None`.
  - `a_result_for_a_previous_game_is_discarded` — `result_is_for_current_target(Some(&game_2), &game_1)` false; same target true; `None` current false.
  - `needs_send_only_when_names_or_portal_data_change` — `None` last → true; same data + same generation → false; same data + new generation → true; different data → true.
- [ ] **Step 2: Run** `cargo test -p overlay --features bridge bridge_network::` — expect compile failure.
- [ ] **Step 3: Implement** the helpers and loop behaviour above; update the module doc (L13-18) and `build_game_data`'s doc line "No player photos..." so neither claims pictures are absent in portal mode.
- [ ] **Step 4: Run** the three feature-on gates, then `just check` from the worktree root. Expect all green.
- [ ] **Step 5: Leave changes uncommitted** and report.

---

## Deviations

- **Task 1:** a third error variant, `PortalFetchError::Refused`, sorts a portal client error
  (other than 408/429) from a retryable outage. Every fetch checks the HTTP status before reading
  the body (task review finding: a JSON error body was read as a normal answer).
- **Task 2, PO decision 2026-10-23:** bridge mode matches the Pi's photo roster. The portal's
  members with no cap number (coaches, managers) are added, and a player's role tag is copied
  from the portal under the same name-match rule as photos. This amends "Portal members not in
  the bridge list are not added", which now applies only to portal members WITH a cap number.
