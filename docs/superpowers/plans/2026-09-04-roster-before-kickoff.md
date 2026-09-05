# Roster Before Kickoff Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the player picker offer the roster of the game an entry would actually land on — including before the first kickoff of a session — and close the entry surfaces during the post-game window, where entries are silently discarded.

**Architecture:** Two small pure predicates, each unit-tested, each consumed at the point of use. `picker_roster_game` decides which game's roster the picker follows (upcoming game between games; the copy pinned at kickoff during play). `is_post_game` decides whether an entry surface is open at all. No state machine changes, no new fields, no new stored state.

**Tech Stack:** Rust 2024, iced 0.13, `refbox` crate only.

**Spec:** `docs/superpowers/specs/2026-09-04-roster-before-kickoff-design.md`

**Worktree:** `/home/estraily/projects/uwh-refbox-rs/.worktrees/fix-refbox-roster-before-kickoff`
**Branch:** `fix/refbox/roster-before-kickoff`, based on `origin/master` at `486c5692`.

## Global Constraints

- **MSRV Rust 1.85, edition 2024.** No APIs newer than 1.85.
- **`cargo clippy --workspace --all-targets --all-features -- -D warnings` must be clean.** No `#[allow(...)]` to silence anything.
- **`uwh-common` must not change.** It is read only. If a task appears to need a change there, stop and raise it — it would move the branch to `fix/uwh-common/...` and widen the review.
- **No new dependencies. No new translation keys.** Nothing gains new text; buttons only change state. A key added to `en-US` alone fails `just check` — all 15 locales or none.
- **`post_game_duration` stays at `Duration::from_secs(120)`.** Shortening it was considered and dropped on 2026-09-04.
- **No `unwrap()` or `expect()` in new non-test code.**
- Commit messages: `type(scope): description`, lowercase, imperative, no trailing period, ~72 chars. Scope is `refbox`.
- Run every command from the worktree path above. `cd` state resets between background/sandbox-off runs — prefer `git -C` and absolute paths.

---

### Task 1: The picker follows the game an entry would land on

**Files:**
- Modify: `refbox/src/app/mod.rs` — add `picker_roster_game` beside `rosters_for_game` (~line 1972); add `borrow::Cow` to the `use std::{...}` block at line 25; rewire the `AppState::KeypadPage` arm of `view()` (~line 7267).
- Test: `refbox/src/app/mod.rs` — a new `#[cfg(test)] mod` beside the existing ones.

**Interfaces:**
- Consumes: existing `RefBoxApp::rosters_for_game(&self, &GameNumber) -> BlackWhiteBundle<Vec<u8>>`, unchanged.
- Produces: `fn picker_roster_game(snapshot: &GameSnapshot) -> Option<&GameNumber>` — free function, module-private. `Some(game)` means "look this game's roster up live"; `None` means "use the copy pinned at kickoff".

**Context:** `GameNumber` is a type alias for `String`. `GameSnapshot` derives `Default`, whose `current_period` is `GamePeriod::BetweenGames` and whose `is_old_game` is `false` — the state the app is in before the first kickoff.

- [ ] **Step 1: Write the failing test**

Add at the end of `refbox/src/app/mod.rs`:

```rust
#[cfg(test)]
mod picker_roster_game_tests {
    use super::*;

    fn snap(period: GamePeriod, game: &str, next: &str) -> GameSnapshot {
        GameSnapshot {
            current_period: period,
            game_number: game.to_string(),
            next_game_number: next.to_string(),
            ..GameSnapshot::default()
        }
    }

    /// Before the first kickoff of a session the app sits in BetweenGames with no
    /// prior game, and the picker must already offer the first game's roster --
    /// the requirement this change exists for. Before the fix nothing is offered
    /// there at all and every picker falls through to the 0-9 pad.
    #[test]
    fn before_the_first_kickoff_follows_the_upcoming_game() {
        assert_eq!(
            picker_roster_game(&snap(GamePeriod::BetweenGames, "0", "27")),
            Some(&"27".to_string()),
        );
    }

    /// An entry made anywhere in the break belongs to the game about to start, so
    /// the picker follows that game from the whistle -- it must not switch partway
    /// through the break. `is_old_game` marks the engine's changeover partway in;
    /// it must make no difference here.
    #[test]
    fn the_whole_break_follows_the_upcoming_game() {
        for is_old_game in [true, false] {
            let mut s = snap(GamePeriod::BetweenGames, "27", "15");
            s.is_old_game = is_old_game;
            assert_eq!(
                picker_roster_game(&s),
                Some(&"15".to_string()),
                "is_old_game={is_old_game} must not change which game the picker follows",
            );
        }
    }

    /// During play the copy pinned at kickoff is used, so a mid-game REFRESH
    /// cannot move the grid under the operator's hand. This is the guarantee the
    /// original grid design was built on and it must survive this change.
    #[test]
    fn during_play_keeps_the_kickoff_copy() {
        for period in [
            GamePeriod::FirstHalf,
            GamePeriod::HalfTime,
            GamePeriod::SecondHalf,
            GamePeriod::PreOvertime,
            GamePeriod::OvertimeFirstHalf,
            GamePeriod::OvertimeHalfTime,
            GamePeriod::OvertimeSecondHalf,
            GamePeriod::PreSuddenDeath,
            GamePeriod::SuddenDeath,
        ] {
            assert_eq!(
                picker_roster_game(&snap(period, "27", "15")),
                None,
                "{period:?} must keep the roster pinned at kickoff",
            );
        }
    }
}
```

- [ ] **Step 2: Run the test and confirm it fails for the right reason**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/fix-refbox-roster-before-kickoff
cargo test -p refbox picker_roster_game_tests
```

Expected: compile error, `cannot find function 'picker_roster_game' in this scope`. If it fails any other way, stop and investigate before writing the implementation.

- [ ] **Step 3: Write the implementation**

In `refbox/src/app/mod.rs`, immediately after the `rosters_for_game` method's closing brace and *outside* the `impl RefBoxApp` block (it is a free function over a snapshot, not app state):

```rust
/// The game whose roster the player picker must offer, or `None` to use the copy
/// pinned at kickoff.
///
/// The picker must always offer the roster of the game an entry made *now* would
/// land on. Between games -- including before the first kickoff of a session,
/// which is where the app sits from launch until the first game begins -- that is
/// the game about to start, so the picker follows `next_game_number`. It is read
/// live rather than pinned, so a roster arriving mid-break appears instead of
/// being locked out until the next kickoff.
///
/// During play it is the running game, and the copy pinned at kickoff is used
/// instead of a fresh lookup. That is what stops a mid-game REFRESH moving numbers
/// under the operator's hand, and with it keeps the grid design's guarantee that a
/// number recorded during a game is present on that game's grid.
///
/// Deliberately **not** `GameSnapshot::game_number()`, which looks like the right
/// answer and is not: that helper names the *finished* game for the whole
/// post-game window (`BetweenGames && !is_old_game`), so using it here would put
/// the previous game's players on offer for the first two minutes of every break
/// -- the exact bug this change exists to fix.
fn picker_roster_game(snapshot: &GameSnapshot) -> Option<&GameNumber> {
    if snapshot.current_period == GamePeriod::BetweenGames {
        Some(&snapshot.next_game_number)
    } else {
        None
    }
}
```

- [ ] **Step 4: Run the test and confirm it passes**

```bash
cargo test -p refbox picker_roster_game_tests
```

Expected: 3 passed.

- [ ] **Step 5: Wire it into the view**

Add `borrow::Cow` to the `use std::{...}` block at `refbox/src/app/mod.rs:25`, keeping alphabetical order:

```rust
use std::{
    borrow::Cow,
    cmp::min,
    collections::{BTreeMap, BTreeSet},
```

Then replace the `AppState::KeypadPage` arm in `view()` (~line 7267). Before:

```rust
            AppState::KeypadPage(page, player_num) =>
                build_keypad_page(
                    data,
                    page,
                    player_num,
                    self.config.fouls_tracked(),
                    self.edited_settings.as_ref().map(|e| e.game_number.clone()),
                    &self.game_rosters,
                    self.config.keypad_numbers_forced(),
                ),
```

After:

```rust
            AppState::KeypadPage(page, player_num) => {
                // Between games the roster is read live so a roster arriving
                // mid-break appears; during play the kickoff copy is used as-is,
                // which is also why this is a Cow rather than an owned clone --
                // the mid-game path allocates nothing per frame.
                let rosters = match picker_roster_game(&self.snapshot) {
                    Some(game_num) => Cow::Owned(self.rosters_for_game(game_num)),
                    None => Cow::Borrowed(&self.game_rosters),
                };
                build_keypad_page(
                    data,
                    page,
                    player_num,
                    self.config.fouls_tracked(),
                    self.edited_settings.as_ref().map(|e| e.game_number.clone()),
                    &rosters,
                    self.config.keypad_numbers_forced(),
                )
            }
```

- [ ] **Step 6: Confirm it builds and nothing else broke**

```bash
cargo clippy -p refbox --all-targets -- -D warnings
cargo test -p refbox
```

Expected: clean, all tests pass. `BlackWhiteBundle` derives `Clone`, which is what `Cow::Owned` needs; if the compiler disagrees, stop rather than adding a derive to `uwh-common`.

- [ ] **Step 7: Commit**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/fix-refbox-roster-before-kickoff
git add refbox/src/app/mod.rs
git commit -m "fix(refbox): offer the upcoming game's roster between games"
```

---

### Task 2: The post-game predicate

**Files:**
- Modify: `refbox/src/app/view_builders/shared_elements.rs` — add `is_post_game` after `team_timeout_in_grace` (~line 196).
- Test: `refbox/src/app/view_builders/shared_elements.rs` — the existing `#[cfg(test)] mod tests` at ~line 1924.

**Interfaces:**
- Produces: `pub(in super::super) fn is_post_game(snapshot: &GameSnapshot) -> bool`. Scope mirrors `team_timeout_in_grace` directly above it, so it is visible throughout `app`. `view_builders/mod.rs:55` already does `pub(super) use shared_elements::*;`, so sibling view builders reach it through their existing `use super::*`.

**This is the highest-risk task in the plan.** Read the doc comment's second paragraph before implementing.

- [ ] **Step 1: Write the failing test**

Inside the existing `mod tests` in `shared_elements.rs` (it already has `use super::*;`):

```rust
    /// The post-game window is `BetweenGames` **and** old-game, and both halves
    /// are load-bearing.
    ///
    /// `is_old_game` is `!has_reset`, and `has_reset` is set false at
    /// `start_game` -- so `is_old_game` is *also true throughout normal play*.
    /// Testing it alone would close foul and warning entry for entire games,
    /// while compiling cleanly and passing every other test in the workspace.
    /// The play cases below are the guard for exactly that.
    #[test]
    fn post_game_is_between_games_and_old_only() {
        let snap = |period, is_old_game| GameSnapshot {
            current_period: period,
            is_old_game,
            ..GameSnapshot::default()
        };

        assert!(
            is_post_game(&snap(GamePeriod::BetweenGames, true)),
            "whistle to changeover is the post-game window",
        );
        assert!(
            !is_post_game(&snap(GamePeriod::BetweenGames, false)),
            "after the changeover an entry lands on the upcoming game",
        );

        for period in [
            GamePeriod::FirstHalf,
            GamePeriod::HalfTime,
            GamePeriod::SecondHalf,
            GamePeriod::OvertimeFirstHalf,
            GamePeriod::SuddenDeath,
        ] {
            assert!(
                !is_post_game(&snap(period, true)),
                "{period:?}: is_old_game is true during play -- entry must stay open",
            );
        }
    }
```

- [ ] **Step 2: Run the test and confirm it fails for the right reason**

```bash
cargo test -p refbox post_game_is_between_games_and_old_only
```

Expected: compile error, `cannot find function 'is_post_game' in this scope`.

- [ ] **Step 3: Write the implementation**

In `shared_elements.rs`, immediately after `team_timeout_in_grace`:

```rust
/// True in the post-game window: the game has ended, but the engine has not yet
/// rolled over to the next one. Runs from the final whistle until the changeover
/// `post_game_duration` later -- 120 seconds by default.
///
/// Nothing recorded in this window survives. The result is sent to the portal at
/// the whistle, in `handle_game_end`, from the stats snapshot `end_game` took, so
/// nothing added afterwards can reach it; and the engine's `reset()` clears the
/// warnings, fouls, penalties and scores at the changeover. An entry made here
/// therefore reaches neither game. The surfaces that would record one are closed
/// rather than silently discarding it.
///
/// **Both halves of the test are required.** `is_old_game` is `!has_reset`, and
/// `has_reset` is set false at `start_game` -- so `is_old_game` is also true
/// throughout normal play. Testing it alone would close foul and warning entry
/// for entire games.
pub(in super::super) fn is_post_game(snapshot: &GameSnapshot) -> bool {
    snapshot.current_period == GamePeriod::BetweenGames && snapshot.is_old_game
}
```

- [ ] **Step 4: Run the test and confirm it passes**

```bash
cargo test -p refbox post_game_is_between_games_and_old_only
```

Expected: 1 passed.

- [ ] **Step 5: Prove the test is a real check**

A check never seen failing is not a check. Temporarily delete `snapshot.current_period == GamePeriod::BetweenGames && ` from the implementation, re-run, and confirm the test **fails** on the `FirstHalf` case. Then restore the line and confirm it passes again. Do not commit the broken version.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/app/view_builders/shared_elements.rs
git commit -m "feat(refbox): add the post-game window predicate"
```

---

### Task 3: Close the entry surfaces during post-game

**Files:**
- Modify: `refbox/src/app/view_builders/warnings_fouls_summary.rs:104-111` — the EDIT FOULS and EDIT WARNINGS buttons.
- Modify: `refbox/src/app/view_builders/main_view.rs:402` — the PENALTIES button's `on_press`.

**Interfaces:**
- Consumes: `is_post_game(&GameSnapshot) -> bool` from Task 2.
- Produces: nothing new.

**Context:** `on_press_maybe` is the established idiom here — 23 existing uses in `refbox`, e.g. `warnings.rs:74`. A button given `None` renders visibly greyed: `theme/button.rs` gives every style a `Status::Disabled` arm (grey background, `disabled_color()` text), asserted by existing tests. It does not stay looking live and inert.

The BACK button and the fouls/warnings *lists* are deliberately untouched — the finished game's entries stay visible throughout the window. Only adding and editing close, because only adding and editing are discarded.

- [ ] **Step 1: Gate the fouls and warnings buttons**

In `warnings_fouls_summary.rs`, immediately before the final `column![` (after the `warnings_and_fouls_row` binding at line ~86):

```rust
    // Closed from the final whistle until the engine's changeover: anything
    // recorded there reaches neither game. The lists above stay visible -- the
    // finished game's entries can be read, just not changed.
    let entry_open = !is_post_game(snapshot);
```

Then change the two buttons at lines ~104-111 from `.on_press(...)` to:

```rust
            make_chrome_button(fl!("edit-fouls"))
                .style(orange_button)
                .width(Length::Fill)
                .on_press_maybe(entry_open.then_some(Message::FoulOverview)),
            make_chrome_button(fl!("edit-warnings"))
                .style(blue_button)
                .width(Length::Fill)
                .on_press_maybe(entry_open.then_some(Message::WarningOverview)),
```

Leave the BACK button's `.on_press(Message::ConfigEditComplete)` exactly as it is.

- [ ] **Step 2: Gate the penalties button**

In `main_view.rs`, inside the `make_penalty_button` closure (which already has `snapshot` in scope), change line ~402 from `.on_press(Message::PenaltyOverview)` to:

```rust
        // Same window, same reason as the fouls and warnings buttons: a penalty
        // entered between the whistle and the changeover is discarded by the
        // engine's reset and never reaches the portal.
        .on_press_maybe((!is_post_game(snapshot)).then_some(Message::PenaltyOverview))
```

- [ ] **Step 3: Confirm it builds clean**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/fix-refbox-roster-before-kickoff
cargo clippy -p refbox --all-targets -- -D warnings
cargo test -p refbox
```

Expected: clean, all tests pass.

- [ ] **Step 4: Commit**

```bash
git add refbox/src/app/view_builders/warnings_fouls_summary.rs refbox/src/app/view_builders/main_view.rs
git commit -m "fix(refbox): close foul, warning and penalty entry after the whistle"
```

---

### Task 4: Full validation and the walkthrough script

**Files:**
- Create: `docs/superpowers/plans/2026-09-04-roster-before-kickoff-walkthrough.md`

**Interfaces:** none — this task produces the numbered steps the human follows, which `.claude/rules/pr-review.md` requires before a PR is proposed.

- [ ] **Step 1: Run the full gate**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/fix-refbox-roster-before-kickoff
just check
```

Expected: fmt, lint, tests and audit all clean. Note that `just check` is host-only and blind to a Windows-only break, and that `cargo audit` can go red with no code change when the advisory database moves — if audit fails, check whether the finding predates this branch before treating it as ours.

- [ ] **Step 2: Confirm the diff is exactly the stated scope**

```bash
git diff --stat origin/master...HEAD
```

Expected: only `refbox/src/app/mod.rs`, `refbox/src/app/view_builders/shared_elements.rs`, `refbox/src/app/view_builders/warnings_fouls_summary.rs`, `refbox/src/app/view_builders/main_view.rs`, plus the spec and this plan. **Any `uwh-common` file in that list is a stop-and-raise.**

- [ ] **Step 3: Build the binary the walkthrough will actually run**

```bash
cargo build -p refbox
```

A walkthrough against a stale binary proves nothing. Note the refbox log file and window title are shared across every worktree on this machine — pin the pid and check `/proc/<pid>/exe` before trusting what is on screen, and never `pkill -x refbox`; peers share this machine.

- [ ] **Step 4: Write the walkthrough script**

Write `docs/superpowers/plans/2026-09-04-roster-before-kickoff-walkthrough.md` containing the launch command below and the eight acceptance criteria from the spec, restated as numbered steps in plain English with an explicit expected result for each. It must be followable by a non-programmer without asking questions.

Launch (fresh config so the walkthrough cannot damage the real portal link; the copied config carries a working per-event access key for `events/1889-B`):

```bash
cp -r ~/.claude/refbox-walkthrough-config-1889B /tmp/claude-1000/wt-config
XDG_CONFIG_HOME=/tmp/claude-1000/wt-config WAYLAND_DISPLAY= DISPLAY=:0 \
UWH_PORTAL_URL_OVERRIDE=https://api.dev.uwhportal.com \
./target/debug/refbox --allow-http --no-simulate \
    --json-port 8020 --binary-port 8021
```

Court 1 game 27 is Brisbane A (7 cap numbers) vs Melbourne Scubadorks (12). The copied config ships with `force_keypad_numbers = true`.

**The script must state the trap in its own words:** before kickoff the picker shows the pad whatever FORCE KEYPAD NUMBERS says, so a working fix and an absent roster look identical on screen. Every roster step is therefore checked at FORCE KEYPAD both YES *and* NO — four states, not two. A pass at NO is only meaningful next to a pass at YES.

It must also warn that criterion 5 needs **2 real minutes** of waiting and cannot be shortened with TIME EDIT, because winding the break clock forward triggers the very changeover being observed — though that *is* the quick way into criterion 6.

- [ ] **Step 5: Commit**

```bash
git add docs/superpowers/plans/2026-09-04-roster-before-kickoff-walkthrough.md
git commit -m "docs(refbox): add the roster-before-kickoff walkthrough script"
```

- [ ] **Step 6: Stop and hand back**

Do **not** open a PR. Report to the human, unprompted, the status of all three pre-PR checks per `.claude/rules/pr-review.md`:

1. **Automated code review** — the built-in `code-review` skill against this diff. Note that the skill takes its diff from the session's working directory and can pick a stale local `master`; pass the base explicitly.
2. **Manual walkthrough by the human** — hand over the script from Step 4 and wait. Claude driving the app is never this check.
3. **Claude-driven verification** — there is **no screen-capture tool on this machine**, so a GUI change cannot be verified this way here. Report it as not possible rather than not done, and recommend accordingly.

---

## Self-review notes

**Spec coverage.** Change 1 (close post-game surfaces) → Tasks 2 and 3. Change 2 (picker follows the upcoming game) → Task 1. Change 3 (before first kickoff) → Task 1, covered by `before_the_first_kickoff_follows_the_upcoming_game` and walkthrough criteria 1-2; it needs no code of its own, which is the point of the single rule. Acceptance criteria 1-8 → Task 4 Step 4. The `is_old_game` risk → Task 2 Steps 1 and 5.

**Deliberately not tested by unit test:** that the three buttons are wired to the predicate. iced elements are not introspectable here, so the wiring is proved by walkthrough criteria 5 and 6, not by a test. Stated plainly rather than papered over with a test that asserts nothing.

**Type consistency.** `GameNumber` is `String`, so `Option<&GameNumber>` compares against `Some(&"27".to_string())`. `picker_roster_game` and `is_post_game` both take `&GameSnapshot` and are named identically in every task that uses them.
