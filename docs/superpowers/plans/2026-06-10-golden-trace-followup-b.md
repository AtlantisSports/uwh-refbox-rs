# Golden-Trace Guard Follow-up B Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expand the time-engine golden-trace guard to also watch `scores` and `is_old_game`, closing the score blind-spot and the deferred between-games auto-reset gap (mutants at `mod.rs:1180-1182`).

**Architecture:** Extend the existing fixed-step replay guard *in place*. Add two columns to `render()`; add one new operator action (`StartPlayNow`) required to reach `has_reset = false`; add one dedicated between-games-reset scenario; add a compile-time field-completeness guard; re-bless all golden files from the baseline commit; prove each new column kills a previously-surviving mutant.

**Tech Stack:** Rust 2024, `refbox` crate (bin-only — in-crate `#[cfg(test)]` tests only), `cargo-mutants` 26.0.0 (already installed at `~/.cargo/bin`).

---

## Context the executing engineer needs

- **Where the guard lives:** `refbox/src/tournament_manager/golden/mod.rs` (driver: `Action` enum, `apply_action`, `render`, `run`, `#[cfg(test)] mod tests`), `golden/scenarios.rs` (scenario library, `all()`), `golden_traces/*.trace` (37 checked-in golden files), `golden_traces/README.md` (re-bless rule).
- **The permanent test:** `golden_traces_match_baseline` in `golden/mod.rs::tests` runs every scenario twice (determinism) then compares each trace against its golden file. Re-bless with `UPDATE_GOLDEN=1 cargo test -p refbox golden_traces_match_baseline`.
- **Build/test commands (this is a bin-only crate — do NOT use `--all-targets`):**
  - Test: `cargo test -p refbox golden_traces_match_baseline -- --nocapture`
  - The completeness test: `cargo test -p refbox render_accounts_for_every_snapshot_field`
  - Lint (mirrors CI): `cargo clippy -p refbox -- -D warnings`
- **Baseline commit:** `46ec0973` (last human-authored commit). The guard is differential vs. this baseline.
- **Process:** Lean (`.claude/rules/plan-execution.md`) — test-only code, the engine is never touched. The mutation-validation step (Task 6) is the verification gate. The cargo-mutants result files are local working docs, NOT committed and NOT in the PR.
- **Do NOT touch the engine** (`tournament_manager/mod.rs` non-test code) or `uwh-common`. All edits are inside `golden/`.

## File map

- Modify: `refbox/src/tournament_manager/golden/mod.rs` — add `StartPlayNow` to `Action` + `apply_action`; extend `render`; add completeness test.
- Modify: `refbox/src/tournament_manager/golden/scenarios.rs` — import `StartPlayNow`; add the between-games scenario + register it in `all()`.
- Modify: all 37 `refbox/src/tournament_manager/golden_traces/*.trace` — re-blessed (two new columns).
- Create: `refbox/src/tournament_manager/golden_traces/between_games_auto_reset.trace` — new scenario's golden file.
- Modify: `refbox/src/tournament_manager/golden_traces/README.md` — note the watched set is now time-state + `scores` + `is_old_game`.

---

## Task 1: Add the `StartPlayNow` action

**Files:**
- Modify: `refbox/src/tournament_manager/golden/mod.rs` (the `Action` enum and `apply_action`)

- [ ] **Step 1: Add the `StartPlayNow` variant to the `Action` enum**

In `golden/mod.rs`, inside `pub(super) enum Action { ... }`, add (after `StartClock`):

```rust
    /// Manually start/advance play — the operator "Start" button.
    ///
    /// Mirrors `Message::StartPlayNow` → `tm.start_play_now(now)`. From
    /// `BetweenGames` this begins a new game via `start_game` (which sets
    /// `has_reset = false` → `is_old_game = true`); from `HalfTime` / `PreOvertime`
    /// / `OvertimeHalfTime` / `PreSuddenDeath` it advances to the next play period.
    ///
    /// DISTINCT from `StartClock` (the bare `start_clock(now)` resume primitive):
    /// `StartClock` never calls `start_game`, so scenarios built only from
    /// `SetupPeriod` + `StartClock` leave `has_reset = true` and can never exercise
    /// the between-games auto-reset.
    StartPlayNow,
```

- [ ] **Step 2: Add the `apply_action` arm**

In `golden/mod.rs` `fn apply_action`, add an arm (after the `Action::StartClock` arm):

```rust
        Action::StartPlayNow => {
            // Mirrors Message::StartPlayNow → start_play_now(now).
            // From BetweenGames: start_game → has_reset = false (is_old_game = true)
            // and the clock starts running (send_clock_running(true)); the driver's
            // latch tick then drives the game forward.
            tm.start_play_now(now).unwrap();
        }
```

- [ ] **Step 3: Extend the KNOWN COUPLING POINT comment block**

In the `KNOWN COUPLING POINT` comment in `golden/mod.rs`, add a line to the cross-reference list:

```
//   StartPlayNow         → Message::StartPlayNow (start_play_now(now)): begin a new
//                          game from BetweenGames (→ start_game, has_reset=false) or
//                          advance HalfTime/PreOvertime/etc. to the next play period.
```

- [ ] **Step 4: Verify it compiles and existing traces are unchanged**

Run: `cargo test -p refbox golden_traces_match_baseline -- --nocapture`
Expected: PASS — `StartPlayNow` is unused so far, so no trace changes. Also run `cargo clippy -p refbox -- -D warnings` → no warnings (the variant is covered by the existing `#[allow(dead_code)]` on `Action`).

- [ ] **Step 5: Commit**

```bash
git add refbox/src/tournament_manager/golden/mod.rs
git commit -m "test(refbox): add StartPlayNow action to golden-trace driver"
```

---

## Task 2: Extend `render()` with `scores` and `is_old_game`

**Files:**
- Modify: `refbox/src/tournament_manager/golden/mod.rs` (`fn render`)

- [ ] **Step 1: Add the two new rendered values**

In `golden/mod.rs` `fn render`, after the `conf_pause` binding and before the `format!`, add:

```rust
    // Scores: fixed-width "B<black>/W<white>".
    let score = format!("B{}/W{}", snap.scores[Color::Black], snap.scores[Color::White]);

    // Between-games "old game" flag: Y while the displayed game is the finished/old
    // one (has_reset == false); flips to N when the engine auto-resets for the next game.
    let old = if snap.is_old_game { "Y" } else { "N" };
```

- [ ] **Step 2: Update the `format!` to a fixed all-columns layout**

Replace the existing `format!(...)` at the end of `render` with:

```rust
    format!(
        "period={period:<13} | clock={:>3}s | score={score:<7} | timeout={timeout:<12} | conf_pause={conf_pause:<6} | old?={old} | pens=[{pens_str}]",
        snap.secs_in_period
    )
```

(`score` is `<7` to accommodate up to `B99/W99`; the two new columns sit in fixed positions on every line.)

- [ ] **Step 3: Confirm the change is the only difference, then re-bless**

First run WITHOUT blessing to see every trace now mismatches by exactly the two new columns:

Run: `cargo test -p refbox golden_traces_match_baseline -- --nocapture`
Expected: FAIL — every scenario differs (new `score=` / `old?=` columns inserted). Skim a few diffs to confirm the *only* change per line is the two inserted columns (existing columns unchanged and in the same relative order).

Then re-bless:

Run: `UPDATE_GOLDEN=1 cargo test -p refbox golden_traces_match_baseline`
Then re-run without the env var: `cargo test -p refbox golden_traces_match_baseline` → Expected: PASS.

- [ ] **Step 4: Sanity-check the new columns in a known scenario**

Run: `git diff refbox/src/tournament_manager/golden_traces/regulation_with_scores.trace`
Expected: the `score=` column shows `B0/W0` until t=5, then `B1/W0` after the Black goal, then `B1/W1` after the White goal at t=12; `old?=Y` throughout (a started-via-SetupPeriod game leaves `has_reset` as-is). This visually confirms `scores` is now observed.

- [ ] **Step 5: Commit (behavior-preserving column expansion)**

```bash
git add refbox/src/tournament_manager/golden/mod.rs refbox/src/tournament_manager/golden_traces/
git commit -m "test(refbox): watch scores and is_old_game in golden traces"
```

---

## Task 3: Add the field-completeness guard test

**Files:**
- Modify: `refbox/src/tournament_manager/golden/mod.rs` (`#[cfg(test)] mod tests`)

- [ ] **Step 1: Add the exhaustive-destructure test**

In `golden/mod.rs` inside `mod tests`, add:

```rust
    /// Compile-time completeness guard for `render`.
    ///
    /// This exhaustive destructure has no `..`, so adding a field to `GameSnapshot`
    /// fails to compile here until someone consciously decides whether to render it.
    /// Fields are grouped into "rendered" and "intentionally omitted" with the
    /// rationale, mirroring the design doc. This catches *new field* rot; it does
    /// not detect semantic changes to existing fields.
    #[test]
    fn render_accounts_for_every_snapshot_field() {
        let mut tm = TournamentManager::new(GameConfig::default());
        let snap = snapshot_with_retry(&mut tm, Instant::now());
        let GameSnapshot {
            // ── Rendered by `render` ──
            current_period: _,
            secs_in_period: _,
            timeout: _,
            scores: _,
            penalties: _,
            is_old_game: _,
            conf_pause_time: _,
            // ── Intentionally NOT rendered (see design doc §4) ──
            warnings: _,             // inert Vec-append; not core time/state logic
            fouls: _,                // inert Vec-append; not core time/state logic
            game_number: _,          // not core timing/state
            next_game_number: _,     // not core timing/state
            event_id: _,             // hardcoded None in the snapshot constructor
            recent_goal: _,          // display sugar; verified deterministic, no outcome effect
            next_period_len_secs: _, // not core timing/state
        } = snap;
    }
```

- [ ] **Step 2: Run it**

Run: `cargo test -p refbox render_accounts_for_every_snapshot_field`
Expected: PASS (and it compiles, which is the real assertion — every current field is named).

- [ ] **Step 3: Commit**

```bash
git add refbox/src/tournament_manager/golden/mod.rs
git commit -m "test(refbox): add field-completeness guard for golden render"
```

---

## Task 4: Add the between-games auto-reset scenario

**Files:**
- Modify: `refbox/src/tournament_manager/golden/scenarios.rs` (import, scenario static, `all()`)
- Create: `refbox/src/tournament_manager/golden_traces/between_games_auto_reset.trace`

- [ ] **Step 1: Import `StartPlayNow`**

In `scenarios.rs`, add `StartPlayNow` to the `use ...golden::Action::{...}` import list (keep alphabetical with the existing entries):

```rust
    golden::Action::{
        AddScore, ConfirmScore, EndTimeout, ScoreSuddenDeath, SetGameClock, SetupPeriod,
        StartClock, StartPenalty, StartPenaltyShot, StartPlayNow, StartRefTimeout,
        StartRugbyPenaltyShot, StartTeamTimeout, StopClock,
    },
```

- [ ] **Step 2: Add a compact config helper for the between-games lifecycle**

In `scenarios.rs` near `reg_config()`, add:

```rust
/// Short two-half regulation config with NO overtime/sudden-death, used by the
/// between-games scenario so a full game completes quickly and lands in
/// BetweenGames. Breaks are short so the auto-reset (which fires
/// `post_game_duration` into the between-games countdown) is reached fast.
fn between_games_config() -> GameConfig {
    GameConfig {
        half_play_duration: Duration::from_secs(3),
        half_time_duration: Duration::from_secs(2),
        overtime_allowed: false,
        sudden_death_allowed: false,
        post_game_duration: Duration::from_secs(2),
        nominal_break: Duration::from_secs(6),
        minimum_break: Duration::from_secs(4),
        ..Default::default()
    }
}
```

- [ ] **Step 3: Add the scenario static**

In `scenarios.rs` (in a clearly-labelled new section at the end of the scenario statics):

```rust
// Family: between-games lifecycle.
//
// between_games_auto_reset — exercises is_old_game and the auto-reset at
// mod.rs:1180-1182. StartPlayNow begins a real game (start_game sets
// has_reset = false → is_old_game = true). The game plays FirstHalf → HalfTime →
// SecondHalf, ends 0-0, and enters BetweenGames with is_old_game still = Y. Once the
// between-games clock counts down past reset_game_time the engine auto-resets, flipping
// is_old_game Y → N. run_secs is generous so the reset is comfortably reached.
static BETWEEN_GAMES_AUTO_RESET_ACTIONS: &[(u64, Action)] = &[(0, StartPlayNow)];
```

- [ ] **Step 4: Register it in `all()`**

In `scenarios.rs` `pub fn all()`, add an entry to the returned list (follow the existing `Scenario { name, config, actions, run_secs }` shape):

```rust
        Scenario {
            name: "between_games_auto_reset",
            config: between_games_config(),
            actions: BETWEEN_GAMES_AUTO_RESET_ACTIONS,
            run_secs: 25,
        },
```

- [ ] **Step 5: Bless the new scenario and inspect the trace**

Run: `UPDATE_GOLDEN=1 cargo test -p refbox golden_traces_match_baseline`
Then: `cat refbox/src/tournament_manager/golden_traces/between_games_auto_reset.trace`

**Acceptance check (this is the whole point of the scenario):** the trace must show, in order:
1. early lines with `period=FirstHalf` … and `old?=Y` (game started, `has_reset=false`),
2. progression `FirstHalf → HalfTime → SecondHalf`,
3. `period=BetweenGames … old?=Y` lines after the game ends,
4. a later `period=BetweenGames … old?=N` line — **the `old?` flip from Y to N is the auto-reset firing.**

If `old?=N` never appears, the reset did not fire — increase `run_secs`, or confirm the game actually reached BetweenGames (check for `period=BetweenGames` lines at all). Do NOT proceed until the Y→N flip is present.

- [ ] **Step 6: Confirm determinism + full suite green**

Run: `cargo test -p refbox golden_traces_match_baseline` → Expected: PASS (the test asserts each scenario is byte-identical across two runs, so a non-deterministic between-games trace would fail here).

- [ ] **Step 7: Commit**

```bash
git add refbox/src/tournament_manager/golden/scenarios.rs refbox/src/tournament_manager/golden_traces/between_games_auto_reset.trace
git commit -m "test(refbox): add between-games auto-reset golden scenario"
```

---

## Task 5: Validate all golden files against the baseline commit

This enforces the differential discipline: the committed traces (new columns + new scenario) must encode **baseline `46ec0973`** behavior, not just HEAD's. Mirrors the original guard's Task-5 bootstrap.

**Files:** none modified on the working branch (this is a cross-check; it either confirms the committed files or surfaces a diff to classify).

- [ ] **Step 1: Create a baseline worktree**

```bash
git worktree add /tmp/gt-baseline 46ec0973
```

- [ ] **Step 2: Copy the updated guard module into the baseline tree**

Copy the current branch's `golden/` module and the `mod golden;` wiring into the baseline checkout (the baseline lacks the guard entirely):

```bash
cp -r refbox/src/tournament_manager/golden /tmp/gt-baseline/refbox/src/tournament_manager/
```

In `/tmp/gt-baseline/refbox/src/tournament_manager/mod.rs`, add `#[cfg(test)] mod golden;` at the same location it appears on the branch (near the other test-module declarations). Confirm the baseline engine still compiles the guard:

Run: `cd /tmp/gt-baseline && cargo test -p refbox --no-run` → Expected: compiles clean (the guard only uses long-stable public/`pub(super)` API). If it fails to compile, STOP and report which API the baseline lacks — that is a finding, not something to patch around.

- [ ] **Step 2b: Generate the traces from the baseline engine**

```bash
cd /tmp/gt-baseline
UPDATE_GOLDEN=1 cargo test -p refbox golden_traces_match_baseline
```

This writes all traces (existing scenarios with the new columns + `between_games_auto_reset`) into the baseline worktree's `golden_traces/`, recorded from the baseline engine's behavior.

- [ ] **Step 3: Diff baseline-generated traces against the committed ones**

```bash
cd /home/estraily/projects/uwh-refbox-rs
diff -ru refbox/src/tournament_manager/golden_traces /tmp/gt-baseline/refbox/src/tournament_manager/golden_traces
```

Expected: **no differences.** That proves `scores` and `is_old_game` (and the new scenario) behave identically at baseline and HEAD — the new columns are a faithful baseline recording.

If there ARE differences: each one is a behavior change in `scores`/`is_old_game` between baseline and HEAD. Classify each (regression vs. intended) exactly as the per-PR re-bless rule requires, and report to the human before committing — do not silently bless HEAD over baseline.

- [ ] **Step 4: Clean up the worktree**

```bash
git worktree remove --force /tmp/gt-baseline
```

(No commit here if Step 3 showed no differences — the already-committed traces are confirmed baseline-faithful. If differences were found and classified-as-intended, re-bless on the branch and commit with the one-line classification in the message.)

---

## Task 6: Mutation-validate the two new columns

Proves each new column actually guards something: it must kill a mutant that the guard previously could not catch. Use the golden test as the SOLE kill-check. All edits are reverted; nothing here is committed. (cargo-mutants is available, but these two targets are precise enough to do as manual one-line edits.)

**Files:** temporary edits to `refbox/src/tournament_manager/mod.rs`, each reverted immediately.

- [ ] **Step 1: Prove `is_old_game` kills the between-games mutants**

Find the auto-reset condition (currently ~`mod.rs:1180-1182`):

```rust
            if !self.has_reset
                && self.current_period == GamePeriod::BetweenGames
                && self.game_clock_time(now).unwrap_or(Duration::ZERO) <= self.reset_game_time
```

**Mutant A** — change `== GamePeriod::BetweenGames` to `!= GamePeriod::BetweenGames`.
Run: `cargo test -p refbox golden_traces_match_baseline`
Expected: **FAIL** on `between_games_auto_reset` (the `old?` flip changes/disappears).
Revert: `git checkout refbox/src/tournament_manager/mod.rs`

**Mutant B** — change `<= self.reset_game_time` to `> self.reset_game_time`.
Run: `cargo test -p refbox golden_traces_match_baseline`
Expected: **FAIL** on `between_games_auto_reset`.
Revert: `git checkout refbox/src/tournament_manager/mod.rs`

If either mutant SURVIVES (test passes), the scenario does not actually pin the reset — return to Task 4 Step 5 and fix the scenario (it likely isn't reaching the reset). Confirm `git status` is clean after both reverts.

- [ ] **Step 2: Prove `scores` kills a score-path mutant**

In `fn add_score` (`mod.rs`), find `scores[color] += 1;` and change it to `scores[color] += 0;` (the goal never lands).
Run: `cargo test -p refbox golden_traces_match_baseline`
Expected: **FAIL** on `regulation_with_scores` (the `score=` column stays `B0/W0`).
Revert: `git checkout refbox/src/tournament_manager/mod.rs`
Confirm `git status` is clean.

- [ ] **Step 3: Record the result**

No commit. Note in the PR description (or the plan's Deviations section) that both between-games mutants and a score-path mutant were kill-proven by the new columns, and that the tree was confirmed clean after reverts.

---

## Task 7: Update the README and open the PR prep

**Files:**
- Modify: `refbox/src/tournament_manager/golden_traces/README.md`

- [ ] **Step 1: Update the watched-state description**

In `README.md`, update the "what is watched" section to state the watched set is now: period, game clock, timeout (type + clock), confirm-pause, penalties (player# + remaining/TD), **scores (B/W)**, and **is_old_game (`old?` column)**. Note explicitly that warnings, fouls, penalty infraction kind, game numbers, recent_goal, next_period_len_secs, and event_id are intentionally NOT watched (record-keeping / display / non-core), guarded by the `render_accounts_for_every_snapshot_field` completeness test.

- [ ] **Step 2: Final full check (mirrors CI)**

Run: `just check`
Expected: PASS (fmt, lint, tests, audit). If `just check` runs `--all-targets` clippy and surfaces pre-existing test-code lints unrelated to this change, fall back to the CI-equivalent `cargo clippy -p refbox -- -D warnings` and note it.

- [ ] **Step 3: Commit**

```bash
git add refbox/src/tournament_manager/golden_traces/README.md
git commit -m "docs(refbox): document scores/is_old_game in golden-trace watched set"
```

- [ ] **Step 4: Hand back to the human for PR**

Do NOT open the PR automatically. Summarize for the human in plain English: what changed, that the two new columns are mutation-proven, and that all traces are baseline-faithful. The human approves the PR and merges via the merge queue.

---

## Notes / deviations

(Record any execution deviations here per the lean process — do not create standalone deviation commits.)
