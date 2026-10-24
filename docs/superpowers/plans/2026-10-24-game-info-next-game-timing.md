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

- [x] **Step 1: Write the failing tests** in `displayed_game_config_tests`. Build a `TimingRule` with `half_play_duration: 900s` (copy the field list of `finals_timing_rule()` in `tournament_manager/mod.rs`) and a committed `GameConfig { half_play_duration: 420s, ..Default::default() }`.
  - (a) `between_games_shows_the_next_games_timing`: `BetweenGames` + next timing `Some` → `half_play_duration == 900s`
  - (b) `between_games_without_a_timing_rule_shows_the_committed_config`: `BetweenGames` with `next_game == None`, and again with `Some(NextGameInfo { timing: None, .. })` → 420s both times
  - (c) `during_a_game_shows_the_game_in_progress`: `FirstHalf` + next timing `Some` → 420s
- [x] **Step 2: Run, expect a compile failure** (function missing): `cargo test -p refbox --bin refbox displayed_game_config -- --test-threads=1`
- [x] **Step 3: Implement `displayed_game_config`** with the signature above.
- [x] **Step 4: Route both arms through it.** MainPage: replace the inline `new_config`/`game_config` block with a call using `self.snapshot.current_period`, `self.tm.lock().next_game_info().as_ref()` and `&self.config.game`, and pass `&game_config` on. GameDetailsPage: same call, replacing `&self.config.game`. Take the lock in a short scope, not across the builder call (the builder args already lock `self.tm` again).
- [x] **Step 5: Run the tests, expect PASS**, then `just check` in the worktree (`--test-threads=1` if ALSA flakes appear; see memory `reference_concurrent_refboxes_fail_tests_on_alsa`).
- [x] **Step 6: Commit** `fix(refbox): show the upcoming game's timing on Game Info between games`

### Task 2: The live feed's "next period length" describes the upcoming game too

Added 2026-10-24 (PO-approved scope widening). PO's second symptom: during the break before PO1, the stream overlay's pre-game page showed the RR half length. Cause: `TournamentManager::generate_snapshot` computes `next_period_len_secs` as `self.current_period.next_period_dur(&self.config)`. Between games, `next_period_dur` is `FirstHalf → config.half_play_duration`, and `self.config` is still the finished game's. The same snapshot field feeds the overlay (`overlay/src/main.rs`, pre-game page), the overlay-bridge feed (`game_feed.rs`, passthrough) and the LED panel (`update_sender.rs`: with `hide_time` on, the last <10 s of a break show `next_period_len_secs` instead of the countdown).

**Heavy process applies** (engine file, and a value that crosses the wire to the panel and overlay): per-task review on Opus after this task.

**Files:**
- Modify: `refbox/src/tournament_manager/mod.rs`: move `displayed_game_config` here from `app/mod.rs` as `pub fn` (place it beside `NextGameInfo`), together with its three tests (into the existing `#[cfg(test)]` module or a sibling module); use it in `generate_snapshot`
- Modify: `refbox/src/app/mod.rs`: delete the function and its test module, import it from `crate::tournament_manager`. The two call sites stay as they are.

**Interfaces:**
- Consumes: `displayed_game_config(period, next_game, committed) -> Cow<'a, GameConfig>` (Task 1, unchanged signature and body)
- Produces: the same function at `crate::tournament_manager::displayed_game_config`

- [x] **Step 1: Write the failing engine test** `the_feed_describes_the_upcoming_game_between_games` in `tournament_manager/mod.rs` tests. RR config `half_play_duration: 420s, half_time_duration: 120s` and a score of 1–0 so the game ends after the second half. `start_play_now`, then `set_next_game` with timing `half_play_duration: 900s`. Drive the real updater sequence (the `tick` shape in `tournament_manager/golden/mod.rs`) one second at a time:
  - during `FirstHalf`, `generate_snapshot(now).next_period_len_secs == Some(120)` (the RR half-time; unchanged behaviour)
  - on reaching `BetweenGames`, `next_period_len_secs == Some(900)`
  - after the PO1 kickoff (`FirstHalf` again), `tm.config().half_play_duration == 900s`
  - plus a break with `next_game` timing `None` → `Some(420)`
- [x] **Step 2: Run it, expect FAIL** at the `BetweenGames` assertion (420 ≠ 900): `cargo test -p refbox --bin refbox the_feed_describes_the_upcoming_game -- --test-threads=1`
- [x] **Step 3: Move the function and its tests**, then in `generate_snapshot` compute `next_period_len_secs` from `displayed_game_config(self.current_period, self.next_game.as_ref(), &self.config)`. `self.config` itself is NOT changed; only the figure reported.
- [x] **Step 4: Run** the new test, `displayed_game_config`, and the whole `tournament_manager` + `golden` tests (`cargo test -p refbox --bin refbox tournament_manager -- --test-threads=1`); clippy `-p refbox -- -D warnings`; `cargo fmt --all`. Leave uncommitted.
- [x] **Step 5: Commit** `fix(refbox): report the upcoming game's period length in the feed between games`

Review Focus additions:
6. **A court finished** (blank next game) between games: the feed figure is the committed config's, as today. Covered by `the_feed_describes_the_finished_game_once_the_court_is_finished` (added in the pre-PR fix round); the `None` test covers a next game with no timing rule, which is a different input.
7. **Half-time and other in-game breaks** must keep reporting the current game's next period. Covered by the `FirstHalf` and `HalfTime` assertions; the test's playoff rule has a different half-time (180s vs 120s) so they can fail (seen failing on a mutant that passes `BetweenGames` as the period).

### Task 3: Game Info's Next Game row shows the next game's own Game Block

Added 2026-10-24 (PO ruling, option 1 of 3, asked in the engine lane's chat). Pre-existing on master, found by the pre-PR review: during a game, Game Info's Next Game block shows `config.game_block`, the game *in progress*'s Game Block, labelled as the next game's. During the last round-robin game that is the round-robin block for PO1; at the whistle the same game moves to the Current block and its figure changes.

**Files:**
- Modify: `refbox/src/tournament_manager/mod.rs`: split `upcoming_game_config(next_game, committed) -> Cow<'a, GameConfig>` (next game's rule if it has one, else the committed config, which `start_game` keeps at kickoff in that case) out of `displayed_game_config`, which becomes "between games: `upcoming_game_config`, else the committed config". Behaviour of `displayed_game_config` unchanged; its three tests stay, plus one for `upcoming_game_config`.
- Modify: `refbox/src/app/view_builders/game_info_table.rs`: `game_info_rows` gains `next_game_block: Duration`, used for the in-game Next block instead of `config.game_block`. Every existing test call passes `config.game_block` of the config it already passes (unchanged expectations), plus one new test: in-game Next block shows `next_game_block`, Current block still shows `config.game_block`.
- Modify: `refbox/src/app/view_builders/game_info.rs`: `build_game_info_page` takes and forwards `next_game_block`.
- Modify: `refbox/src/app/mod.rs`: the `GameDetailsPage` arm passes `upcoming_game_config(next_game.as_ref(), &self.config.game).game_block`.
- Modify: `refbox/src/app/view_builders/main_view.rs` (added during execution; PO ruling 2026-10-24, "fix both"): the main page's table under the clock calls `game_info_rows` too, with the same in-game Next block. `build_main_view` takes `next_game_block` and passes it to both calls; the `MainPage` arm passes the same `upcoming_game_config(..).game_block`, so the two screens agree.

- [x] Step 1: failing tests (table + `upcoming_game_config`), run, see them fail
- [x] Step 2: implement, run the touched tests, clippy, fmt; leave uncommitted
- [x] Step 3: review (the branch's final pre-PR `code-review` covers this task, as the last one), commit `fix(refbox): show the next game's own Game Block in the Next row during play`

Review Focus additions:
8. Manual mode / a next game with no timing rule: the Next block shows the committed Game Block, as today (correct: the engine keeps that config at kickoff).
9. A court finished: the Next block stays dashed (`no_next_game`), unaffected.
10. The main screen's table and Game Info show the same Next block figure during play.

## Verification (pre-PR)

- **Check 1:** `code-review` skill on the branch diff, plus `just check`.
- **Check 2 (PO walkthrough):** linked to a schedule where game N and game N+1 use different timing rules. After game N ends, open Game Info during the break: it shows N+1's times. Before the fix it showed N's. Fixture route: a local custom site serving such a schedule, to be settled when the walk is prepared (book the refbox app with the coordinator, or isolate with `XDG_CONFIG_HOME`).
- **Check 3:** not recommended; the rule is pinned by the tests and the screen is the walkthrough's job.

## Deviations

- Task 1: the helper used a let-chain (`if … && let Some(…)`), which needs Rust 1.88 against MSRV 1.85 and compiled only because the local toolchain is newer. Rewritten as a `match` with a guard before commit `02fdf114`.
- Scope widened 2026-10-24 (PO): Task 2 added for the overlay / feed symptom. Global Constraints' "app/mod.rs only, no engine changes" is superseded for Task 2 only. The engine's *config* still changes only at kickoff; only the reported figure moves.
- Check 2 gains a step: the overlay's pre-game page during the same break shows the upcoming game's half length. If an LED panel is available with hide time on, the last 10 s of that break show the upcoming game's half length too.
- Task 2 review (Opus, 2026-10-24): approve with fixes; production code unchanged. Important 1 (the half-time check could not fail, because finals_timing_rule's half-time equals the RR's) fixed in the follow-up commit. Minors not taken: a third copy of golden's `tick` in the test, and a TimingRule clone per snapshot between games (negligible).
- Pre-PR `code-review` (high, 2026-10-24) on `b4376274`: 10 findings, triaged against the code.
  - Taken: a court-finished engine test (the `None` test only covered a next game with no timing rule); the feed tests now drive the engine's real `updater_tick` instead of a third copy of golden's `tick`; this plan's boxes ticked and its scratchpad citation dropped; commit `b4376274` reworded from `test(` to `fix(refbox)` (unpushed).
  - Declined: moving the choice into the engine under one lock (the kickoff flicker it fixes needs a redraw in the milliseconds before the new snapshot arrives, the main page has had it on master, and it would change where in-game timing is read from); the per-snapshot and per-redraw copies (negligible); two sources for the stop-clock row (between games both read the upcoming game).
  - Raised with the PO: during a game, Game Info's "Next" row shows the game in progress's Game Block rather than the upcoming game's. Older than this branch. PO ruled 2026-10-24: fix it here (Task 3), which supersedes Global Constraints' "no view-builder changes" for Task 3.
- Pre-PR `code-review` round 2 (high, 2026-10-24) on `c4878166`, the whole branch: 6 findings, triaged against the code.
  - Taken: the `MainPage` and `GameDetailsPage` arms of `view` repeated the same three steps (copy the next game, pick the displayed config, pick the next Game Block). Both now call one `RefBoxApp::displayed_timing()`, which locks `tm` once, borrows the next game instead of copying it, and releases the lock before the builder arguments lock it again. Same figures on both screens, no behaviour change. This plan's Task 3 boxes ticked and its commit title corrected.
  - Declined: dashing the Next row when the next game's rule has no Game Block of its own. The derived figure is what the engine plays that game with (`start_game` adopts the rule), and what the Current row shows after the whistle; dashing only the Next row would bring back the flip at the whistle that Task 3 removed. Also declined: the snapshot's period against the live engine's next game (a window of milliseconds, and with no next game the committed block is what the engine keeps), and the second rule conversion per redraw (negligible).
