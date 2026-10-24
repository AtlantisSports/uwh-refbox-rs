# Game Info shows the upcoming game's timing between games — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** During a break, the Game Info page shows the timing of the game about to start, the same timing the main screen already shows, instead of the previous game's.

**Architecture:** The main page already chooses "next game's timing rule while between games, else the committed config" inline in `RefBoxApp::view` (`refbox/src/app/mod.rs`, the `AppState::MainPage` arm). Lift that choice into one pure function and call it from both the `MainPage` and `GameDetailsPage` arms, so the two screens cannot disagree again.

**Tech Stack:** Rust 2024, MSRV 1.85, iced 0.13 (refbox crate only).

**Spec:** this plan (no separate spec). Root cause, 2026-10-24, engine lane, PO report from a live tournament: "the first playoff game did not pick up the new timing rules"; the PO saw round-robin timing on Game Info during the break before PO1.

## Root cause (proven)

- `handle_game_start` stores the next game's `TimingRule` in `NextGameInfo.timing` at the kickoff of the game before it. The engine keeps the outgoing game's config through the break and adopts the stored rule in `start_game` at kickoff.
- A probe test on an `origin/master` snapshot (`afbce414`) confirmed it: RR half 420s → break: engine config 420s, next-game timing 900s → PO1 kickoff: engine config 900s, with no operator action. **Gameplay was correct. The fault is display only.**
- `AppState::GameDetailsPage` passes `&self.config.game` to `build_game_info_page`. `self.config.game` is refreshed only at kickoff, so during a break it holds the previous game's config. `game_info_table.rs` says in its own comment that between games it displays "the upcoming game", so the page is fed the wrong input against its stated intent.
- The main page is already right: it uses `next_game_info().timing` while `snapshot.current_period == BetweenGames`.
- Settings shows timing only in manual mode (portal OFF), where `next_game.timing` is always `None`, so it is out of scope.

## Global Constraints

- Scope: `refbox/src/app/mod.rs` only. No engine (`tournament_manager`), `uwh-common`, view-builder or translation changes.
- No behaviour change to the clock, kickoff or APPLY; only what Game Info displays.
- No new `unwrap()`/`expect()` in production code. Clippy `-D warnings`.
- Branch `fix/refbox/game-info-next-game-timing`, worktree `.claude/worktrees/fix+refbox+game-info-next-game-timing`, cut from a fresh `origin/master`.
- Commit format `fix(refbox): …`.

## Review Focus

1. **Court finished** (blank next game, `next_game == None`) between games: Game Info falls back to the committed config, as today. Pinned by test (b).
2. **Manual mode** (`timing: None` in the next game): unchanged, committed config. Pinned by test (b).
3. **During a game** with next-game timing already stored: Game Info shows the game *in progress* (committed config), never the next one. Pinned by test (c).
4. **Main page unchanged:** it must pick exactly what it picks today. Covered by routing it through the same function plus test (a).
5. **Tap-through from Game Info into Settings** must not be affected: Settings seeds from `tm.config()`, which this change does not touch. Checked in review, no test.

---

### Task 1: One "timing to display" rule, used by Main and Game Info

**Files:**
- Modify: `refbox/src/app/mod.rs`: the `AppState::MainPage` arm of `view` (the inline `new_config`/`game_config` block, ~line 8268 on `afbce414`) and the `AppState::GameDetailsPage` arm (~line 8357)
- Test: a new `#[cfg(test)] mod displayed_game_config_tests` in the same file, next to `refresh_next_game_tests`

**Interfaces:**
- Produces: `fn displayed_game_config<'a>(period: GamePeriod, next_game: Option<&NextGameInfo>, committed: &'a GameConfig) -> Cow<'a, GameConfig>`, a free function in `app/mod.rs`. `Cow::Owned(timing.clone().into())` when `period == BetweenGames` and `next_game` has `Some(timing)`; otherwise `Cow::Borrowed(committed)`.

- [ ] **Step 1: Write the failing tests** in `displayed_game_config_tests`. Build a `TimingRule` with `half_play_duration: 900s` (copy the field list of `finals_timing_rule()` in `tournament_manager/mod.rs`) and a committed `GameConfig { half_play_duration: 420s, ..Default::default() }`.
  - (a) `between_games_shows_the_next_games_timing`: `BetweenGames` + next timing `Some` → `half_play_duration == 900s`
  - (b) `between_games_without_a_timing_rule_shows_the_committed_config`: `BetweenGames` with `next_game == None`, and again with `Some(NextGameInfo { timing: None, .. })` → 420s both times
  - (c) `during_a_game_shows_the_game_in_progress`: `FirstHalf` + next timing `Some` → 420s
- [ ] **Step 2: Run, expect a compile failure** (function missing): `cargo test -p refbox --bin refbox displayed_game_config -- --test-threads=1`
- [ ] **Step 3: Implement `displayed_game_config`** with the signature above.
- [ ] **Step 4: Route both arms through it.** MainPage: replace the inline `new_config`/`game_config` block with a call using `self.snapshot.current_period`, `self.tm.lock().next_game_info().as_ref()` and `&self.config.game`, and pass `&game_config` on. GameDetailsPage: same call, replacing `&self.config.game`. Take the lock in a short scope, not across the builder call (the builder args already lock `self.tm` again).
- [ ] **Step 5: Run the tests, expect PASS**, then `just check` in the worktree (`--test-threads=1` if ALSA flakes appear; see memory `reference_concurrent_refboxes_fail_tests_on_alsa`).
- [ ] **Step 6: Commit** `fix(refbox): show the upcoming game's timing on Game Info between games`

## Verification (pre-PR)

- **Check 1:** `code-review` skill on the branch diff, plus `just check`.
- **Check 2 (PO walkthrough):** linked to a schedule where game N and game N+1 use different timing rules. After game N ends, open Game Info during the break: it shows N+1's times. Before the fix it showed N's. Fixture route: a local custom site serving such a schedule, to be settled when the walk is prepared (book the refbox app with the coordinator, or isolate with `XDG_CONFIG_HOME`).
- **Check 3:** not recommended; the rule is pinned by the tests and the screen is the walkthrough's job.

## Deviations

(none yet)
