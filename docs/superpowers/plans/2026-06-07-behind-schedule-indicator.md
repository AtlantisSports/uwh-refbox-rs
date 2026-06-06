# Persistent Behind-Schedule Indicator + Setting — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Game Block per-game overrun figure on the main time bar with a persistent "behind schedule" figure that carries across games and breaks until the schedule is caught back up, and add an admin "Show Behind Schedule Time" on/off setting (default On) on the App Options page.

**Architecture:** Add a read-only `behind_schedule(now)` method to the tournament manager that compares each game's *actual* start to its *scheduled* start (portal printed times when present, else the Game Block grid the manager already keeps), carrying lateness forward and letting longer breaks claw it back down to the minimum break. It needs two new recorded `Instant`s (`current_scheduled_start`, `last_game_end_time`) and a small scheduled-start helper. A new persisted app-config bool gates the display, wired exactly like the existing `track_fouls_and_warnings` toggle. The main-view indicator switches from the old overrun figure to `behind_schedule`.

**Tech Stack:** Rust 2024, `iced` 0.13, `tokio::time::Instant`, `time::OffsetDateTime`, `confy`/TOML config, Fluent (`i18n-embed-fl`), `just`.

**Spec:** `docs/superpowers/specs/2026-06-06-behind-schedule-indicator-design.md`

**Branch:** continue on `feat/uwh-common/game-block` (this reworks that feature's Phase E1 indicator; not yet merged). All paths below are relative to the repo root in the worktree `.worktrees/game-block`.

**Process:** Phase A (tournament-manager timing read-model) and Phase B (config + migration) are higher blast radius → TDD with real unit tests per task. Phase C (UI/toggle/translations) is lower risk → build + `just check` + manual. `refbox` is a bin crate: test with `cargo test -p refbox`, lint with `cargo clippy -p refbox -- -D warnings` (no `--all-targets`).

---

## File structure

| File | Responsibility (this change) |
|------|------------------------------|
| `refbox/src/tournament_manager/mod.rs` | new `current_scheduled_start` + `last_game_end_time` fields; populate in `start_game`/`end_game`/`reset`; `next_game_scheduled_start` helper; `behind_schedule(now)` read-model + tests |
| `refbox/src/config.rs` | new persisted `show_behind_schedule_time: bool` (default `true`) + migration + test |
| `refbox/src/app/view_builders/configuration.rs` | `EditableSettings` field; App-Options half-width toggle button; change-detection |
| `refbox/src/app/message.rs` | `BoolGameParameter::ShowBehindScheduleTime` variant |
| `refbox/src/app/mod.rs` | toggle handler + apply mapping + `EditableSettings` construction; pass `behind_schedule` + setting into `build_main_view` |
| `refbox/src/app/view_builders/main_view.rs` | indicator uses `behind_schedule`, gated on the setting |
| `refbox/translations/*/refbox.ftl` | `show-behind-schedule-time` label, 15 locales |

---

## Phase A — tournament-manager behind-schedule read-model

### Task A1: Record scheduled start of the current game, game-end time, and a scheduled-start helper

**Files:** Modify `refbox/src/tournament_manager/mod.rs` (struct fields ~39-56; `new()` ~63-87; `start_game` ~1015; `end_game` ~950; `reset` ~212). No new test here (exercised by A2), but the crate must still compile and all existing tests pass.

- [ ] **Step 1 — Add two fields** to `struct TournamentManager` (near `game_start_time: Instant,` / `next_scheduled_start: Option<Instant>,`):

```rust
    /// Scheduled start of the game currently in progress (portal printed time when
    /// present, else the Game Block grid slot). Set at `start_game`. `None` before
    /// the first game. Used by `behind_schedule`.
    current_scheduled_start: Option<Instant>,
    /// When the last game ended (entering BetweenGames). Used by `behind_schedule`
    /// to measure lateness while waiting for the next game. `None` before any game ends.
    last_game_end_time: Option<Instant>,
```

- [ ] **Step 2 — Initialise both in `new()`** (in the `Self { ... }` literal, near `next_scheduled_start: None,`):

```rust
            current_scheduled_start: None,
            last_game_end_time: None,
```

- [ ] **Step 3 — Add the scheduled-start helper** as a method on `impl TournamentManager` (place it just above `calc_time_to_next_game`, ~line 886). It mirrors `calc_time_to_next_game`'s portal→`Instant` conversion but returns the scheduled-start `Instant` (handling a scheduled time already in the past, which means we are overdue):

```rust
    /// The scheduled start of the next (or currently-about-to-start) game as an
    /// `Instant`: the portal/CSV printed `start_time` when present, otherwise the
    /// Game Block grid value `next_scheduled_start`. Returns `None` when neither is
    /// known (manual mode before the first game).
    fn next_game_scheduled_start(&self, now: Instant) -> Option<Instant> {
        if let Some(start_time) = self.next_game.as_ref().and_then(|info| info.start_time) {
            let delta = start_time - OffsetDateTime::now_utc(); // signed time::Duration
            if delta.is_negative() {
                Some(now.checked_sub(delta.unsigned_abs()).unwrap_or(now))
            } else {
                delta.try_into().ok().map(|d: Duration| now + d)
            }
        } else {
            self.next_scheduled_start
        }
    }
```

(`OffsetDateTime`, `Duration`, `Instant` are already imported in this file; `time::Duration::is_negative`/`unsigned_abs` exist.)

- [ ] **Step 4 — Capture `current_scheduled_start` at the TOP of `start_game`**, BEFORE `self.next_game.take()` (so the about-to-start game's portal `start_time` is still available) and before `next_scheduled_start` is reassigned. Add as the first lines inside `fn start_game(&mut self, start_time: Instant)`:

```rust
        // The scheduled start of the game we're starting: portal time if present,
        // else this game's Game Block grid slot (`next_scheduled_start` still holds
        // it here — it is reassigned to the *next* game's slot below). Fall back to
        // the actual start for the very first manual game (=> no inherited lateness).
        self.current_scheduled_start =
            Some(self.next_game_scheduled_start(start_time).unwrap_or(start_time));
```

- [ ] **Step 5 — Record `last_game_end_time` in `end_game`** (`fn end_game(&mut self, now: Instant)`, ~line 950). Add near the top of the body:

```rust
        self.last_game_end_time = Some(now);
```

- [ ] **Step 6 — Clear both in `reset`** (`fn reset(&mut self)`, ~line 212), so a tournament reset starts fresh. Add:

```rust
        self.current_scheduled_start = None;
        self.last_game_end_time = None;
```

- [ ] **Step 7 — Build + existing tests pass.**
Run: `cargo test -p refbox` → expected PASS (201 tests; no behaviour change yet).
Run: `cargo clippy -p refbox -- -D warnings` → clean. (The two fields and the helper are currently only written, not read; if clippy flags `next_game_scheduled_start` or a field as dead code, that is expected and is resolved in Task A2 which reads them — do A1 and A2 together in one commit if needed to avoid a transient dead-code warning. See Step 8.)

- [ ] **Step 8 — Do NOT commit yet.** Proceed directly to Task A2 (which adds the reader `behind_schedule`); commit A1+A2 together so there is never a dead-code state. (A1 alone has no readers.)

### Task A2: `behind_schedule(now)` read-model + tests

**Files:** Modify `refbox/src/tournament_manager/mod.rs` (add method near `accumulated_overrun` ~2016; tests in the `#[cfg(test)] mod test` near `test_accumulated_overrun` ~2655).

- [ ] **Step 1 — Write the failing tests** in the test module:

```rust
    #[test]
    fn test_behind_schedule_inherited_lateness_persists_in_manual_mode() {
        initialize();
        // Manual mode, Game Block 40s slot. Game 2's grid slot is 40s after game 1's
        // scheduled start; if game 2 actually starts late, that lateness shows all game.
        let config = GameConfig {
            half_play_duration: Duration::from_secs(10),
            half_time_duration: Duration::from_secs(3),
            minimum_break: Duration::from_secs(2),
            game_block: Duration::from_secs(40),
            overtime_allowed: false,
            sudden_death_allowed: false,
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);
        let g1 = Instant::now();
        tm.start_clock(g1);
        tm.start_play_now(g1).unwrap(); // game 1 starts on time -> anchor
        // next_scheduled_start is now g1 + 40 (game 2's grid slot).
        // Simulate game 2 starting 6s late (at g1 + 46) with the clock stopped first.
        let g2 = g1 + Duration::from_secs(46);
        tm.stop_clock(g2).unwrap();
        tm.set_period_and_game_clock_time(GamePeriod::FirstHalf, Duration::from_secs(10));
        tm.start_play_now(g2).unwrap(); // game 2 begins
        // Inherited = actual start (g1+46) - scheduled slot (g1+40) = 6s; no in-game overrun yet.
        assert_eq!(tm.behind_schedule(g2), Duration::from_secs(6));
        // Running on schedule does not grow it.
        assert_eq!(tm.behind_schedule(g2 + Duration::from_secs(5)), Duration::from_secs(6));
    }

    #[test]
    fn test_behind_schedule_grows_with_in_game_stoppage_beyond_buffer() {
        initialize();
        // Slot 40s; regulation = 2*10+3 = 23; minimum_break 2 => buffer = 40-23-2 = 15.
        let config = GameConfig {
            half_play_duration: Duration::from_secs(10),
            half_time_duration: Duration::from_secs(3),
            minimum_break: Duration::from_secs(2),
            game_block: Duration::from_secs(40),
            overtime_allowed: false,
            sudden_death_allowed: false,
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);
        let start = Instant::now();
        tm.start_clock(start);
        tm.start_play_now(start).unwrap(); // game 1, on time, scheduled = actual
        // Stop the clock and let real time pass (a long stoppage).
        let t1 = start + Duration::from_secs(5);
        tm.stop_clock(t1).unwrap();
        // 20s stopped => accumulated_overrun = 20. buffer 15 => developing = 5. inherited 0.
        let t2 = t1 + Duration::from_secs(20);
        assert_eq!(tm.behind_schedule(t2), Duration::from_secs(5));
        // Below the buffer => nothing.
        assert_eq!(tm.behind_schedule(t1 + Duration::from_secs(10)), Duration::ZERO);
    }

    #[test]
    fn test_behind_schedule_zero_before_first_game_and_when_ahead() {
        initialize();
        let tm = TournamentManager::new(GameConfig::default());
        // Fresh: BetweenGames, no last game end -> zero.
        assert_eq!(tm.behind_schedule(Instant::now()), Duration::ZERO);
    }

    #[test]
    fn test_behind_schedule_between_games_overdue_then_recovered_by_long_break() {
        initialize();
        let config = GameConfig {
            half_play_duration: Duration::from_secs(10),
            half_time_duration: Duration::from_secs(3),
            minimum_break: Duration::from_secs(2),
            game_block: Duration::from_secs(40),
            overtime_allowed: false,
            sudden_death_allowed: false,
            ..Default::default()
        };
        let mut tm = TournamentManager::new(config);
        let start = Instant::now();
        tm.start_clock(start);
        tm.start_play_now(start).unwrap(); // game 1 -> next_scheduled_start = start + 40
        // Force end of game 1 at start+50 (10s past its 40s slot).
        let end = start + Duration::from_secs(50);
        tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(0));
        tm.stop_clock(end).unwrap();
        tm.end_game(end);
        // Between games: next scheduled start = start+40 (grid). Earliest next =
        // max(end + min_break(2), now). At now=end: earliest = end+2 = start+52.
        // behind = (start+52) - (start+40) = 12.
        assert_eq!(tm.behind_schedule(end), Duration::from_secs(12));
    }
```

(Notes for the implementer: `end_game` is `pub(super)`/private — call it from the in-module test as `tm.end_game(end)`. If `end_game` requires the clock stopped or specific state, mirror the setup used by the existing `test_end_*` tests in this file. Adjust the exact asserted constants only if the manager's `end_game`/`start_play_now` transitions demand it — but keep each test's intent: inherited-persists, grows-with-stoppage, zero-when-fresh, between-games-overdue.)

- [ ] **Step 2 — Run, expect failure:**
Run: `cargo test -p refbox behind_schedule` → FAIL (method `behind_schedule` not found).

- [ ] **Step 3 — Implement `behind_schedule`** on `impl TournamentManager`, just below `accumulated_overrun` (~line 2046):

```rust
    /// How far behind its scheduled start times the run of games currently is, as a
    /// positive duration (`ZERO` when on time, ahead, or before the first game).
    /// Carries lateness across games; a longer scheduled break claws it back down to
    /// the always-enforced minimum break. See
    /// docs/superpowers/specs/2026-06-06-behind-schedule-indicator-design.md.
    pub fn behind_schedule(&self, now: Instant) -> Duration {
        if self.current_period == GamePeriod::BetweenGames {
            // Waiting for the next game: the next game cannot start before
            // `last_game_end + minimum_break`; once `now` passes that without it
            // starting, lateness grows. A far-out scheduled next start (long break)
            // keeps this at zero (recovered).
            let (Some(end), Some(sched_next)) =
                (self.last_game_end_time, self.next_game_scheduled_start(now))
            else {
                return Duration::ZERO;
            };
            let earliest_next = max(end + self.config.minimum_break, now);
            earliest_next.saturating_duration_since(sched_next)
        } else {
            // A game is in progress: lateness inherited at its start plus the part of
            // this game's already-elapsed stoppage that has eaten past its slot slack.
            let Some(sched_start) = self.current_scheduled_start else {
                return Duration::ZERO;
            };
            let inherited = self.game_start_time.saturating_duration_since(sched_start);
            let developing = self
                .accumulated_overrun(now)
                .saturating_sub(self.config.game_block_buffer());
            inherited + developing
        }
    }
```

(`max` from `std::cmp` is already imported and used in `calc_time_to_next_game`. `Instant::saturating_duration_since` and `Duration::saturating_sub` are std.)

- [ ] **Step 4 — Run, expect pass:**
Run: `cargo test -p refbox behind_schedule` → PASS (4 new tests).
Run: `cargo test -p refbox` → PASS (now 205 tests).
Run: `cargo clippy -p refbox -- -D warnings` → clean (the A1 fields/helper now have a reader).

- [ ] **Step 5 — Commit (A1 + A2 together):**

```bash
git add refbox/src/tournament_manager/mod.rs
git commit -m "feat(refbox): add behind_schedule read-model to tournament manager"
```

---

## Phase B — persisted "Show Behind Schedule Time" setting

### Task B1: Add `show_behind_schedule_time` to the app config with migration

**Files:** Modify `refbox/src/config.rs` (the app `Config` struct ~216-221; its `Default` impl; `migrate` ~239/260/297; the migrate test ~510-546). Mirror the existing `track_fouls_and_warnings` bool exactly.

- [ ] **Step 1 — Write the failing test.** In the `#[cfg(test)]` module (near the existing migrate test that sets `hide_time`), add:

```rust
    #[test]
    fn test_migrate_show_behind_schedule_time_defaults_true_when_absent() {
        let old: toml::Table = Default::default(); // no key present
        let config = Config::migrate(&old);
        assert!(config.show_behind_schedule_time);
    }

    #[test]
    fn test_migrate_show_behind_schedule_time_respects_present_false() {
        let mut old: toml::Table = Default::default();
        old.insert(
            "show_behind_schedule_time".to_string(),
            toml::Value::Boolean(false),
        );
        let config = Config::migrate(&old);
        assert!(!config.show_behind_schedule_time);
    }
```

(If `Config::migrate` requires other mandatory keys to produce a valid config, mirror the setup of the existing `hide_time` migrate test in this file — copy its baseline `old` table and add/omit only `show_behind_schedule_time`.)

- [ ] **Step 2 — Run, expect failure:**
Run: `cargo test -p refbox -- show_behind_schedule_time` → FAIL (field does not exist).

- [ ] **Step 3 — Add the field** to the `Config` struct (next to `pub track_fouls_and_warnings: bool,`):

```rust
    pub show_behind_schedule_time: bool,
```

- [ ] **Step 4 — Default it to `true`.** In `Config`'s `Default` impl (the `Self { ... }` literal), add:

```rust
            show_behind_schedule_time: true,
```

- [ ] **Step 5 — Wire migration**, mirroring `track_fouls_and_warnings` at all three spots in `migrate`:
  - In the destructured defaults block (`let Self { ... mut track_fouls_and_warnings, ... } = Default::default();`) add `mut show_behind_schedule_time,`.
  - After the existing `get_boolean_value(old, "track_fouls_and_warnings", &mut track_fouls_and_warnings);` add:

```rust
        get_boolean_value(
            old,
            "show_behind_schedule_time",
            &mut show_behind_schedule_time,
        );
```
  - In the returned `Self { ... }` literal add `show_behind_schedule_time,`.

(Because the `mut` binding starts from `Default::default()` which is `true`, an absent key stays `true`; a present `false` is read by `get_boolean_value` — satisfying both tests.)

- [ ] **Step 6 — Run, expect pass:**
Run: `cargo test -p refbox -- show_behind_schedule_time` → PASS.
Run: `cargo test -p refbox` → PASS. `cargo clippy -p refbox -- -D warnings` → clean.

- [ ] **Step 7 — Commit:**

```bash
git add refbox/src/config.rs
git commit -m "feat(refbox): add show_behind_schedule_time app setting with migration"
```

---

## Phase C — UI: toggle, App-Options button, indicator

### Task C1: Stage the setting through EditableSettings + the toggle message

**Files:** Modify `refbox/src/app/view_builders/configuration.rs` (`EditableSettings` struct ~26-45; change-detection ~205-225), `refbox/src/app/message.rs` (`BoolGameParameter` ~664-680), `refbox/src/app/mod.rs` (the `EditableSettings` construction from `Config` ~271-330; the `ToggleBoolParameter` match arm; the Apply mapping that writes edited settings back to `self.config`). Mirror `track_fouls_and_warnings` / `BoolGameParameter::FoulsAndWarnings` at every site.

- [ ] **Step 1 — `EditableSettings` field.** In `struct EditableSettings` (configuration.rs ~26), next to `pub track_fouls_and_warnings: bool,` add:

```rust
    pub show_behind_schedule_time: bool,
```

- [ ] **Step 2 — Message variant.** In `enum BoolGameParameter` (message.rs ~664), next to `FoulsAndWarnings,` add:

```rust
    ShowBehindScheduleTime,
```

- [ ] **Step 3 — Build to find every site.** Run `cargo build -p refbox`. The compiler will flag: the `EditableSettings` construction (mod.rs), the `ToggleBoolParameter` match (mod.rs), the Apply mapping (mod.rs), and any `EditableSettings { ... }` test literals (configuration.rs). Fix each by mirroring `track_fouls_and_warnings` / `FoulsAndWarnings`:
  - **Construction from config** (mod.rs, where `track_fouls_and_warnings: self.config.track_fouls_and_warnings,` appears): add `show_behind_schedule_time: self.config.show_behind_schedule_time,`.
  - **Toggle handler** (the `match param { ... BoolGameParameter::FoulsAndWarnings => { ... } }` arm that flips the staged bool): add an arm
    ```rust
    BoolGameParameter::ShowBehindScheduleTime => {
        edited_settings.show_behind_schedule_time ^= true;
    }
    ```
    (Match the exact toggle idiom used by the neighbouring arm — e.g. `= !...` if that is what `FoulsAndWarnings` uses.)
  - **Apply mapping** (where `self.config.track_fouls_and_warnings = edited.track_fouls_and_warnings;` or equivalent writes staged → config on Apply): add `self.config.show_behind_schedule_time = edited.show_behind_schedule_time;` (mirror the exact form used there).
  - **Change-detection** (configuration.rs ~213-225, where `edited.track_fouls_and_warnings != *track_fouls_and_warnings` contributes to "settings changed"): add the analogous `|| edited.show_behind_schedule_time != *show_behind_schedule_time` and include `show_behind_schedule_time,` in that function's destructure.
  - **Test literals** (configuration.rs tests that build `EditableSettings { ... }`): add `show_behind_schedule_time: true,` (or mirror the sibling's value).

- [ ] **Step 4 — Build + tests + lint:**
Run: `cargo build -p refbox` → compiles. `cargo test -p refbox` → PASS. `cargo clippy -p refbox -- -D warnings` → clean.

- [ ] **Step 5 — Commit:**

```bash
git add refbox/src/app/message.rs refbox/src/app/mod.rs refbox/src/app/view_builders/configuration.rs
git commit -m "feat(refbox): wire ShowBehindScheduleTime toggle through settings"
```

### Task C2: App-Options half-width button + translations

**Files:** Modify `refbox/src/app/view_builders/configuration.rs` (`make_app_config_page` ~870-940), `refbox/translations/*/refbox.ftl` (15 files).

- [ ] **Step 1 — Destructure the staged value** in `make_app_config_page`'s `let EditableSettings { ... } = settings;` (add `show_behind_schedule_time,` next to `track_fouls_and_warnings,`).

- [ ] **Step 2 — Add the button** in the first currently-empty row. Replace the first `row![horizontal_space()].height(Length::Fill),` (the one immediately after the `track-fouls-and-warnings` / `confirm-score-at-game-end` row, ~line 933) with a half-width toggle paired with a spacer so it stays half-width:

```rust
        row![
            make_value_button(
                fl!("show-behind-schedule-time"),
                bool_string(*show_behind_schedule_time),
                (false, true),
                Some(Message::ToggleBoolParameter(
                    BoolGameParameter::ShowBehindScheduleTime,
                )),
            ),
            horizontal_space(),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
```

- [ ] **Step 3 — Add the label to all 15 locales.** In each `refbox/translations/<loc>/refbox.ftl` (de-DE, en-US, es, fr, id-ID, it-IT, ja-JP, ko-KR, ms-MY, nl-NL, pt-PT, th-TH, tl-PH, tr-TR, zh-CN), add a `show-behind-schedule-time` key near `track-fouls-and-warnings`. en-US EXACTLY:

```
show-behind-schedule-time = Show Behind Schedule Time
```
For the other 14 locales provide a natural best-guess translation (title case, no English placeholder), e.g. es: `Mostrar Tiempo de Retraso`; fr: `Afficher le Retard sur l'Horaire`; de-DE: `Verzögerung Anzeigen`. Translate appropriately for ja-JP/ko-KR/zh-CN/th-TH scripts.

- [ ] **Step 4 — Build + lint + per-locale check:**
Run: `cargo build -p refbox` → compiles. `cargo clippy -p refbox -- -D warnings` → clean.
Run: `for d in de-DE en-US es fr id-ID it-IT ja-JP ko-KR ms-MY nl-NL pt-PT th-TH tl-PH tr-TR zh-CN; do echo "$d: $(grep -c '^show-behind-schedule-time' refbox/translations/$d/refbox.ftl)"; done` → each prints 1.

- [ ] **Step 5 — Commit:**

```bash
git add refbox/src/app/view_builders/configuration.rs refbox/translations/*/refbox.ftl
git commit -m "feat(refbox): add Show Behind Schedule Time button to App Options"
```

### Task C3: Switch the main-screen indicator to `behind_schedule`, gated on the setting

**Files:** Modify `refbox/src/app/mod.rs` (the `build_main_view` call site ~3788, where Task E1 of the Game Block feature added `let overrun = self.tm.lock().unwrap().accumulated_overrun(Instant::now());`), `refbox/src/app/view_builders/main_view.rs` (`build_main_view` ~14-34).

- [ ] **Step 1 — Compute the figure at the call site.** In `mod.rs`, replace the existing overrun computation before `build_main_view(` with:

```rust
                let behind_schedule = if self.config.show_behind_schedule_time {
                    self.tm.lock().unwrap().behind_schedule(Instant::now())
                } else {
                    std::time::Duration::ZERO
                };
```
and pass `behind_schedule` to `build_main_view` in the argument slot currently holding `overrun` (rename that parameter — see Step 2). (`Instant` here is the same `Instant` already used at this call site for `accumulated_overrun`.)

- [ ] **Step 2 — Use it in `build_main_view`.** In `main_view.rs`, the function currently takes `overrun: Duration` (added in Game Block E1) and computes `over_slot = overrun.saturating_sub(game_config.game_block_buffer())` to build the red label. Replace that with the already-final figure:
  - Rename the parameter `overrun: Duration` → `behind_schedule: Duration`.
  - Replace the label computation with:
    ```rust
    let behind_label = if behind_schedule > std::time::Duration::ZERO {
        Some(format!("-{}", time_string(behind_schedule)))
    } else {
        None
    };
    ```
  - Pass `behind_label` to `make_game_time_button(...)` in place of the old overrun label argument. (Do NOT subtract `game_block_buffer` here — `behind_schedule` is already the final figure; the buffer is accounted for inside it.)

- [ ] **Step 3 — Build + tests + lint + manual:**
Run: `cargo build -p refbox` → compiles. `cargo test -p refbox` → PASS. `cargo clippy -p refbox -- -D warnings` → clean.
Manual: with the setting On, a game running behind shows a growing red `-M:SS` that persists into the next game; toggling the App-Options setting Off hides it.

- [ ] **Step 4 — Commit:**

```bash
git add refbox/src/app/mod.rs refbox/src/app/view_builders/main_view.rs
git commit -m "feat(refbox): show persistent behind-schedule figure, gated on setting"
```

---

## Phase D — Final verification

- [ ] **Step 1 — `just check`** (fmt, clippy across crates, all tests, audit) → green; the 5 pre-existing allowed audit advisories are not failures.
- [ ] **Step 2 — Downstream build check** is covered by `just check` (workspace build).
- [ ] **Step 3 — Manual walkthrough** of the spec's acceptance criteria 1–4 (persist across games, grow during stoppage, recover via long break never below minimum break, App-Options toggle hides it and persists across restart / older config defaults On).
- [ ] **Step 4 — Code review** (`superpowers:requesting-code-review`) over the new commits, with focus on the `behind_schedule` math (inherit/grow/recover/between-games continuity, portal-vs-manual scheduled start) and the config migration.

---

## Self-Review

**Spec coverage:**
- Persistent figure + carry-across-games + recover + hidden-when-ahead → A2 (`behind_schedule`) + C3 (display). Scheduled-start sources (portal/manual) → A1 (`next_game_scheduled_start`, `current_scheduled_start`). Minimum-break-capped claw-back → A2 between-games branch (`max(end + minimum_break, now)`) and the buffer term. Setting (default On, App Options, migration) → B1 + C1 + C2. 15-locale label → C2. Tests → A2, B1. Acceptance criteria → Phase D. ✓ no gaps.
- Out-of-scope (scheduling math, validation colours, wire format, other crates, ahead-of-schedule display) untouched by any task. ✓

**Placeholder scan:** Phase A/B carry full code + tests. Phase C UI/wiring tasks give exact new code and enumerate the mirror sites (with the compiler as the completeness backstop, per the project's lean-UI rule), and exact en-US strings + best-guess guidance for the other locales. No "TBD"/"add error handling"/"similar to" placeholders.

**Type consistency:** `behind_schedule(now: Instant) -> Duration` (A2) is consumed in mod.rs/C3 and rendered in main_view.rs/C3. `next_game_scheduled_start(now) -> Option<Instant>` (A1) feeds A2. `current_scheduled_start: Option<Instant>` / `last_game_end_time: Option<Instant>` (A1) read in A2. `show_behind_schedule_time: bool` is consistent across config.rs (B1), EditableSettings + message + mod.rs (C1), configuration.rs button (C2), and the C3 gate. `BoolGameParameter::ShowBehindScheduleTime` (C1) used in C2's button. The C3 indicator replaces the Game Block E1 `overrun` param (renamed `behind_schedule`) so there is exactly one figure on the time bar, not two.
