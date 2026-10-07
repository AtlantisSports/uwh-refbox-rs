# Stream Manager review fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring PR #3521 (`feat/workspace/stream-manager`) in line with ADR 026 as amended on
2026-10-07, so it can be reviewed and released as one clean PR into `master`.

**Architecture:** The overlay and overlay-bridge commits move to their own PR, and the Stream
Manager branch is rebuilt on `master` without them. On the rebuilt branch, the portal watch-link
code is removed, then five features are added inside the `stream-manager` crate only:
single-connection mode, a per-court YouTube allowance ledger, restart recovery, portal title sync,
and Companion live status.

**Tech Stack:** Rust 2024 (MSRV 1.85), tokio, axum 0.8, reqwest 0.12, serde, `time` 0.3. The
control page is plain HTML/JS in `stream-manager/web/index.html`.

**Spec:** `docs/decisions/026-per-game-youtube-streams.md` (amended in commit `2c2078e9`, "docs(workspace): amend ADR 026 after review").
Read §2, §4, §5, §7, §9 and "Amendments, 2026-10-07" before starting any task.

## Global Constraints

- Every code change in Tasks 3–8 stays inside `stream-manager/`. No changes to `refbox`,
  `uwh-common`, `overlay`, `overlay-bridge` or `wireless-remote` on this branch (ADR Decision
  section, `.claude/rules/scope.md`).
- No new dependencies (`.claude/rules/rust.md`). Everything here is buildable with the crate's
  existing dependencies. `time` has no time-zone database, so US Pacific time is computed by
  hand (Task 5).
- No `unwrap()`/`expect()` in non-test code without a comment saying why it can't panic. No new
  `unsafe`.
- `cargo fmt --all`, and `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  must be clean. No `#[allow]` to silence warnings.
- Tests live in `#[cfg(test)]` modules in the same file. Run with `cargo test -p stream-manager`.
- Settings files written by the current version must still load: every new `Config` /
  `CourtConfig` / `VideoState` field gets `#[serde(default)]` (or a default function) and a test.
- Settings that change switching (stream mode, allowance share) can only change while no day is
  running, the same rule `practice_mode` already follows (`web.rs` `save_settings`).
- Operator-visible text is plain English, in the same tone as the existing log lines
  ("HOLD ON — automatic switching paused").
- Helpers leave their changes **uncommitted**. The main session reviews and commits each task
  (`type(scope): description`, e.g. `feat(workspace): …`).

## Review Focus

1. **Stream mode changed after Prepare.** Videos bound to key B, then the court is switched to
   one-key mode: Start day must refuse with "Re-run Prepare: some of today's videos use stream
   key B", not start a stream nobody sends to (test in Task 4).
2. **Restart in the middle of a switch.** Two of the court's videos are both `live` on YouTube:
   recovery picks the later game in schedule order and logs the other one so the operator can end
   it (test in Task 6).
3. **Daylight-saving days.** The allowance ledger must reset at midnight US Pacific on the
   March and November change days too (tests in Task 5).
4. **Games whose start already passed.** Prepare moves their YouTube start to 15 minutes from
   now. Title sync must compare against the portal's own start time, or it would "update" those
   games on every check and use 50 units each time (test in Task 7).
5. **Companion missing or slow.** A push that can't connect must give up within 2 seconds, and
   switching must never wait for it (test in Task 8).

---

### Task 1: Overlay fixes on their own branch (main session, git only)

Branch `feat/overlay/streaming-pc-setup` from `origin/master`, carrying Cagatay's three overlay
commits with him kept as author:

- `f044f480` feat(overlay): install NDI's engine automatically when it's missing
- `7715a9c9` fix(overlay): move overlay-bridge off vMix's port 8099 to 8098, **without** its
  `docs/streaming-setup.md` hunk (that file doesn't exist on `master`)
- `f9ad306a` fix(overlay): list a refbox on this PC once in the bridge's scan

- [ ] Before anything: `git fetch origin` and confirm `origin/feat/workspace/stream-manager` is
  still `69af9a4b`. If it moved, stop and tell the user.
- [ ] Keep a local copy of the original tip: `git branch backup/stream-manager-original-69af9a4b 69af9a4b`.
- [ ] Worktree `.claude/worktrees/feat+overlay+streaming-pc-setup`, cherry-pick the three
  commits. For `7715a9c9`, resolve by dropping the `docs/streaming-setup.md` change.
- [ ] `just check` passes.
- [ ] Draft the PR body (What changed / Why / Scope / How to verify, `pr-review.md` format).
  Push and PR **only on the user's OK**.

**Acceptance:** `git diff origin/master --stat` lists only `overlay/` and `overlay-bridge/`
files, and `just check` is green.

### Task 2: Rebuild the Stream Manager branch on master (main session, git only)

Rebuild `feat/workspace/stream-manager` on `origin/master` (no merge commits; `pr-review.md`):

- Drop `f044f480` and `f9ad306a`.
- Replace `7715a9c9` with its `docs/streaming-setup.md` hunk only, keeping Cagatay as author
  (message `docs(workspace): the bridge's new port 8098 in the setup guide`).
- Keep every other commit, including `959c6d30`/`5e603629` (watch links; removed in Task 3 so
  his code stays in history for the follow-up PR) and the ADR amendment `2c2078e9`.

- [ ] `git rebase -i`-equivalent done non-interactively (see the `non-interactive-rebase`
  approach: `GIT_SEQUENCE_EDITOR` with a prepared todo list).
- [ ] `git diff backup/stream-manager-original-69af9a4b HEAD --stat` shows only the removed overlay
  files plus the ADR amendment, and nothing under `stream-manager/` changed.
- [ ] `cargo test -p stream-manager` passes (the PowerShell test only runs on Windows).
- [ ] Force-push to `origin/feat/workspace/stream-manager` **only on the user's OK**, after
  confirming Cagatay has stopped and the remote is still `69af9a4b` (`--force-with-lease=feat/workspace/stream-manager:69af9a4b`).

**Acceptance:** the branch has no `overlay/` or `overlay-bridge/` changes against `master`.

### Task 3: Remove the portal watch links

ADR Amendment 9. Removes the admin sign-in, the PowerShell password encryption and its failing
Windows test.

**Files:**
- Delete: `stream-manager/src/portal_links.rs`
- Modify: `stream-manager/src/main.rs` (drop `mod portal_links;` and the `portal_links::sync`
  call after the `prepare` command, ~line 151)
- Modify: `stream-manager/src/web.rs` (drop the routes and handlers under
  `// ----- Portal watch links -----` (~line 745 on), the `/api/portal-links/send` route, and the
  `portal_links::sync` calls in `prepare_run` (~496) and `cleanup` (~736))
- Modify: `stream-manager/src/app.rs` (drop `Status.portal_login` and where it is filled)
- Modify: `stream-manager/web/index.html` (drop the portal sign-in block in Settings and any
  "Send watch links" control; find them by `portal-login` / `portal-links`)
- Modify: `docs/streaming-setup.md` (drop the watch-links part added in `5e603629`)

- [ ] Remove the items above. `grep -rn "portal_links\|portal-login\|portal-links\|watch link" stream-manager docs/streaming-setup.md` returns nothing.
- [ ] `cargo build -p stream-manager` and `cargo test -p stream-manager` pass, and clippy is clean.
- [ ] Commit: `refactor(workspace): remove portal watch links until the portal supports code linking`

### Task 4: Single-connection mode as a per-court setting

ADR §5, Amendment 4.

**Files:**
- Modify: `stream-manager/src/config.rs`: add
  ```rust
  #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
  #[serde(rename_all = "kebab-case")]
  pub enum StreamMode {
      /// Two stream keys taking turns: no gap at a switch (ADR 026 §5).
      #[default]
      TwoKeys,
      /// One stream key all day: a few seconds' gap before kickoff.
      OneKey,
  }
  ```
  and `#[serde(default)] pub stream_mode: StreamMode` on `CourtConfig`.
- Modify: `stream-manager/src/prepare.rs`: in one-key mode, every video is bound to the A key
  (`court_streams`/binding logic). Re-running Prepare rebinds videos bound to B.
- Modify: `stream-manager/src/live.rs`:
  - `start`: in one-key mode, before anything else, check that every one of today's videos on
    this court is bound to stream A. Otherwise fail with "Re-run Prepare: some of today's videos
    use stream key B".
  - `switch`: in one-key mode always take the existing shared-key path (end the old video, put
    the new one live on the running stream), without starting or stopping vMix destinations.
  - `end`: stop destination 1.
- Modify: `stream-manager/src/web.rs` (`SettingsBody`, `save_settings`: refuse to change
  `stream_mode` while that court's day is running, same as practice mode) and
  `stream-manager/web/index.html` (a "Stream keys: Two (A/B, no gap) / One (short gap)" choice per
  court, with the B key field hidden in one-key mode).

**Tests (in-file):**
- `config.rs`: a court entry without `stream_mode` loads as `TwoKeys`; `one-key` round-trips.
- `live.rs` or `prepare.rs`: the "which stream does game N use" helper returns index 0 for every
  game in one-key mode and alternates in two-key mode.
- Review Focus 1: the start-day pre-check reports the "Re-run Prepare" error when a video is bound
  to B in one-key mode. Make the check a pure function
  (`fn one_key_ready(court: &CourtConfig, state: &EventState, games: &[&str]) -> Result<(), String>`)
  so it can be tested without YouTube.

- [ ] Write the tests, see them fail, implement, see them pass. Clippy is clean.
- [ ] Commit: `feat(workspace): single-connection mode as a per-court setting`

### Task 5: Per-court YouTube allowance ledger

ADR §7, Amendment 2.

**Files:**
- Create: `stream-manager/src/quota.rs`
- Modify: `stream-manager/src/config.rs`: `#[serde(default = …)] pub quota_daily_limit: u32`
  (10_000) and `pub quota_share_percent: u8` (50), plus validation (1..=100 for the share).
- Modify: `stream-manager/src/app.rs`: replace the session-only `youtube_units` counter with the
  ledger. `record_youtube` (already the single place every YouTube call is counted, called from
  `live::carry_out` and the prepare/cleanup jobs) records into the ledger and saves it.
  `Status.youtube_units` becomes `quota_remaining: u32` and `quota_share: u32`.
- Modify: `stream-manager/src/live.rs`: before the chat message and the "Next game" description
  update in `switch`, ask `app.extras_allowed(court_name)`. When false, skip them and log
  "Allowance low: skipped the chat message and Next game link".
- Modify: `stream-manager/web/index.html`: show "YouTube allowance left today: N of M" on the Live
  tab and the two settings on the Settings tab, with a "Low: extras paused" warning.

**`quota.rs` interface (later tasks use these names):**
```rust
/// Units each switch can cost: go live and end (50 each) plus status checks while waiting.
pub const SWITCH_COST: u32 = 120;
/// Kept spare on top of the switches still to come.
pub const MARGIN: u32 = 200;
pub const LEDGER_FILE: &str = "youtube-allowance.json";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ledger { pub day: Option<time::Date>, pub used: u32 }

/// The date in US Pacific time (Google's allowance day), with US daylight-saving rules:
/// UTC−7 from the second Sunday in March 02:00 to the first Sunday in November 02:00, else UTC−8.
pub fn pacific_date(now: time::OffsetDateTime) -> time::Date;

impl Ledger {
    pub fn load(path: &Path) -> Ledger;            // missing or unreadable file → default
    pub fn save(&self, path: &Path) -> Result<(), BoxError>;
    pub fn record(&mut self, units: u32, now: time::OffsetDateTime); // resets when the Pacific day changed
    pub fn used_today(&self, now: time::OffsetDateTime) -> u32;
}

pub fn share(limit: u32, percent: u8) -> u32;      // limit * percent / 100
/// Extras are allowed while what's left still covers the remaining switches plus MARGIN.
pub fn extras_allowed(remaining: u32, switches_left: u32) -> bool;
```
`App::extras_allowed(court)` counts the games still to come today on that court from the plan,
then calls `quota::extras_allowed`.

The ledger belongs to the Stream Manager program, and the deployed setup runs one program per
court (ADR, Step 3 deviation), so "this court's share" and "this program's share" are the same
thing. If one program is ever set up with two courts, the share covers both, and
`extras_allowed` counts the switches left on all of its running courts. Say so in the setting's
help text.

**Tests:** `pacific_date` around midnight PT in winter (UTC−8) and summer (UTC−7), and on the
2026 change days (8 March and 1 November 2026, the Review Focus 3 pair). Also: `record` resets on
a new Pacific day and adds within the same day; `load` of a missing file gives the default;
`extras_allowed` at, below and above the boundary; `share` rounding; a config file without the new
fields loads with 10,000 / 50.

- [ ] Tests first, then the implementation; clippy is clean.
- [ ] Commit: `feat(workspace): track each court's share of the YouTube allowance`

### Task 6: Restart recovery

ADR §9, Amendment 1.

**Files:**
- Modify: `stream-manager/src/switcher.rs`: add
  `pub fn resume(&mut self, live: GameNumber)`. It sets `day_running = true`, `live = Some(live)`
  and `hold = false`, and leaves the phase to the next snapshot. Returns nothing; no action is
  produced.
- Modify: `stream-manager/src/app.rs`: add `pub async fn recover_live_videos(self: &Arc<Self>)`.
  Called once from startup in `main.rs`/`web::serve` after the refbox connections start, and
  again after settings change the courts. For each court:
  1. From the state file, collect the broadcast IDs of that court's games today.
  2. One `youtube.broadcast_statuses(&ids)` call (1 unit per 50 IDs), recorded in the ledger.
  3. Choose with a pure function
     `fn pick_live(games_in_order: &[(&str, &str /*life cycle*/)]) -> (Option<String>, Vec<String>)`,
     which returns the latest game whose life cycle is `live` plus any other live ones.
  4. If one is found: `switcher.resume(game)`, and note "Resumed: Game 14 is live" (and for each
     other live one: "⚠ Game 13 is also still live on YouTube; end it from YouTube Studio").
  5. If none is found, or YouTube isn't connected: note nothing and wait for Start day as today.
- Modify: `stream-manager/web/index.html`: no new control needed. The court log already shows the
  note.

**Tests:** `switcher.rs`: after `resume("14")`, a break snapshot counting down to game 15
produces `Action::Switch { from: "14", to: "15" }` at the lead time, and Hold/Switch now behave as
after Start day. `app.rs` (or a small `recovery.rs` if `app.rs` grows past ~750 lines): `pick_live`
with none, one and two live videos (Review Focus 2).

- [ ] Tests first, then the implementation; clippy is clean.
- [ ] Commit: `feat(workspace): resume from the live video after a restart`

### Task 7: Portal title sync

ADR §2, Amendment 3. Depends on Task 5 (`App::extras_allowed`).

**Files:**
- Modify: `stream-manager/src/prepare.rs`:
  - Add `#[serde(default)] pub portal_start: Option<String>` to `VideoState`: the portal's own
    start time, before the 15-minute move for past games (`PAST_START_OFFSET`).
  - In `run`, the update check at ~line 383 compares title, description **and** `portal_start`.
    It sets `portal_start` on create and update.
  - Extract the per-video compare-and-update into
    `pub async fn sync_video(yt, config, plan, game, state, state_file, log) -> Result<bool, BoxError>`
    (true = updated), so Prepare and title sync share it.
- Create: `stream-manager/src/title_sync.rs`:
  `pub async fn sync_court(app: &App, court: &CourtConfig, only_game: Option<&str>, log) -> Result<SyncReport, BoxError>`.
  It fetches the plan (`portal::fetch_event_plan`), calls `sync_video` for each of the court's
  games today that hasn't been live yet (or only `only_game`), and lists games recorded in the
  state but gone from the portal in `SyncReport { updated: usize, removed: Vec<String> }`. It also
  replaces the app's cached plan with the fresh one.
- Modify: `stream-manager/src/app.rs`: while any day is running, a background task runs
  `sync_court` every 10 minutes for each running court, **skipped** when
  `extras_allowed(court)` is false. Removed games are noted:
  "⚠ Game 22 is no longer on the portal; its video was kept".
- Modify: `stream-manager/src/live.rs`: in `switch`, before the new video goes live, call
  `sync_court(app, court, Some(to), log)` (never skipped for the allowance). A failure there is
  logged as a warning and doesn't stop the switch.

**Tests:** `prepare.rs`: a game whose portal start is in the past and unchanged is **not**
re-updated on a second compare (Review Focus 4); a changed portal start is. A pure helper
`fn removed_games(state: &EventState, plan: &EventPlan, court: &str, day: usize) -> Vec<String>`
with one game gone. `VideoState` without `portal_start` still loads.

- [ ] Tests first, then the implementation; clippy is clean.
- [ ] Commit: `feat(workspace): keep YouTube titles in step with the portal during the day`

### Task 8: Companion live status

ADR §4, Amendment 5.

**Files:**
- Create: `stream-manager/src/companion.rs`:
  ```rust
  /// Companion's address, e.g. "127.0.0.1:8000". Empty = off.
  pub async fn set_variable(address: &str, name: &str, value: &str) -> Result<(), BoxError>;
  /// Custom-variable names for one court, e.g. "sm_court_1_hold".
  pub fn variable_names(court_name: &str) -> CourtVariables; // hold, rosters, now, next
  ```
  The HTTP client has a 2-second timeout. **Before writing it, confirm Companion 5.x's
  custom-variable HTTP interface from Bitfocus's documentation** (the ADR leaves it to be
  confirmed), and record the exact request in the module comment and the setup guide. Variable
  names only use `[a-z0-9_]`.
- Modify: `stream-manager/src/config.rs`: `#[serde(default)] pub companion_address: String`.
- Modify: `stream-manager/src/app.rs`: after every court status change (snapshot, command,
  switch outcome), compute the four values (Hold "ON"/"OFF"; "Rosters in m:ss" / "Rosters on
  screen" / ""; "Now: Game 14"; "Next: Game 15"). Send only the values that changed since the last
  send, on a spawned task so nothing waits on it. Remember the last error per court and show it in
  `CourtStatus` as `companion_error`.
- Modify: `stream-manager/web/index.html`: the Companion address on the Settings tab, and the
  Companion error on the court card.
- Modify: `docs/streaming-setup.md`: how to create the four custom variables in Companion and put
  them on buttons.

**Tests:** `variable_names` slugging ("Court 1" → `sm_court_1_…`); the value-formatting function
for each phase; the change filter (same values twice → nothing sent the second time); Review
Focus 5: `set_variable` to a closed local port returns an error in under 2 seconds.

- [ ] Tests first, then the implementation; clippy is clean.
- [ ] Commit: `feat(workspace): show live status on Stream Deck buttons through Companion`

### Task 9: Setup guide and PR description (main session)

- [ ] `docs/streaming-setup.md`: the new settings (stream keys mode, allowance limit and share,
  Companion address), the "Resumed" behaviour, and the bridge port 8098 / NDI engine notes stated
  as needing the overlay PR's release.
- [ ] Record any execution deviations in this plan's Deviations section. Per
  `plan-execution.md`, the ADR is not amended mid-execution. If anything deviates, one amendment
  goes in at the end, with the user's OK.
- [ ] Rewrite the PR #3521 description in the `pr-review.md` format. The scope statement says it
  changes no overlay or overlay-bridge code. "How to verify" covers the new settings, and the
  follow-ups listed are watch links (portal PR + Stream Manager PR).

### Task 10: Pre-PR checks (main session)

Per `.claude/rules/pr-review.md`:

- [ ] `just check` green (fmt, clippy, tests, audit).
- [ ] Check 1: built-in `code-review` skill over `origin/master...HEAD`; fix or answer every finding.
- [ ] Check 2 (mandatory, visible change): numbered walkthrough steps for the user on a court
  mini PC with vMix 29, the real channel (unlisted) and a real or fake refbox. Cover one-key
  and two-key switching, Hold through the rosters, a mid-day restart showing "Resumed", a portal
  team-name change appearing within 10 minutes, the allowance display, and Companion buttons on
  the Stream Deck.
- [ ] Check 3: recommend whether to drive the control page myself in practice mode (needs the
  user's go-ahead).
- [ ] Dangling-work sweep, then push and update the PR **only on the user's OK**.

## Deviations

Recorded here per `plan-execution.md` (the ADR is not amended mid-execution; any ADR change is one
amendment at the end, with the user's OK).

- **Task 4:** the one-key readiness message reads "Re-run Prepare: some of today's videos use a
  stream key other than A", not the plan's "…use stream key B". The check catches any video bound to
  a key that isn't the court's key A (for example a key renamed in Settings after Prepare), not
  only key B.
- **Task 5:** the allowance share and limit can change while a day is running (the Global
  Constraints listed the share among switching settings). It only decides when the extras pause,
  never a switch.
- **Task 5:** units are recorded per YouTube call into the ledger file, not through
  `App::record_youtube` (which received a cumulative count that resets on reconnect). CLI commands
  count too.
- **Task 7:** the 10-minute title check is also skipped in practice mode (nothing is sent to
  YouTube in practice mode). Each court's 10-minute check is limited to 60 seconds. The check
  before each switch never delays it by more than 5 seconds. With two keys it runs while YouTube
  warms up to the new key and is given up if it hasn't answered by then. With one key it is
  limited to 5 seconds. This was changed after the final review: 15 seconds could push a switch
  into the rosters.
- **Final review:** a one-key switch checks that the next video can go live (ready, testing or
  already live, and its stream key receiving) before ending the old one. "Already live" is
  accepted so that a court can't get stuck with two live videos and every Switch now refused.
- **Task 7:** removed games are found from the court and day saved on each video record (new
  fields), not from the title. Records made before this change are skipped until their next update.
- **Task 8:** "Next" shows whenever the refbox knows the next game. In the break after a switch it
  stays blank until kickoff, because the refbox doesn't yet know the game after.
- **Task 8:** Companion values are re-sent every 60 seconds and after any failed send, so buttons
  recover if Companion restarts.
