# Behind-Schedule Test Hardening — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development or executing-plans. Checkbox steps.

**Goal:** Close the non-happy-path gaps in behind-schedule test coverage and reframe two tests that assert an unreachable state — verifying the *existing* model, with NO production behaviour change.

**Model under test (final, raw tally):**
- In-game: `behind = max(0, inherited + (real_elapsed + remaining_regulation(now) − regulation_play))`.
- `remaining_regulation` = live remaining on the current regulation period + later regulation periods; ZERO in extra time. Reads the MAIN clock (not the timeout clock).
- Between-games: `behind = max(0, (now + remaining_break) − scheduled_next_start)`; the next game auto-starts when the break elapses (`update()` → `start_game`), so the figure only climbs between games while the break is **paused** or **edited longer**.
- Specs: `2026-06-08-behind-schedule-raw-tally-design.md` (+ projection/refinement specs).

**Process:** Heavy (game-clock state machine) but test-only. One file: `refbox/src/tournament_manager/mod.rs` (+ a doc note). **If any new test reveals the production code does something different from the model above, STOP and report it — do NOT change production code to make a test pass.**

---

## Task 1: Add non-happy-path tests; reframe the two unreachable-state tests

**Files:** Modify/Test: `refbox/src/tournament_manager/mod.rs`

Each new test: pick a config with a real slack (e.g. half 60 / ht 10 / sh 60 → reg 130; block 180; min_break 5), set up the scenario through the real API, sample `behind_schedule` at two or more instants, and assert the intended direction with **exact recomputed literals** (show the calculation in your report). Add near the other `test_behind_schedule_*` tests.

- [ ] **Step 1 — Rugby penalty shot FREEZES (the deliberate special case).** Start a game, create a stoppage so the figure is a nonzero value (e.g. 30s behind) with the clock then **running**, start a rugby penalty shot (`start_rugby_penalty_shot`, which keeps the main clock running), and sample `behind_schedule` at two instants during the shot. Assert they are **equal** (frozen) and nonzero. Worked target with the config above: stop at +10 (clock 50), resume at +40 (30s stopped → figure 30), `start_rugby_penalty_shot(+40)`, sample +41 and +43 → both **30**. Add an inline comment that this is the rugby-PS-keeps-clock-running exception.

- [ ] **Step 2 — Team timeout climbs.** Like the ref-timeout test but `start_team_timeout(Color::Black, …)`. The main clock stops, so two samples during the timeout must **increase** (e.g. +X then +X+something). Assert the climb with exact literals.

- [ ] **Step 3 — Normal penalty shot climbs.** `start_penalty_shot(…)` (stops the main clock). Two samples during it must increase. Exact literals.

- [ ] **Step 4 — Score-confirmation pause climbs.** Drive a game to its end and `pause_for_confirm(…)`. While paused (clock stopped, before the confirm ends), two samples must increase. Verify the period/state during the pause and compute literals; if the confirm pause transitions to BetweenGames immediately so the in-game branch isn't exercised, document that and assert whatever the model genuinely yields (climb via the between-games projection while the confirm clock is stopped).

- [ ] **Step 5 — Sudden death climbs.** Reach sudden death (use `set_period_and_game_clock_time(GamePeriod::SuddenDeath, …)` as sibling tests do, with an `overtime_allowed`/`sudden_death_allowed` config). `remaining_regulation` is ZERO there, so two samples as time advances must **increase**. Exact literals.

- [ ] **Step 6 — Multi-game accumulate-then-recover lifecycle** (the "maintained for some games, then catches up" behaviour). In manual mode with a real slack:
  1. Game 1 starts on time; create an overrun larger than the slot slack; end the game. Assert the between-games figure equals the residual `game1_end + min_break − sched_next1` (> 0).
  2. Start game 2 (at the compressed break). Assert game 2's in-game figure carries that **inherited residual** while it plays cleanly.
  3. Game 2 plays with no overrun and ends; the break compresses again. Assert the figure for game 3 is **smaller** (residual − one slot's slack).
  4. Continue until the figure reaches **ZERO** (fully recovered). Assert the monotonic decrease and the final zero.
  Compute every asserted value by hand and show the chain in your report. (This is the most involved test — take care with `current_scheduled_start`/`next_scheduled_start` across games in manual mode.)

- [ ] **Step 7 — Reframe the two unreachable-state tests** to realistic causes:
  - `test_behind_schedule_between_games_overdue` and `test_behind_schedule_between_games_climbs_when_break_overdue` currently advance `now` while holding the manager in BetweenGames *without* calling `update()` — but the live app auto-starts the next game when the break elapses, so that state is unreachable. Reframe them so the climb comes from **pausing the break** (`stop_clock` during the between-games countdown, then advance `now` → the figure climbs) — the real operator action. Keep one focused test per realistic cause (pause-the-break-climbs; the edit case is already covered by `between_games_follows_break_edit`). If that leaves the two tests redundant, merge into one well-named test (e.g. `test_behind_schedule_between_games_climbs_when_break_paused`) and delete the other. Verify `stop_clock` is valid between games; if not, use the real pause path and document it.

- [ ] **Step 8 — Run and verify.** `cd /home/estraily/projects/uwh-refbox-rs/.worktrees/game-block && cargo test -p refbox behind_schedule` (all pass), then `cargo test -p refbox` (whole crate, all pass), then `cargo clippy -p refbox -- -D warnings` (clean). If any NEW test fails because the model behaves unexpectedly (e.g. rugby PS does NOT freeze, or the lifecycle doesn't recover), STOP and report — that is a real finding, not a literal to adjust.

- [ ] **Step 9 — Commit** (only this file):
```
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/game-block
git rev-parse --abbrev-ref HEAD   # must be feat/uwh-common/game-block
git add refbox/src/tournament_manager/mod.rs
git commit -m "test(refbox): cover timeouts, penalty shots, sudden death, and multi-game behind-schedule lifecycle

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

**CRITICAL git/workspace safety (for the implementer):** work only in the worktree `/home/estraily/projects/uwh-refbox-rs/.worktrees/game-block`; never touch the main checkout at `/home/estraily/projects/uwh-refbox-rs`; `cd` into the worktree before every cargo/git command; confirm branch is `feat/uwh-common/game-block` before/after; never run git checkout/switch/restore/stash/worktree/reset/rebase; commit only `refbox/src/tournament_manager/mod.rs` with `git add <that path>`. The Read tool may serve a stale cache — verify on disk.

---

## Task 2: Doc correction + full check

- [ ] **Step 1 — Correct the reference.** In `docs/superpowers/specs/2026-06-08-behind-schedule-raw-tally-design.md`, add a short "Behaviour reference" note (or amend the behaviour summary) stating: the next game auto-starts when the between-games break elapses (`update()`), so a delay can be *carried in* and grow only while a break is **paused or edited longer** — "sitting idle past the scheduled start" is not a real cause. Commit this doc edit separately: `git add docs/... && git commit -m "docs(refbox): note auto-start; correct between-games delay causes"`.
- [ ] **Step 2 — `just check`** → EXIT 0 (fmt, lint, tests, audit clean). Report the refbox/uwh-common test counts.

## Deviations
(Record during execution — especially any test that revealed a real production discrepancy.)
