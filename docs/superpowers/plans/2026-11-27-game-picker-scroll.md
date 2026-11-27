# SELECT GAME Picker Scroll Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every game on a court can be reached on the SELECT GAME screen, whatever game the picker opens on.

**Architecture:** One pure function decides the list's window start (the position of the top visible row).
The picker's opening handler stores that clamped start, and the list view builds its rows from the same clamped
value. The rows, the arrows and the stored scroll position then always agree. `make_scroll_list` already clamps its
arrow math with the identical formula, so it is not touched.

**Tech Stack:** Rust 2024, iced 0.13, `refbox` bin crate.

**Spec:** memory `project_game_picker_cannot_reach_games_above_start.md` (diagnosis), plus the PO ruling of
2026-11-27 (fix for v0.5.6, branch `fix/refbox/game-picker-scroll`).

## Global Constraints

- MSRV 1.85: no let-chains or newer std APIs (`just check` will NOT catch this; the toolchain is pinned newer).
- Clippy `-D warnings`; no new `unwrap`/`expect`; no new dependencies.
- Only `refbox/src/app/view_builders/list_selector.rs` and `refbox/src/app/mod.rs` change.
- `make_scroll_list` (shared with penalties/warnings/fouls/portal detail/remote pairing) is NOT modified.
- refbox is a BIN crate: `cargo test -p refbox --bin refbox <filter>`. Every cargo run: `CARGO_BUILD_JOBS=4
  RUST_TEST_THREADS=4`, and only after the coordinator's heavy GO.

## Review Focus

1. A court with 2, 3 or 4 games opened on its last game: all games visible, starting from the first one.
2. A court with 5+ games opened on one of its last 3 games: 4 rows filled (no blank rows), the up arrow
   live, and the selected game in view.
3. A court with 5+ games opened on an early game: unchanged (opens with that game at the top).
4. Selected game not on the court (or no game selected): opens at the top, as today.
5. Pressing up straight after opening near the end moves the rows on the FIRST press. A stored start bigger than
   the clamp would make the first presses do nothing; storing the clamped start prevents that.

---

### Task 1: One window-start rule, used by the opener and the list view

**Files:**
- Modify: `refbox/src/app/view_builders/list_selector.rs` (`const LIST_LEN` at :28, `make_buttons!` at :55-87 and
  its three call sites :98/:117/:135, the `make_scroll_list` call at :139)
- Modify: `refbox/src/app/mod.rs` (`Message::SelectParameter` arm, ~:6878-6903)
- Test: `#[cfg(test)] mod tests` already at the foot of `list_selector.rs`

**Interfaces:**
- Produces, in `list_selector.rs` (re-exported to `app` by `view_builders/mod.rs`'s `pub(super) use list_selector::*`):
  - `pub(in super::super) const PARAMETER_LIST_LEN: usize = 4;` replaces the local `LIST_LEN` (keep the local name
    as `const LIST_LEN: usize = PARAMETER_LIST_LEN;` if that keeps the diff small).
  - `pub(in super::super) fn list_window_start(position: usize, num_items: usize) -> usize`, which returns
    `position.min(num_items.saturating_sub(PARAMETER_LIST_LEN))`. Give it a doc comment naming the bug: a
    start past the last full window strands every item above it when the list fits on screen.

- [ ] **Step 1: Write the failing tests** in `list_selector.rs`'s test module:

```rust
#[test]
fn a_list_that_fits_always_starts_at_the_top() {
    // 2, 3 and 4 games, opened on the last one: the bug stranded the rest
    assert_eq!(list_window_start(1, 2), 0);
    assert_eq!(list_window_start(2, 3), 0);
    assert_eq!(list_window_start(3, 4), 0);
    assert_eq!(list_window_start(2, 4), 0); // the PO's case: court 1/3/6/7, opened on 6
}

#[test]
fn a_longer_list_opened_near_its_end_shows_a_full_last_window() {
    assert_eq!(list_window_start(4, 5), 1);
    assert_eq!(list_window_start(2, 5), 1);
    assert_eq!(list_window_start(5, 6), 2);
}

#[test]
fn a_longer_list_opened_early_keeps_its_position() {
    assert_eq!(list_window_start(0, 5), 0);
    assert_eq!(list_window_start(1, 5), 1);
    assert_eq!(list_window_start(3, 30), 3);
}

#[test]
fn an_empty_list_starts_at_the_top() {
    assert_eq!(list_window_start(0, 0), 0);
    assert_eq!(list_window_start(3, 0), 0);
}
```

- [ ] **Step 2: Run, expect compile failure** (`list_window_start` not found):
  `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 cargo test -p refbox --bin refbox list_selector`

- [ ] **Step 3: Implement `list_window_start` and wire it in**
  - View: in each of the three `match param` arms, compute `let start = list_window_start(index, num_items);`
    before building the rows, and pass `start` into `make_buttons!` as a third macro argument (`$start:expr`) used
    by `.skip($start)`. It must be an argument because of macro hygiene: a shadowed `index` at the call site would
    NOT reach the macro body. Return `start` out of the match alongside `num_items` and pass it to
    `make_scroll_list` in place of `index`.
  - Opener: in `Message::SelectParameter`, the `ListableParameter::Game` arm must count that court's games as
    well as find the position, and yield `list_window_start(position.unwrap_or(0), count)`. The Event and Court
    arms stay `0`. Keep the existing iteration (`schedule.games.iter().filter(court)`) so the order matches the
    view's `schedule.games.values().filter(court)`.

- [ ] **Step 4: Run the tests, expect PASS**, same command as Step 2.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/app/view_builders/list_selector.rs refbox/src/app/mod.rs
git commit -m "fix(refbox): let SELECT GAME reach every game on a short court"
```

### Task 2: Gates and walkthrough prep (main session)

- [ ] `just check` (heavy GO, `RUST_TEST_THREADS=1`), plus `cargo +1.85` is NOT available, so read the diff for
  let-chains/new APIs by eye.
- [ ] Find a schedule with a court of ≤4 games for the PO's walk: the court-finished mock portal
  (`docs/backlog/court-finished-panel-state/mock-portal/`, untracked in the main checkout, court 1 = 1/3/6/7).
  Walk steps: open on game 6 → game 1 and 3 visible at the top, all four rows filled; open on game 7 → same;
  pick a 30-game court opened on its last game → last 4 games shown, up arrow works on the first press.

## Deviations

(none yet)
