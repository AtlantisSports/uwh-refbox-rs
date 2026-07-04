# Apply-button Rollout (Done → Apply) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rename the green "Done" button to "Apply" and gray it out until a real change is made, on six commit-style refbox pages (Parameter editor, Game-number editor, Score EDIT, and the Penalties/Warnings/Fouls overviews).

**Architecture:** Pure `refbox` UI change. The green button keeps its `green_button` style and is grayed by withholding `on_press` (`on_press_maybe`) — the exact mechanism the time editor (PR #1218) and the config footer already use. "Has changes" is determined per page: value editors compare the current edit buffer against the pre-edit value re-derived at render time (no new state); list pages scan the `FormatHint` each row already carries. No `RefBoxApp` struct fields and no `AppState` enum changes.

**Tech Stack:** Rust 2024, `iced` 0.13, Fluent translations (`fl!`).

## Global Constraints

- **MSRV:** Rust 1.85 — no APIs newer than 1.85.
- **Edition:** Rust 2024.
- **Clippy:** zero warnings under `cargo clippy -p refbox -- -D warnings`.
- **No `unwrap()`/`expect()`** in new non-test code without a justifying comment.
- **Reuse the existing `apply` Fluent key** (`apply = APPLY`, already in all 15 locales). NO new key, NO English placeholders, NO `.ftl` edits.
- **No new dependencies.** No `uwh-common` / wire-format / state-machine / embedded changes.
- **`refbox` is a bin crate:** test with `cargo test -p refbox` (no `--lib`); lint with `cargo clippy -p refbox -- -D warnings` (no `--all-targets`, mirrors `just lint`). Local `--all-targets` shows ~90 pre-existing test-code lints that are NOT failures.
- **Greying mechanism:** keep `.style(green_button)`; gate the press with `.on_press_maybe(<has_changes>.then_some(Message::…Complete { canceled: false }))`. A `green_button` with no `on_press` renders grayed.
- **Process:** lean (one code review at the end; deviations tracked in this plan's Deviations section; no per-task deviation commits).
- **Docs:** this plan and its spec stay **local** — never committed to the feature branch/PR.
- **Branch:** `feat/refbox/apply-button-rollout`, cut off fresh `origin/master`.

**Reference precedent (read once):** PR #1218 `refbox/src/app/view_builders/time_edit.rs` — `time_edit_has_changes` helper + its `#[cfg(test)] mod tests`. Mirror that helper-and-test shape.

**Out of scope (do not touch):** `score_add`, `foul_add`, `warning_add`, `penalty_edit`, `portal_login` (keep "Done"); the config sub-pages (already Apply); `time_edit.rs` (PR #1218); the parameter-editor open-from-`config.game`/apply-to-`edited_settings.config` quirk.

---

## File Structure

| File | Responsibility | Tasks |
|------|----------------|-------|
| `refbox/src/app/view_builders/configuration.rs` | Parameter editor: `param_edit_has_changes` helper + button + match-fold | 1 |
| `refbox/src/app/view_builders/score_edit.rs` | Score EDIT: `score_edit_has_changes` helper + confirmation-aware button | 2 |
| `refbox/src/app/view_builders/keypad_pages/game_number_edit.rs` | Game-number editor: `game_number_has_changes` helper + button | 3 |
| `refbox/src/app/view_builders/keypad_pages/mod.rs` | Thread one new arg through `build_keypad_page` to the GameNumber arm | 3 |
| `refbox/src/app/view_builders/shared_elements.rs` | `any_pending_change` helper (shared by the 3 list pages) + test | 4 |
| `refbox/src/app/view_builders/penalties.rs` | Penalties overview: rename + gate | 4 |
| `refbox/src/app/view_builders/warnings.rs` | Warnings overview: rename + gate | 4 |
| `refbox/src/app/view_builders/fouls.rs` | Fouls overview: rename + gate | 4 |
| `refbox/src/app/mod.rs` | `view()` dispatch lines pass the new "original" arg to pages 1–3 | 1, 2, 3 |

---

## Task 0: Branch setup

- [ ] **Step 1: Fetch and cut the branch off fresh master**

```bash
git fetch origin master
git switch -c feat/refbox/apply-button-rollout origin/master
```

Expected: new branch created at `origin/master`'s tip. (If using a worktree, create it here instead — see superpowers:using-git-worktrees.)

- [ ] **Step 2: Confirm clean baseline**

```bash
git status
cargo build -p refbox 2>&1 | tail -5
```

Expected: clean tree (this plan/spec live under `docs/superpowers/` and are untracked — leave them untracked); build succeeds.

---

## Task 1: Parameter editor → Apply + gray

**Files:**
- Modify: `refbox/src/app/view_builders/configuration.rs` — `build_game_parameter_editor` (~line 1267), new `param_edit_has_changes`, test module (~line 1592)
- Modify: `refbox/src/app/mod.rs` — `view()` dispatch for `AppState::ParameterEditor` (~line 3786)

**Interfaces:**
- Produces: `fn param_edit_has_changes(length: Duration, old: Duration) -> bool` (private to configuration.rs); `build_game_parameter_editor` gains a 4th parameter `game_config: &GameConfig`.
- Consumes: `GameConfig` (already imported as `uwh_common::config::Game as GameConfig`), the `apply` Fluent key.

- [ ] **Step 1: Write the failing test**

Add to the existing `#[cfg(test)] mod tests` in `configuration.rs`:

```rust
#[test]
fn param_edit_no_change_is_false() {
    let d = Duration::from_secs(900);
    assert!(!param_edit_has_changes(d, d));
}

#[test]
fn param_edit_second_change_is_true() {
    assert!(param_edit_has_changes(
        Duration::from_secs(901),
        Duration::from_secs(900)
    ));
}

#[test]
fn param_edit_sub_second_matches_displayed_whole_seconds() {
    // Original 900.6s displays as "15:00" (900 whole seconds); rebuilding to an
    // exact 900.0s lands on the same displayed value, so it is not a change.
    assert!(!param_edit_has_changes(
        Duration::from_secs(900),
        Duration::from_millis(900_600)
    ));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p refbox param_edit_ 2>&1 | tail -20`
Expected: FAIL — `cannot find function param_edit_has_changes`.

- [ ] **Step 3: Add the helper**

Add near the top of `configuration.rs` (e.g. just above `build_game_parameter_editor`):

```rust
/// Returns true when the edited length differs from the value shown when the
/// parameter editor was opened. Comparison is on whole seconds — the precision
/// the mm:ss editor displays — so zeroing and rebuilding to the same displayed
/// value counts as "no change". Mirrors `time_edit_has_changes` from PR #1218.
fn param_edit_has_changes(length: Duration, old: Duration) -> bool {
    length.as_secs() != old.as_secs()
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p refbox param_edit_ 2>&1 | tail -20`
Expected: PASS (3 tests).

- [ ] **Step 5: Fold the original-length lookup into the title/hint match**

In `build_game_parameter_editor`, change the signature to add `game_config`:

```rust
pub(in super::super) fn build_game_parameter_editor<'a>(
    data: ViewData<'_, '_>,
    param: LengthParameter,
    length: Duration,
    game_config: &GameConfig,
) -> Element<'a, Message> {
```

Replace the existing `let (title, hint) = match param { … };` block with one that also yields the original length:

```rust
    let (title, hint, old_length) = match param {
        LengthParameter::Half => (
            fl!("half-length"),
            fl!("length-of-half-during-regular-play"),
            game_config.half_play_duration,
        ),
        LengthParameter::HalfTime => (
            fl!("half-time-lenght"),
            fl!("length-of-half-time-period"),
            game_config.half_time_duration,
        ),
        LengthParameter::NominalBetweenGame => (
            fl!("nom-break"),
            fl!("system-will-keep-game-times-spaced"),
            game_config.nominal_break,
        ),
        LengthParameter::MinimumBetweenGame => (
            fl!("min-break"),
            fl!("min-time-btwn-games"),
            game_config.minimum_break,
        ),
        LengthParameter::PreOvertime => (
            fl!("pre-ot-break-abreviated"),
            fl!("pre-sd-brk"),
            game_config.pre_overtime_break,
        ),
        LengthParameter::OvertimeHalf => (
            fl!("ot-half-len"),
            fl!("time-during-ot"),
            game_config.ot_half_play_duration,
        ),
        LengthParameter::OvertimeHalfTime => (
            fl!("ot-half-tm-len"),
            fl!("len-of-overtime-halftime"),
            game_config.ot_half_time_duration,
        ),
        LengthParameter::PreSuddenDeath => (
            fl!("pre-sd-break"),
            fl!("pre-sd-len"),
            game_config.pre_sudden_death_duration,
        ),
    };
```

- [ ] **Step 6: Rename + gate the green button**

In the same function, replace the `make_button(fl!("done"))` block:

```rust
            make_button(fl!("apply"))
                .style(green_button)
                .width(Length::Fill)
                .on_press_maybe(
                    param_edit_has_changes(length, old_length)
                        .then_some(Message::ParameterEditComplete { canceled: false }),
                ),
```

- [ ] **Step 7: Pass the game config at the dispatch site**

In `refbox/src/app/mod.rs`, update the `AppState::ParameterEditor` arm of `view()`:

```rust
            AppState::ParameterEditor(param, dur) =>
                build_game_parameter_editor(data, param, dur, &self.config.game),
```

- [ ] **Step 8: Build + lint**

Run: `cargo build -p refbox 2>&1 | tail -5 && cargo clippy -p refbox -- -D warnings 2>&1 | tail -5`
Expected: builds clean, zero clippy warnings.

- [ ] **Step 9: Commit**

```bash
git add refbox/src/app/view_builders/configuration.rs refbox/src/app/mod.rs
git commit -m "feat(refbox): apply-button rename + gating on parameter editor"
```

---

## Task 2: Score EDIT → Apply + gray (edit mode only)

**Files:**
- Modify: `refbox/src/app/view_builders/score_edit.rs` — `build_score_edit_view`, new `score_edit_has_changes`, new test module
- Modify: `refbox/src/app/mod.rs` — `view()` dispatch for `AppState::ScoreEdit` (~line 3753)

**Interfaces:**
- Produces: `fn score_edit_has_changes(scores: BlackWhiteBundle<u8>, old: BlackWhiteBundle<u8>) -> bool` (private to score_edit.rs); `build_score_edit_view` gains a 5th parameter `old_scores: BlackWhiteBundle<u8>`.
- Consumes: `BlackWhiteBundle<u8>` (already in scope via `super::*`), the `apply` Fluent key, `self.snapshot.scores`.

- [ ] **Step 1: Write the failing test**

Add a new test module at the bottom of `score_edit.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_no_change_is_false() {
        let s = BlackWhiteBundle { black: 3, white: 5 };
        assert!(!score_edit_has_changes(s, s));
    }

    #[test]
    fn score_change_is_true() {
        assert!(score_edit_has_changes(
            BlackWhiteBundle { black: 4, white: 5 },
            BlackWhiteBundle { black: 3, white: 5 }
        ));
    }

    #[test]
    fn score_round_trip_is_false() {
        // +1 then -1 returns to the original bundle.
        let original = BlackWhiteBundle { black: 2, white: 2 };
        let after = BlackWhiteBundle {
            black: original.black + 1 - 1,
            white: original.white,
        };
        assert!(!score_edit_has_changes(after, original));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p refbox score_ 2>&1 | tail -20`
Expected: FAIL — `cannot find function score_edit_has_changes`.

- [ ] **Step 3: Add the helper**

Add above `build_score_edit_view` in `score_edit.rs`:

```rust
/// Returns true when the edited scores differ from the scores shown when the
/// Score EDIT screen was opened (the tournament-manager scores, which are not
/// changed until Apply). Only consulted in edit mode — never in confirmation.
fn score_edit_has_changes(scores: BlackWhiteBundle<u8>, old: BlackWhiteBundle<u8>) -> bool {
    scores != old
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p refbox score_ 2>&1 | tail -20`
Expected: PASS (3 tests).

- [ ] **Step 5: Add the `old_scores` parameter**

Change the `build_score_edit_view` signature:

```rust
pub(in super::super) fn build_score_edit_view<'a>(
    data: ViewData<'_, '_>,
    scores: BlackWhiteBundle<u8>,
    is_confirmation: bool,
    confirmation_time: Option<u32>,
    old_scores: BlackWhiteBundle<u8>,
) -> Element<'a, Message> {
```

- [ ] **Step 6: Make the green button confirmation-aware**

Mirror the config language-page pattern: bind the confirm button to a `let` before the
footer row. Add this just before the final `main_col.push(...)` chain that builds the footer
(both arms return the same `Button` type, so no boxing is needed):

```rust
    let confirm_btn = if is_confirmation {
        make_button(fl!("done"))
            .style(green_button)
            .on_press(Message::ScoreEditComplete { canceled: false })
    } else {
        make_button(fl!("apply"))
            .style(green_button)
            .on_press_maybe(
                score_edit_has_changes(scores, old_scores)
                    .then_some(Message::ScoreEditComplete { canceled: false }),
            )
    };
```

Then, in the footer `row![...]`, replace the existing `make_button(fl!("done"))…` element with
`confirm_btn`:

```rust
            row![
                make_button(fl!("cancel"))
                    .on_press_maybe(cancel_btn_msg)
                    .style(red_button),
                horizontal_space(),
                confirm_btn,
            ]
            .spacing(SPACING),
```

(Confirmation mode keeps "Done" and stays always-clickable; edit mode gets "Apply" + gating.)

- [ ] **Step 7: Pass the original scores at the dispatch site**

In `refbox/src/app/mod.rs`, `AppState::ScoreEdit` arm of `view()`:

```rust
                build_score_edit_view(
                    data,
                    scores,
                    is_confirmation,
                    self.snapshot.conf_pause_time,
                    self.snapshot.scores,
                ),
```

- [ ] **Step 8: Build + lint**

Run: `cargo build -p refbox 2>&1 | tail -5 && cargo clippy -p refbox -- -D warnings 2>&1 | tail -5`
Expected: builds clean, zero clippy warnings.

- [ ] **Step 9: Commit**

```bash
git add refbox/src/app/view_builders/score_edit.rs refbox/src/app/mod.rs
git commit -m "feat(refbox): apply-button rename + gating on score edit (edit mode only)"
```

---

## Task 3: Game-number editor → Apply + gray

**Files:**
- Modify: `refbox/src/app/view_builders/keypad_pages/game_number_edit.rs` — `make_game_number_edit_page`, new `game_number_has_changes`, new test module
- Modify: `refbox/src/app/view_builders/keypad_pages/mod.rs` — `build_keypad_page` signature + GameNumber arm (~lines 38–43, 205)
- Modify: `refbox/src/app/mod.rs` — `view()` dispatch for `AppState::KeypadPage` (~line 3769)

**Interfaces:**
- Produces: `fn game_number_has_changes(value: u32, original: Option<&str>) -> bool` (private to game_number_edit.rs); `make_game_number_edit_page(value: u32, original: Option<String>)`; `build_keypad_page` gains a trailing parameter `original_game_number: Option<String>`.
- Consumes: `self.edited_settings.as_ref().map(|e| e.game_number.clone())` (a `GameNumber`, i.e. `String`); the `apply` Fluent key.

- [ ] **Step 1: Write the failing test**

Add a new test module at the bottom of `game_number_edit.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_number_is_no_change() {
        assert!(!game_number_has_changes(5, Some("5")));
        assert!(!game_number_has_changes(12, Some("12")));
    }

    #[test]
    fn different_number_is_change() {
        assert!(game_number_has_changes(5, Some("6")));
    }

    #[test]
    fn missing_original_enables_apply() {
        // Defensive: the GameNumber keypad is only reached with edited settings
        // present, but if the original is unknown, don't block committing.
        assert!(game_number_has_changes(5, None));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p refbox game_number_has_changes 2>&1 | tail -20`
Expected: FAIL — `cannot find function game_number_has_changes`.

- [ ] **Step 3: Add the helper + update `make_game_number_edit_page`**

Rewrite `game_number_edit.rs` body (keeping the existing `use` lines, adding `horizontal_space` is not needed — current layout has no horizontal space):

```rust
use super::*;
use iced::{
    Length,
    widget::{column, row, vertical_space},
};

pub(super) fn make_game_number_edit_page<'a>(
    value: u32,
    original: Option<String>,
) -> Element<'a, Message> {
    column![
        vertical_space(),
        row![
            make_button(fl!("cancel"))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::ParameterEditComplete { canceled: true }),
            make_button(fl!("apply"))
                .style(green_button)
                .width(Length::Fill)
                .on_press_maybe(
                    game_number_has_changes(value, original.as_deref())
                        .then_some(Message::ParameterEditComplete { canceled: false }),
                ),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .into()
}

/// Returns true when the typed game number differs from the one stored when the
/// editor opened — i.e. when pressing Apply would actually change the stored
/// value. `value.to_string()` is exactly what `ParameterEditComplete` writes.
fn game_number_has_changes(value: u32, original: Option<&str>) -> bool {
    match original {
        Some(o) => value.to_string() != o,
        None => true,
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p refbox game_number_has_changes 2>&1 | tail -20`
Expected: PASS (3 tests).

- [ ] **Step 5: Thread the original through `build_keypad_page`**

In `keypad_pages/mod.rs`, add a trailing parameter to `build_keypad_page`:

```rust
pub(in super::super) fn build_keypad_page<'a>(
    data: ViewData<'_, '_>,
    page: KeypadPage,
    player_num: u32,
    track_fouls_and_warnings: bool,
    original_game_number: Option<String>,
) -> Element<'a, Message> {
```

Update the GameNumber arm (~line 205):

```rust
                KeypadPage::GameNumber =>
                    make_game_number_edit_page(player_num, original_game_number),
```

- [ ] **Step 6: Pass the original at the dispatch site**

In `refbox/src/app/mod.rs`, `AppState::KeypadPage` arm of `view()`:

```rust
            AppState::KeypadPage(page, player_num) => build_keypad_page(
                data,
                page,
                player_num,
                self.config.track_fouls_and_warnings,
                self.edited_settings.as_ref().map(|e| e.game_number.clone()),
            ),
```

- [ ] **Step 7: Build + lint**

Run: `cargo build -p refbox 2>&1 | tail -5 && cargo clippy -p refbox -- -D warnings 2>&1 | tail -5`
Expected: builds clean, zero clippy warnings.

- [ ] **Step 8: Commit**

```bash
git add refbox/src/app/view_builders/keypad_pages/game_number_edit.rs refbox/src/app/view_builders/keypad_pages/mod.rs refbox/src/app/mod.rs
git commit -m "feat(refbox): apply-button rename + gating on game-number editor"
```

---

## Task 4: List overview pages (Penalties / Warnings / Fouls) → Apply + gray

**Files:**
- Modify: `refbox/src/app/view_builders/shared_elements.rs` — new `any_pending_change` helper + test in existing `#[cfg(test)] mod tests`
- Modify: `refbox/src/app/view_builders/penalties.rs` — `build_penalty_overview_page`
- Modify: `refbox/src/app/view_builders/warnings.rs` — `build_warning_overview_page`
- Modify: `refbox/src/app/view_builders/fouls.rs` — `build_foul_overview_page`

**Interfaces:**
- Produces: `pub(super) fn any_pending_change(hints: impl IntoIterator<Item = FormatHint>) -> bool`.
- Consumes: `FormatHint` (in scope via `super::*`), the per-row `format_hint: FormatHint` field (Copy), the `apply` Fluent key.

- [ ] **Step 1: Write the failing test**

Add to the existing `#[cfg(test)] mod tests` in `shared_elements.rs`:

```rust
#[test]
fn any_pending_change_detects_edits() {
    let empty: [FormatHint; 0] = [];
    assert!(!any_pending_change(empty));
    assert!(!any_pending_change([FormatHint::NoChange, FormatHint::NoChange]));
    assert!(any_pending_change([FormatHint::NoChange, FormatHint::Edited]));
    assert!(any_pending_change([FormatHint::New]));
    assert!(any_pending_change([FormatHint::Deleted]));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p refbox any_pending_change 2>&1 | tail -20`
Expected: FAIL — `cannot find function any_pending_change`.

- [ ] **Step 3: Add the helper**

Add to `shared_elements.rs` (e.g. just below `inf_short_name`, before the test module):

```rust
/// Returns true when any of the given format hints represents a pending change
/// (anything other than `NoChange`). Used to gray the Apply button on the
/// penalty / warning / foul overview pages until a row is added, edited, or
/// deleted.
pub(super) fn any_pending_change(hints: impl IntoIterator<Item = FormatHint>) -> bool {
    hints.into_iter().any(|h| h != FormatHint::NoChange)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p refbox any_pending_change 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 5: Penalties — compute changes, rename + gate**

In `penalties.rs` `build_penalty_overview_page`, add this just before the `column![` (while `penalties` is still borrowable):

```rust
    let has_changes = any_pending_change(
        penalties
            .black
            .iter()
            .chain(penalties.white.iter())
            .map(|p| p.format_hint),
    );
```

Then replace the `make_button(fl!("done"))` block in the footer row:

```rust
            make_button(fl!("apply"))
                .style(green_button)
                .width(Length::Fill)
                .on_press_maybe(
                    has_changes.then_some(Message::PenaltyOverviewComplete { canceled: false }),
                ),
```

- [ ] **Step 6: Warnings — compute changes, rename + gate**

In `warnings.rs` `build_warning_overview_page`, add just before the `column![` (the per-color lists `.rev().collect()` consume the vecs inside the macro, so compute first):

```rust
    let has_changes = any_pending_change(
        warnings
            .black
            .iter()
            .chain(warnings.white.iter())
            .map(|w| w.format_hint),
    );
```

Replace the `make_button(fl!("done"))` block:

```rust
            make_button(fl!("apply"))
                .style(green_button)
                .width(Length::Fill)
                .on_press_maybe(
                    has_changes.then_some(Message::WarningOverviewComplete { canceled: false }),
                ),
```

- [ ] **Step 7: Fouls — compute changes (3 lists), rename + gate**

In `fouls.rs` `build_foul_overview_page`, add just before the `column![`:

```rust
    let has_changes = any_pending_change(
        warnings
            .black
            .iter()
            .chain(warnings.equal.iter())
            .chain(warnings.white.iter())
            .map(|w| w.format_hint),
    );
```

Replace the `make_button(fl!("done"))` block:

```rust
            make_button(fl!("apply"))
                .style(green_button)
                .width(Length::Fill)
                .on_press_maybe(
                    has_changes.then_some(Message::FoulOverviewComplete { canceled: false }),
                ),
```

- [ ] **Step 8: Build + test + lint**

Run: `cargo build -p refbox 2>&1 | tail -5 && cargo test -p refbox 2>&1 | tail -10 && cargo clippy -p refbox -- -D warnings 2>&1 | tail -5`
Expected: builds clean, all tests pass, zero clippy warnings.

- [ ] **Step 9: Commit**

```bash
git add refbox/src/app/view_builders/shared_elements.rs refbox/src/app/view_builders/penalties.rs refbox/src/app/view_builders/warnings.rs refbox/src/app/view_builders/fouls.rs
git commit -m "feat(refbox): apply-button rename + gating on penalty/warning/foul overviews"
```

---

## Task 5: Full verification + walkthrough

- [ ] **Step 1: Full check**

Run: `just check`
Expected: fmt, clippy, tests, audit all clean.

- [ ] **Step 2: Rebuild the real binary before the walkthrough**

```bash
cargo build -p refbox
```

(`just check` builds a test binary, NOT `target/debug/refbox` — rebuild so the walkthrough exercises current code.)

- [ ] **Step 3: Launch and walk through each page**

Launch the built binary in the background with `WAYLAND_DISPLAY=` (X11) and sandbox disabled, then verify against the spec's acceptance criteria:

1. **Parameter editor** (Config → Game → tap a length): button reads "APPLY", grayed on open; edit the time → active; revert to original → grayed again.
2. **Game-number editor** (Config → Game → game number): "APPLY" grayed on open; type a different number → active; retype the original → grayed.
3. **Score EDIT** (main → EDIT scores): "APPLY" grayed on open; +1 → active; −1 back to original → grayed. Then trigger an **end-of-game final-score confirmation** and confirm the button still reads **"DONE"** and is always clickable.
4. **Penalties / Warnings / Fouls overviews**: "APPLY" grayed with no pending changes; add/edit/delete a row → active; the Cancel and blue "New" buttons behave as before.

- [ ] **Step 4: Code review**

Invoke `superpowers:requesting-code-review` on the full branch diff (lean process — one review at the end).

- [ ] **Step 5: Hand off to the user for PR**

Do not open the PR until the user approves (approval gate). Prepare the plain-language PR body (What changed / Why / Scope / How to verify) for their review.

---

## Deviations

- **Task 1 (Parameter editor):** the worktree's `origin/master` was ahead of the files the plan
  was written against. The editor already received `config: &GameConfig` and already *disabled*
  the green button when a Game Block is too short, and it now also commits a 2 Halves / 1 Period
  (`single_half`) toggle for the Half parameter. Consequences vs. the plan:
  - **No new arg and no dispatch change** were needed — the dispatch already passes the correct
    seed source (`edited_settings.config` ?? `self.config.game`).
  - `param_edit_has_changes` gained two params (`param`, `single_half` + `old_single_half`) so that
    flipping 2 Halves / 1 Period on the Half editor counts as a change (Apply commits it). For all
    other parameters `single_half` is ignored.
  - The new "has changes" gate is **AND-ed** with the existing "not too short" gate:
    `apply_enabled = !too_short && param_edit_has_changes(...)`.
  - The help sub-page (`build_parameter_help_page`) uses a "Back" button, not Done/Apply — left
    untouched, correctly out of scope.
- Tasks 2–4 matched the plan (only `make_game_time_button` gained a trailing `None` arg, which the
  surrounding code already passes). All helper unit tests added as specified.
- Verified: `just check` green (fmt, lint, 294 refbox tests + workspace, audit allowed-warnings only).
