# Live Updates v3 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the four v3 fields (`lastGoal`, `penalties`, `timeoutSeconds`, `timeoutRunning`) to the live update the refbox already sends every 5 s, so that no body the refbox can build breaks a portal validation rule.

**Architecture:** The body type in `uwh-common` is reshaped so that most portal rules cannot be broken at all: `secondsLeft` is a `NonZeroU16`, a Total Dismissal carries no seconds, and the team-timeout seconds and running flag travel as one pair inside the White/Black timeout variants. A hand-written `Serialize` flattens that pair back into the agreement's keys. The engine gains one read-only method that returns what the snapshot does not carry: penalty kinds, the recent goal's clock time and the team-timeout clock's running state. It is read under the same lock as v2's snapshot. `live_frame` turns that into the v3 fields, and `needs_send_now` gains the agreement's "send at once" rules.

**Tech Stack:** Rust 2024, MSRV 1.85, serde / serde_json, tokio.

**Spec:** the refbox/portal live-updates agreement, `2026-10-19-live-schedule-refbox-live-updates-agreement.md`, section "v3". It is held by the uwh-portal team and, as of 2026-10-20, is not committed to either repo (READ ONLY: it belongs to uwh-portal and is never edited from here). Base: v2, PR #3633 (`feat/uwh-common/live-updates`, 5740f519). This branch is stacked on it.

**Scope Card**
Task: Build Live Updates v3 in refbox: the four v3 body fields and their send-at-once rules, with a test pinning every portal rule.
Explicitly not doing: the running-stats send (next branch, `live-running-stats`, stacked on this one); anything in uwh-portal; any change to `GameSnapshot`/`PenaltySnapshot` (the LED panel's binary format) or to the overlay / overlay-bridge.

## Global Constraints

- MSRV 1.85, edition 2024; clippy `-D warnings`; no new dependencies; no `unwrap`/`expect` in production code without a "why this cannot panic" comment.
- `uwh-common` must stay `no_std`-clean: `live.rs` is already behind the `std` portal module; use `core::num::NonZeroU16`.
- Every field is PUBLIC (PO ruling). Cap numbers only, never names. No infraction type, warnings, fouls or earlier goals.
- Portal validation is STRICT: one bad field returns 400 and the WHOLE send is dropped, live score included.
- Never touch `GameSnapshot`, `GameSnapshotNoHeap` or `PenaltySnapshot`. The snapshot goes to the LED panel in a binary format, and adding a field there changes that format.
- Leave any walkthrough refbox that is already running alone. Build and test only in the v3 worktree, never the shared checkout, using absolute paths.
- Tests need `--test-threads=1` when ALSA flakes appear (this machine has no sound card).

### The portal rules (agreement, "Portal review of the field table"), verbatim intent

1. `team` is `Black`/`White`. 2. `cap` 0-255. 3. `kind` is one of `ThirtySecond`, `OneMinute`, `TwoMinute`, `FourMinute`, `FiveMinute`, `TotalDismissal`. 4. `secondsLeft` > 0, and null if and only if `kind` is `TotalDismissal`. 5. `lastGoal` and `penalties` null in `BetweenGames`. 6. `timeoutSeconds` ≥ 0 and present only with a `White`/`Black` timeout. 7. `timeoutRunning` non-null if and only if `timeoutSeconds` is non-null.

Field meanings the tests pin: `lastGoal = {team, cap, clockSeconds}`, where `clockSeconds` is the game clock at the goal (time LEFT, but time PLAYED in `SuddenDeath`). It is shown for 15 s of game clock, cleared at the next period, and does not clear while the clock is stopped. `penalties`: every ACTIVE penalty of both teams, with no cap of 3. Served (0 left) are never sent. `[]` in play when none are active. Order is Black's in the order given, then White's. `timeoutSeconds` = seconds LEFT in a White/Black team timeout. `timeoutRunning` = whether that team-timeout clock is running.

Send-at-once rules (agreement, "Timing additions"): send at once when a penalty is added, edited or removed, and when the team-timeout clock starts or stops. Do NOT send at once when a penalty runs out or a goal expires. A team-timeout clock ticking is not a change; its seconds count only while it is stopped, which means an edit (the same rule v2 uses for the game clock).

## Review Focus

1. **A penalty under one second from served.** `as_snapshot` floors to `Seconds(0)`, so it must be dropped, never sent as `0` (rule 4). Pinned in Task 3.
2. **More than three penalties on one team.** All are sent; the LED panel's limit of 3 must not leak in. Pinned in Task 3.
3. **A team timeout stopped at 0 and waiting.** `timeoutSeconds: 0` with `timeoutRunning: false` is valid and must serialise (rule 6 says ≥ 0). Pinned in Task 1.
4. **The engine read failing** (`generate_snapshot` returns `None`, so v2 falls back to the app's last snapshot). The v3 fields go out null and the body stays valid; no stale penalty list is sent. Pinned in Task 3.
5. **A penalty moved to the other team by an edit.** It appears at the end of the other team's list and sends at once. Pinned in Task 3.

---

### Task 1: v3 body types and their serialisation (uwh-common)

**Files:**
- Modify: `uwh-common/src/uwhportal/live.rs` (types, `Serialize`, tests)
- Modify: `uwh-common/src/uwhportal/mod.rs` (the `live_request_tests::state()` helper only)
- Modify: `refbox/src/live_updates/mod.rs`, `refbox/src/live_updates/sender.rs`: only enough to compile. Every existing `LiveGameState { .. }` literal gets `last_goal: None, penalties: None`; every `LiveTimeout::from(..)` call becomes `LiveTimeout::from_snapshot(.., None)`. Do not change the logic here; Task 3 does that.

**Interfaces — Produces** (all `pub`, in `uwh_common::uwhportal::live`):
```rust
pub struct LiveGameState {            // Debug, Clone, PartialEq, Eq; Serialize hand-written
    pub period: GamePeriod,
    pub clock_seconds: u32,
    pub clock_running: bool,
    pub timeout: Option<LiveTimeout>,
    pub scores: Option<LiveScores>,
    pub last_goal: Option<LiveGoal>,
    pub penalties: Option<Vec<LivePenalty>>,
}
pub enum LiveTimeout { White(Option<TeamTimeoutClock>), Black(Option<TeamTimeoutClock>), Ref, PenaltyShot }
impl LiveTimeout {
    /// `team_clock_running` is used only for a White/Black timeout: `Some(r)` gives
    /// `Some(TeamTimeoutClock { seconds_left: <snapshot secs>, running: r })`, `None` gives no clock.
    pub fn from_snapshot(t: TimeoutSnapshot, team_clock_running: Option<bool>) -> Self;
}
pub struct TeamTimeoutClock { pub seconds_left: u16, pub running: bool }   // Copy, Eq
pub struct LiveGoal { pub team: Color, pub cap: u8, pub clock_seconds: u32 } // derive Serialize, camelCase
pub struct LivePenalty { pub team: Color, pub cap: u8, pub time: LivePenaltyTime } // Serialize hand-written
pub enum LivePenaltyTime { Timed { length: TimedPenaltyLength, seconds_left: NonZeroU16 }, TotalDismissal }
pub enum TimedPenaltyLength { ThirtySecond, OneMinute, TwoMinute, FourMinute, FiveMinute } // Serialize = variant name
```
Remove `impl From<TimeoutSnapshot> for LiveTimeout`. A team variant now needs to know whether its clock is running, and a `From` cannot carry that.

`LiveGameState` serialises as one JSON object with all nine keys, ALWAYS present (null when absent, the v2 convention), in this order: `period, clockSeconds, clockRunning, timeout, timeoutSeconds, timeoutRunning, scores, lastGoal, penalties`. `timeout` is the bare name (`"White"`, …) or null. `timeoutSeconds`/`timeoutRunning` come from the team variant's clock: both null for `Ref`, for `PenaltyShot`, for no timeout and for a team variant with no clock. `LivePenalty` serialises as `{"team","cap","kind","secondsLeft"}`, where `kind` is the `TimedPenaltyLength` name, or `"TotalDismissal"` with `"secondsLeft":null`.

- [ ] **Step 1: Write the failing tests** in `live.rs`'s test module:
  - `agreement_example_body_is_exact`: build the agreement's v3 example (SecondHalf, 462, clock not running, `White(Some({41, true}))`, scores 1/3, lastGoal White #7 at 470, penalties Black #4 TwoMinute 83 and White #11 TotalDismissal) and `assert_eq!` against this exact string:
    `{"period":"SecondHalf","clockSeconds":462,"clockRunning":false,"timeout":"White","timeoutSeconds":41,"timeoutRunning":true,"scores":{"black":1,"white":3},"lastGoal":{"team":"White","cap":7,"clockSeconds":470},"penalties":[{"team":"Black","cap":4,"kind":"TwoMinute","secondsLeft":83},{"team":"White","cap":11,"kind":"TotalDismissal","secondsLeft":null}]}`
  - `in_play_body_is_exact` (existing v2 test, updated): the same v2 state with the four new keys serialised as null.
  - `each_penalty_kind_serialises_its_name`: all five timed lengths and TotalDismissal give exactly the six agreement names.
  - `ref_and_penalty_shot_carry_no_timeout_seconds`: for `Ref`, `PenaltyShot` and `None`, `timeoutSeconds` and `timeoutRunning` are both JSON null.
  - `team_timeout_without_clock_sends_both_null` and `team_timeout_at_zero_stopped_is_sent`: `Black(Some({0,false}))` gives `"timeoutSeconds":0,"timeoutRunning":false` (Review Focus 3).
  - `empty_penalty_list_serialises_as_array`: `Some(vec![])` gives `"penalties":[]`, not null.
  - Replace `timeout_kind_drops_seconds` with `from_snapshot_fills_team_clock_only`: `White(10)`+`Some(true)` gives `White(Some({10,true}))`; `Black(20)`+`None` gives `Black(None)`; `Ref(5)`+`Some(true)` gives `Ref`; `PenaltyShot(37)` gives `PenaltyShot`.
- [ ] **Step 2: Run** `cargo test -p uwh-common live` in the v3 worktree. Expected: FAIL (types missing).
- [ ] **Step 3: Implement** the types and the two hand-written `Serialize` impls (`serialize_struct`), then update the compile sites listed under Files.
- [ ] **Step 4: Run** `cargo test -p uwh-common` and `cargo test -p refbox live_updates`. Expected: PASS (refbox logic unchanged; its v2 tests still pass).
- [ ] **Step 5: Commit** `feat(uwh-common): add the v3 live update fields`

### Task 2: the engine's live details (refbox tournament manager)

**Files:**
- Create: `refbox/src/tournament_manager/live_details.rs` (the two structs below)
- Modify: `refbox/src/tournament_manager/mod.rs` (the module declaration, the shared recent-goal predicate, `live_details`, tests)

**Interfaces — Produces:**
```rust
// refbox/src/tournament_manager/live_details.rs, re-exported from tournament_manager
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveDetails {
    /// The goal the stream is showing now: team, cap, game clock at the goal (whole seconds).
    pub recent_goal: Option<(Color, u8, u32)>,
    /// EVERY penalty the engine holds, served ones included, Black's in list order then White's.
    pub penalties: Vec<EnginePenalty>,
    /// `Some` only during a White/Black team timeout: whether its clock is running.
    pub team_timeout_running: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnginePenalty {
    pub team: Color,
    pub cap: u8,
    pub kind: PenaltyKind,
    pub time: PenaltyTime,          // from Penalty::as_snapshot: Seconds(0) when served
    pub given: (GamePeriod, Duration), // start_period, start_time: the penalty's identity
}
impl TournamentManager {
    /// Read-only. `None` when the game clock time or any penalty's time cannot be computed (the
    /// same failures that make `generate_snapshot` return `None`).
    pub fn live_details(&self, now: Instant) -> Option<LiveDetails>;
}
```
The recent goal goes through ONE predicate, `fn recent_goal_showing(goal_period, goal_time, current_period, cur_time) -> bool` (`goal_period == current_period && goal_time.saturating_sub(cur_time) <= RECENT_GOAL_TIME`). `generate_snapshot`'s expiry uses its negation, and `live_details` filters with it. That makes `live_details` correct whether or not `generate_snapshot` ran first, and the two can never disagree. `team_timeout_running` reads `TimeoutState::Team(_, cs).is_running()`. Never use `clock_is_running()` here: it reports whichever clock the timeout made current.

- [ ] **Step 1: Write the failing tests** (in `tournament_manager/mod.rs`'s test module, using the patterns of the existing recent-goal test near `generate_snapshot(later).unwrap().recent_goal`):
  - `live_details_lists_every_penalty_with_kind_and_time`: start Black #4 TwoMinute, Black #5 TotalDismissal, White #11 ThirtySecond. Run the clock 40 s. Expect three entries in order (Black 4, Black 5, White 11), with kinds as given, times `Seconds(80)`, `TotalDismissal`, `Seconds(0)` (served, still listed), and `given` equal to the period and clock at the start.
  - `live_details_goal_carries_clock_and_expires_with_the_stream`: a goal at clock 470 gives `Some((White, 7, 470))`. At 14 s later `Some`, at 16 s later `None`. For each of those moments, `generate_snapshot(t).recent_goal` agrees (`is_some()` equal).
  - `live_details_goal_survives_a_stopped_clock`: a goal, then the clock stopped for 60 s wall time: still `Some`.
  - `live_details_goal_cleared_by_next_period`: a goal near the end of FirstHalf, then the period moves to HalfTime: `None`.
  - `live_details_team_timeout_running_reports_team_clock`: a White team timeout running gives `Some(true)`, stopped gives `Some(false)`. A Ref timeout gives `None`, no timeout gives `None`.
- [ ] **Step 2: Run** `cargo test -p refbox live_details`. Expected: FAIL.
- [ ] **Step 3: Implement** the predicate, `live_details` and the module.
- [ ] **Step 4: Run** `cargo test -p refbox tournament_manager -- --test-threads=1`. Expected: PASS, including every existing recent-goal test.
- [ ] **Step 5: Commit** `feat(refbox): read penalties, last goal and team timeout clock for live updates`

### Task 3: build the v3 fields and the send-at-once rules (refbox live_updates)

**Files:**
- Modify: `refbox/src/live_updates/mod.rs`
- Modify: `refbox/src/app/mod.rs`, compile-only: `live_inputs` sets `details: None` so the workspace builds between tasks. Task 4 wires the real value.

**Interfaces:**
- Consumes: Task 1's types; Task 2's `LiveDetails`, `EnginePenalty`.
- Produces:
```rust
pub struct LiveInputs<'a> { /* v2 fields */ pub details: Option<&'a LiveDetails> }
pub struct LiveFrame { /* v2 fields */ pub penalty_key: Vec<PenaltyIdentity> }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PenaltyIdentity { pub team: Color, pub cap: u8, pub kind: PenaltyKind, pub given: (GamePeriod, Duration) }
/// `None` for a served penalty (Seconds(0)) and for a kind/time mismatch the engine never produces.
fn live_penalty(p: &EnginePenalty) -> Option<LivePenalty>;
```
Rules for `live_frame`:
- **In play** (the v2 in-play branch only): if `details` is `Some`, `last_goal` comes from `recent_goal`, `penalties` is `Some(<every live_penalty that is Some, in details order>)`, `penalty_key` lists every `EnginePenalty` (served ones too) as a `PenaltyIdentity`, and `timeout` = `from_snapshot(snapshot.timeout, details.team_timeout_running)`. If `details` is `None` (the engine read failed; v2 then used the app's last snapshot): `last_goal: None`, `penalties: None`, `penalty_key: []`, `timeout` = `from_snapshot(.., None)`. Sending nothing is correct here. A list from an older read would look plausible and be wrong.
- **Both `BetweenGames` branches**: `last_goal: None`, `penalties: None`, `penalty_key: []`, whatever `details` holds (rule 5; the refbox resets partway through the break).

`needs_send_now`: replace v2's `last.state.timeout != next.state.timeout` with a timeout comparison where these count as a change: a different kind or team, a team clock appearing or disappearing, a different `running`, or different `seconds_left` while the NEW clock is stopped. Add `|| last.penalty_key != next.penalty_key`. Do not compare `last_goal` or `penalties`. That keeps a penalty running out (its key is unchanged until a period start culls it, and a period start is already a change) and a goal expiring off the send-at-once path, as the agreement requires.

- [ ] **Step 1: Write the test-only validator first, and prove it can fail.** `fn portal_rule_violations(body: &serde_json::Value) -> Vec<&'static str>` in the test module checks rules 1-7 above against the SERIALISED body, as the portal would. Test `validator_catches_each_rule`: one hand-written bad JSON body per rule (team `"Red"`, cap 300, kind `"TenMinute"`, `secondsLeft: 0`, a TD with seconds, a timed penalty with null, `penalties` in BetweenGames, `lastGoal` in BetweenGames, `timeoutSeconds` with `"Ref"`, `timeoutRunning` without seconds). Each one must report its violation, and the agreement example must report none.
- [ ] **Step 2: Write the failing behaviour tests:**
  - `in_play_frame_carries_v3_fields`: details with a goal, a running TwoMinute, a served OneMinute and a TD give `last_goal` set and `penalties` = the TwoMinute and the TD only, in order.
  - `penalty_under_a_second_is_not_sent`: `Seconds(0)` from a sub-second remainder is dropped (Review Focus 1).
  - `more_than_three_penalties_all_sent`: 5 Black penalties give 5 sent (Review Focus 2).
  - `no_active_penalties_is_an_empty_list`: in play with none gives `Some(vec![])`.
  - `break_frames_null_goal_and_penalties`: for BetweenGames with a next game AND the finished-court frame, details full of penalties and a goal still give both null and an empty key.
  - `failed_engine_read_sends_v3_fields_null`: `details: None` in play gives `last_goal`/`penalties` None and a team timeout with no clock (Review Focus 4).
  - `every_frame_passes_the_portal_rules`: a sweep over every `GamePeriod`, timeouts {none, White running, Black stopped at 0, Ref, PenaltyShot}, and details {None, empty, goal + every kind with times {0, 1, 83} + TD}. Every `Some(frame)` from `live_frame` serialises to a body with `portal_rule_violations(..).is_empty()`.
  - `needs_send_now`: `penalty_added_is_a_change`, `penalty_edited_is_a_change` (kind changed), `penalty_moved_team_is_a_change` (Review Focus 5), `penalty_running_out_is_not_a_change` (same key; `penalties` lost one entry), `goal_expiring_is_not_a_change`, `team_timeout_ticking_is_not_a_change`, `team_timeout_stop_is_a_change`, `team_timeout_seconds_edit_while_stopped_is_a_change`.
- [ ] **Step 3: Run** `cargo test -p refbox live_updates`. Expected: the new tests FAIL; the validator test PASSES.
- [ ] **Step 4: Implement** `live_penalty`, the frame rules and the `needs_send_now` change. The existing v2 tests' helpers get `details: None` / `penalty_key: vec![]`.
- [ ] **Step 5: Run** `cargo test -p refbox live_updates`. Expected: PASS (v2 tests included).
- [ ] **Step 6: Commit** `feat(refbox): send last goal, penalties and team timeout in live updates`

### Task 4: wire the engine read into the app, then docs

**Files:**
- Modify: `refbox/src/app/mod.rs` (`live_engine_read`, `live_inputs`, `refresh_live_frame`, their tests)
- Modify: `docs/superpowers/specs/2026-10-19-live-updates-design.md`. This is refbox's own design doc, not the portal agreement. Line 44 ("the kind only, no seconds"), line 139 ("a timeout timer" as a non-goal) and any "penalties are not sent" statement become stale: add a short "v3" note pointing to the agreement's v3 section rather than rewriting the v2 text.

**Interfaces:**
```rust
struct LiveEngineRead { snapshot: Option<GameSnapshot>, game_clock_running: bool, details: Option<LiveDetails> }
fn live_engine_read(tm: &mut TournamentManager, now: Instant) -> LiveEngineRead; // one lock, same `now` for all three
fn live_inputs<'a>(/* v2 params */, details: Option<&'a LiveDetails>) -> LiveInputs<'a>;
```
In `refresh_live_frame`, when `snapshot` is `None` (the fallback to `self.snapshot`), pass `details: None` even if `live_details` returned `Some`. The details must describe the same moment as the snapshot they go with.

- [ ] **Step 1: Write the failing test** `live_frame_from_engine_carries_penalty_and_goal`: a real `TournamentManager` in FirstHalf with a goal and a penalty, then `live_engine_read` → `live_inputs` → `live_frame`. Expect both in the frame, and a body that passes the portal rules (re-use Task 3's validator through `pub(crate)` test visibility or a copy kept in sync; choose one and say which in the commit). Plus `fallback_snapshot_drops_details`.
- [ ] **Step 2: Run** `cargo test -p refbox app::` (with `--test-threads=1`). Expected: FAIL.
- [ ] **Step 3: Implement**, then update the design doc note.
- [ ] **Step 4: Run** `just check` in the v3 worktree. Expected: all green.
- [ ] **Step 5: Commit** `feat(refbox): feed the engine's live details into each live frame`

---

## Acceptance (what Eric can observe or run)

- `just check` green. The tests above name each portal rule, and the validator test shows the check fails on every bad body.
- **Leg 3 (Claude-driven, needs Eric's go-ahead):** a v3 build pointed at the local portal (slot 3, PR #1135, v2 code, `UWH_PORTAL_URL_OVERRIDE` + `--allow-http`) plays a game with a goal, penalties of three kinds and a White timeout. Pass: every PUT is accepted (no 400) and the live score still reads back. This proves the agreement's item 6: a v3 refbox may ship before portal v3. The new fields themselves cannot be seen until portal v3 exists.
- **Leg 2:** nothing new is visible in refbox or on the portal's v2 Schedule tab. The walkthrough is "the live score still updates on the phone while penalties and a team timeout are running", which is the regression this branch could cause.

## Deviations

- Task 3 also edited `refbox/src/live_updates/sender.rs` test helpers (compile-only, `penalty_key: vec![]`).
- `portal_rule_violations` lives once as `#[cfg(test)] pub(crate)` in `live_updates`, and the app tests reuse it.
- Task 4 added `LiveEngineRead::snapshot_and_details`, so the rule that a fallback snapshot drops its details is tested in isolation.
- The Task 2 and Task 3 commits carry clippy `dead_code` for `live_details` (no caller until Task 4). The branch tip is clean. No `#[allow]` was added (rust.md).
- Final review: the team-timeout seconds now come from `LiveDetails.team_timeout` (the team clock's `clock_time(now)`, 0 past zero), never from the snapshot's `u16::MAX` stand-in. `LiveTimeout::from_snapshot` takes the clock rather than a running flag.
- Pre-PR review (2026-10-20, after rebasing onto `origin/master` 6b0e9534): a failed engine read in the instant a
  period ends carried an empty penalty key, so `needs_send_now` read "penalties removed" and sent `penalties: null`
  at once. `penalty_key` is now `Option` (`None` = unknown). An unknown read is not a change apart from a different
  timeout kind or team, and the first good read after a sent unknown one goes out at once. The plan's sender
  helpers above now read `penalty_key: Some(vec![])`.
- Acceptance went further than planned: the walk ran against portal v3 itself (slot 4, PRs #1139 server and #1137
  web), not a v2 portal. Eric walked goals (including a cap-only hidden player), all three timed penalty lengths plus a
  Total Dismissal, both team timeouts, a penalty running out, a half ending with a penalty running and the final, all
  on the website. All 116 live PUTs got 204.
