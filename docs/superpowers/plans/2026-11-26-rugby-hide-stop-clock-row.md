# Rugby: hide the "Stop Clock in Last 2 Min" cell — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** In Rugby mode, the bottom-right cell of the Game Info settings table is empty (no label, no value) instead of "Stop Clock in Last 2 Min ???". Hockey is unchanged.

**Architecture:** `game_info_rows` (refbox/src/app/view_builders/game_info_table.rs) builds the table as plain data; it gains a `mode: Mode` parameter. In Rugby the 6th `SettingPair`'s right cell becomes an empty, non-greyed cell. The caller `build_game_info_page` (game_info.rs) already destructures `mode` from `ViewData` and passes it on.

**Tech Stack:** Rust 2024, iced 0.13, refbox crate only.

**Spec:** PO rulings 2026-11-26 (design lane): "rugby is always stop-time, so that row is not needed in Rugby mode"; layout option 1 chosen — empty slot, no label and no value, grid shape kept (not label-only, not greyed).

## Global Constraints

- Scope: `refbox/src/app/view_builders/game_info_table.rs`, `refbox/src/app/view_builders/game_info.rs` and `refbox/src/app/view_builders/main_view.rs` (see Deviations) only. No translation, config, uwh-common or settings-page changes.
- MSRV 1.85; clippy `-D warnings`; no new `unwrap`/`expect` in production code.
- Table keeps six setting rows in fixed slots — no row or cell is removed from the `Vec<Row>`; the cell is present with empty strings.
- Heavy jobs (cargo test/check) need the coordinator's GO; run with `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4`.

## Review Focus

1. Court schedule finished (`no_current_game`) in Rugby: the dash loop must NOT turn the empty cell into "—" — it stays empty (label and value). Pinned by `rugby_empty_stop_clock_cell_stays_empty_when_court_finished`.
2. Rugby with a Portal timing rule that has `last_2_min_stop_time` set: still empty — the mode wins over the schedule. Covered because the rugby branch never calls `stop_clock_value`; the main rugby test passes `None` schedule, which is enough since the branch is decided before the schedule is read.
3. Hockey (6v6 and 3v3) unchanged: label "Stop Clock in Last 2 Min", value from the schedule or "???". Existing tests stay green with `Mode::Hockey6V6`; new test adds a 3v3 assertion.
4. Rugby between games and in-game: same rows — the mode check is independent of period.
5. The other five setting rows in Rugby are untouched (labels unchanged).

---

### Task 1: Empty stop-clock cell in Rugby mode

**Files:**
- Modify: `refbox/src/app/view_builders/game_info_table.rs` (`game_info_rows` signature + 6th `SettingPair` + no-game dash loop + test module call sites)
- Modify: `refbox/src/app/view_builders/game_info.rs` (the one production call, ~line 88)

**Interfaces:**
- Produces: `game_info_rows(snapshot: &GameSnapshot, config: &GameConfig, mode: Mode, next_game_block: Duration, uses_remote: bool, schedule: Option<&Schedule>, teams: Option<&TeamList>, last_game: Option<(GameNumber, BlackWhiteBundle<u8>)>) -> Vec<Row>` — `mode` inserted as the 3rd parameter.

- [x] **Step 1: Write the failing tests** (in the existing `#[cfg(test)] mod tests`)

```rust
#[test]
fn rugby_hides_stop_clock_cell() {
    let snapshot = GameSnapshot { next_game_number: "2".to_string(), ..Default::default() };
    let rows = game_info_rows(&snapshot, &cfg_all_on(), Mode::Rugby, cfg_all_on().game_block,
        false, None, None, None);
    let pairs = setting_pairs(&rows);
    assert_eq!(pairs.len(), 6);
    assert_eq!(pairs[5], (fl!("gi-overtime-half-time-length"), String::new()));
    // The empty cell has no value and is not greyed.
    let last = rows.iter().filter_map(|r| match r {
        Row::SettingPair { right, .. } => Some(right.clone()), _ => None }).last().unwrap();
    assert_eq!(last, SettingCell { label: String::new(), value: String::new(), grayed: false });
    // Hockey 3v3 still shows it.
    let rows = game_info_rows(&snapshot, &cfg_all_on(), Mode::Hockey3V3, cfg_all_on().game_block,
        false, None, None, None);
    assert_eq!(setting_pairs(&rows)[5].1, fl!("gi-stop-clock-last-2"));
}

#[test]
fn rugby_empty_stop_clock_cell_stays_empty_when_court_finished() {
    // Use whatever snapshot the existing court-finished tests use so that
    // `snapshot.court_schedule_finished()` is true; assert the other cells are dashed
    // (value == NO_VALUE) and the rugby stop-clock cell is still label "" / value "".
}
```

The second test's snapshot setup copies the existing court-finished test in this module; its assertions are: `pairs[5].1 == ""`, that cell's `value == ""`, and `pairs[5].0`'s cell `value == NO_VALUE`.

- [x] **Step 2: Update every existing `game_info_rows(` call in the test module** to pass `Mode::Hockey6V6` as the 3rd argument (24 sites; mechanical).

- [x] **Step 3: Run the tests — expect a compile failure** (signature not yet changed). (Needs coordinator GO.)

Run: `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 cargo test -p refbox game_info_table`

- [x] **Step 4: Implement**
  - Add `mode: Mode` as 3rd parameter of `game_info_rows`.
  - 6th pair's right cell: `if mode == Mode::Rugby { SettingCell::active(String::new(), String::new()) } else { <existing stop-clock cell> }`, with a one-line comment citing the ruling (rugby is always stop-time).
  - No-game dash loop: leave a cell untouched when its label is empty (an empty slot stays empty).
  - `game_info.rs`: pass `mode` (already destructured from `ViewData`) as the 3rd argument.

- [x] **Step 5: Run tests — expect PASS**

Run: `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 cargo test -p refbox game_info_table`

- [x] **Step 6: `just check`** (coordinator GO; `-j 4`). Expected: all green.

- [x] **Step 7: Commit** (main session, under the PO's plan approval)

```bash
git add refbox/src/app/view_builders/game_info_table.rs refbox/src/app/view_builders/game_info.rs refbox/src/app/view_builders/main_view.rs
git commit -m "fix(refbox): leave the stop-clock cell empty in rugby mode"
```

## Pre-PR

Code review (built-in `code-review`), `just check`, PO walkthrough in Rugby (isolated `XDG_CONFIG_HOME`, Rugby/Manual and Hockey for contrast), dangling-work sweep. Claude-driven verification: not worth it — one cell's text, best judged by eye on the walk.

## Deviations

- **Third file: `refbox/src/app/view_builders/main_view.rs`.** The main game screen draws the same table through two more `game_info_rows` calls, which the plan missed (it searched only the Game Info page). Each now passes `mode`; without it the main screen would still show "???" in Rugby. Same table, same ruling, so no scope change.
- 21 existing test call sites, not 24.
- Code review (high, 2026-11-26, on 28e1359b): no bugs, 4 nits, all fixed in the follow-up commit: comments updated for the blank-cell exception, `SettingCell::blank()` marker applied to both sides of the dash loop, a Rugby in-game + Portal-rule test added, this plan's scope line and Step 7 corrected.
