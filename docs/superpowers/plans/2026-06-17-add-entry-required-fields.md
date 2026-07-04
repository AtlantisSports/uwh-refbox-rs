# Add/Edit Entry Required-Fields Gate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Disable the green "Done" button on the foul / warning / penalty add-edit screens until the required fields (player number, and infraction) are entered, so an incomplete entry can't be saved to the list.

**Architecture:** Each of the three screen builders gets the current keypad player number and disables its own "Done" via `on_press_maybe`, using a small local `*_can_commit` helper (mirrors the game-number-editor pattern). The keypad dispatcher `build_keypad_page` passes its existing `player_num` through to the three builders. No state/message/translation changes; the "Done" label is unchanged.

**Tech Stack:** Rust 2024, `iced` 0.13.

## Global Constraints

- **MSRV** Rust 1.85; **Edition** 2024.
- **Clippy** zero warnings under `cargo clippy -p refbox -- -D warnings`.
- No `unwrap()`/`expect()` in new non-test code without a justifying comment.
- No new translation key, no `.ftl` edits (the "Done" label stays "Done"); no new dependency; no `RefBoxApp`/`AppState`/message changes.
- Gated fields: **player number** (`> 0`; `0` = not entered) **and infraction** (`!matches!(infraction, Infraction::Unknown)`). Penalty kind and team colour never gate. **Cancel and Delete stay enabled.**
- Use `!matches!(infraction, Infraction::Unknown)` (no reliance on `Infraction: PartialEq`).
- Greying mechanism: keep `.style(green_button)`; gate with `.on_press_maybe(<can_commit>.then_some(Message::…EditComplete { … }))`. The Cancel/Delete buttons in the same `exit_row` are unchanged.
- `refbox` is a bin crate: test with `cargo test -p refbox` (no `--lib`); lint with `cargo clippy -p refbox -- -D warnings` (no `--all-targets`).
- Lean process: one code review at the end; deviations tracked in this file; no per-task deviation commits.
- Spec/plan stay local — not committed to the branch/PR.

**Per-screen criteria (the source of truth for the helpers):**

| Screen | `*_can_commit` returns true when |
|--------|----------------------------------|
| Foul | infraction set **and** (`color.is_none()` *(Equal)* **or** `player_num > 0`) |
| Warning | infraction set **and** (`team_warning` **or** `player_num > 0`) |
| Penalty | `player_num > 0` **and** (`!track_fouls_and_warnings` **or** infraction set) |

---

## File Structure

| File | Change | Task |
|------|--------|------|
| `refbox/src/app/view_builders/keypad_pages/foul_add.rs` | `foul_add_can_commit` + test; `player_num` param; gate Done | 1 |
| `refbox/src/app/view_builders/keypad_pages/warning_add.rs` | `warning_add_can_commit` + test; `player_num` param; gate Done | 2 |
| `refbox/src/app/view_builders/keypad_pages/penalty_edit.rs` | `penalty_edit_can_commit` + test; `player_num` param; gate Done | 3 |
| `refbox/src/app/view_builders/keypad_pages/mod.rs` | `build_keypad_page` passes `player_num` to all three builders (3 call sites) | 1, 2, 3 |

---

## Task 0: Branch setup (needs user OK before running)

This branch builds **on top of** `feat/refbox/apply-button-rollout` because both edit
`keypad_pages/mod.rs`; stacking avoids a merge collision.

- [ ] **Step 1: Create a worktree branched off the Apply-button branch**

```bash
cd /home/estraily/projects/uwh-refbox-rs
git worktree add -b feat/refbox/add-entry-required-fields .worktrees/add-entry-required-fields feat/refbox/apply-button-rollout
```

Expected: new worktree at `.worktrees/add-entry-required-fields`, branch
`feat/refbox/add-entry-required-fields` at the Apply-button branch tip.

- [ ] **Step 2: Confirm baseline builds**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/add-entry-required-fields
cargo build -p refbox 2>&1 | tail -3
```

Expected: builds. (All paths below are relative to this worktree.)

---

## Task 1: Foul screen — require infraction (+ player number unless Equal)

**Files:**
- Modify: `refbox/src/app/view_builders/keypad_pages/foul_add.rs` — add `foul_add_can_commit`, `player_num` param, gate Done, add test module
- Modify: `refbox/src/app/view_builders/keypad_pages/mod.rs` — pass `player_num` at the `FoulAdd` call site (~line 238)

**Interfaces:**
- Produces: `fn foul_add_can_commit(infraction: Infraction, color: Option<GameColor>, player_num: u32) -> bool`; `make_foul_add_page` gains a trailing `player_num: u32` parameter.
- Consumes: `Infraction` and `GameColor` (already in scope in this file), the keypad `player_num`.

- [ ] **Step 1: Write the failing test** — append to `foul_add.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foul_needs_infraction() {
        // Equal foul, infraction unset → blocked.
        assert!(!foul_add_can_commit(Infraction::Unknown, None, 0));
    }

    #[test]
    fn foul_equal_with_infraction_ok_without_number() {
        assert!(foul_add_can_commit(Infraction::StickInfringement, None, 0));
    }

    #[test]
    fn foul_individual_needs_number() {
        assert!(!foul_add_can_commit(
            Infraction::StickInfringement,
            Some(GameColor::Black),
            0
        ));
        assert!(foul_add_can_commit(
            Infraction::StickInfringement,
            Some(GameColor::Black),
            5
        ));
    }

    #[test]
    fn foul_individual_with_number_still_needs_infraction() {
        assert!(!foul_add_can_commit(
            Infraction::Unknown,
            Some(GameColor::Black),
            5
        ));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p refbox foul_ 2>&1 | tail -15`
Expected: FAIL — `cannot find function foul_add_can_commit`.

- [ ] **Step 3: Add the helper + `player_num` param + gate the Done button**

Add the helper near the top of `foul_add.rs` (below the imports/`type StyleFn` line):

```rust
/// Returns true when the foul entry has everything it needs to be saved: an
/// infraction must always be selected, and an individual foul (Black/White)
/// also needs a player number. An "equal" foul (`color == None`) has no player,
/// so it needs only the infraction.
fn foul_add_can_commit(infraction: Infraction, color: Option<GameColor>, player_num: u32) -> bool {
    !matches!(infraction, Infraction::Unknown) && (color.is_none() || player_num > 0)
}
```

Change the signature to add `player_num`:

```rust
pub(super) fn make_foul_add_page<'a>(
    origin: Option<(Option<GameColor>, usize)>,
    color: Option<GameColor>,
    foul: Infraction,
    ret_to_overview: bool,
    player_num: u32,
) -> Element<'a, Message> {
```

Replace the Done-button push:

```rust
    exit_row = exit_row.push(
        make_button(fl!("done"))
            .style(green_button)
            .width(Length::Fill)
            .on_press_maybe(foul_add_can_commit(foul, color, player_num).then_some(
                Message::FoulEditComplete {
                    canceled: false,
                    deleted: false,
                    ret_to_overview,
                },
            )),
    );
```

- [ ] **Step 4: Pass `player_num` at the call site** — in `keypad_pages/mod.rs`, the `FoulAdd` arm:

```rust
                } => make_foul_add_page(origin, color, infraction, ret_to_overview, player_num),
```

- [ ] **Step 5: Run test + build + lint**

Run: `cargo test -p refbox foul_ 2>&1 | tail -15 && cargo build -p refbox 2>&1 | tail -3 && cargo clippy -p refbox -- -D warnings 2>&1 | tail -3`
Expected: tests PASS (4), builds clean, zero clippy warnings.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/app/view_builders/keypad_pages/foul_add.rs refbox/src/app/view_builders/keypad_pages/mod.rs
git commit -m "feat(refbox): require infraction (and player number) before saving a foul"
```

---

## Task 2: Warning screen — require infraction (+ player number unless TEAM)

**Files:**
- Modify: `refbox/src/app/view_builders/keypad_pages/warning_add.rs` — add `warning_add_can_commit`, `player_num` param, gate Done, add test module
- Modify: `refbox/src/app/view_builders/keypad_pages/mod.rs` — pass `player_num` at the `WarningAdd` call site (~line 246)

**Interfaces:**
- Produces: `fn warning_add_can_commit(infraction: Infraction, team_warning: bool, player_num: u32) -> bool`; `make_warning_add_page` gains a trailing `player_num: u32` parameter.
- Consumes: `Infraction` (in scope), the keypad `player_num`.

- [ ] **Step 1: Write the failing test** — append to `warning_add.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warning_needs_infraction() {
        assert!(!warning_add_can_commit(Infraction::Unknown, true, 0));
    }

    #[test]
    fn warning_team_with_infraction_ok_without_number() {
        assert!(warning_add_can_commit(Infraction::StickInfringement, true, 0));
    }

    #[test]
    fn warning_individual_needs_number() {
        assert!(!warning_add_can_commit(Infraction::StickInfringement, false, 0));
        assert!(warning_add_can_commit(Infraction::StickInfringement, false, 7));
    }

    #[test]
    fn warning_individual_with_number_still_needs_infraction() {
        assert!(!warning_add_can_commit(Infraction::Unknown, false, 7));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p refbox warning_ 2>&1 | tail -15`
Expected: FAIL — `cannot find function warning_add_can_commit`.

- [ ] **Step 3: Add the helper + `player_num` param + gate the Done button**

Add the helper near the top of `warning_add.rs` (below the `type StyleFn` line):

```rust
/// Returns true when the warning entry can be saved: an infraction must always
/// be selected, and an individual warning also needs a player number. A team
/// warning (`team_warning == true`) has no player, so it needs only the infraction.
fn warning_add_can_commit(infraction: Infraction, team_warning: bool, player_num: u32) -> bool {
    !matches!(infraction, Infraction::Unknown) && (team_warning || player_num > 0)
}
```

Change the signature to add `player_num`:

```rust
pub(super) fn make_warning_add_page<'a>(
    origin: Option<(GameColor, usize)>,
    color: GameColor,
    foul: Infraction,
    team_warning: bool,
    ret_to_overview: bool,
    player_num: u32,
) -> Element<'a, Message> {
```

Replace the Done-button push:

```rust
    exit_row = exit_row.push(
        make_button(fl!("done"))
            .style(green_button)
            .width(Length::Fill)
            .on_press_maybe(warning_add_can_commit(foul, team_warning, player_num).then_some(
                Message::WarningEditComplete {
                    canceled: false,
                    deleted: false,
                    ret_to_overview,
                },
            )),
    );
```

- [ ] **Step 4: Pass `player_num` at the call site** — in `keypad_pages/mod.rs`, the `WarningAdd` arm:

```rust
                    make_warning_add_page(origin, color, infraction, team_warning, ret_to_overview, player_num),
```

- [ ] **Step 5: Run test + build + lint**

Run: `cargo test -p refbox warning_ 2>&1 | tail -15 && cargo build -p refbox 2>&1 | tail -3 && cargo clippy -p refbox -- -D warnings 2>&1 | tail -3`
Expected: tests PASS (4), builds clean, zero clippy warnings.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/app/view_builders/keypad_pages/warning_add.rs refbox/src/app/view_builders/keypad_pages/mod.rs
git commit -m "feat(refbox): require infraction (and player number) before saving a warning"
```

---

## Task 3: Penalty screen — require player number (+ infraction when tracking is on)

**Files:**
- Modify: `refbox/src/app/view_builders/keypad_pages/penalty_edit.rs` — add `penalty_edit_can_commit`, `player_num` param, gate Done, add test module
- Modify: `refbox/src/app/view_builders/keypad_pages/mod.rs` — pass `player_num` at the `Penalty` call site (~line 219)

**Interfaces:**
- Produces: `fn penalty_edit_can_commit(infraction: Infraction, track_fouls_and_warnings: bool, player_num: u32) -> bool`; `make_penalty_edit_page` gains a trailing `player_num: u32` parameter.
- Consumes: `Infraction` (in scope), the existing `track_fouls_and_warnings` arg, the keypad `player_num`.

- [ ] **Step 1: Write the failing test** — append to `penalty_edit.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn penalty_needs_number() {
        // Tracking off: only a number is required.
        assert!(!penalty_edit_can_commit(Infraction::Unknown, false, 0));
        assert!(penalty_edit_can_commit(Infraction::Unknown, false, 5));
    }

    #[test]
    fn penalty_needs_infraction_when_tracking_on() {
        assert!(!penalty_edit_can_commit(Infraction::Unknown, true, 5));
        assert!(penalty_edit_can_commit(Infraction::StickInfringement, true, 5));
    }

    #[test]
    fn penalty_tracking_on_still_needs_number() {
        assert!(!penalty_edit_can_commit(Infraction::StickInfringement, true, 0));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p refbox penalty_ 2>&1 | tail -15`
Expected: FAIL — `cannot find function penalty_edit_can_commit`.

- [ ] **Step 3: Add the helper + `player_num` param + gate the Done button**

Add the helper near the top of `penalty_edit.rs` (below the `type StyleFn` line):

```rust
/// Returns true when the penalty entry can be saved: a player number is always
/// required (penalties are always individual). The infraction is required only
/// when "track fouls & warnings" is on — that is exactly when the infraction
/// picker is shown on this screen.
fn penalty_edit_can_commit(
    infraction: Infraction,
    track_fouls_and_warnings: bool,
    player_num: u32,
) -> bool {
    player_num > 0 && (!track_fouls_and_warnings || !matches!(infraction, Infraction::Unknown))
}
```

Change the signature to add `player_num`:

```rust
pub(super) fn make_penalty_edit_page<'a>(
    origin: Option<(GameColor, usize)>,
    color: GameColor,
    kind: PenaltyKind,
    mode: Mode,
    track_fouls_and_warnings: bool,
    infraction: Infraction,
    player_num: u32,
) -> Element<'a, Message> {
```

Replace the Done-button push:

```rust
    exit_row = exit_row.push(
        make_smaller_button(fl!("done"))
            .style(green_button)
            .width(Length::Fill)
            .on_press_maybe(
                penalty_edit_can_commit(infraction, track_fouls_and_warnings, player_num).then_some(
                    Message::PenaltyEditComplete {
                        canceled: false,
                        deleted: false,
                    },
                ),
            ),
    );
```

- [ ] **Step 4: Pass `player_num` at the call site** — in `keypad_pages/mod.rs`, the `Penalty` arm:

```rust
                KeypadPage::Penalty(origin, color, kind, foul) => {
                    make_penalty_edit_page(
                        origin,
                        color,
                        kind,
                        mode,
                        track_fouls_and_warnings,
                        foul,
                        player_num,
                    )
                }
```

- [ ] **Step 5: Run test + build + lint**

Run: `cargo test -p refbox penalty_ 2>&1 | tail -15 && cargo build -p refbox 2>&1 | tail -3 && cargo clippy -p refbox -- -D warnings 2>&1 | tail -3`
Expected: tests PASS (3), builds clean, zero clippy warnings.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/app/view_builders/keypad_pages/penalty_edit.rs refbox/src/app/view_builders/keypad_pages/mod.rs
git commit -m "feat(refbox): require player number (and infraction when tracked) before saving a penalty"
```

---

## Task 4: Full verification + walkthrough

- [ ] **Step 1: Full check** — Run: `just check` — Expected: fmt, clippy, tests, audit all clean (audit shows only the pre-existing `lru`/`macroquad` allowed advisories).
- [ ] **Step 2: Rebuild the real binary** — Run: `cargo build -p refbox` (a `just check` test binary is not `target/debug/refbox`).
- [ ] **Step 3: Walkthrough** — launch `target/debug/refbox` (background, `WAYLAND_DISPLAY=`, sandbox disabled) and verify:
  - **Foul**: New foul (opens "=") → Done greys until an infraction is picked; pick Black/White → greys until a number is entered; back to "=" → enabled with just the infraction.
  - **Warning**: New warning (Individual) → Done greys until both infraction and number; switch on TEAM → enabled with just the infraction.
  - **Penalty** (tracking ON): Done greys until both number and infraction. (tracking OFF): Done greys until just a number.
  - Cancel and Delete always work; editing an existing complete entry shows Done enabled.
- [ ] **Step 4: Code review** — run a review on the branch diff (lean process — one review at the end).
- [ ] **Step 5: Hand off for PR** — do not open the PR without the user's approval; recommend merging the Apply-button PR first, then rebasing this onto master.

---

## Deviations

_(Record any execution deviations here — do not make standalone deviation commits.)_
