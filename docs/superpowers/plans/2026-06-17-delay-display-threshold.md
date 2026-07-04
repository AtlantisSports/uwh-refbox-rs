# Delay Display Threshold Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show the behind-schedule DELAY figure as *genuine, unrecoverable* delay — the raw tally with the slot's spare time discounted — instead of the raw tally, without touching the timing engine's math.

**Architecture:** Add one read-only derived accessor `TournamentManager::behind_schedule_shown(now)` next to the existing `behind_schedule`. During a game it returns `behind_schedule − game_block_buffer` (floored at zero); between games it returns `behind_schedule` unchanged (the engine already applied the slack step-down there, so discounting again would double-count). The single UI call site switches to the new accessor. Raw `behind_schedule`, the wire format, and the golden-trace guard are untouched.

**Tech Stack:** Rust 2024 (MSRV 1.85), `refbox` crate, `iced` 0.13 UI. Spare-time value is the existing `uwh_common::config::Game::game_block_buffer()`.

**Spec:** `docs/superpowers/specs/2026-06-17-delay-display-threshold-design.md`

---

## Approval gates (project rule — do not skip)

The human is a non-programmer and must approve **before** a branch is created and **before** each
commit (see `.claude/rules/communication.md`). At execution time:

1. `git fetch origin master` (local master is routinely stale here).
2. Get approval, then create an isolated worktree on a new branch off **origin/master**:
   `feat/refbox/delay-display-threshold` (use `superpowers:using-git-worktrees`).
3. The detached investigation worktree at `.worktrees/delay-timer-investigation` can be removed
   (`git worktree remove`) — it was read-only.
4. Present each commit's plain-language summary (what / why / how to verify) and wait for approval
   before committing.

---

## File structure

- **Modify:** `refbox/src/tournament_manager/mod.rs`
  - Add the `behind_schedule_shown` method (production), immediately after `behind_schedule`
    (currently ends at line 2149).
  - Add unit tests in the existing `#[cfg(test)] mod` (alongside the `test_behind_schedule_*`
    tests, near line 3410).
- **Modify:** `refbox/src/app/mod.rs`
  - Swap the single DELAY call site (currently line 4293) from `behind_schedule` to
    `behind_schedule_shown`.

No other files change. No `uwh-common`, `overlay`, wire-format, config-schema, or translation
changes.

---

## Task 1: Add `behind_schedule_shown` (TDD)

**Files:**
- Modify: `refbox/src/tournament_manager/mod.rs` (add method after `behind_schedule`, ~line 2149)
- Test: `refbox/src/tournament_manager/mod.rs` (`#[cfg(test)] mod`, near line 3410)

Reference facts for the test math (both helpers already exist in the test module):
- `behind_test_config()` → `regulation_play = 2×10+3 = 23s`, `minimum_break = 2s`,
  `game_block = 40s` ⇒ `game_block_buffer = 40−23−2 = 15s`.
- `behind_real_slack_config()` → `regulation_play = 2×60+10 = 130s`, `minimum_break = 5s`,
  `game_block = 180s` ⇒ `game_block_buffer = 180−135 = 45s`.

- [ ] **Step 1: Write the failing tests**

Add these five tests inside the `#[cfg(test)]` module, right after
`test_behind_schedule_climbs_during_team_timeout` (~line 3410):

```rust
#[test]
fn test_behind_schedule_shown_blanks_team_timeout_within_slack() {
    // The user's case: a team timeout within the slot's spare time must NOT surface as
    // delay. Raw figure climbs (existing behaviour) but the shown figure stays blank.
    initialize();
    let mut tm = TournamentManager::new(behind_real_slack_config()); // slack = 45s
    let start = Instant::now();
    tm.start_clock(start);
    tm.start_play_now(start).unwrap();
    let to_at = start + Duration::from_secs(5);
    tm.start_team_timeout(Color::Black, to_at).unwrap();
    // Raw climbs to 15 and 25, both within the 45s slack -> shown stays zero (blank).
    assert_eq!(
        tm.behind_schedule(to_at + Duration::from_secs(15)),
        Duration::from_secs(15)
    );
    assert_eq!(
        tm.behind_schedule_shown(to_at + Duration::from_secs(15)),
        Duration::ZERO
    );
    assert_eq!(
        tm.behind_schedule_shown(to_at + Duration::from_secs(25)),
        Duration::ZERO
    );
}

#[test]
fn test_behind_schedule_shown_shows_excess_beyond_slack() {
    // Once the raw tally exceeds the slot's spare time, the shown figure is the excess.
    initialize();
    let mut tm = TournamentManager::new(behind_test_config()); // slack = 15s
    let start = Instant::now();
    tm.start_clock(start);
    tm.start_play_now(start).unwrap();
    let pause_at = start + Duration::from_secs(5);
    tm.stop_clock(pause_at).unwrap();
    let t = pause_at + Duration::from_secs(20);
    // Raw 20, slack 15 -> shown excess = 5.
    assert_eq!(tm.behind_schedule(t), Duration::from_secs(20));
    assert_eq!(tm.behind_schedule_shown(t), Duration::from_secs(5));
}

#[test]
fn test_behind_schedule_shown_continuous_across_game_end() {
    // The in-game discount previews the engine's break step-down, so the shown figure
    // does NOT jump at the end of a game (and the break is not double-discounted).
    initialize();
    let mut tm = TournamentManager::new(behind_test_config()); // slack = 15s
    let start = Instant::now();
    tm.start_clock(start);
    tm.start_play_now(start).unwrap();
    tm.update(start + Duration::from_secs(10)).unwrap(); // FirstHalf -> HalfTime
    tm.update(start + Duration::from_secs(13)).unwrap(); // HalfTime -> SecondHalf
    tm.stop_clock(start + Duration::from_secs(23)).unwrap();
    let end = start + Duration::from_secs(50);
    // In-game: raw 27, shown 27 - 15 = 12.
    assert_eq!(tm.behind_schedule(end), Duration::from_secs(27));
    let shown_in_game = tm.behind_schedule_shown(end);
    assert_eq!(shown_in_game, Duration::from_secs(12));
    tm.end_game(end);
    // Between games: raw already stepped down to 12; shown unchanged = 12.
    assert_eq!(tm.behind_schedule(end), Duration::from_secs(12));
    let shown_between = tm.behind_schedule_shown(end);
    assert_eq!(shown_between, Duration::from_secs(12));
    // Smooth across the boundary -> no jump.
    assert_eq!(shown_in_game, shown_between);
}

#[test]
fn test_behind_schedule_shown_equals_raw_with_no_slack() {
    // A slot with no spare time has nothing to discount: shown == raw (today's behaviour).
    initialize();
    let mut config = behind_test_config();
    config.game_block = Duration::from_secs(25); // == regulation_play(23) + minimum_break(2) => slack 0
    let mut tm = TournamentManager::new(config);
    let start = Instant::now();
    tm.start_clock(start);
    tm.start_play_now(start).unwrap();
    let pause_at = start + Duration::from_secs(5);
    tm.stop_clock(pause_at).unwrap();
    let t = pause_at + Duration::from_secs(20);
    assert_eq!(tm.behind_schedule_shown(t), tm.behind_schedule(t));
    assert_eq!(tm.behind_schedule_shown(t), Duration::from_secs(20));
}

#[test]
fn test_behind_schedule_shown_blanks_recoverable_late_start() {
    // A game that starts late but whose slot can recover it shows blank (stay-blank rule).
    initialize();
    let mut tm = TournamentManager::new(behind_test_config()); // slack = 15s
    let g1 = Instant::now();
    tm.start_clock(g1);
    tm.start_play_now(g1).unwrap();
    tm.stop_clock(g1).unwrap();
    tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::from_secs(0));
    tm.end_game(g1);
    let g2 = g1 + Duration::from_secs(46);
    tm.start_play_now(g2).unwrap(); // game 2 begins 6s late vs its 40s slot
    // Raw shows the 6s late start; slack 15 can recover it -> shown blank.
    assert_eq!(tm.behind_schedule(g2), Duration::from_secs(6));
    assert_eq!(tm.behind_schedule_shown(g2), Duration::ZERO);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p refbox behind_schedule_shown`
Expected: **compile error** — `no method named `behind_schedule_shown` found for ... TournamentManager`.

- [ ] **Step 3: Write the minimal implementation**

Insert this method immediately after the closing brace of `behind_schedule` (after line 2149,
before `timeout_clock_time`):

```rust
    /// The behind-schedule figure as shown to the operator: the genuine, unrecoverable
    /// delay. During a game the slot's spare time (`game_block_buffer`) is discounted --
    /// this previews the exact step-down the engine applies at the break, so the figure
    /// stays blank while the slot can still absorb the loss and is continuous across the
    /// end of a game. Between games the engine has already applied that step-down, so the
    /// raw figure is returned unchanged (no double discount). See
    /// docs/superpowers/specs/2026-06-17-delay-display-threshold-design.md.
    pub fn behind_schedule_shown(&self, now: Instant) -> Duration {
        let raw = self.behind_schedule(now);
        if self.current_period == GamePeriod::BetweenGames {
            raw
        } else {
            raw.saturating_sub(self.config.game_block_buffer())
        }
    }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p refbox behind_schedule_shown`
Expected: **PASS** — all 5 new tests green.

- [ ] **Step 5: Confirm the existing timing tests are untouched**

Run: `cargo test -p refbox behind_schedule`
Expected: **PASS** — all existing `test_behind_schedule_*` tests still green (the raw figure is unchanged).

- [ ] **Step 6: Commit (after human approval)**

```bash
git add refbox/src/tournament_manager/mod.rs
git commit -m "feat(refbox): add behind_schedule_shown discounting slot spare time"
```

---

## Task 2: Show the discounted figure in the UI

**Files:**
- Modify: `refbox/src/app/mod.rs` (the DELAY call site, currently line 4293)

- [ ] **Step 1: Swap the call site**

Find this block (currently at lines 4292-4296):

```rust
                let behind_schedule = if self.config.show_behind_schedule_time {
                    self.tm.lock().unwrap().behind_schedule(Instant::now())
                } else {
                    std::time::Duration::ZERO
                };
```

Change the one inner line to call the new accessor:

```rust
                let behind_schedule = if self.config.show_behind_schedule_time {
                    self.tm.lock().unwrap().behind_schedule_shown(Instant::now())
                } else {
                    std::time::Duration::ZERO
                };
```

- [ ] **Step 2: Verify it compiles cleanly**

Run: `cargo clippy -p refbox -- -D warnings`
Expected: **no warnings, no errors** (mirrors CI / `just lint` for this bin crate).

- [ ] **Step 3: Commit (after human approval)**

```bash
git add refbox/src/app/mod.rs
git commit -m "feat(refbox): display only unrecoverable delay beyond slot spare time"
```

---

## Task 3: Full validation

- [ ] **Step 1: Run the full check suite**

Run: `just check`
Expected: fmt, lint, tests, audit all clean.

- [ ] **Step 2: Manual walkthrough (the human drives the UI)**

Build and launch (per project run conventions — `WAYLAND_DISPLAY=` prefix on WSL, background
launch the built binary):

```bash
cargo build -p refbox
```

Confirm the acceptance criteria by observation:
1. On-time game; take a team timeout shorter than the slot's spare time → DELAY stays **blank**
   during and after the timeout.
2. Build stoppage time beyond the slot's spare time → DELAY shows only the **excess**.
3. A game that starts late but whose slot can recover it → DELAY stays **blank**.
4. DELAY does **not jump** at the end of a game.

To exercise these quickly, a config with small durations and a large Game Block gives a big,
easy-to-see spare-time window.

---

## Self-review notes

- **Spec coverage:** display rule (Task 1 method), in-game discount + between-games unchanged
  (method body + Test 3), excess-only (Test 2), stay-blank late start (Test 5), no-slack
  degeneracy (Test 4), UI wired (Task 2), unchanged look/toggle (no view/config changes made).
  All spec sections map to a task.
- **No placeholders:** every code and command step is concrete.
- **Type consistency:** `behind_schedule_shown(&self, now: Instant) -> Duration` is referenced
  identically in the method, all five tests, and the UI call site; `game_block_buffer()` and
  `GamePeriod::BetweenGames` already exist and are in scope in `mod.rs`.
