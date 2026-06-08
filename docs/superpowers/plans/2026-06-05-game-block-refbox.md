# Game Block (refbox side of ADR-008) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make "Game Block" (the start-to-start slot duration) the authoritative, operator-edited scheduling parameter in the refbox — replacing "Nominal Break Between Games" — with config migration, a portal-ready optional field, scheduling driven by Game Block, red/yellow editor validation, and a quiet main-screen overrun indicator.

**Architecture:** Add `game_block` to `uwh-common`'s `GameConfig` as the authoritative value, with pure helper methods (`regulation_play`, `game_block_minimum`, `team_timeout_allotment`, `game_block_buffer`) so the math is testable in `uwh-common`. `refbox` scheduling uses `game_block` directly; the Game Options UI edits it with validation colours driven by those helpers; the main game screen computes an overrun figure from the tournament-manager clock and the buffer (no serialized-snapshot change).

**Tech Stack:** Rust 2024, `iced` 0.13, `i18n-embed-fl` Fluent, `confy`/TOML config, `just`.

**Spec:** `docs/superpowers/specs/2026-06-05-game-block-refbox-design.md`
**Background:** `docs/decisions/008-game-block.md`

---

## Prerequisite & process

- **Branch off `master` AFTER PR #1002 (Change 1) merges.** This feature rewrites the same
  `start_game` scheduling expression that #1002 fixed, and uses the `single_half` distinction.
  Create `feat/uwh-common/game-block` off updated `master` (scope = broadest crate = `uwh-common`).
- **Heavy process** (`.claude/rules/plan-execution.md`): `uwh-common` shared types + migration and
  the tournament-manager scheduling are high-blast-radius. Each math/logic task is TDD with real
  unit tests. UI-only steps (button/editor wiring, translations) use build + `just check` + manual.
- After every `uwh-common` change, confirm all downstream crates build (`just check` covers it).

## File structure

| File | Responsibility |
|------|----------------|
| `uwh-common/src/config.rs` | `game_block` field + default; `Config`/`Game::migrate` conversion; pure helper methods |
| `uwh-common/src/uwhportal/schedule.rs` | optional `game_block` on `TimingRule` + `Into<GameConfig>` mapping |
| `refbox/src/tournament_manager/mod.rs` | scheduling uses `game_block`; `accumulated_overrun` helper |
| `refbox/src/app/message.rs` | `LengthParameter::NominalBetweenGame` → `GameBlock` |
| `refbox/src/app/mod.rs` | `EditParameter` / `ParameterEditComplete` wiring for `GameBlock` |
| `refbox/src/app/view_builders/configuration.rs` | Game Block button + editor with red/yellow validation; Done-disable on red |
| `refbox/src/app/view_builders/shared_elements.rs`, `game_info.rs` | info "time between games" line → Game Block; main time-bar overrun indicator |
| `refbox/translations/*/refbox.ftl` | `game-block` + help/validation strings, all 15 locales |

---

## Phase A — `uwh-common`: data model, helpers, migration

### Task A1: Add `game_block` field, default, and pure helper methods

**Files:** `uwh-common/src/config.rs` (struct line 8, `Default` ~46, add an `impl Game`); tests in the same file's `#[cfg(test)]` module.

- [ ] **Step 1 — Write failing tests** for the helpers (place in the config tests module):

```rust
#[test]
fn test_game_block_helpers_two_period() {
    let g = Game {
        single_half: false,
        half_play_duration: Duration::from_secs(900),
        half_time_duration: Duration::from_secs(180),
        minimum_break: Duration::from_secs(240),
        num_team_timeouts_allowed: 1,
        team_timeout_duration: Duration::from_secs(60),
        timeouts_counted_per_half: false,
        game_block: Duration::from_secs(2880),
        ..Default::default()
    };
    assert_eq!(g.regulation_play(), Duration::from_secs(1980)); // 2*900+180
    assert_eq!(g.game_block_minimum(), Duration::from_secs(2220)); // 1980+240
    // per-game, both teams, 1 each * 60s = 120s
    assert_eq!(g.team_timeout_allotment(), Duration::from_secs(120));
    assert_eq!(g.game_block_buffer(), Duration::from_secs(660)); // 2880-2220
}

#[test]
fn test_game_block_helpers_single_period_and_per_half() {
    let g = Game {
        single_half: true,
        half_play_duration: Duration::from_secs(600),
        half_time_duration: Duration::from_secs(180), // ignored when single_half
        minimum_break: Duration::from_secs(120),
        num_team_timeouts_allowed: 2,
        team_timeout_duration: Duration::from_secs(60),
        timeouts_counted_per_half: true, // single period => counted once
        game_block: Duration::from_secs(800),
        ..Default::default()
    };
    assert_eq!(g.regulation_play(), Duration::from_secs(600)); // single period
    assert_eq!(g.game_block_minimum(), Duration::from_secs(720)); // 600+120
    // per-half but single period => 1 period; 2 teams * 2 * 60 = 240
    assert_eq!(g.team_timeout_allotment(), Duration::from_secs(240));
    assert_eq!(g.game_block_buffer(), Duration::from_secs(80)); // 800-720
}
```

- [ ] **Step 2 — Run, expect failure** (`game_block` field + methods don't exist):
`cargo test -p uwh-common game_block_helpers` → FAIL (compile errors).

- [ ] **Step 3 — Add the field + default.** In `struct Game`, add near `nominal_break`:
`pub game_block: Duration,`. In `Default`, add `game_block: Duration::from_secs(2880),`
(= 2×900 + 180 + 900, the default cadence). Keep `nominal_break` for compatibility.

- [ ] **Step 4 — Add helper methods** in an `impl Game` block:

```rust
impl Game {
    /// Playing time excluding overtime: two halves + half-time, or a single period.
    pub fn regulation_play(&self) -> Duration {
        if self.single_half {
            self.half_play_duration
        } else {
            2 * self.half_play_duration + self.half_time_duration
        }
    }

    /// Smallest Game Block that fits the game plus the minimum break.
    pub fn game_block_minimum(&self) -> Duration {
        self.regulation_play() + self.minimum_break
    }

    /// Total team-timeout time both teams could use in a game (referee timeouts excluded).
    pub fn team_timeout_allotment(&self) -> Duration {
        let periods = if self.timeouts_counted_per_half && !self.single_half {
            2
        } else {
            1
        };
        2 * periods * self.num_team_timeouts_allowed * self.team_timeout_duration
    }

    /// Slack between the Game Block and the math minimum (saturating at zero).
    pub fn game_block_buffer(&self) -> Duration {
        self.game_block.saturating_sub(self.game_block_minimum())
    }
}
```

(Use `u32`/`u16` casts as needed so `periods * num_team_timeouts_allowed * duration` type-checks;
`num_team_timeouts_allowed` is `u16` — multiply via `u32`.)

- [ ] **Step 5 — Run tests, expect pass.** `cargo test -p uwh-common game_block_helpers` → PASS.

- [ ] **Step 6 — Commit:** `feat(uwh-common): add game_block field and slot-math helpers`.

### Task A2: Migrate existing configs to populate `game_block`

**Files:** `uwh-common/src/config.rs` `Game::migrate` (line 63) + existing `test_migrate_game` (~200).

- [ ] **Step 1 — Failing test:** extend/duplicate `test_migrate_game` to assert that a config
  table **without** `game_block` yields `game_block == regulation_play + nominal_break`, and one
  **with** an explicit `game_block` keeps it:

```rust
#[test]
fn test_migrate_game_block_derived_when_absent() {
    let mut old = Table::new();
    old.insert("half_play_duration".into(), Value::Integer(900));
    old.insert("half_time_duration".into(), Value::Integer(180));
    old.insert("nominal_break".into(), Value::Integer(900));
    old.insert("minimum_break".into(), Value::Integer(240));
    let gm = Game::migrate(&old);
    assert_eq!(gm.game_block, Duration::from_secs(2880)); // 1980 + 900
}

#[test]
fn test_migrate_game_block_kept_when_present() {
    let mut old = Table::new();
    old.insert("half_play_duration".into(), Value::Integer(900));
    old.insert("half_time_duration".into(), Value::Integer(180));
    old.insert("nominal_break".into(), Value::Integer(900));
    old.insert("minimum_break".into(), Value::Integer(240));
    old.insert("game_block".into(), Value::Integer(1500));
    let gm = Game::migrate(&old);
    assert_eq!(gm.game_block, Duration::from_secs(1500));
}
```

- [ ] **Step 2 — Run, expect failure.**

- [ ] **Step 3 — Implement.** In `migrate`, add a `mut game_block` binding, then after the
  existing `process_duration` calls add:

```rust
let mut game_block = Duration::ZERO;
let had_game_block = old.get("game_block").and_then(|v| v.as_integer()).is_some();
process_duration(old, "game_block", &mut game_block);
if !had_game_block {
    // Derive from the migrated play durations so the prior cadence is preserved.
    let regulation = if single_half {
        half_play_duration
    } else {
        2 * half_play_duration + half_time_duration
    };
    game_block = regulation + nominal_break;
}
```
Then include `game_block` in the returned `Game { ... }`.

- [ ] **Step 4 — Run tests, expect pass.** `cargo test -p uwh-common migrate` → PASS.

- [ ] **Step 5 — Commit:** `feat(uwh-common): migrate existing configs to game_block`.

---

## Phase B — `uwh-common`: portal `TimingRule` optional Game Block

### Task B1: Accept an optional `game_block` from the portal

**Files:** `uwh-common/src/uwhportal/schedule.rs` `TimingRule` (line 237), `Into<GameConfig>` (line 268), tests (`test_deserialize_timing_rule` ~880, `test_timing_rule_single_half...` ~1099).

- [ ] **Step 1 — Failing tests:** (a) deserializing a `TimingRule` JSON **without** `gameBlock`
  yields a `GameConfig` whose `game_block == regulation_play + nominal_break` (derived); (b) JSON
  **with** `gameBlock` yields that exact value. Mirror the existing `test_deserialize_timing_rule`
  setup for field names/serde rename style.

- [ ] **Step 2 — Run, expect failure.**

- [ ] **Step 3 — Implement.** Add to `TimingRule`:
```rust
#[serde(default)]
pub game_block: Option<Duration>,
```
(match the crate's existing `Duration` serde convention; `#[serde(default)]` makes it optional so
old payloads still decode). In the `Into<GameConfig>` impl, set the config's `game_block`:
```rust
game_block: game_block.unwrap_or_else(|| {
    let regulation = if half_time_duration == Duration::ZERO {
        half_play_duration
    } else {
        2 * half_play_duration + half_time_duration
    };
    regulation + nominal_break // nominal_break here is the default pulled in by this impl
}),
```
(Reuse the impl's existing `single_half = half_time_duration == Duration::ZERO` logic for
consistency, and the `nominal_break` it already sources from `GameConfig::default`.)

- [ ] **Step 4 — Run tests + serialization round-trip test, expect pass.**
`cargo test -p uwh-common timing_rule` → PASS.

- [ ] **Step 5 — Commit:** `feat(uwh-common): accept optional game_block in portal timing rule`.

---

## Phase C — `refbox`: scheduling driven by Game Block

### Task C1: Next-game scheduling uses `game_block`

**Files:** `refbox/src/tournament_manager/mod.rs` — `start_game` (~1015, the `next_scheduled_start` assignment), the `#[cfg(test)] set_game_start` helper, and the `between_games` clock initialisation (lines ~68/81 that seed it from `nominal_break`). Tests: the existing `test_between_game_timing*` (~2503) plus a new one.

- [ ] **Step 1 — Failing test:** add `test_between_game_timing_game_block` modelled on
  `test_between_game_timing`, but set `game_block` directly and assert `next_scheduled_start ==
  now + game_block`:

```rust
#[test]
fn test_between_game_timing_game_block() {
    initialize();
    let config = GameConfig {
        half_play_duration: Duration::from_secs(10),
        half_time_duration: Duration::from_secs(3),
        minimum_break: Duration::from_secs(2),
        game_block: Duration::from_secs(40), // start-to-start
        overtime_allowed: false,
        sudden_death_allowed: false,
        ..Default::default()
    };
    let mut tm = TournamentManager::new(config);
    let now = Instant::now();
    tm.start_clock(now);
    tm.start_play_now(now).unwrap();
    assert_eq!(tm.next_scheduled_start, Some(now + Duration::from_secs(40)));
}
```

- [ ] **Step 2 — Run, expect failure** (still computes `2*half + half_time + nominal_break`).

- [ ] **Step 3 — Implement.** Replace the `next_scheduled_start` expression in `start_game`
  (and the `set_game_start` test helper) with:
```rust
self.next_scheduled_start = Some(sched_start + self.config.game_block);
```
  Update the `between_games` clock seed (lines ~68/81) to use `self.config.game_block` where it
  currently uses `nominal_break` for the no-schedule cadence (keep `minimum_break` as the floor in
  `calc_time_to_next_game`, unchanged).

- [ ] **Step 4 — Run the full timing test group, expect pass** — update the pre-existing
  `test_between_game_timing` / `test_between_game_timing_single_half` expectations to set
  `game_block` instead of relying on `nominal_break` (their configs must now specify `game_block`;
  e.g. the two-period 32s case becomes `game_block: 32`, the single-period 19s case `game_block: 19`).
  `cargo test -p refbox between_game_timing` → PASS.

- [ ] **Step 5 — Commit:** `feat(refbox): schedule next game from game_block`.

### Task C2: `accumulated_overrun` helper for the indicator

**Files:** `refbox/src/tournament_manager/mod.rs` (add a method + test).

- [ ] **Step 1 — Failing test:** add `test_accumulated_overrun` that starts a game, advances wall
  time with the clock stopped (timeout/stoppage) and asserts `accumulated_overrun(now)` equals the
  stopped wall-time (real elapsed minus game-clock time consumed). Model setup on existing
  stop/timeout tests in the file.

- [ ] **Step 2 — Run, expect failure.**

- [ ] **Step 3 — Implement** a method returning how much real time the current game has lost to
  stoppages so far: `wall_elapsed_since_game_start(now) − regulation_time_consumed`. Use existing
  `game_start_time` and the clock-state accessors already in the manager. Return `Duration::ZERO`
  when not in a live game.

- [ ] **Step 4 — Run test, expect pass.**

- [ ] **Step 5 — Commit:** `feat(refbox): add accumulated_overrun to tournament manager`.

---

## Phase D — `refbox`: Game Block button, editor, validation, info line

### Task D1: Rename the parameter and add translations

**Files:** `refbox/src/app/message.rs` (`LengthParameter::NominalBetweenGame` → `GameBlock`, line 648), all references in `refbox/src/app/mod.rs` (`EditParameter` ~2362 reads `config.game.nominal_break` → `config.game.game_block`; `ParameterEditComplete` ~2407 writes `config.game_block`), `refbox/src/app/view_builders/configuration.rs` (button + editor title/hint), `refbox/translations/*`.

- [ ] **Step 1 — Rename** `LengthParameter::NominalBetweenGame` to `GameBlock`; let the compiler
  list every match site and update each (the read in `EditParameter` to `self.config.game.game_block`,
  the write in `ParameterEditComplete` to `edited_settings.config.game_block`, and the editor
  title/hint arm in `build_game_parameter_editor`).

- [ ] **Step 2 — Replace the Game Options button** (configuration.rs ~778, currently
  `fl!("nominal-break-between-games")` + `time_string(config.nominal_break)` +
  `Message::EditParameter(LengthParameter::NominalBetweenGame)`): label `fl!("game-block")`,
  value `time_string(config.game_block)`, message `EditParameter(LengthParameter::GameBlock)`.

- [ ] **Step 3 — Editor title/hint** for `LengthParameter::GameBlock`: title `fl!("game-block")`,
  hint `fl!("game-block-help")` ("Time from the start of one game to the start of the next").

- [ ] **Step 4 — Add translations** `game-block`, `game-block-help` to all 15 locales (best-guess
  per locale, no English placeholders — see `feedback_translate_all_locales_no_placeholders`).

- [ ] **Step 5 — Build + commit:** `cargo build -p refbox`; commit
  `feat(refbox): replace Nominal Break button with Game Block`.

### Task D2: Red/yellow validation on the Game Block editor

**Files:** `refbox/src/app/view_builders/configuration.rs` (`build_game_parameter_editor`), reusing `red_button`/`yellow_button` from `theme/button.rs`. The editor stages the edited `Duration` in `AppState::ParameterEditor`; the validation reads the staged value against the edited config's other fields.

- [ ] **Step 1 — Define the validation outcome.** In refbox, compute from the *staged* Game Block
  + the edited config: `TooShort` if `staged < cfg.game_block_minimum()`; `Tight` if
  `staged - game_block_minimum < cfg.team_timeout_allotment()`; else `Ok`. (Build a temporary
  `Game` with the staged value, or compare against `cfg.regulation_play()`/`minimum_break`/
  `team_timeout_allotment()` directly using the uwh-common helpers from Phase A.)

- [ ] **Step 2 — Style the value editor / Done button:** `TooShort` → red styling on the value +
  **Done disabled** (no `on_press`); `Tight` → yellow styling, Done enabled; `Ok` → normal, Done
  enabled. The colour recomputes on each keystroke since the view re-renders from the staged value.

- [ ] **Step 3 — Help text** under the editor reflects the state (e.g. append a short red note when
  TooShort / yellow note when Tight). Add the note strings to all 15 locales.

- [ ] **Step 4 — Build + lint + manual check.** `cargo build -p refbox`; `cargo clippy -p refbox -- -D warnings`.
  Manually: enter a too-short value → red + Done greyed; a tight value → yellow + savable; a roomy
  value → grey.

- [ ] **Step 5 — Commit:** `feat(refbox): validate Game Block against slot minimum and timeout buffer`.

### Task D3: Info displays show Game Block

**Files:** `refbox/src/app/view_builders/shared_elements.rs` (`game-config` builder ~874) and `game_info.rs` (`time-btwn-games` line ~244). Today these show "Nominal Break"/`nominal_break`.

- [ ] **Step 1 — Replace** the "time between games" / nominal-break line in both builders with a
  Game Block line using `time_string(config.game_block)` and a `game-block` info label. Add the
  label string to all 15 locales.

- [ ] **Step 2 — Build + manual check** (main-screen panel and Game Info page show Game Block).

- [ ] **Step 3 — Commit:** `feat(refbox): show Game Block in game-info displays`.

---

## Phase E — `refbox`: main-screen overrun indicator

### Task E1: Show the quiet `-M:SS` overrun on the time bar

**Files:** the main game-screen time-bar builder (`make_game_time_button` / the main view in `shared_elements.rs` — locate the time-bar widget that renders "FIRST HALF 17:42"), using `accumulated_overrun` (Task C2) and `cfg.game_block_buffer()` (Task A1). No change to the serialized `GameSnapshot` — compute in the refbox view layer from the live tournament-manager state + `now` + config.

- [ ] **Step 1 — Compute the figure** in the time-bar builder: `shown = accumulated_overrun(now).saturating_sub(buffer)`. If `shown == 0`, render nothing on the right of the time bar. If `shown > 0`, render a red `-M:SS` (reuse the existing clock time-formatting helper, prefixed with `-`).

- [ ] **Step 2 — Placement:** right side of the time bar, matching the layout sketch in the spec;
  red text consistent with other red indicators in `theme/`.

- [ ] **Step 3 — Manual check:** start a game, take long timeouts/stoppages until overrun exceeds
  the buffer; confirm the red `-M:SS` appears and grows, and is absent before that.

- [ ] **Step 4 — Commit:** `feat(refbox): show overrun indicator when a game exceeds its slot buffer`.

---

## Phase F — Final verification

- [ ] **Step 1 — `just check`** (fmt, clippy across crates, all tests, audit) → green; the 5
  pre-existing allowed audit advisories are not failures.
- [ ] **Step 2 — Downstream build check:** `just check` builds `refbox`, `schedule-processor`,
  `overlay`, `led-panel-sim` against the changed `GameConfig` — confirm no breakage.
- [ ] **Step 3 — Manual walkthrough** of every acceptance criterion in the spec (Game Block button
  + value, red/yellow validation, fixed-cadence behaviour when changing half length, overrun
  indicator, migrated config).
- [ ] **Step 4 — Code review** (`superpowers:requesting-code-review`) before opening the PR, given
  the high blast radius.

---

## Self-Review

**Spec coverage:**
- Game Block authoritative + scheduling → Tasks A1, C1. Migration → A2. Portal optional field → B1.
- Always-show-Game-Block / retire Nominal Break button → D1, D3. Validation red/yellow → D2 (math
  from A1). Overrun indicator `-M:SS` → C2 + E1. Acceptance criteria → Phase F.
- Out-of-scope items (schedule-processor, removing `nominal_break`, portal-send, wireless) are not
  touched by any task. ✓ no gaps.

**Placeholder scan:** Math-heavy tasks (A1, A2, B1, C1, C2) carry full code/tests. UI tasks (D1–D3,
E1) specify exact files, labels, messages, and the validation/overrun formulas, with build+manual
verification per the lean-UI rule — deliberately not keystroke-scripted, per the project's
plan-execution "rough task list, not step-by-step scripts" guidance for UI. The one genuinely
unpinned location (the time-bar widget in E1) names the search target and the inputs.

**Type consistency:** `game_block: Duration` and the helpers `regulation_play`, `game_block_minimum`,
`team_timeout_allotment`, `game_block_buffer` (defined A1) are used consistently in D2/E1.
`LengthParameter::GameBlock` (D1) replaces `NominalBetweenGame` everywhere. `accumulated_overrun`
(C2) feeds E1. Default `game_block = 2880s` matches the derived-migration formula (1980 + 900).

---

## Deviations

- **Task C1 — between-games clock seeds/fallback left on `nominal_break` (NOT switched to
  `game_block`).** The plan's C1 Step 3 said to change the constructor seeds (`mod.rs` lines ~68/81,
  `clock_time`/`reset_game_time`) and implied the no-schedule cadence should use `game_block`.
  During execution this was found to be a semantic error: those values represent the *break shown
  on the between-games clock* (≈15 min), not the start-to-start *slot* (`game_block`, ≈48 min).
  Seeding a between-games countdown with the full slot would display the wrong number. `nominal_break`
  is intentionally retained as a legacy field (default 900s) precisely for these break-placeholder
  uses, and the spec itself says `calc_time_to_next_game` is "otherwise unchanged". So C1 changed
  ONLY the two scheduling expressions (`start_game` and the `set_game_start` test helper) to
  `sched_start + game_block`; the constructor seeds (68/81) and the `calc_time_to_next_game`
  fallback (~906) were left exactly as-is. Net effect matches the spec's intent (next start =
  this start + Game Block) without breaking the break display.
- **Task C1 — 8 existing tournament-manager tests updated.** Switching the `set_game_start` test
  helper to `game_block` made 8 tests that assert the between-games clock derive their slot from the
  default `game_block` (2880s) instead of `regulation + nominal_break`. Each was given an explicit
  `game_block` equal to the start-to-start slot already documented in that test's own comment
  (e.g. `2*9 + 2 + 5 = 25` → `game_block: 25`). No assertions were changed.
