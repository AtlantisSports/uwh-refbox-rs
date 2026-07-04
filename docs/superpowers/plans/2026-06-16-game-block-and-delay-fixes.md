# Game Block & Delay Draft-Release Fixes — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix three defects in the v0.4.2 draft release: the behind-schedule "DELAY" showing before the first game, the derived Game Block coming out as 37 min instead of 26 min, and the missing Game Block line on the Game Information page.

**Architecture:** Three independent, small fixes — one in the shared core library (`uwh-common`, the portal→config derivation), one in the game state machine (`refbox` tournament_manager), and one in a `refbox` view builder. Each is its own TDD task and its own commit. Heavy process (per-task verification + downstream checks) because Bug 2 touches `uwh-common` and Bug 1 touches the timing state machine.

**Tech Stack:** Rust 2024, MSRV 1.85, `iced` 0.13, fluent translations, `cargo`/`just`.

**Scope boundary:**
- Bug 2 fix is **only** the portal derivation in `uwh-common/src/uwhportal/schedule.rs`. Do **NOT** change `uwh-common/src/config.rs::Game::migrate()` (line ~162) — its use of `nominal_break` is a different, intentional context (migrating a user's local TOML config, where `nominal_break` is their own configured gap).
- No portal/server changes. No new translation keys (`game-block-info` already exists and is rendered on the main page for all locales).
- No layout/styling changes beyond moving/un-gating the one Game Block line.

**Branch:** worktree `fix+refbox+game-block-and-delay` off `origin/master` (7d208fe9). One PR for the cluster, three clean commits. Note for PR time: touches `uwh-common`, so confirm branch naming / possible split with the human before opening the PR.

---

## File Structure

| File | Change |
|------|--------|
| `uwh-common/src/uwhportal/schedule.rs` | Bug 2: derive `game_block` from `minimum_break` not `nominal_break`; update 2 existing tests + add 1 regression test |
| `refbox/src/tournament_manager/mod.rs` | Bug 1: guard `behind_schedule()` BetweenGames branch on `current_scheduled_start.is_none()`; add 1 test |
| `refbox/src/app/view_builders/game_info.rs` | Bug 3: show `game-block-info` unconditionally, right after team names (remove the `!using_uwhportal`-gated copy) |

---

## Task 1 — Bug 2: derive portal Game Block from the schedule's minimum break

**Why:** When the portal omits `gameBlock`, `TimingRule::into::<GameConfig>()` derives it as `regulation + nominal_break`, but `nominal_break` is pulled from `Default` (900 s = 15 min) because the portal never sends it. The portal *does* send `minimum_break` (the real between-games gap). For the reported schedule: regulation `2*600 + 120 = 1320` + default `900` = `2220`… wait — defaults differ; concretely with the user's numbers regulation `1320` + `nominal_break` default `900` = `37:00`. Correct is regulation `1320` + `minimum_break` `240` = `1560 s = 26:00`.

**Files:**
- Modify: `uwh-common/src/uwhportal/schedule.rs:319-327`
- Test: `uwh-common/src/uwhportal/schedule.rs` (`mod test`, near lines 945-987)

- [ ] **Step 1: Update the two existing derivation tests + add a regression test (these will fail first)**

In `test_timing_rule_game_block_absent_is_derived` (currently ~945-954), change the comment and the expected value:

```rust
    #[test]
    fn test_timing_rule_game_block_absent_is_derived() {
        // Portal payload WITHOUT gameBlock (today's case): derive game_block = regulation + minimum_break.
        let json = r#"{"name":"RR","teamTimeoutCount":1,"teamTimeoutsCountedPerHalf":true,"overtimeAllowed":true,"suddenDeathAllowed":true,"halfPlayDuration":900,"halfTimeDuration":180,"teamTimeoutDuration":60,"overtimeHalfPlayDuration":300,"overtimeHalfTimeDuration":180,"preOvertimeBreak":180,"preSuddenDeathDuration":60,"minimumBreak":240}"#;
        let rule: TimingRule = serde_json::from_str(json).unwrap();
        assert_eq!(rule.game_block, None);
        let config: GameConfig = rule.into();
        // regulation = 2*900 + 180 = 1980; minimum_break = 240 -> 2220
        assert_eq!(config.game_block, Duration::from_secs(2220));
    }
```

In `test_timing_rule_game_block_single_half_derived` (currently ~965-973), change the comment and expected value (regulation = 600 single half, minimum_break = 120 -> 720):

```rust
    #[test]
    fn test_timing_rule_game_block_single_half_derived() {
        // halfTimeDuration == 0 signals single-half; regulation = half_play only.
        let json = r#"{"name":"RR","teamTimeoutCount":0,"teamTimeoutsCountedPerHalf":false,"overtimeAllowed":false,"suddenDeathAllowed":false,"halfPlayDuration":600,"halfTimeDuration":0,"teamTimeoutDuration":0,"overtimeHalfPlayDuration":0,"overtimeHalfTimeDuration":0,"preOvertimeBreak":0,"preSuddenDeathDuration":0,"minimumBreak":120}"#;
        let rule: TimingRule = serde_json::from_str(json).unwrap();
        let config: GameConfig = rule.into();
        // single half: regulation = 600; minimum_break = 120 -> 720
        assert_eq!(config.game_block, Duration::from_secs(720));
    }
```

Add a new regression test directly after `test_timing_rule_game_block_single_half_derived`, encoding the exact reported scenario (10-min halves, 2-min half-time, 4-min gap → 26:00):

```rust
    #[test]
    fn test_timing_rule_game_block_uses_schedule_minimum_break() {
        // Reported bug: 10-min halves + 2-min half-time + 4-min gap must derive a
        // 26:00 Game Block (regulation 1320 + minimum_break 240 = 1560), NOT 37:00
        // (which is regulation + the 15-min default nominal_break).
        let json = r#"{"name":"RR","teamTimeoutCount":0,"teamTimeoutsCountedPerHalf":false,"overtimeAllowed":false,"suddenDeathAllowed":false,"halfPlayDuration":600,"halfTimeDuration":120,"teamTimeoutDuration":0,"overtimeHalfPlayDuration":0,"overtimeHalfTimeDuration":0,"preOvertimeBreak":0,"preSuddenDeathDuration":0,"minimumBreak":240}"#;
        let rule: TimingRule = serde_json::from_str(json).unwrap();
        assert_eq!(rule.game_block, None);
        let config: GameConfig = rule.into();
        assert_eq!(config.game_block, Duration::from_secs(1560)); // 26:00
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p uwh-common uwhportal::schedule::test::test_timing_rule_game_block -- --nocapture`
Expected: FAIL — `test_timing_rule_game_block_absent_is_derived` expects 2220 but gets 2880; the new test expects 1560 but gets 1980+900=… (regulation 1320 + 900 = 2220). (Exact "got" values confirm the old `nominal_break` path is active.)

- [ ] **Step 3: Apply the one-line fix**

In `schedule.rs`, the derivation closure (currently lines 319-327) — change `nominal_break` to `minimum_break` and refresh the comment:

```rust
            game_block: game_block.unwrap_or_else(|| {
                // No portal-sent Game Block: derive from this rule's play durations
                // plus the schedule's own minimum break (the gap the portal packs
                // games at). This matches schedule-processor's slot math. Using the
                // refbox default nominal_break here was the bug: the portal never
                // sends nominal_break, so it injected a 15-min default gap.
                let regulation = if half_time_duration == Duration::ZERO {
                    half_play_duration
                } else {
                    2 * half_play_duration + half_time_duration
                };
                regulation + minimum_break
            }),
```

Note: the `nominal_break` binding (destructured from `Default::default()` higher in the function) is still used to set `GameConfig.nominal_break`, so it stays — no unused-variable warning.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p uwh-common uwhportal::schedule::test::test_timing_rule_game_block -- --nocapture`
Expected: PASS (all 4 game-block derivation tests).

- [ ] **Step 5: Downstream check (uwh-common is high blast radius)**

Run:
```bash
cargo build -p uwh-common --no-default-features   # no_std still compiles
cargo check -p refbox -p schedule-processor -p overlay -p led-panel-sim
cargo test -p uwh-common
```
Expected: all succeed. If any other test asserts a `TimingRule::into()`-derived `game_block`, update it to the `regulation + minimum_break` value and note it in Deviations.

- [ ] **Step 6: Commit**

```bash
git add uwh-common/src/uwhportal/schedule.rs
git commit -m "fix(uwh-common): derive portal game block from minimum break"
```

---

## Task 2 — Bug 1: don't report behind-schedule before the first game starts

**Why:** `behind_schedule()`'s in-game branch already returns `ZERO` when there's no schedule anchor (`current_scheduled_start` is `None`), but the BetweenGames branch does not. Before the first game, `current_scheduled_start` is `None` yet the branch still projects the pre-game break countdown against the scheduled start; clock granularity leaves a sub-minute positive value, rendered as "DELAY -0:0". `current_scheduled_start` is `None` only before the first game has started (set in `start_game` at line ~1055, cleared by `reset()`), so it is the correct guard.

**Files:**
- Modify: `refbox/src/tournament_manager/mod.rs:2079-2090` (the `behind_schedule` BetweenGames branch)
- Test: `refbox/src/tournament_manager/mod.rs` (`mod test`, near the other `test_behind_schedule_*` tests, ~line 3100)

- [ ] **Step 1: Write the failing test**

Add next to the other behind-schedule tests (e.g. after `test_behind_schedule_far_future_portal_time_is_safe`):

```rust
    #[test]
    fn test_behind_schedule_zero_before_first_game() {
        // Pre-first-game: a portal schedule is loaded and the between-games countdown
        // is running toward game 1, but NO game has started yet (current_scheduled_start
        // is None). Even when the projection would compute a positive delta, the figure
        // must be ZERO -- you cannot be "behind" before the first game (Bug: "DELAY -0:0").
        initialize();
        let mut tm = TournamentManager::new(behind_test_config());
        let now = Instant::now();
        assert!(
            tm.current_scheduled_start.is_none(),
            "no game has started yet"
        );
        // In the between-games countdown to the first game, with 20s left on the break...
        tm.set_period_and_game_clock_time(GamePeriod::BetweenGames, Duration::from_secs(20));
        // ...and the grid saying the first game is scheduled 5s from now. Without the
        // guard, projected (now+20) is 15s past the scheduled start (now+5) => 15s behind.
        tm.next_scheduled_start = Some(now + Duration::from_secs(5));
        assert_eq!(tm.behind_schedule(now), Duration::ZERO);
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p refbox tournament_manager::test::test_behind_schedule_zero_before_first_game`
Expected: FAIL — asserts `ZERO` but gets `15s` (the spurious pre-game delta).

- [ ] **Step 3: Add the guard**

In `behind_schedule()`, at the top of the `if self.current_period == GamePeriod::BetweenGames {` block (before the `let Some(sched_next) = ...` line):

```rust
        if self.current_period == GamePeriod::BetweenGames {
            // Before the first game has started there is no schedule anchor yet, so the
            // event cannot be "behind". (The in-game branch below applies the same guard
            // via `current_scheduled_start`.) Without this, the pre-game break countdown
            // projects ~= the scheduled start and clock granularity leaks a sub-minute
            // positive value, shown as "DELAY -0:0".
            if self.current_scheduled_start.is_none() {
                return Duration::ZERO;
            }
            // Project the next game's start from the *live* break clock...
            let Some(sched_next) = self.next_game_scheduled_start(now) else {
                return Duration::ZERO;
            };
```

(Leave the rest of the branch unchanged.)

- [ ] **Step 4: Run the new test + the full behind-schedule suite to verify**

Run:
```bash
cargo test -p refbox tournament_manager::test::test_behind_schedule_zero_before_first_game
cargo test -p refbox tournament_manager::test::test_behind_schedule
```
Expected: PASS, including all pre-existing `test_behind_schedule_*` tests (they start a game first, so `current_scheduled_start` is `Some` and the guard does not fire).

- [ ] **Step 5: Commit**

```bash
git add refbox/src/tournament_manager/mod.rs
git commit -m "fix(refbox): hide behind-schedule delay before first game"
```

---

## Task 3 — Bug 3: show Game Block on the Game Information page (match the main page)

**Why:** The main page (`shared_elements.rs:911-917`) shows `game-block-info` unconditionally; the Game Information page (`game_info.rs:253-259`) shows it only when `!using_uwhportal`, so a portal schedule (normal tournament case) hides it. Human-approved placement: right after the team names, before "Half Length" — matching the main page.

**Files:**
- Modify: `refbox/src/app/view_builders/game_info.rs` (insert after the team-names block ~line 193; remove the gated copy at ~253-259)

This is a view-only change (no behaviour, no new translation key). Verified by build + running the app.

- [ ] **Step 1: Remove the `!using_uwhportal`-gated Game Block block**

Delete these lines (currently ~253-259):

```rust
    if !using_uwhportal {
        left_string += &fl!(
            "game-block-info",
            game_block = time_string(config.game_block)
        );
        left_string += "\n";
    }
```

- [ ] **Step 2: Add the Game Block line right after the team-names block**

The team-names block ends at the close of `if using_uwhportal { if let Some(games) = games { ... } }` (~line 193), immediately before the `left_string += &if config.single_half { ... game-length-ot-allowed ... }` block (~line 195). Insert between them:

```rust
    // Game Block (the start-to-start slot) sits right after the team names and before
    // the play-length lines -- matching the main page layout.
    left_string += &fl!("game-block-info", game_block = time_string(config.game_block));
    left_string += "\n";

```

- [ ] **Step 3: Build to verify it compiles**

Run: `cargo check -p refbox`
Expected: success, no warnings.

- [ ] **Step 4: Commit**

```bash
git add refbox/src/app/view_builders/game_info.rs
git commit -m "fix(refbox): show game block on game-information page"
```

---

## Task 4 — Full verification

- [ ] **Step 1: Run the full check suite**

Run: `just check` (fmt-check, lint with `-D warnings`, test, audit)
Expected: all green. If `just fmt-check` flags formatting, run `just fmt` and amend the relevant commit.

- [ ] **Step 2: Manual app verification (the human can observe these)**

Launch the app on the reported schedule (10-min halves, 2-min half-time, 4-min gap, no OT/SD/timeouts) and confirm:
1. Main page no longer shows "DELAY -0:0" before the first game has started.
2. Main page shows **Game Block: 26:00** (was 37:00).
3. Game Information page now shows **Game Block: 26:00**, positioned right after the team names (before "Half Length").
4. Once a game is started and genuinely runs long, the DELAY figure still appears (the guard only suppresses the pre-first-game case).

Launch (per project run rules — native WSLg needs X11):
```bash
WAYLAND_DISPLAY= cargo run -p refbox
```

- [ ] **Step 3: Code review**

Run `superpowers:requesting-code-review` on the three-commit diff before opening the PR.

---

## Self-Review (author checklist — completed at write time)

- **Spec coverage:** Bug 1 → Task 2; Bug 2 → Task 1; Bug 3 → Task 3. Verification → Task 4. ✓
- **Placeholders:** none — all test code, edits, and commands are concrete. ✓
- **Type/name consistency:** `game_block`, `minimum_break`, `nominal_break`, `current_scheduled_start`, `behind_schedule`, `game-block-info`, `behind_test_config`, `next_scheduled_start`, `set_period_and_game_clock_time` all match the source as read. ✓
- **Guardrail:** explicit instruction NOT to touch `config.rs::Game::migrate()` (different, intentional `nominal_break` context). ✓

## Deviations

(Record any divergence here during execution.)
