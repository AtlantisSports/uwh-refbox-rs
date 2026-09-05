# Roster Before Kickoff — Implementation Plan and Execution Record

**Goal:** Make the player picker offer the roster of the game an entry would actually land on,
including before the first kickoff of a session.

**Spec:** `docs/superpowers/specs/2026-09-04-roster-before-kickoff-design.md`

**Branch:** `fix/refbox/roster-before-kickoff`, based on `origin/master` at `486c5692`.
**Worktree:** `/home/estraily/projects/uwh-refbox-rs/.worktrees/fix-refbox-roster-before-kickoff`

**Status: executed 2026-09-04.** All tasks complete; the branch is unpushed and awaiting the
human's walkthrough. This file now serves as the execution record — what was planned, what
actually happened, and why they differ.

## Global constraints (as applied)

- MSRV Rust 1.85, edition 2024.
- The real lint gate is `cargo clippy --all -- -D warnings` plus a `--no-default-features` pass.
  **Not** `--all-targets --all-features` — see Deviation 3.
- `uwh-common` must not change. Verified: it does not appear in the diff.
- No new dependencies, no new translation keys, no `unwrap()`/`expect()` in new non-test code.
- `post_game_duration` stays at 120 seconds.

---

## Task 1 — The picker follows the game an entry would land on ✅

**Files:** `refbox/src/app/mod.rs`.
**Commit:** `d688c3d4 fix(refbox): offer the upcoming game's roster between games`.

Added `picker_roster_game(&GameSnapshot) -> Option<&GameNumber>`: `Some(next_game_number)` when
the period is `BetweenGames`, `None` otherwise, meaning "use the copy pinned at kickoff". Wired
into the `AppState::KeypadPage` arm of `view()` through a `Cow`, so the mid-game path still
allocates nothing per frame.

Three tests in `picker_roster_game_tests`, written first and confirmed failing with
`no picker_roster_game in app`:

- `before_the_first_kickoff_follows_the_upcoming_game` — the requirement.
- `the_whole_break_follows_the_upcoming_game` — asserts `is_old_game` makes no difference, so the
  picker cannot switch games partway through a break.
- `during_play_keeps_the_kickoff_copy` — all nine play and in-game break periods.

## Task 2 — Never offer a roster from another court ✅

**Files:** `refbox/src/app/mod.rs`.
**Commit:** `147d9ab9 fix(refbox): never offer a roster from another court`.

**Not in the original plan.** Added after a code review found that following the engine's
synthesised `next_game_number` could name a real game on a different court, which the roster
lookup would happily resolve.

Extracted the body of `rosters_for_game` into a free, testable
`rosters_for_scheduled_game(schedule, team_rosters, current_court, game_num)` that refuses a game
whose court does not match. `current_court: None` is not treated as a mismatch, so no existing
caller changes behaviour. Four tests in `rosters_for_scheduled_game_tests`.

**Proved load-bearing:** replacing the court check with `if true` fails exactly
`a_game_on_another_court_supplies_nothing` and no other test; restoring gives 4/4.

## Task 3 — Validation and the walkthrough ✅

- `just check` — passed (fmt, lint, tests, vendor, audit). The "15 allowed warnings" line is the
  pre-existing `cargo audit` ignore list, not this branch.
- `cargo clippy --all -- -D warnings` and the `--no-default-features` pass — both clean.
- 741 tests pass.
- Diff scope confirmed: `refbox/src/app/mod.rs` plus these documents. No `uwh-common`.
- Walkthrough written to `2026-09-04-roster-before-kickoff-walkthrough.md`.
- Debug binary built for the walkthrough.

## Task 4 — Post-game entry closure ➡️ **split out**

Originally tasks 2 and 3 of this plan; approved as design, implemented, then removed from this
branch on 2026-09-04 after review found six defects, three needing decisions rather than fixes.

Preserved on **`wip/refbox/post-game-entry-closure`** (`e2173939`, `a13c2355`). The design, the
trap it already avoids, and all six findings are written up under *Deferred* in the spec. Do not
resume it without reading that section.

---

## Deviations

**1. `picker_roster_game` went somewhere else.** The plan said "immediately after the
`rosters_for_game` method's closing brace and *outside* the `impl RefBoxApp` block". Those are
contradictory — `rosters_for_game` is a method *inside* that block. Placed instead beside
`recorded_result_matches_ended_game` immediately before `impl RefBoxApp`, where this file already
keeps free helpers of the same shape.

**2. Test-module imports are explicit, not wildcard**, matching the adjacent `reply_source_tests`
module. (`rosters_for_scheduled_game_tests` does use `use super::*`, matching the larger test
module it sits beside, plus two explicit imports for `Game` and `ScheduledTeam`.)

**3. The plan's lint command was stricter than the project's own gate.** The plan specified
`cargo clippy -p refbox --all-targets -- -D warnings`. The real gate — `Justfile:28` and
`.github/workflows/rust.yml:37-38` — has **no `--all-targets`**. Verification switched to the real
gate.

The stricter form surfaced two failures, both confirmed outside this diff and both left alone per
`.claude/rules/scope.md`:

- `clippy::items_after_test_module` — `keypad_pages/player_grid.rs:456`, untouched by this branch.
- `clippy::field_reassign_with_default` — a test in `app/mod.rs`, untouched by this branch,
  referencing `force_keypad_numbers`, so it arrived with PR #3135.

**Worth raising separately:** `CLAUDE.md` and `.claude/rules/rust.md` both state that CI enforces
`cargo clippy --workspace --all-targets --all-features -- -D warnings`. It does not. Two clippy
failures currently sit on `master` that no gate will surface. Documenting a check that does not
exist is its own defect and belongs on its own branch.

**4. Both risky predicates were proved by mutation, not assertion.** For the deferred
`is_post_game`, and again for the court check here: run clean, break the specific line, watch the
right test fail with the right message, restore byte-identically, run clean again. A check never
seen failing is not a check.

**5. The branch was split mid-execution.** The original plan carried the roster fix and the
post-game closure together, on the recommendation that they were one rule and equally simple. That
recommendation was wrong: review showed the closure needed three decisions from the human. The
branch was rewound to the roster work and the closure preserved on its own branch. Nothing had
been pushed.

## What a reviewer should check that the tests cannot

- That the grid on screen actually changes when a break's changeover happens (walkthrough Part 3).
- That FORCE KEYPAD NUMBERS still overrides the grid in every state (walkthrough Steps 2 and 4) —
  without this, a missing roster is indistinguishable from the setting working.
- That a mid-game REFRESH does not move the numbers (walkthrough Step 3).
