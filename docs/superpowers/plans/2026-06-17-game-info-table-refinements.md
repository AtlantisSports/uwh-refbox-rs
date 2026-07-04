# Game Info Table Refinements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refine the game-info table so scores only appear on games that have a score (the just-finished Prior game and the in-progress Current game), show the T/S Helper row and all referees on both tables, and rename "Last Game" → "Prior Game".

**Architecture:** All changes live in `refbox` view-builders + translations. The table model (`Row`/`TeamLine`) already carries `score: Option<u8>`; we drive the new behaviour from whether a block has a score (merge name+score into one wide cell when it doesn't), make the between-games Current block carry no score, and delete the now-unused `Variant` distinction so both tables list every referee.

**Tech Stack:** Rust 2024, iced 0.13, Fluent (`.ftl`) translations.

## Global Constraints

- MSRV Rust 1.85; Edition 2024. No APIs newer than 1.85.
- Clippy `-D warnings` on all platforms — zero warnings (no unused params/enums).
- Crate scope: `refbox` only. Do NOT touch `uwh-common` or any other crate.
- No new dependencies.
- Translation rule: every new/renamed key must have a real value in all 15 locales — no English placeholders.
- iced styling stays in `refbox/src/app/theme/`; reuse existing cell helpers — no new styling approach.
- Lean process (refbox UI): one code review at the end, no per-task deviation commits.

## The lifecycle model (the rule we are implementing)

A game moves Next → Current → Prior:
- **Next** (upcoming, not played) → **no score** → merge name+score into one wide name cell.
- **Current** *while in progress* (in a game) → **live score** → show name + score.
- **Current** *between games* (the upcoming game, not yet played) → **no score** → merge (wide name).
- **Prior** (just finished, between games) → **final score** → show name + score.

So: a game block shows a score column **iff it carries a score**. Between games, the Current block must carry no score.

---

## File Structure

- `refbox/src/app/view_builders/game_info_table.rs` — model (`game_info_rows`, `referee_rows`), renderer (`render_game_info_table`), and unit tests. Most changes here.
- `refbox/src/app/view_builders/game_info.rs` — full-page call site (drop `Variant::Full`).
- `refbox/src/app/view_builders/main_view.rs` — compact call site (drop `Variant::Compact`; add `last_game_scores` param so the main page shows the previous game's score).
- `refbox/src/app/mod.rs` — `build_main_view` call site (pass `last_game_info().scores`).
- `refbox/translations/*/refbox.ftl` — rename key `gi-last-game` → `gi-prior-game` (15 files); change only the `en-US` value to "Prior Game".

---

## Task 1: Score only on played/in-progress game blocks

**Files:**
- Modify: `refbox/src/app/view_builders/game_info_table.rs` (`game_info_rows` Current block; `render_game_info_table` Current/Next arm)
- Test: same file `#[cfg(test)]` module

**Interfaces:**
- Consumes: `Row::GameBlock { role, number, game_block, white: TeamLine, black: TeamLine }`, `TeamLine { name: Option<String>, score: Option<u8> }`, helpers `name_cell(name, dark, fp)`, `score_cell(score, dark, fp)`, `label_cell`, `value_cell`, `grid_row`, consts `LABEL_FP`, `VALUE_FP`, `HALF_FP`.
- Produces: between-games Current block has `white.score == None && black.score == None`; renderer emits 3 cells (label | value | wide-name spanning `HALF_FP`) per team row when a block has no score, 4 cells (label | value | name | score) when it does.

- [ ] **Step 1: Write the failing test — between-games Current block carries no score**

Add to the tests module (near `next_block_has_no_scores`):

```rust
#[test]
fn between_games_current_block_has_no_score() {
    // Between games the "Current" block is the upcoming (not-yet-played) game.
    let rows = game_info_rows(
        &between_games_snapshot(),
        &cfg_all_on(),
        false,
        None,
        None,
        None,
    );
    let current = rows
        .iter()
        .find_map(|r| match r {
            Row::GameBlock {
                role: GameRole::Current,
                white,
                black,
                ..
            } => Some((white.score, black.score)),
            _ => None,
        })
        .unwrap();
    assert_eq!(current, (None, None));
}

#[test]
fn in_game_current_block_carries_live_score() {
    let snapshot = GameSnapshot {
        current_period: GamePeriod::FirstHalf,
        scores: BlackWhiteBundle { black: 2, white: 3 },
        ..GameSnapshot::default()
    };
    let rows = game_info_rows(&snapshot, &cfg_all_on(), false, None, None, None);
    let current = rows
        .iter()
        .find_map(|r| match r {
            Row::GameBlock {
                role: GameRole::Current,
                white,
                black,
                ..
            } => Some((white.score, black.score)),
            _ => None,
        })
        .unwrap();
    assert_eq!(current, (Some(3), Some(2)));
}
```

Note: these call `game_info_rows` with **six** args (no trailing `Variant`) — Task 2 removes that param. If executing Task 1 before Task 2, temporarily keep `Variant::Full` as the 7th arg here and in Step 3's call, then remove it in Task 2. (Recommended: do Task 2's signature change first if running inline.)

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p refbox between_games_current_block_has_no_score in_game_current_block_carries_live_score`
Expected: `in_game_current_block_carries_live_score` passes (already wired), `between_games_current_block_has_no_score` FAILS (currently shows `(Some(0), Some(0))`).

- [ ] **Step 3: Make the between-games Current block carry no score**

In `game_info_rows`, replace the Current-block push (currently passing `Some(snapshot.scores)`):

```rust
    // The current game shows a live score only while a game is in progress; between
    // games the "current" block is the upcoming game, which has not been played yet.
    let current_scores = if between { None } else { Some(snapshot.scores) };

    // --- Current game block ---
    rows.push(game_block_row(
        GameRole::Current,
        current_game_num,
        Some(time_string(config.game_block)),
        current_scores,
        using_uwhportal,
        schedule,
        teams,
    ));
```

- [ ] **Step 4: Run the model tests to verify they pass**

Run: `cargo test -p refbox between_games_current_block_has_no_score in_game_current_block_carries_live_score`
Expected: PASS.

- [ ] **Step 5: Update the renderer — merge name+score into one wide cell when a block has no score**

In `render_game_info_table`, replace the Current / Next arm body (the `Row::GameBlock { role, number, game_block, white, black }` match arm) with:

```rust
            // Current / Next game: two rows — header (role + number) over Game
            // Block — each beside its team row, all on the shared 4-column grid.
            // A block with no score (an upcoming game) merges its name+score into
            // one wide name cell spanning the right half.
            Row::GameBlock {
                role,
                number,
                game_block,
                white,
                black,
            } => {
                let role_label = match role {
                    GameRole::Current => fl!("gi-current-game"),
                    _ => fl!("gi-next-game"),
                };
                let block = game_block.unwrap_or_default();
                let has_score = white.score.is_some() || black.score.is_some();

                let mut white_row = vec![label_cell(role_label, LABEL_FP), value_cell(number, VALUE_FP)];
                let mut black_row = vec![label_cell(fl!("gi-game-block"), LABEL_FP), value_cell(block, VALUE_FP)];
                if has_score {
                    white_row.push(name_cell(white.name, false, LABEL_FP));
                    white_row.push(score_cell(white.score, false, VALUE_FP));
                    black_row.push(name_cell(black.name, true, LABEL_FP));
                    black_row.push(score_cell(black.score, true, VALUE_FP));
                } else {
                    white_row.push(name_cell(white.name, false, HALF_FP));
                    black_row.push(name_cell(black.name, true, HALF_FP));
                }
                table = table.push(grid_row(white_row));
                table = table.push(grid_row(black_row));
            }
```

- [ ] **Step 6: Verify it compiles and all table tests pass**

Run: `cargo test -p refbox game_info_table`
Expected: PASS (including existing `next_block_has_no_scores`, which still holds).

- [ ] **Step 7: Commit**

```bash
git add refbox/src/app/view_builders/game_info_table.rs
git commit -m "feat(refbox): show game-info score only on played games"
```

---

## Task 2: Always show T/S Helper row + all referees on both tables (remove Variant)

**Files:**
- Modify: `refbox/src/app/view_builders/game_info_table.rs` (`referee_rows`, `game_info_rows` signature, tests)
- Modify: `refbox/src/app/view_builders/game_info.rs` (call site + import)
- Modify: `refbox/src/app/view_builders/main_view.rs` (call site + import)

**Interfaces:**
- Produces: `game_info_rows(snapshot, config, using_uwhportal, schedule, teams, last_game_scores)` — six args, no `Variant`. `referee_rows(game_number, schedule)` returns Chief, Time/Score Keeper, **T/S Helper (always, "-" when unassigned)**, Water 1, Water 2, Water 3. The `Variant` enum is removed.

- [ ] **Step 1: Update the failing tests for the new referee list**

Replace `compact_variant_keeps_only_chief_and_keeper` and `full_variant_lists_standard_referees_without_helper` with one test (both tables now list the same referees):

```rust
#[test]
fn referee_rows_always_include_blank_helper_and_all_water() {
    // Portal on but no schedule => referee section renders its fixed labels with "-".
    let rows = game_info_rows(
        &GameSnapshot::default(),
        &cfg_all_on(),
        true,
        None,
        None,
        None,
    );
    assert_eq!(
        ref_labels(&rows),
        vec![
            fl!("gi-ref-chief"),
            fl!("gi-ref-timekeeper"),
            fl!("gi-ref-timekeeper-helper"),
            fl!("gi-ref-water-1"),
            fl!("gi-ref-water-2"),
            fl!("gi-ref-water-3"),
        ]
    );
}
```

Also remove the trailing `Variant::Full` / `Variant::Compact` argument from **every** `game_info_rows(...)` call in the tests module (search the test module for `Variant::`).

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p refbox game_info_table`
Expected: FAIL to compile (extra `Variant::*` args / removed test names) — that is expected; the signature changes next.

- [ ] **Step 3: Rewrite `referee_rows` — always emit Helper + all water, drop `variant`**

Replace the whole `referee_rows` function with:

```rust
fn referee_rows(game_number: &GameNumber, schedule: Option<&Schedule>) -> Vec<Row> {
    // Resolve assigned names by role; "-" for an assigned-but-unnamed or absent slot.
    let mut chief = "-".to_string();
    let mut keeper = "-".to_string();
    let mut helper = "-".to_string();
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
                    "TimeOrScoreKeeperHelper" => helper = name,
                    "Water1" => water[0] = name,
                    "Water2" => water[1] = name,
                    "Water3" => water[2] = name,
                    _ => {}
                }
            }
        }
    }

    vec![
        Row::Referee {
            label: fl!("gi-ref-chief"),
            name: chief,
        },
        Row::Referee {
            label: fl!("gi-ref-timekeeper"),
            name: keeper,
        },
        Row::Referee {
            label: fl!("gi-ref-timekeeper-helper"),
            name: helper,
        },
        Row::Referee {
            label: fl!("gi-ref-water-1"),
            name: water[0].clone(),
        },
        Row::Referee {
            label: fl!("gi-ref-water-2"),
            name: water[1].clone(),
        },
        Row::Referee {
            label: fl!("gi-ref-water-3"),
            name: water[2].clone(),
        },
    ]
}
```

- [ ] **Step 4: Remove the `Variant` enum and its parameter**

In `game_info_table.rs`:
- Delete the `Variant` enum definition (the `pub(in super::super) enum Variant { Full, Compact }` block and its doc/derive).
- Change the `game_info_rows` signature: remove the `variant: Variant,` parameter (last param).
- Change the referee call inside `game_info_rows` from `referee_rows(current_game_num, schedule, variant)` to `referee_rows(current_game_num, schedule)`.

- [ ] **Step 5: Update both call sites**

`refbox/src/app/view_builders/game_info.rs`:
- Line ~46: change `use super::game_info_table::{Variant, game_info_rows, render_game_info_table};` → `use super::game_info_table::{game_info_rows, render_game_info_table};`
- Remove the `Variant::Full` argument from the `game_info_rows(...)` call.

`refbox/src/app/view_builders/main_view.rs`:
- Line 1: change `use super::game_info_table::{Variant, game_info_rows, render_game_info_table};` → `use super::game_info_table::{game_info_rows, render_game_info_table};`
- Remove the `Variant::Compact` argument from the `game_info_rows(...)` call (~line 267).

- [ ] **Step 6: Run tests + clippy**

Run: `cargo test -p refbox game_info_table && cargo clippy -p refbox -- -D warnings`
Expected: tests PASS; clippy clean (no unused `Variant`/param warnings).

- [ ] **Step 7: Commit**

```bash
git add refbox/src/app/view_builders/game_info_table.rs refbox/src/app/view_builders/game_info.rs refbox/src/app/view_builders/main_view.rs
git commit -m "feat(refbox): always show T/S helper + all referees on both tables"
```

---

## Task 3: Show the previous game (with its final score) on the main page

The main page already renders the previous-game block between games, but is fed `None` for scores (blank score boxes). Feed it the real final score — same source the full Game Info page uses. No extra rows; the existing block's score cells fill in.

**Files:**
- Modify: `refbox/src/app/view_builders/main_view.rs` (`build_main_view` signature + the `game_info_rows(...)` call)
- Modify: `refbox/src/app/mod.rs` (`build_main_view` call site, ~line 4421)

**Interfaces:**
- Consumes: `last_game_info().map(|i| i.scores)` → `Option<BlackWhiteBundle<u8>>` (the tournament manager's just-finished game scores; identical to the value passed to `build_game_info_page` at `mod.rs:4456`).
- Produces: `build_main_view(..., last_game_scores: Option<BlackWhiteBundle<u8>>)` — one new trailing param threaded into `game_info_rows`.

- [ ] **Step 1: Import the bundle type in main_view.rs**

In the `use uwh_common::{ ... };` group, add `bundles::BlackWhiteBundle,`:

```rust
use uwh_common::{
    bundles::BlackWhiteBundle,
    color::Color as GameColor,
    config::Game as GameConfig,
    game_snapshot::{GamePeriod, GameSnapshot, PenaltyTime, TimeoutSnapshot},
    uwhportal::schedule::Schedule,
};
```

- [ ] **Step 2: Add the parameter to `build_main_view`**

Add `last_game_scores: Option<BlackWhiteBundle<u8>>,` as the last parameter of `build_main_view` (the function already carries `#[allow(clippy::too_many_arguments)]`, so no new lint).

- [ ] **Step 3: Pass it into the compact table**

In the `game_info_rows(...)` call inside `build_main_view`, replace the `None,` argument (the `last_game_scores` slot, currently a literal `None` just before the removed `Variant::Compact`) with `last_game_scores,`.

- [ ] **Step 4: Pass the real score at the call site**

In `refbox/src/app/mod.rs`, the `AppState::MainPage` arm's `build_main_view(...)` call (~line 4421): add a final argument mirroring the Game Info page:

```rust
                build_main_view(
                    data,
                    game_config,
                    self.using_uwhportal,
                    self.schedule.as_ref(),
                    self.config.track_fouls_and_warnings,
                    self.config.sound.sound_enabled && self.config.sound.manual_alarm_enabled,
                    self.mouse_alarm_held || self.spacebar_held,
                    behind_schedule,
                    self.tm.lock().unwrap().last_game_info().map(|i| i.scores),
                )
```

(Safe: this arm already locks `self.tm` inline for `behind_schedule`/`new_config` and releases it; the lock is not held across the call.)

- [ ] **Step 5: Build + lint**

Run: `cargo build -p refbox && cargo clippy -p refbox -- -D warnings`
Expected: compiles, clippy clean.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/app/view_builders/main_view.rs refbox/src/app/mod.rs
git commit -m "feat(refbox): show previous game score on main page table"
```

---

## Task 4: Rename "Last Game" → "Prior Game"

**Confirm before doing (literal value + propagation surface):**
- Literal displayed value (English): **"Prior Game"**.
- Surface: rename key `gi-last-game` → `gi-prior-game` in all **15** `refbox/translations/*/refbox.ftl` files and the single code reference `fl!("gi-last-game")` in `game_info_table.rs`. Only the **en-US value** changes ("Last Game" → "Prior Game"); the other 14 values already mean "previous game" and stay unchanged.

**Files:**
- Modify: `refbox/src/app/view_builders/game_info_table.rs` (one `fl!` call)
- Modify: `refbox/translations/*/refbox.ftl` (15 files: rename key; en-US value change)

- [ ] **Step 1: Rename the key in code**

In `game_info_table.rs`, the `GameRole::Last` render arm: change `tall_cell(fl!("gi-last-game"), ...)` → `tall_cell(fl!("gi-prior-game"), ...)`.

- [ ] **Step 2: Rename the key in every locale; set en-US to "Prior Game"**

In each `refbox/translations/<locale>/refbox.ftl`, rename `gi-last-game = ...` to `gi-prior-game = ...`, keeping the existing value — EXCEPT `en-US`, whose value becomes `Prior Game`:

```
en-US:  gi-prior-game = Prior Game
de-DE:  gi-prior-game = Letztes Spiel
es:     gi-prior-game = Último partido
fr:     gi-prior-game = Dernier Match
id-ID:  gi-prior-game = Pertandingan Terakhir
it-IT:  gi-prior-game = Ultima Partita
ja-JP:  gi-prior-game = 前の試合
ko-KR:  gi-prior-game = 이전 경기
ms-MY:  gi-prior-game = Perlawanan Lepas
nl-NL:  gi-prior-game = Vorige Wedstrijd
pt-PT:  gi-prior-game = Último Jogo
th-TH:  gi-prior-game = เกมที่แล้ว
tl-PH:  gi-prior-game = Huling Laro
tr-TR:  gi-prior-game = Önceki Oyun
zh-CN:  gi-prior-game = 上场
```

- [ ] **Step 3: Verify no stale key remains**

Run: `grep -rn "gi-last-game" refbox/`
Expected: no output (key fully renamed).

- [ ] **Step 4: Build to confirm the key resolves**

Run: `cargo build -p refbox`
Expected: compiles (a missing/typoed key would compile but show "gi-prior-game" at runtime — Step 5 in Task 4 catches that visually).

- [ ] **Step 5: Commit**

```bash
git add refbox/src/app/view_builders/game_info_table.rs refbox/translations
git commit -m "feat(refbox): rename game-info Last Game label to Prior Game"
```

---

## Task 5: Build, full check, and live walkthrough

**Files:** none (verification only)

- [ ] **Step 1: Full quality gate**

Run: `just check`
Expected: fmt, lint, tests, audit all clean.

- [ ] **Step 2: Build the binary for the walkthrough**

Run: `cargo build -p refbox`
Expected: Finished.

- [ ] **Step 3: Launch and walk through (user drives)**

Launch (background, sandbox disabled): `WAYLAND_DISPLAY= <worktree>/target/debug/refbox`

Confirm:
1. **Between games** (header "NEXT GAME"): the upcoming game block shows **no score column** — team names fill the wider cell; the **Prior Game** block (if a game just finished) shows its final score.
2. **During a game**: the **Current Game** block shows the **live score** on BOTH the main page table and the full Game Info page; the **Next Game** preview shows no score (wider names).
3. **Main page table** now lists **all referees** (Chief, Time/Score Keeper, T/S Helper, Water 1–3), truncating if space runs out.
4. **T/S Helper** row is present even when unassigned (shows "-").
5. **Main page** shows the **previous game with its final score** between games (score boxes filled, not blank).
6. Label reads **"Prior Game"** (not "Last Game"), no raw `gi-prior-game` key showing.

- [ ] **Step 4: Final whole-branch code review**

Run `superpowers:requesting-code-review` (opus) over the full branch diff vs `origin/master`.

---

## Deviations

(record any execution deviations here)
