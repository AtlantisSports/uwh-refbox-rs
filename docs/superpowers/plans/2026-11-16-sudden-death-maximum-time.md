# Sudden Death Maximum Time Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** An event can give sudden death a maximum time. With one set, the sudden-death clock counts down from it, a goal still ends the game, and at 0:00 the game ends level through the normal score-confirmation window. With none set, sudden death behaves exactly as it does today.

**Architecture:** One optional field on `uwh_common::config::Game` (`sudden_death_max`), read only through one helper (`Game::sudden_death_limit`). One predicate (`GamePeriod::counts_up`) decides the clock's direction, and every site that hard-codes "sudden death counts up" asks that predicate instead. Expiry joins the existing end-of-game path (`could_end_game → pause_for_confirm → end_confirm_pause`), never `end_game` alone. The settings page gets a new SUDDEN DEATH page built from the TEAM TIMEOUTS page and the standard time editor. The wire format and the displays are unchanged.

**Tech Stack:** Rust 2024, MSRV 1.85, iced 0.13, serde (TOML for saved settings, JSON for the Portal), Fluent translations (15 locales).

**Spec:** `docs/superpowers/specs/2026-11-16-sudden-death-maximum-time-design.md` (same branch). Read it before Task 1.

**Process:** HEAVY (`.claude/rules/plan-execution.md`: uwh-common plus the game-clock state machine). That means a per-task review and per-task verification.

## Global Constraints

- MSRV 1.85: **no let-chains** (`if let … && …`). The pinned toolchain compiles them and every gate passes, but 1.85 does not.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` must pass with zero warnings. No new `#[allow]`.
- No `unwrap()`/`expect()` in production code unless a comment says why it cannot panic. **This feature removes two `unwrap()`s that a counting-down sudden death would panic on** (Task 4).
- Screen words, copied literally: **"SUDDEN DEATH"**, **"MAXIMUM TIME"**, **"MAX"**, **"NONE"**, **"SET"**. "Golden Goal" never appears in code identifiers, strings, translations, logs or wire fields.
- Portal field name: **`suddenDeathMaxDuration`**, integer seconds, optional. Absent or `null` means no maximum.
- **A zero maximum (PO ruling 2026-11-16, "1"):** from the Portal or a hand-edited settings file, a zero maximum is played as today's unlimited sudden death and the refbox logs a warning. The settings page never lets an operator choose it (APPLY is greyed). That rule lives in exactly one place, `Game::sudden_death_limit`; no other code reads `sudden_death_max` directly.
- No new dependencies. No wire-format change (`GameSnapshot` unchanged).
- Every cargo command runs with `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4`, and **only after the coordinator's heavy-job GO**. Tell the coordinator "heavy done" afterwards. A fresh worktree's first build is announced to the coordinator before it starts. This machine has no sound card: if a few unrelated tests fail on ALSA, rerun with `--test-threads=1` before treating the failure as real.
- Read code from the feature worktree, never the main checkout (it is hundreds of commits stale).
- Implementation branch: `feat/uwh-common/sudden-death-maximum`, cut from this spec branch so the spec and plan travel with the feature. Creating the branch needs the PO's yes.

## Review Focus

1. **A deciding goal disallowed late in a counting-down sudden death.** The window opens at, say, 0:03 left; the operator restores the tie. Sudden death must resume from 0:03, neither ending the game nor restarting from the full maximum. Pinned in Task 4 (`disallowed_goal_resumes_remaining_countdown`).
2. **Level at 0:00, then the operator enters a winning score inside the confirmation window.** The game ends with that winner. Pinned in Task 4 (`expiry_window_score_edit_ends_with_winner`).
3. **A referee timeout during a counting-down sudden death.** The clock stops and resumes the remaining time; it must not flip to counting up. Pinned in Task 3 (`ref_timeout_resumes_sd_countdown`).
4. **A Portal game with a maximum, played after a manual game without one.** The buzzer and the last-10-seconds flash must follow the **engine's** live config, not the app's saved `self.config.game`, which only changes when settings are applied. The structure enforces this (Task 5 reads the flag from `tm.config()`), and walk step 4 checks it.
5. **A rugby penalty shot still running when the sudden-death clock reaches 0:00.** The game waits for the shot, then ends through the confirmation window. Pinned in Task 4 (`rugby_shot_at_sd_expiry_waits_then_confirms`).

---

### Task 1: The setting and the Portal field (uwh-common)

**Files:**
- Modify: `uwh-common/src/config.rs` (struct `Game`, `Default`, `migrate`, `secs_only_duration`, tests)
- Modify: `uwh-common/src/uwhportal/schedule.rs` (`TimingRule`, `Into<GameConfig>`, tests)
- Modify, compile-fixes only: every full `TimingRule { … }` / `Game { … }` literal without `..` in the workspace. Known: `schedule-processor/src/{csv_parser.rs:807, schedule_checks.rs:1158,1347,1606, scoresheets.rs:2583,3068}`, 3 in `schedule.rs` tests, 1 in `refbox/src/app/mod.rs`. Let the compiler find the rest.

**Interfaces:**
- Produces: `pub sudden_death_max: Option<Duration>` on `Game` (default `None`); `pub fn sudden_death_limit(&self) -> Option<Duration>` on `Game`; `pub sudden_death_max_duration: Option<Duration>` on `TimingRule`.

- [ ] **Step 1: Write failing tests** in `config.rs`'s test module:
  - `sd_max_absent_from_toml_reads_as_none`: a TOML string of a full `Game` **without** the key deserialises with `sudden_death_max == None`.
  - `sd_max_round_trips_through_toml`: a `Game` with `sudden_death_max: Some(Duration::from_secs(300))` serialises and deserialises equal, and the TOML text contains `sudden_death_max = 300`.
  - `sd_limit_reads_zero_as_none`: `sudden_death_limit()` returns `None` for `None` and for `Some(Duration::ZERO)`, and `Some(300s)` for `Some(300s)`.
  - `migrate_keeps_sd_max_none`: `Game::migrate` on an old table gives `sudden_death_max == None`.
  
  In `schedule.rs`'s tests:
  - `timing_rule_sd_max_absent_is_none`: JSON without `suddenDeathMaxDuration` → `None`; with `"suddenDeathMaxDuration": null` → `None`; with `300` → `Some(300s)`, and the converted `GameConfig` has `sudden_death_max == Some(300s)`.
  - `timing_rule_sd_max_zero_converts_raw`: `0` converts to `sudden_death_max == Some(ZERO)`, and its `sudden_death_limit()` is `None`. The conversion stays raw; the helper applies the rule.
- [ ] **Step 2: Run** `cargo test -p uwh-common sd_max timing_rule_sd_max` → FAIL (field missing).
- [ ] **Step 3: Implement.**
  - `Game.sudden_death_max`: placed after `game_block`, with `#[serde(default, skip_serializing_if = "Option::is_none", with = "secs_only_duration::option")]`. Add `pub mod option` inside `config.rs`'s `secs_only_duration`, copied from `schedule.rs:657-675`. The field's doc comment says: *read only through `sudden_death_limit`; a zero is played as unlimited (PO ruling 2026-11-16)*. Add it to `Default` (`None`) and to `migrate`'s destructure and rebuild, keeping its default (old files never had it).
  - `Game::sudden_death_limit(&self) -> Option<Duration>` = `self.sudden_death_max.filter(|d| !d.is_zero())`.
  - `TimingRule.sudden_death_max_duration` with `#[serde(default, rename = "suddenDeathMaxDuration", skip_serializing_if = "Option::is_none", with = "secs_only_duration::option")]`, mapped straight to `sudden_death_max` in `Into<GameConfig>`.
  - Literals: add `sudden_death_max: None` / `sudden_death_max_duration: None`. In `csv_parser.rs`'s drift test (`timing_rule_field_names_match_the_type`), the literal gets `None` with the comment *"None on purpose: the mothballed schedule-processor gets no column for this field (PO 2026-11-16); skip_serializing_if keeps it out of the JSON, so TIMING_RULE_FIELDS stays unchanged."* Do **not** add the field to `TIMING_RULE_FIELDS`.
- [ ] **Step 4: Run** `cargo test -p uwh-common` and `cargo test -p schedule-processor` → PASS, then `cargo check --workspace --all-targets` → clean.
- [ ] **Step 5: Commit** `feat(uwh-common): add an optional sudden death maximum time`

### Task 2: Sudden death timing helpers (uwh-common + penalty maths)

**Files:**
- Modify: `uwh-common/src/game_snapshot.rs` (`GamePeriod::{duration, time_elapsed_at, time_between, next_period_dur}`, new `counts_up`; tests ~l.745-1005)
- Modify: `refbox/src/tournament_manager/penalty.rs` (the `time_between` caller ~l.91; tests)

**Interfaces:**
- Consumes: `Game::sudden_death_limit` (Task 1).
- Produces: `#[cfg(feature = "std")] pub fn counts_up(self, config: &Game) -> bool` (true only for `SuddenDeath` when `sudden_death_limit()` is `None`), and **the new signature** `pub fn time_between(self, start: SignedDuration, end: SignedDuration, config: &Game) -> SignedDuration`.

- [ ] **Step 1: Write failing tests** in `game_snapshot.rs`:
  - `sd_with_limit_counts_down`: with `sudden_death_max: Some(300s)`, `SuddenDeath.counts_up(&c)` is false, `duration` is `Some(300s)`, `time_elapsed_at(280s)` is `Some(20s)`, `time_between(6s, 2s)` is `4s`, and `PreSuddenDeath.next_period_dur` is `Some(300s)`.
  - `sd_without_limit_unchanged`: with `None` **and** with `Some(ZERO)`, `counts_up` is true, `duration` is `None`, `time_elapsed_at(3s)` is `Some(3s)`, `time_between(6s, 10s)` is `4s`, and `PreSuddenDeath.next_period_dur` is `None`.
  - Update the existing assertions at l.786, 836-892 and 987-1003 for the new `time_between` argument; their expected values must not change.
  
  In `penalty.rs`:
  - `penalty_in_sd_countdown_counts_served_time`: a 1-minute penalty started in `SuddenDeath` at 4:30 left (max 5:00) has 0:30 left at 4:00 and is served at 3:30.
  - The existing sudden-death penalty tests stay green unchanged (config without a max).
- [ ] **Step 2: Run** `cargo test -p uwh-common sd_ && cargo test -p refbox penalty` → FAIL.
- [ ] **Step 3: Implement** the four helper changes and `counts_up`. `time_between`'s arm becomes `if self.counts_up(config) { end - start } else { start - end }`. Fix the `penalty.rs` caller; it already has `config` in scope. Then search the whole workspace for `time_between(` and `.duration(` so no caller is missed: a `None` from `duration` was read as "untimed" in several places.
- [ ] **Step 4: Run** the same tests → PASS. `cargo check --workspace --all-targets` → clean.
- [ ] **Step 5: Commit** `feat(uwh-common): time a sudden death that has a maximum as a countdown`

### Task 3: Engine — sudden death clocks count down when limited

**Files:**
- Modify: `refbox/src/tournament_manager/mod.rs`

**Interfaces:**
- Consumes: `GamePeriod::counts_up`, `GamePeriod::duration` (Task 2).
- Produces (private to the module): `fn play_clock(&self, start_time: Instant, clock_time: Duration) -> ClockState`, which builds `CountingUp { start_time, time_at_start: clock_time }` when `self.current_period.counts_up(&self.config)` and `CountingDown { start_time, time_remaining_at_start: clock_time }` otherwise; and `fn fresh_period_clock(&self, start_time: Instant) -> ClockState` = `play_clock(start_time, self.current_period.duration(&self.config).unwrap_or_default())`. `fresh_period_clock` also logs a `warn!` once when entering `SuddenDeath` with `sudden_death_max == Some(ZERO)`: *"Sudden death maximum is 0:00, which is not a real length: playing sudden death without a maximum"*.

**Sites (the class is "code that assumes sudden death counts up"; search `SuddenDeath` and `CountingUp` in this file and fix every hit; this list is a floor, not a ceiling):** `update()` entering SD (~l.2573-2585); `start_game_clock` SD arm (~l.2860); `start_play_now` (~l.3133-3143); `end_confirm_pause`'s SD resume (~l.3415; leave its period decision to Task 4); `next_update_time`'s `(None, SuddenDeath)` and rugby arms (~l.3796-3810), which pick `now_plus_subsec` vs `now_plus_invert_subsec` by `counts_up`. **Leave unchanged:** `remaining_play`/`remaining_regulation` (spec P5: catch-up maths ignore the maximum, as they ignore overtime), `confirm_pause_duration`, `set_scores`.

- [ ] **Step 1: Write failing tests** in the module's test section, driving the shipping tick `updater_tick` (not `update()` alone). Config: `GameConfig { sudden_death_max: Some(Duration::from_secs(300)), ..Default::default() }` plus whatever the existing SD tests use to reach SD (read ~l.4707 and ~l.6891).
  - `sd_with_max_enters_counting_down`: after `PreSuddenDeath` expires, the period is `SuddenDeath`, `game_clock_time(now)` is 5:00 and, 10 s later, 4:50.
  - `sd_with_max_after_overtime_counts_down` (spec P4): `overtime_allowed: true`, level through both overtime halves → `PreSuddenDeath` → `SuddenDeath` counting down from 5:00.
  - `sd_start_play_now_counts_down`: `start_play_now` from `PreSuddenDeath` gives a 5:00 countdown.
  - `ref_timeout_resumes_sd_countdown`: at 4:00 left, start a ref timeout, wait 30 s, end it → the clock reads 4:00 and then counts **down**.
  - `sd_zero_max_plays_unlimited`: `Some(ZERO)` → enters SD counting **up** from 0:00 with no panic.
  - `sd_without_max_unchanged`: with the default config the existing behaviour holds: counts up and never ends on time. The existing golden SD scenarios (`golden/scenarios.rs` 16 and 27) must stay byte-identical.
- [ ] **Step 2: Run** `cargo test -p refbox tournament_manager::tests::sd_` → FAIL.
- [ ] **Step 3: Implement** `play_clock` and `fresh_period_clock` and switch every site above to them.
- [ ] **Step 4: Run** `cargo test -p refbox tournament_manager` (including `golden` and `zero_probe`) → PASS.
- [ ] **Step 5: Commit** `feat(refbox): count a limited sudden death down`

> Task 3 alone leaves a countdown that runs out with nowhere to go (`update()` logs "Impossible state"). Task 4 completes it. Never push between them.

### Task 4: Engine — 0:00 ends the game level through the confirmation window

**Files:**
- Modify: `refbox/src/tournament_manager/mod.rs`

**Interfaces:**
- Consumes: `play_clock` (Task 3), `counts_up` (Task 2).
- Produces: free fn `fn sudden_death_resumes_after_confirm(scores: BlackWhiteBundle<u8>, counts_down: bool, clock_at_pause: Duration) -> bool`, which returns true only when the score is level **and** not (`counts_down && clock_at_pause.is_zero()`). It is a pure function because the app's handlers return `Task`s that tests cannot observe.

**Changes. Read memory `reference_end_game_is_not_the_end_of_game_seam` first.**
- `check_time_remaining`: the period test also accepts `SuddenDeath`. A counting-up SD never reaches it, because `could_end_game` returns earlier for `CountingUp`. Expiry now arrives as `TickKind::ConfirmScores`, exactly like the end of the second half.
- `pause_for_confirm`: the SD `clock_at_pause` is `self.clock_state.clock_time(now).unwrap_or(Duration::ZERO)`. **Crash trap:** a countdown read after its expiry returns `None`, and the current `.unwrap()` would panic with the game lock held.
- `end_confirm_pause`'s SD arm decides the next period with `sudden_death_resumes_after_confirm(scores, !counts_up, confirm_pause.clock_time)`: resume → `SuddenDeath` (keep the `recent_goal = None` line), otherwise → `BetweenGames` (`end_game` runs as for any game end). The SD resume clock is `play_clock(now, confirm_pause.clock_time)`. It must **not** use `next_period_remaining_duration`, which is now `Some(max - pause)` for SD.
- `update()`: split the `(SuddenDeath, _)` arm into `(SuddenDeath, true)` (extend for an unfinished rugby penalty shot, same as `(SecondHalf, true)`) and `(SuddenDeath, false)` → `self.end_game(now)`, with a comment that `could_end_game` normally catches expiry first and this is the backstop.
- `handle_rugby_pen_shot_end`'s SD arm (clock `Stopped` at zero): when SD counts down, set `self.clock_state = CountingDown { start_time: min(now, start_time + time_remaining_at_start), time_remaining_at_start: Duration::ZERO }`, clear `timeout_state` and **return before the function's tail**. The tail would reset the clock to the full period length, and after an `end_game` its `duration(BetweenGames).unwrap()` panics. The next tick's `could_end_game` then opens the window. A counting-up SD keeps today's `InvalidState` error.

- [ ] **Step 1: Write failing tests**, all through `updater_tick`:
  - `sd_expiry_ends_level_via_window`: SD at 0:02 left, running, 0-0. A tick at +3 s returns `TickKind::ConfirmScores`; a tick after the window returns `AutoConfirmScores`; the period is `BetweenGames`, the scores are 0-0, `last_game_info` holds 0-0, and no tick returned `Err`.
  - `expiry_window_score_edit_ends_with_winner`: as above, but `set_scores(1-0)` inside the window, then `end_confirm_pause` → `BetweenGames` with 1-0.
  - `disallowed_goal_resumes_remaining_countdown`: at 0:03 left `add_deciding_goal(Black)`, then restore 0-0 and `end_confirm_pause` → still `SuddenDeath`, and the clock reads 0:03 counting down.
  - `rugby_shot_at_sd_expiry_waits_then_confirms`: rugby mode, SD with a max, a rugby penalty shot running when the game clock reaches 0:00 → `SuddenDeath` holds at 0:00 until the shot ends, then the next tick returns `ConfirmScores`.
  - `sd_expiry_sleep_parks_then_ends` (manual mode): `absorb_lost_time` with more lost time than is left parks SD at 0:00 stopped without panicking; `start_clock` then ends it level through the window.
  - Pure-fn table `sudden_death_resumes_after_confirm_rules`: (level, down, 0s) → false; (level, down, 3s) → true; (level, up, 0s) → true; (1-0, down, 3s) → false; (1-0, up, 30s) → false.
- [ ] **Step 2: Run** `cargo test -p refbox tournament_manager::tests::sd_ sudden_death_resumes` → FAIL.
- [ ] **Step 3: Implement** the changes above.
- [ ] **Step 4: Run** `cargo test -p refbox tournament_manager` → PASS, with golden scenarios 16 and 27 byte-identical.
- [ ] **Step 5: Commit** `feat(refbox): end a limited sudden death level at 0:00`

### Task 5: App — buzzer, last-10-seconds flash, panel hide-time

**Files:**
- Modify: `refbox/src/app/mod.rs` (`maybe_play_sound` ~l.1935-2000, `publish_snapshot` ~l.1712 and its 2 callers, `view()` `ViewData` ~l.8788)
- Modify: `refbox/src/app/view_data.rs` (`ViewData`)
- Modify: `refbox/src/app/view_builders/shared_elements.rs` (`make_game_time_button` ~l.609-640) and its ~12 callers
- Modify: `refbox/src/app/update_sender.rs` (~l.505-520)

**Interfaces:**
- Consumes: `GamePeriod::counts_up`; `TournamentManager::config()`.
- Produces:
  - `fn period_end_sounds(period: GamePeriod, sd_counts_down: bool) -> (bool, bool)` in `app/mod.rs`, returning `(end_starts_play, end_stops_play)`. `SuddenDeath` gives `(false, sd_counts_down)`; the other arms are unchanged. `maybe_play_sound`'s `prereqs` drops its `!= SuddenDeath` clause, because a counting-up SD already returns `(false, false)` and is never a whistle period.
  - `fn game_clock_is_red(period: GamePeriod, secs_in_period: u32, sd_counts_down: bool) -> bool` in `shared_elements.rs`: the existing `None`-timeout expression, with `period != SuddenDeath` replaced by `!(period == SuddenDeath && !sd_counts_down)`.
  - `ViewData.sd_counts_down: bool`; `make_game_time_button(…, clock_running: bool, sd_counts_down: bool, …)` (new parameter right after `clock_running`); `maybe_play_sound(&self, new_snapshot: &GameSnapshot, sd_counts_down: bool)`; `publish_snapshot(&mut self, new_snapshot: GameSnapshot, sd_counts_down: bool)`.
- **The flag always comes from the engine:** `!GamePeriod::SuddenDeath.counts_up(tm.config())`, read while the caller holds the `tm` lock: in `view()` next to `clock_running`, and in each `publish_snapshot` caller before its guard drops. **Never from `self.config.game`**: it changes only when settings are applied, so it is stale for a Portal game, whose timing the engine adopts at kick-off (Review Focus 4). `publish_snapshot` must not take the lock itself.
- `update_sender.rs`: move `PreSuddenDeath` into the first hide-time arm (`secs = next_time`). `next_period_len_secs` is now the maximum when there is one, and 0 when there is not, which is what the old arm did anyway.

- [ ] **Step 1: Write failing tests:**
  - `period_end_sounds_sd`: `(SuddenDeath, true)` → `(false, true)`; `(SuddenDeath, false)` → `(false, false)`; `(SecondHalf, _)` → `(false, true)`; `(PreSuddenDeath, _)` → `(true, false)`.
  - `game_clock_red_last_ten_in_limited_sd`: `(SuddenDeath, 8, true)` → true; `(SuddenDeath, 8, false)` → false; `(SuddenDeath, 7, true)` → false (odd second); `(SecondHalf, 8, _)` → true.
  - An update_sender test (beside the existing hide-time tests): with `hide_time`, `PreSuddenDeath` at 5 s with `next_period_len_secs = Some(300)` sends 300, and with `None` sends 0.
- [ ] **Step 2: Run** `cargo test -p refbox period_end_sounds game_clock_red hide_time` → FAIL.
- [ ] **Step 3: Implement** the changes and thread the flag through every `make_game_time_button` caller (the compiler lists them).
- [ ] **Step 4: Run** the same tests → PASS; `cargo clippy -p refbox --all-targets -- -D warnings` → clean.
- [ ] **Step 5: Commit** `feat(refbox): sound and flash the end of a limited sudden death`

### Task 6: Settings — the SUDDEN DEATH page and its button

**Files:**
- Create: `refbox/src/app/view_builders/sudden_death_edit.rs` (page view + pure rules + tests); register it in `view_builders/mod.rs`
- Modify: `refbox/src/app/mod.rs` (`AppState`, the `EditSuddenDeath`/`Set*` handlers, the `ChangeTime` match ~l.4790-4805, the `ParameterEditComplete` match ~l.6494, view dispatch ~l.8925, every `AppState::ParameterEditor` match site, e.g. ~l.6589)
- Modify: `refbox/src/app/message.rs` (new messages, plus the arms of its hand-written `matches`/category impls ~l.401-825)
- Modify: `refbox/src/app/view_builders/configuration.rs` (the SD button ~l.1062-1069; `MIN_PERIOD_LENGTH` becomes `pub(super)`)
- Modify: `refbox/src/app/view_builders/shared_elements.rs` (`make_time_editor` gains `enabled: bool`, 3 callers; new `make_max_time_value_button`)
- Modify: `refbox/translations/*/refbox.ftl`, all 15 locales

**Interfaces:**
- Produces:
  - `#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub struct SuddenDeathEdit { pub allowed: bool, pub max_set: bool, pub length: Duration }`, and `AppState::SuddenDeathEditor(SuddenDeathEdit)`.
  - `Message::EditSuddenDeath`, `Message::SetSuddenDeathAllowed(bool)`, `Message::SetSuddenDeathMaxSet(bool)`. The time editor reuses `Message::ChangeTime { timeout: false }`, and CANCEL/APPLY reuse `Message::ParameterEditComplete { canceled }`, each with a new `SuddenDeathEditor` arm.
  - `fn sudden_death_edit_seed(config: &GameConfig) -> SuddenDeathEdit` = `{ allowed: sudden_death_allowed, max_set: sudden_death_max.is_some(), length: sudden_death_max.unwrap_or(Duration::from_secs(300)) }`. 5:00 is only the editor's starting value; it is committed only by SET + APPLY.
  - `fn sudden_death_edit_has_changes(edit: SuddenDeathEdit, seed: SuddenDeathEdit) -> bool`: compares `allowed` and `max_set`, and compares `length.as_secs()` only when `edit.max_set`.
  - `fn sudden_death_edit_apply_enabled(edit: SuddenDeathEdit, seed: SuddenDeathEdit) -> bool` = has_changes && !(`allowed && max_set && length < MIN_PERIOD_LENGTH`).
  - `fn sudden_death_edit_commit(edit: SuddenDeathEdit, config: &mut GameConfig)`: sets `sudden_death_allowed = edit.allowed` and `sudden_death_max = edit.max_set.then_some(edit.length)`.
  - `pub(super) fn make_max_time_value_button<'a>(first_label: impl IntoFragment<'a>, max: Duration, message: Option<Message>) -> Button<'a, Message>`: `make_value_button`'s layout, with a two-line value: `fl!("sd-max")` at the size `make_value_button` uses for a **label** (`SMALL_TEXT`), over `time_string(max)` at the size it uses for a **large value**. Nothing may clip in any locale. The TEAM TIMEOUTS "1/HALF" value is clipped today; do not repeat that.
- **Page layout** (spec mockup): row `fl!("sudden-death")` with [NO][YES] (`fl!("no")`/`fl!("yes")`); row `fl!("maximum-time")` with [NONE][SET] (`fl!("sd-max-none")`/`fl!("sd-max-set")`); `make_time_editor(fl!("maximum-time"), length, false, colour, enabled)`; footer CANCEL (`cancel_or_back_label(has_changes)`) / APPLY. Styles and greying are copied from `keypad_pages/team_timeout_edit.rs`: NO greys the MAXIMUM TIME row and the editor, NONE greys the editor, and a greyed control still shows its stored choice. The value shows red when it is a SET 0:00 that APPLY refuses (`param_value_color`'s rule).
- **The SD button** on the Game page (portal mode off): `!allowed` → `make_value_button(…, fl!("no"))`; `allowed` with `sudden_death_limit() == None` → `fl!("yes")`; `allowed` with `Some(max)` → `make_max_time_value_button`. `on_press` → `Message::EditSuddenDeath`. If `BoolGameParameter::SuddenDeathAllowed` is then never constructed, delete the variant and its handler arm.
- **Traps:** `ChangeTime`'s match ends in `_ => unreachable!()`, so a missing arm panics on the first ± press. Seed from `edited_settings.config` when present, else `self.config.game` (as `EditParameter` does at ~l.6423). The editor writes only into `edited_settings` on APPLY. The Game page's own APPLY commits the whole config, and `page_has_changes` already compares the whole `config`, so no new commit site or snapshot field is needed. Confirm that while implementing; memory `reference_new_settings_value_needs_snapshot_and_commit_site` explains why it matters.
- **Translations:** new keys, in all 15 locales or `just check` fails: `sd-max = MAX`, `maximum-time = MAXIMUM TIME`, `sd-max-none = NONE`, `sd-max-set = SET`, in capitals like the rest of the page. Translate like the neighbouring keys. They ship unchecked by native speakers, and the PR says so. Do not reuse the existing `max` key: it is the sound volume level and may translate differently.

- [ ] **Step 1: Write failing tests** in `sudden_death_edit.rs`:
  - `seed_from_config`: `None` → `{allowed, false, 300s}`; `Some(420s)` → `{allowed, true, 420s}`.
  - `has_changes_ignores_length_under_none`: changing only `length` while `max_set == false` → false; while `max_set == true` → true.
  - `apply_refuses_set_zero`: `{true, true, 0s}` vs a different seed → false; `{false, true, 0s}` → true (the row is greyed and not in play); `{true, true, 60s}` → true; no change → false.
  - `commit_writes_none_or_some`: `{true, false, 300s}` → `sudden_death_max == None`; `{true, true, 300s}` → `Some(300s)`; `{false, true, 300s}` → allowed false, max kept `Some(300s)`.
- [ ] **Step 2: Run** `cargo test -p refbox sudden_death_edit` → FAIL.
- [ ] **Step 3: Implement** the page, the button, the messages and handlers, `make_time_editor(…, enabled)` and the translations.
- [ ] **Step 4: Run** `cargo test -p refbox` → PASS (the translation coverage check runs in `build.rs`).
- [ ] **Step 5: Commit** `feat(refbox): set a sudden death maximum time on the settings page`

### Task 7: Game Info page shows the maximum

**Files:**
- Modify: `refbox/src/app/view_builders/game_info_table.rs` (~l.162; tests ~l.700-940)

**PO ruling 2026-11-16 (first-hand, "option 2", confirming a ruling relayed from the Portal):** the sudden-death value in the game parameters table is ONE value: `NO`, `YES`, or **the time alone** (`5:00`). There is no `MAX` word, because it would not fit the table's narrow value column. The same table also appears on the main page between games. The settings **button** keeps its approved two-line `MAX` / `5:00`.

- [ ] **Step 1: Write failing test** `game_info_shows_sd_max`: with `sudden_death_allowed: true, sudden_death_max: Some(300s)`, the `gi-sudden-death` row's value is `time_string(Duration::from_secs(300))` ("5:00"); with `None` it is `bool_string(true)`; with not allowed it is `bool_string(false)`, even when a max is stored.
- [ ] **Step 2: Run** `cargo test -p refbox game_info` → FAIL.
- [ ] **Step 3: Implement** using `config.sudden_death_limit()`.
- [ ] **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat(refbox): show the sudden death maximum on the game info page`

### Task 8: Class sweep and full gate

- [ ] **Step 1:** `git grep -n "SuddenDeath\|CountingUp\|sudden_death" -- '*.rs'` on the branch. For every hit outside tests, record in the plan's Deviations section either "handled in Task N" or "not affected because …". Expected not-affected: `matrix-drawing`, `led-panel-sim`, `overlay`, `overlay-bridge` (they label by period and show whatever time is sent), and `live_updates` (relays period + clock). Any other hit is a finding: fix it or stop and report.
- [ ] **Step 2:** `cargo audit` (light, no GO needed), then, with the GO, `just check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` → all clean.
- [ ] **Step 3:** Commit any Deviations-section update together with the last code task. No standalone deviation commits.

## Acceptance → walkthrough (pre-PR check 2, written in full when the walk is cued)

The spec's six acceptance items, plus Review Focus 4. Rough order: (1) settings page NO / YES+NONE / YES+SET 5:00, with the button reading `MAX` / `5:00` unclipped (also check one long locale, e.g. German); (2) a level game into sudden death counting down from a short maximum (use TIME EDIT to get near 0:00), flashing in the last 10 s; (3) a goal before 0:00 ends the game; (4) no goal: buzzer, confirmation window, the game ends level, the break begins (and the Portal receives the level score if linked); (5) no maximum: counts up as today; (6) the LED panel sim shows SUDDEN DEATH with the countdown. The refbox app is booked through the coordinator, or isolated with its own `XDG_CONFIG_HOME`.

## Deviations

- 2026-11-16 (plan time): the spec said a Portal zero maximum is "refused the same way the refbox refuses other zero-length periods". The refbox has no such refusal; Portal values are checked only by the Portal (and the mothballed schedule-processor). **PO ruling, "1":** a zero maximum plays as unlimited sudden death and logs a warning. The spec has been corrected to match.
- Found while planning, **out of scope**: `handle_rugby_pen_shot_end`'s tail calls `self.current_period.duration(&self.config).unwrap()` after `end_second_half` / `end_overtime_second_half` may have moved the game to `BetweenGames`, where `duration` is `None`. A rugby penalty shot outlasting a level second half with no overtime or sudden death looks like a panic. This plan avoids that tail for sudden death; the existing path goes to the PO as a separate item.
- 2026-11-16, **Task 8 sweep** (every `SuddenDeath` / `CountingUp` / `sudden_death` hit outside test code, grouped by file; test-only files and modules — `golden/`, `zero_probe.rs`, `led-panel-sim/src/test.rs`, every `mod test` — are not listed):
  - `uwh-common/src/config.rs` (`sudden_death_max`, `sudden_death_limit`, old-file upgrade) and `uwh-common/src/uwhportal/schedule.rs` (Portal `suddenDeathMaxDuration` both ways): handled in Task 1.
  - `uwh-common/src/game_snapshot.rs`: `counts_up`, `duration`, `time_elapsed_at`, `time_between`, `next_period_dur` handled in Task 2; the period list, `Display` and the wire numbers 8/9 are not affected (the period itself did not change).
  - `schedule-processor` (`csv_parser.rs`, `schedule_checks.rs`, `scoresheets.rs`, `json_loader.rs`): the new field was threaded through in Task 1; the rest is the pre-sudden-death break and the yes/no switch, not affected.
  - `refbox/src/tournament_manager/mod.rs`: the clock start/resume sites (`play_clock`, `fresh_period_clock`, `start_game_clock`, `start_play_now`, `next_update_time`) handled in Task 3; ending at 0:00 (`could_end_game`, `check_time_remaining`, the `update()` sudden-death arms, `handle_rugby_pen_shot_end`, `timeout_end_would_end_game`, `pause_for_confirm`, `end_confirm_pause`, `sudden_death_resumes_after_confirm`) handled in Task 4. Not affected: the timeout/penalty-shot permission checks (sudden death is still a play period), the timeout clocks (`CountingUp` there is the timeout's own clock), `confirm_pause_duration`, `status_string`, `models_period`/`absorb_lost_time` (still leaves sudden death alone after a clock jump — still right, the length still depends on the score), and `remaining_play` (sudden death adds nothing to the schedule projection, left as is on purpose by the spec).
  - `refbox/src/tournament_manager/penalty.rs`: handled in Task 2 (penalties count against the sudden-death countdown); its loop never steps *through* sudden death (it is the last period), so its `duration(..).unwrap()` is not reached for it.
  - `refbox/src/tournament_manager/placement.rs`: not affected — sudden death stays outside the schedule arithmetic.
  - `refbox/src/app/mod.rs`: buzzer, red clock and flash (`period_end_sounds`, `game_clock_is_red`, the three `sd_counts_down` reads) handled in Task 5; the settings page messages and `SuddenDeathEditor` handled in Task 6. Not affected: the sudden-death goal paths (`AddNewScore`, score edit, keypad goal — already go through the confirm window), `manual_alarm_hold_duration`, the pre-sudden-death break length editor.
  - `refbox/src/app/update_sender.rs`: the panel hide-time in the pre-sudden-death break handled in Task 5.
  - `refbox/src/app/view_builders/shared_elements.rs`: clock colour and flash handled in Task 5; the period names are not affected.
  - `refbox/src/app/message.rs`, `view_builders/configuration.rs`, `view_builders/sudden_death_edit.rs`, `view_builders/mod.rs`: handled in Task 6.
  - `refbox/src/app/view_builders/game_info_table.rs`: handled in Task 7.
  - `refbox/src/app/view_builders/main_view.rs`: not affected (which buttons show in a play period).
  - `refbox/src/live_updates/mod.rs`: hits are test-only; not affected (relays period and clock as sent).
  - `matrix-drawing`, `led-panel-sim`, `overlay`, `overlay-bridge`: not affected — they colour and label by period and show whatever time the refbox sends (`led-panel-sim/src/main.rs` hits are demo snapshots; `overlay-bridge` hit is a doc comment).
  - `stream-manager` (not in the workspace map): one comment only, not affected.
  - No hit was found that a counting-down sudden death breaks; no code change came out of the sweep.
- **Rulings made while building (plain English):**
  - R1: if the operator chooses NO for sudden death but leaves a maximum of 0:00 set, the 0:00 is saved but never used (sudden death is off, and a zero maximum plays as no maximum anyway).
  - The "next period length" for sudden death is only reported when sudden death is switched on, the same way overtime is.
  - vMix: during the break before sudden death, the "next period length" (`next_period_len_secs`) now shows the sudden-death maximum when one is set (it used to be blank). The PR must say so.
  - Two new on/off values for the clock (running, sudden death counting down) were bundled into one small `GameClockFlags` value instead of silencing the code checker's "too many inputs" warning.
  - Task 4 fix: if a rugby penalty shot is ended by hand exactly when a sudden death with a maximum reaches 0:00, the score-confirmation window now opens (before the fix the game ended without it).
  - Task 6: the old internal "sudden death allowed" switch on the settings page (`BoolGameParameter::SuddenDeathAllowed`) was removed, because the new sudden-death page replaces it.
  - Task 6: NO + maximum SET + 0:00 stores a one-microsecond value, which is saved to the settings file as 0 — harmless while sudden death is off (same reasoning as R1).
  - Portal goal times: with a maximum set, a sudden-death goal's period time sent to the Portal is now the time *left* on the clock, like every other period (the field is "time left in period"); sudden death without a maximum still sends time elapsed, as today. The PR must say so, and the Portal side is told through the coordinators.
- **Deferred minors** (found by task reviews, judged not worth fixing now):
  - Task 4: the "sudden death ran out" backstop can also be reached after a TIME EDIT or a score confirmation — the same as the second half today.
  - Task 4: one sleep test covers a state today's wake-up handling never produces.
  - Task 5: no test ties the buzzer/flash flag to the engine's settings (walk step 4 covers it).
  - Task 5: the red-clock test lacks the edge cases at 0 s and 10 s.
  - Task 6: NO + SET + 0:00 stores one microsecond rather than exactly zero (fix if wanted: round to whole seconds when saving).
  - Task 6: the maximum-time button repeats existing button scaffolding; a duplicate style alias; no whole-app test of the new edit/apply messages.
  - Task 7: no test case for a stored zero maximum showing as YES.
  - Task 8 sweep: three comments now only describe a sudden death without a maximum — `game_stats.rs` (test-only `goals()` note: a counting-down sudden death records time *left*, like every other period), `placement.rs` ("sudden death has no length at all") and `absorb_lost_time` ("counts up until somebody scores"). Behaviour is right; wording only.
