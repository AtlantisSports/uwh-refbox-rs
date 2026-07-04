# Game-Info Table Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Re-present the refbox game-info display as a tappable table (full version on the Game Information page, condensed on the Main UI page), with the table built from a typed, unit-testable row model.

**Architecture:** A new `game_info_table` module exposes a pure builder `game_info_rows(...) -> Vec<Row>` (encodes which blocks/settings/referees appear, by game state and variant) and a renderer `render_game_info_table(rows) -> Element<Message>`. `game_info.rs` (full) and `main_view.rs` (compact) both call them. Tapping the table opens Game Options directly via a new message. No `uwh-common`/wire-format change.

**Tech Stack:** Rust 2024, `iced` 0.13, Fluent translations (`fl!`), `just` for checks.

**Spec:** `docs/superpowers/specs/2026-06-17-game-info-table-design.md`

## Global Constraints

- MSRV Rust 1.85; Edition 2024. No APIs newer than 1.85.
- Clippy clean: `cargo clippy -p refbox -- -D warnings` (mirrors `just lint`; do **not** use `--all-targets` locally — it surfaces ~90 pre-existing test lints that are not CI failures).
- Tests: `cargo test -p refbox` (no `--lib`).
- No `unwrap()`/`expect()` in non-test code without a justifying comment.
- All user-facing strings via `fl!`; **every new key gets a real translation in all 15 locales** (`de-DE en-US es fr id-ID it-IT ja-JP ko-KR ms-MY nl-NL pt-PT th-TH tl-PH tr-TR zh-CN`) — never an English placeholder.
- Scope: `refbox` crate only. Do not touch `uwh-common`, `overlay`, or LED/panel code.
- Lean process (refbox UI): TDD the row-model builder; verify renderer/wiring by compilation + walkthrough; record any deviations in the "Deviations" section at the bottom rather than separate commits.
- Branch: create `feat/refbox/game-info-table` off fresh `origin/master` before Task 1 (the worktree/branch is set up via `superpowers:using-git-worktrees` at execution time). Confirm with the user before the branch is created if not already done.

---

## File Structure

- **Create** `refbox/src/app/view_builders/game_info_table.rs` — row model (`Row`, `TeamLine`, `GameRole`, `TeamColor`, `Variant`), the `game_info_rows` builder, the `render_game_info_table` renderer, and `#[cfg(test)]` builder tests. One responsibility: turn game state into table rows and draw them.
- **Modify** `refbox/src/app/view_builders/mod.rs` — declare `mod game_info_table;`.
- **Modify** `refbox/src/app/view_builders/game_info.rs` — replace `details_strings` text body with the full table wrapped in a tap-to-Game-Options button.
- **Modify** `refbox/src/app/view_builders/main_view.rs` — replace the `config_string` text panel with the compact table; keep the `config_string_game_num` fallback for 4+ warnings.
- **Modify** `refbox/src/app/message.rs` — add `EditGameConfigPage(ConfigPage)`.
- **Modify** `refbox/src/app/mod.rs` — handle the new message (shared init helper), thread last-game scores into the game-info view dispatch.
- **Modify** `refbox/translations/*/refbox.ftl` (15 files) — new label-only keys (en-US in Task 1, other locales in Task 9).

---

## Task 1: New en-US label vocabulary

The table splits "Label: value" into separate cells, so it needs **label-only** keys. Add these to **en-US only** now (other locales in Task 9; `fl!` falls back to en-US so builder tests pass meanwhile).

**Files:**
- Modify: `refbox/translations/en-US/refbox.ftl`

- [ ] **Step 1: Add the new keys** to `refbox/translations/en-US/refbox.ftl` (group them under a `# Game-info table labels` comment):

```ftl
# Game-info table labels
gi-last-game = Last Game
gi-current-game = Current Game
gi-next-game = Next Game
gi-game-block = Game Block
gi-half-length = Half Length
gi-half-time-length = Half-Time Length
gi-game-length = Game Length
gi-timeouts = Timeouts
gi-timeout-duration = Timeout Duration
gi-overtime = Overtime
gi-sudden-death = Sudden Death
gi-pre-overtime-break = Pre-Overtime Break
gi-pre-sudden-death-break = Pre-Sudden Death Break
gi-overtime-half-length = Overtime Half Length
gi-overtime-half-time-length = Overtime Half-Time Length
gi-minimum-game-break = Minimum Game Break
gi-stop-clock-last-2 = Stop Clock in Last 2 Min
gi-ref-chief = Chief Referee
gi-ref-timekeeper = Time/Score Keeper
gi-ref-timekeeper-helper = Time/Score Helper
gi-ref-water-1 = Water Referee 1
gi-ref-water-2 = Water Referee 2
gi-ref-water-3 = Water Referee 3
```

- [ ] **Step 2: Verify it parses** — `cargo build -p refbox` (Expected: builds; Fluent keys are loaded at runtime).
- [ ] **Step 3: Commit** — `git add refbox/translations/en-US/refbox.ftl && git commit -m "feat(refbox): add game-info table label keys (en-US)"`

---

## Task 2: Row model + builder (Current block + settings grid)

**Files:**
- Create: `refbox/src/app/view_builders/game_info_table.rs`
- Modify: `refbox/src/app/view_builders/mod.rs` (add `mod game_info_table;`)
- Test: in `game_info_table.rs` `#[cfg(test)]`

**Interfaces:**
- Produces (used by Tasks 3–8):
  - `pub(in super::super) enum Variant { Full, Compact }`
  - `pub(in super::super) enum GameRole { Last, Current, Next }`
  - `pub(in super::super) struct TeamLine { pub name: Option<String>, pub score: Option<u8> }`
  - `pub(in super::super) enum Row { GameBlock { role, number: String, game_block: Option<String>, white: TeamLine, black: TeamLine }, SettingPair { left: (String, String), right: Option<(String, String)> }, Referee { label: String, name: String } }`
  - `pub(in super::super) fn game_info_rows(snapshot, config, using_uwhportal, schedule, teams, last_game_scores: Option<BlackWhiteBundle<u8>>, variant: Variant) -> Vec<Row>`

- [ ] **Step 1: Declare the module** — add `mod game_info_table;` to `refbox/src/app/view_builders/mod.rs` (alongside the other `mod` lines).

- [ ] **Step 2: Write the failing tests** in `game_info_table.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use uwh_common::game_snapshot::GameSnapshot;

    fn cfg_all_on() -> GameConfig {
        GameConfig {
            single_half: false,
            overtime_allowed: true,
            sudden_death_allowed: true,
            num_team_timeouts_allowed: 1,
            ..Default::default()
        }
    }

    // Helper: collect the (label, value) pairs the settings grid emits, in order.
    fn setting_pairs(rows: &[Row]) -> Vec<(String, Option<String>)> {
        rows.iter()
            .filter_map(|r| match r {
                Row::SettingPair { left, right } => {
                    Some((left.0.clone(), right.as_ref().map(|p| p.0.clone())))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn current_block_always_present() {
        // GameSnapshot::default() is BetweenGames; the Current block is present in any state.
        let snapshot = GameSnapshot::default();
        let rows = game_info_rows(&snapshot, &cfg_all_on(), false, None, None, None, Variant::Full);
        assert!(rows.iter().any(|r| matches!(
            r,
            Row::GameBlock { role: GameRole::Current, game_block: Some(_), .. }
        )));
    }

    #[test]
    fn settings_order_all_features_on() {
        let snapshot = GameSnapshot::default();
        let rows = game_info_rows(&snapshot, &cfg_all_on(), false, None, None, None, Variant::Full);
        let pairs = setting_pairs(&rows);
        // Six rows, paired exactly as in the mockup.
        assert_eq!(pairs[0], (fl!("gi-half-length"), Some(fl!("gi-half-time-length"))));
        assert_eq!(pairs[1], (fl!("gi-timeouts"), Some(fl!("gi-timeout-duration"))));
        assert_eq!(pairs[2], (fl!("gi-overtime"), Some(fl!("gi-sudden-death"))));
        assert_eq!(pairs[3], (fl!("gi-pre-overtime-break"), Some(fl!("gi-pre-sudden-death-break"))));
        assert_eq!(pairs[4], (fl!("gi-overtime-half-length"), Some(fl!("gi-minimum-game-break"))));
        assert_eq!(pairs[5], (fl!("gi-overtime-half-time-length"), Some(fl!("gi-stop-clock-last-2"))));
    }

    #[test]
    fn overtime_off_hides_overtime_rows() {
        let snapshot = GameSnapshot::default();
        let config = GameConfig { overtime_allowed: false, ..cfg_all_on() };
        let labels: Vec<String> = setting_pairs(&game_info_rows(
            &snapshot, &config, false, None, None, None, Variant::Full,
        ))
        .into_iter()
        .flat_map(|(l, r)| std::iter::once(l).chain(r))
        .collect();
        assert!(!labels.contains(&fl!("gi-pre-overtime-break")));
        assert!(!labels.contains(&fl!("gi-overtime-half-length")));
        assert!(!labels.contains(&fl!("gi-overtime-half-time-length")));
        assert!(labels.contains(&fl!("gi-minimum-game-break")));
        assert!(labels.contains(&fl!("gi-stop-clock-last-2")));
    }

    #[test]
    fn zero_timeouts_hides_duration() {
        let snapshot = GameSnapshot::default();
        let config = GameConfig { num_team_timeouts_allowed: 0, ..cfg_all_on() };
        let labels: Vec<String> = setting_pairs(&game_info_rows(
            &snapshot, &config, false, None, None, None, Variant::Full,
        ))
        .into_iter()
        .flat_map(|(l, r)| std::iter::once(l).chain(r))
        .collect();
        assert!(!labels.contains(&fl!("gi-timeout-duration")));
    }

    #[test]
    fn single_half_shows_game_length_hides_half_time() {
        let snapshot = GameSnapshot::default();
        let config = GameConfig { single_half: true, ..cfg_all_on() };
        let labels: Vec<String> = setting_pairs(&game_info_rows(
            &snapshot, &config, false, None, None, None, Variant::Full,
        ))
        .into_iter()
        .flat_map(|(l, r)| std::iter::once(l).chain(r))
        .collect();
        assert!(labels.contains(&fl!("gi-game-length")));
        assert!(!labels.contains(&fl!("gi-half-length")));
        assert!(!labels.contains(&fl!("gi-half-time-length")));
    }
}
```

- [ ] **Step 3: Run to confirm failure** — `cargo test -p refbox game_info_table` (Expected: FAIL — `game_info_rows` not found).

- [ ] **Step 4: Implement the module head + types + builder** (Current block + settings only; context blocks/referees come in Tasks 3–4). Note the stop-clock value uses the displayed game number, exactly like `details_strings` does today.

```rust
use super::*;
use uwh_common::{
    config::Game as GameConfig,
    game_snapshot::{BlackWhiteBundle, GameNumber, GamePeriod, GameSnapshot},
    uwhportal::schedule::{Schedule, TeamList},
};

const TEAM_NAME_LEN_LIMIT: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum Variant { Full, Compact }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum GameRole { Last, Current, Next }

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct TeamLine {
    pub name: Option<String>,
    pub score: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) enum Row {
    GameBlock {
        role: GameRole,
        number: String,
        game_block: Option<String>,
        white: TeamLine,
        black: TeamLine,
    },
    SettingPair {
        left: (String, String),
        right: Option<(String, String)>,
    },
    Referee {
        label: String,
        name: String,
    },
}

pub(in super::super) fn game_info_rows(
    snapshot: &GameSnapshot,
    config: &GameConfig,
    using_uwhportal: bool,
    schedule: Option<&Schedule>,
    teams: Option<&TeamList>,
    last_game_scores: Option<BlackWhiteBundle<u8>>,
    variant: Variant,
) -> Vec<Row> {
    let between = snapshot.current_period == GamePeriod::BetweenGames;
    // The "current" game whose config + settings are displayed: the in-progress
    // game when playing, the upcoming game between games. Matches details_strings.
    let current_game_num: &GameNumber = if between {
        &snapshot.next_game_number
    } else {
        &snapshot.game_number
    };

    let mut rows = Vec::new();

    // --- Current game block ---
    rows.push(game_block_row(
        GameRole::Current,
        current_game_num,
        Some(time_string(config.game_block)),
        Some(snapshot.scores),
        using_uwhportal,
        schedule,
        teams,
    ));

    // --- Settings grid (belongs to the current game) ---
    let mut settings: Vec<(String, String)> = Vec::new();
    if config.single_half {
        settings.push((fl!("gi-game-length"), time_string(config.half_play_duration)));
    } else {
        settings.push((fl!("gi-half-length"), time_string(config.half_play_duration)));
        settings.push((fl!("gi-half-time-length"), time_string(config.half_time_duration)));
    }
    settings.push((fl!("gi-timeouts"), team_timeouts_value(config)));
    if config.num_team_timeouts_allowed != 0 {
        settings.push((fl!("gi-timeout-duration"), time_string(config.team_timeout_duration)));
    }
    settings.push((fl!("gi-overtime"), bool_string(config.overtime_allowed)));
    settings.push((fl!("gi-sudden-death"), bool_string(config.sudden_death_allowed)));
    if config.overtime_allowed {
        settings.push((fl!("gi-pre-overtime-break"), time_string(config.pre_overtime_break)));
    }
    if config.sudden_death_allowed {
        settings.push((fl!("gi-pre-sudden-death-break"), time_string(config.pre_sudden_death_duration)));
    }
    if config.overtime_allowed {
        settings.push((fl!("gi-overtime-half-length"), time_string(config.ot_half_play_duration)));
    }
    settings.push((fl!("gi-minimum-game-break"), time_string(config.minimum_break)));
    if config.overtime_allowed {
        settings.push((fl!("gi-overtime-half-time-length"), time_string(config.ot_half_time_duration)));
    }
    settings.push((fl!("gi-stop-clock-last-2"), stop_clock_value(schedule, current_game_num)));

    let mut iter = settings.into_iter();
    while let Some(left) = iter.next() {
        let right = iter.next();
        rows.push(Row::SettingPair { left, right });
    }

    let _ = (variant, last_game_scores); // consumed in Tasks 3–4
    rows
}

fn team_timeouts_value(config: &GameConfig) -> String {
    if config.num_team_timeouts_allowed == 0 {
        "0".to_string()
    } else if config.timeouts_counted_per_half {
        format!("{}/{}", config.num_team_timeouts_allowed, fl!("half"))
    } else {
        format!("{}/{}", config.num_team_timeouts_allowed, fl!("game"))
    }
}

fn stop_clock_value(schedule: Option<&Schedule>, game_number: &GameNumber) -> String {
    match schedule.and_then(|s| s.get_game_timing(game_number)) {
        Some(rule) => bool_string(rule.last_2_min_stop_time),
        None => fl!("unknown"),
    }
}

// Builds a GameBlock row. `scores` populates the team lines when present; team
// names resolve from the schedule (portal only), else None.
fn game_block_row(
    role: GameRole,
    game_number: &GameNumber,
    game_block: Option<String>,
    scores: Option<BlackWhiteBundle<u8>>,
    using_uwhportal: bool,
    schedule: Option<&Schedule>,
    teams: Option<&TeamList>,
) -> Row {
    let (white_name, black_name, number) = resolve_game(game_number, using_uwhportal, schedule, teams);
    Row::GameBlock {
        role,
        number,
        game_block,
        white: TeamLine { name: white_name, score: scores.map(|s| s.white) },
        black: TeamLine { name: black_name, score: scores.map(|s| s.black) },
    }
}

// Returns (white_name, black_name, display_number). Names are Some only when the
// portal schedule has the game; the display number falls back to the raw number.
fn resolve_game(
    game_number: &GameNumber,
    using_uwhportal: bool,
    schedule: Option<&Schedule>,
    teams: Option<&TeamList>,
) -> (Option<String>, Option<String>, String) {
    if using_uwhportal {
        if let Some(game) = schedule.and_then(|s| s.games.get(game_number)) {
            let black = limit_team_name_len(&get_team_name(&game.dark, teams), TEAM_NAME_LEN_LIMIT);
            let white = limit_team_name_len(&get_team_name(&game.light, teams), TEAM_NAME_LEN_LIMIT);
            return (Some(white), Some(black), game.number.to_string());
        }
    }
    (None, None, game_number.to_string())
}
```

- [ ] **Step 5: Run tests** — `cargo test -p refbox game_info_table` (Expected: PASS).
- [ ] **Step 6: Lint** — `cargo clippy -p refbox -- -D warnings` (Expected: clean).
- [ ] **Step 7: Commit** — `git add -A && git commit -m "feat(refbox): add game-info table row model + settings builder"`

---

## Task 3: State-dependent context block (Last / Next) + scores

**Files:** Modify `refbox/src/app/view_builders/game_info_table.rs`

**Interfaces:** Consumes `game_block_row`, `Row`, `GameRole`, `last_game_scores` from Task 2.

- [ ] **Step 1: Add failing tests** to the `tests` module:

```rust
fn between_games_snapshot() -> GameSnapshot {
    // Equivalent to GameSnapshot::default() (BetweenGames), spelled out for clarity.
    GameSnapshot { current_period: GamePeriod::BetweenGames, ..GameSnapshot::default() }
}

fn in_game_snapshot() -> GameSnapshot {
    GameSnapshot { current_period: GamePeriod::FirstHalf, ..GameSnapshot::default() }
}

fn roles(rows: &[Row]) -> Vec<GameRole> {
    rows.iter().filter_map(|r| match r {
        Row::GameBlock { role, .. } => Some(*role),
        _ => None,
    }).collect()
}

#[test]
fn between_games_shows_last_then_current_no_next() {
    let rows = game_info_rows(&between_games_snapshot(), &cfg_all_on(), false, None, None, None, Variant::Full);
    assert_eq!(roles(&rows).first(), Some(&GameRole::Last));
    assert!(roles(&rows).contains(&GameRole::Current));
    assert!(!roles(&rows).contains(&GameRole::Next));
}

#[test]
fn in_game_shows_current_then_next_no_last() {
    let rows = game_info_rows(&in_game_snapshot(), &cfg_all_on(), false, None, None, None, Variant::Full);
    assert!(!roles(&rows).contains(&GameRole::Last));
    assert_eq!(roles(&rows).first(), Some(&GameRole::Current));
    assert_eq!(roles(&rows).last(), Some(&GameRole::Next));
}

#[test]
fn last_block_has_no_game_block_line_and_uses_last_scores() {
    let scores = BlackWhiteBundle { black: 5, white: 3 };
    let rows = game_info_rows(&between_games_snapshot(), &cfg_all_on(), false, None, None, Some(scores), Variant::Full);
    let last = rows.iter().find_map(|r| match r {
        Row::GameBlock { role: GameRole::Last, game_block, white, black, .. } => Some((game_block.clone(), white.score, black.score)),
        _ => None,
    }).unwrap();
    assert_eq!(last, (None, Some(3), Some(5)));
}

#[test]
fn next_block_has_no_scores() {
    let rows = game_info_rows(&in_game_snapshot(), &cfg_all_on(), false, None, None, None, Variant::Full);
    let next = rows.iter().find_map(|r| match r {
        Row::GameBlock { role: GameRole::Next, white, black, game_block, .. } => Some((game_block.is_some(), white.score, black.score)),
        _ => None,
    }).unwrap();
    assert_eq!(next, (true, None, None)); // Next keeps its Game Block line, no scores
}
```

- [ ] **Step 2: Run to confirm failure** — `cargo test -p refbox game_info_table` (Expected: FAIL).

- [ ] **Step 3: Implement** — in `game_info_rows`, replace the `let _ = (variant, last_game_scores);` line and bracket the Current block with context blocks:

```rust
    // Context block BEFORE the current block, between games only: the just-finished game.
    if between {
        let last = game_block_row(
            GameRole::Last,
            &snapshot.game_number,
            None, // prior game's Game Block is intentionally not shown
            last_game_scores,
            using_uwhportal,
            schedule,
            teams,
        );
        rows.insert(0, last);
    }
```

…and AFTER the settings grid (still inside `game_info_rows`, before `let _ = variant;`):

```rust
    // Context block AFTER the current block, in-game only: the upcoming game (no score).
    if !between {
        rows.push(game_block_row(
            GameRole::Next,
            &snapshot.next_game_number,
            Some(time_string(config.game_block)),
            None,
            using_uwhportal,
            schedule,
            teams,
        ));
    }
    let _ = variant; // consumed in Task 4
```

(Remove the now-obsolete `let _ = (variant, last_game_scores);`.)

- [ ] **Step 4: Run tests** — `cargo test -p refbox game_info_table` (Expected: PASS, incl. Task 2 tests).
- [ ] **Step 5: Commit** — `git add -A && git commit -m "feat(refbox): add state-dependent game blocks + scores to game-info table"`

---

## Task 4: Referees (full vs compact variant; Helper only when present)

**Files:** Modify `refbox/src/app/view_builders/game_info_table.rs`

**Interfaces:** Consumes `Row`, `Variant`. Reads `game.referee_assignments` (roles: `Chief`, `TimeOrScoreKeeper`, `Water1`, `Water2`, `Water3`; a future `TimeOrScoreKeeperHelper` lights up the Helper row when present).

- [ ] **Step 1: Add failing tests**:

```rust
fn ref_labels(rows: &[Row]) -> Vec<String> {
    rows.iter().filter_map(|r| match r { Row::Referee { label, .. } => Some(label.clone()), _ => None }).collect()
}

#[test]
fn no_referees_without_portal() {
    let rows = game_info_rows(&GameSnapshot::default(), &cfg_all_on(), false, None, None, None, Variant::Full);
    assert!(ref_labels(&rows).is_empty());
}

#[test]
fn compact_variant_keeps_only_chief_and_keeper() {
    // Portal on but no schedule => referee section still renders its fixed labels with "-".
    let rows = game_info_rows(&GameSnapshot::default(), &cfg_all_on(), true, None, None, None, Variant::Compact);
    assert_eq!(ref_labels(&rows), vec![fl!("gi-ref-chief"), fl!("gi-ref-timekeeper")]);
}

#[test]
fn full_variant_lists_standard_referees_without_helper() {
    let rows = game_info_rows(&GameSnapshot::default(), &cfg_all_on(), true, None, None, None, Variant::Full);
    // Helper omitted when no Helper assignment is present.
    assert_eq!(
        ref_labels(&rows),
        vec![
            fl!("gi-ref-chief"),
            fl!("gi-ref-timekeeper"),
            fl!("gi-ref-water-1"),
            fl!("gi-ref-water-2"),
            fl!("gi-ref-water-3"),
        ]
    );
}
```

- [ ] **Step 2: Run to confirm failure** — `cargo test -p refbox game_info_table` (Expected: FAIL).

- [ ] **Step 3: Implement** — append referees inside `game_info_rows`, just before the `if !between { ... Next ... }` block (referees attach to the current game, above the Next block). Remove the `let _ = variant;` line from Task 3 — `variant` is now consumed:

```rust
    if using_uwhportal {
        rows.extend(referee_rows(current_game_num, schedule, variant));
    }
```

Add the helper:

```rust
fn referee_rows(
    game_number: &GameNumber,
    schedule: Option<&Schedule>,
    variant: Variant,
) -> Vec<Row> {
    // Resolve assigned names by role; "-" for an assigned-but-unnamed or absent slot.
    let mut chief = "-".to_string();
    let mut keeper = "-".to_string();
    let mut helper: Option<String> = None;
    let mut water = ["-".to_string(), "-".to_string(), "-".to_string()];

    if let Some(game) = schedule.and_then(|s| s.games.get(game_number)) {
        if let Some(refs) = &game.referee_assignments {
            for r in refs {
                if r.user_id.is_none() {
                    continue;
                }
                let name = r.display_name.clone().unwrap_or_else(|| "-".to_string());
                match r.role.as_str() {
                    "Chief" => chief = name,
                    "TimeOrScoreKeeper" => keeper = name,
                    "TimeOrScoreKeeperHelper" => helper = Some(name),
                    "Water1" => water[0] = name,
                    "Water2" => water[1] = name,
                    "Water3" => water[2] = name,
                    _ => {}
                }
            }
        }
    }

    let mut out = vec![
        Row::Referee { label: fl!("gi-ref-chief"), name: chief },
        Row::Referee { label: fl!("gi-ref-timekeeper"), name: keeper },
    ];
    if matches!(variant, Variant::Compact) {
        return out; // main page: Chief + Keeper only
    }
    if let Some(h) = helper {
        out.push(Row::Referee { label: fl!("gi-ref-timekeeper-helper"), name: h });
    }
    out.push(Row::Referee { label: fl!("gi-ref-water-1"), name: water[0].clone() });
    out.push(Row::Referee { label: fl!("gi-ref-water-2"), name: water[1].clone() });
    out.push(Row::Referee { label: fl!("gi-ref-water-3"), name: water[2].clone() });
    out
}
```

- [ ] **Step 4: Run tests** — `cargo test -p refbox game_info_table` (Expected: PASS).
- [ ] **Step 5: Lint** — `cargo clippy -p refbox -- -D warnings`.
- [ ] **Step 6: Commit** — `git add -A && git commit -m "feat(refbox): add referee rows + variant to game-info table"`

---

## Task 5: Renderer (rows → iced Element)

Lean: verified by compilation + walkthrough (iced rendering is not unit-tested). Exact cell backgrounds/widths are finalised in the walkthrough — start from these.

**Files:** Modify `refbox/src/app/view_builders/game_info_table.rs`

**Interfaces:** Produces `pub(in super::super) fn render_game_info_table(rows: Vec<Row>) -> Element<'static, Message>`.

- [ ] **Step 1: Implement the renderer.** Draw a `column` of rows: `GameBlock` as a 2-row × 4-column grid (left column: header label+number, then optional Game Block; right: white team line over black team line, the black line on a dark background with light text); `SettingPair` as a 4-column row (label, value, label, value — right pair blank if `None`); `Referee` as a full-width label + name row. Use `SMALL_TEXT`, `SPACING`, `PADDING`, `Length::Fill`/`FillPortion` (no fixed pixel widths), and container background styles consistent with the existing `white_button`/`black_button` colours for the team lines. Build helper fns `team_cell`, `label_cell`, `value_cell` returning containers so the four row kinds share cell styling.

```rust
use iced::{
    Length,
    alignment::Horizontal,
    widget::{column, container, row, text},
};

pub(in super::super) fn render_game_info_table(rows: Vec<Row>) -> Element<'static, Message> {
    let mut col = column![].spacing(SPACING / 4.0).width(Length::Fill);
    for r in rows {
        col = col.push(match r {
            Row::GameBlock { role, number, game_block, white, black } => {
                render_game_block(role, number, game_block, white, black)
            }
            Row::SettingPair { left, right } => render_setting_pair(left, right),
            Row::Referee { label, name } => render_referee(label, name),
        });
    }
    col.into()
}
```

Implement `render_game_block`, `render_setting_pair`, `render_referee`, and the cell helpers in the same file (text in `value`/`name` cells right- or centre-aligned per the mockup; the `GameRole` maps to the `gi-last-game`/`gi-current-game`/`gi-next-game` label). Keep each helper small and focused.

- [ ] **Step 2: Build** — `cargo build -p refbox` (Expected: builds).
- [ ] **Step 3: Lint** — `cargo clippy -p refbox -- -D warnings`.
- [ ] **Step 4: Commit** — `git add -A && git commit -m "feat(refbox): render game-info table rows to iced widgets"`

---

## Task 6: Navigation message — tap to Game Options

**Files:** Modify `refbox/src/app/message.rs`, `refbox/src/app/mod.rs`

**Interfaces:** Produces `Message::EditGameConfigPage(ConfigPage)`. Reuses the existing `EditGameConfig` init.

- [ ] **Step 1: Add the variant** in `message.rs` next to `EditGameConfig`:

```rust
    EditGameConfig,
    EditGameConfigPage(ConfigPage),
```

Add it to the same match arms `EditGameConfig` appears in (the discriminant/`PartialEq`/category `match`es around lines 320, 413, 562, 636): treat `EditGameConfigPage(_)` like `EditGameConfig`, with `(Self::EditGameConfigPage(a), Self::EditGameConfigPage(b)) => a == b` in the equality impl.

- [ ] **Step 2: Refactor the handler** in `mod.rs` — extract the body of the `Message::EditGameConfig` arm (the `edited_settings` setup + auth task) into a method `fn enter_game_config(&mut self, landing: ConfigPage) -> Task<Message>` that ends with `self.app_state = AppState::EditGameConfig(landing);` and returns the task. Replace the existing arm with:

```rust
            Message::EditGameConfig => self.enter_game_config(ConfigPage::Main),
            Message::EditGameConfigPage(page) => self.enter_game_config(page),
```

- [ ] **Step 3: Build + lint** — `cargo build -p refbox` then `cargo clippy -p refbox -- -D warnings` (Expected: clean; footer "Settings" still lands on the menu).
- [ ] **Step 4: Commit** — `git add -A && git commit -m "feat(refbox): add EditGameConfigPage message for direct Game Options nav"`

---

## Task 7: Wire the full table into the Game Information page

**Files:** Modify `refbox/src/app/view_builders/game_info.rs`, `refbox/src/app/mod.rs`

- [ ] **Step 1: Thread last-game scores into the view dispatch.** In `mod.rs` at the `AppState::GameDetailsPage` arm (~line 4447), add a 6th argument:

```rust
            AppState::GameDetailsPage(is_refreshing) => build_game_info_page(
                data,
                &self.config.game,
                self.using_uwhportal,
                is_refreshing,
                self.schedule.as_ref(),
                self.tm.lock().unwrap().last_game_info().map(|i| i.scores),
            ),
```

- [ ] **Step 2: Rebuild `build_game_info_page`.** Add the `last_game_scores: Option<BlackWhiteBundle<u8>>` param; delete `details_strings` and the two `text()` columns; build and render the table, wrapped in a single tap button:

```rust
    use super::game_info_table::{game_info_rows, render_game_info_table, Variant};
    let table = render_game_info_table(game_info_rows(
        snapshot, config, using_uwhportal, schedule, teams, last_game_scores, Variant::Full,
    ));
    let table_button = button(table)
        .padding(PADDING)
        .style(light_gray_button)
        .width(Length::Fill)
        .height(Length::Fill)
        .on_press(Message::EditGameConfigPage(ConfigPage::Game));
```

Keep the existing footer row (Back / Refresh-or-spacer / Settings) and the top `make_game_time_button` unchanged; place `table_button` between them. Remove the now-unused `details_strings` fn and its `#[cfg(test)] mod tests` (the behaviour they covered now lives in `game_info_table` tests).

- [ ] **Step 3: Build + lint + test** — `cargo build -p refbox`, `cargo clippy -p refbox -- -D warnings`, `cargo test -p refbox`.
- [ ] **Step 4: Walkthrough check** — build the binary (`cargo build -p refbox`) and launch it (background, `WAYLAND_DISPLAY=` prefix, `dangerouslyDisableSandbox`); open Game Information; confirm the table renders and tapping it lands on Game Options. Record result.
- [ ] **Step 5: Commit** — `git add -A && git commit -m "feat(refbox): render game info page as a tappable table"`

---

## Task 8: Wire the compact table into the Main UI page

**Files:** Modify `refbox/src/app/view_builders/main_view.rs`

- [ ] **Step 1: Replace the `config_string` text panel** (the `max_num_warns < 4` branch, ~line 256) with the compact table. Keep the surrounding `button(...).on_press(Message::ShowGameDetails)` wrapper, `light_gray_button` style, `height(Length::FillPortion(2))`, and the `else` branch (`config_string_game_num` fallback for 4+ warnings) unchanged:

```rust
        center_col = center_col.push(if max_num_warns < 4 {
            button(
                container(render_game_info_table(game_info_rows(
                    snapshot, game_config, using_uwhportal, schedule, teams, None, Variant::Compact,
                )))
                .center_y(Length::Fill)
                .width(Length::Fill),
            )
            .padding(PADDING)
            .style(light_gray_button)
            .height(Length::FillPortion(2))
            .width(Length::Fill)
            .on_press(Message::ShowGameDetails)
        } else {
            // unchanged config_string_game_num fallback
            ...
        });
```

Add `use super::game_info_table::{game_info_rows, render_game_info_table, Variant};` and drop the now-unused `config_string` import (keep `config_string_game_num`). (Compact passes `None` for last-game scores — the main page shows the current game live; the Last/Next context block still renders without a final score there, which is acceptable for the compact glance; confirm in walkthrough.)

- [ ] **Step 2: Build + lint + test** — `cargo build -p refbox`, `cargo clippy -p refbox -- -D warnings`, `cargo test -p refbox`.
- [ ] **Step 3: Walkthrough check** — launch; confirm the main-page panel shows the compact table (Chief + Keeper only, no Water/Helper), fits alongside the warnings panel, taps through to Game Information, and still collapses to game-numbers with 4+ warnings. Record fit result; if too tall, note tuning (row spacing / dropping the context block on compact) in Deviations.
- [ ] **Step 4: Commit** — `git add -A && git commit -m "feat(refbox): show compact game-info table on main page"`

---

## Task 9: Propagate new keys to the other 14 locales

**Files:** Modify the 14 non-en-US `refbox/translations/*/refbox.ftl`

- [ ] **Step 1: For each locale**, add all `gi-*` keys from Task 1 with a **real translation** (no English). Derive each from text the locale already uses: the setting labels appear in that locale's existing combined keys (`game-length-ot-allowed`, `team-to-len`, `sd-allowed`, `overtime-details`, `stop-clock-last-2`, `game-block-info`, `team-timeouts`) — copy the label portion (before the `:`/value). Referee labels derive from the locale's existing `ref-list` (`Chief Ref`/`Timer`/`Water Ref n`) adjusted to the fuller wording; `gi-current-game`/`gi-last-game`/`gi-next-game` derive from existing game wording (`one-game`, `last-game-next-game`). Where a locale has no prior equivalent, translate the English value.
- [ ] **Step 2: Verify coverage** — confirm each locale defines every `gi-*` key (diff each file's `gi-` keys against en-US's):

```bash
for l in de-DE es fr id-ID it-IT ja-JP ko-KR ms-MY nl-NL pt-PT th-TH tl-PH tr-TR zh-CN; do
  miss=$(comm -23 <(grep -oE '^gi-[a-z0-9-]+' refbox/translations/en-US/refbox.ftl | sort) \
                  <(grep -oE '^gi-[a-z0-9-]+' refbox/translations/$l/refbox.ftl | sort))
  [ -n "$miss" ] && echo "$l missing: $miss"
done
```

Expected: no output (all locales complete).
- [ ] **Step 3: Build + check** — `just check` (fmt, clippy, tests, audit) — Expected: green.
- [ ] **Step 4: Commit** — `git add -A && git commit -m "feat(refbox): translate game-info table labels (14 locales)"`

---

## Task 10: Portal follow-up write-up

Non-code deliverable: a short note for the Portal team (does not affect refbox behaviour).

**Files:** Create `docs/portal-followup-game-info-fields.md`

- [ ] **Step 1: Write the note** describing the two refbox-side rows that stay dormant until the
  Portal sends data: (a) a **Time/Score Helper** referee assignment (role string
  `TimeOrScoreKeeperHelper`, which the refbox already matches — the row appears automatically once
  present); (b) optional future **Game Type** (values *Round Robin / Crossover / Playoff / Final /
  Medal Game*) for re-adding a Game Type row later. Include what JSON the refbox expects and that
  no refbox change is needed for (a).
- [ ] **Step 2: Commit** — `git add docs/portal-followup-game-info-fields.md && git commit -m "docs(refbox): note Portal-side fields for game-info table"`

---

## Acceptance / Done

- `just check` green.
- Game Information page renders the full table; tapping it opens Game Options; footer Settings still opens the menu.
- Main page shows the compact table (Chief + Keeper), with the 4+ warnings fallback intact.
- Builder unit tests cover state-dependent blocks, settings show/hide, single-half, referee variants, and scores.
- All `gi-*` keys translated in 15 locales.
- Walkthrough confirms both surfaces; any fit/label tuning recorded in Deviations.

## Deviations

Tasks 1–10 committed (`ac044b59..17804b98`), `just check` green at Task 9. The live
walkthrough then drove a substantial renderer/builder rework (UNCOMMITTED at the 2026-06-17
checkpoint — must be committed):

- **Renderer rebuilt as a single fixed 4-column grid** (Label | Value | Label/Name | Value/Score)
  instead of the original mixed flat rows. Square cells; thin gridlines via 1px gaps over a dark
  `table_grid_container` backing.
- **Settings: hide → grey.** All six setting rows are now FIXED slots and always present; items
  that don't apply to the game's config are shown **greyed** (`SettingCell { grayed }` +
  `table_*_cell_grayed` styles), not hidden/reflowed. Builder + tests updated accordingly
  (`overtime_off_grays_*`, `zero_timeouts_grays_*`, `single_half_*_grays_*`,
  `settings_keep_six_fixed_rows_when_mostly_off`).
- **Last Game = merged cells** spanning its two team rows (fixed height `2*ROW_H + GRID`, NOT Fill).
  Team rows split into name (col 3) + score (col 4).
- **Theme:** added `table_grid_container`, `table_label_cell`, `table_value_cell`,
  `table_white_cell`, `table_black_cell` (+ `_grayed` variants) in `theme/container.rs`; all
  label/value cells use one uniform light-grey fill (greyed text = `disabled_color`).
- **`gi-unknown = ???`** key added (all 15 locales) for unknown stop-clock (table-only; global
  `unknown` unchanged).
- **Sizing:** `TABLE_TEXT=15`, `ROW_H=22`, cells have horizontal-only padding so rows are short
  enough to fit all params + referees.
- **Layout wrappers:** Game-info page and main-page panel top-align the table (transparent
  fill-height wrapper) so the button colour fills empty space below the grid.

Still open: confirm all rows fit on the full page; final whole-branch review; finish-branch/PR.
The carry-forward `#![allow(dead_code)]`/variant-allow items were resolved during Tasks 7–8.
