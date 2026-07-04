# Portal → Manual Clean-Slate Reset Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When the operator turns the UWH Portal setting off, return the refbox to a fresh-manual-launch state — clear the loaded event/court/game/schedule and reset the before-game clock to the nominal break — gated mid-game by the existing confirmation prompt.

**Architecture:** A small new routine in the game-clock engine (`tournament_manager`) does the clock-touching reset and is covered by engine unit tests. The app layer detects the portal off-switch in the Game-Options apply path, clears the on-screen selections, and calls the engine routine — directly between games, or behind a new mid-game confirmation that reuses the existing End/Keep/Discard/GoBack options. ADR 017 is amended to record the reversed off-switch behavior.

**Tech Stack:** Rust 2024, `iced` 0.13 GUI, `fluent`-based translations (`fl!`), `time`/`std::time` durations.

**Spec:** `docs/superpowers/specs/2026-06-22-portal-off-manual-reset-design.md`

## Global Constraints

- MSRV Rust 1.85; edition 2024. Do not use newer language/stdlib features.
- Clippy clean: `cargo clippy -p refbox -- -D warnings` (refbox is bin-only; do NOT use `--all-targets` locally — it surfaces ~90 pre-existing test lints that are not CI failures).
- Tests for refbox: `cargo test -p refbox` (no `--lib`).
- No `unwrap()`/`expect()` in non-test production code without a justifying comment.
- Every new user-facing translation key must be added to ALL 15 locale `.ftl` files with a best-guess translation — never leave an English placeholder.
- Heavy process: `tournament_manager` changes require `just test` green and a golden-trace check before completion.
- Scope is the `refbox` crate only. Do not touch `uwh-common`, the wire format, or any other crate.
- Rebuild the actual binary (`cargo build -p refbox`) before any manual walkthrough — `just check`/clippy build a different test binary.

---

## File Structure

- `refbox/src/tournament_manager/mod.rs` — new engine routines `reset_to_manual_break` and `clear_portal_next_game`, plus their unit tests (in the existing `#[cfg(test)] mod tests`).
- `refbox/src/app/mod.rs` — new `ConfirmationKind::SwitchToManualFromApply` variant; off-switch detection in `apply_game_options`; new arm in `apply_game_confirmation`; staged-selection clear in the portal toggle handler.
- `refbox/src/app/view_builders/confirmation.rs` — header text + button set for the new confirmation kind.
- `refbox/translations/*/*.ftl` (15 locales) — one new header key.
- `docs/decisions/017-portal-data-lifecycle.md` — amendment recording the reversed ON → OFF behavior.

---

### Task 1: Engine routines `clear_portal_next_game` and `reset_to_manual_break`

**Files:**
- Modify: `refbox/src/tournament_manager/mod.rs` (add two public methods near `clear_scheduled_game_start`, ~line 166)
- Test: `refbox/src/tournament_manager/mod.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Produces:
  - `pub fn clear_portal_next_game(&mut self)` — sets `next_game = None` and `next_scheduled_start = None`; does NOT touch the clock.
  - `pub fn reset_to_manual_break(&mut self)` — calls `clear_portal_next_game()`, then sets `clock_state = ClockState::Stopped { clock_time: self.config.nominal_break }`. Precondition: caller is in `BetweenGames`.

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module (these access private fields directly, like the existing `test_between_game_timing` at ~line 2734; `OffsetDateTime` is already imported in the test module):

```rust
#[test]
fn test_clear_portal_next_game_leaves_clock_untouched() {
    initialize();
    let mut tm = TournamentManager::new(GameConfig::default());
    let now = Instant::now();
    tm.start_clock(now);
    tm.start_play_now(now).unwrap(); // FirstHalf, clock running, next_scheduled_start set
    tm.set_next_game(NextGameInfo {
        number: "5".to_string(),
        timing: None,
        start_time: Some(OffsetDateTime::now_utc()),
    });
    let before = tm.clock_state;

    tm.clear_portal_next_game();

    assert!(tm.next_game.is_none(), "next_game should be cleared");
    assert_eq!(tm.next_scheduled_start, None, "grid slot should be cleared");
    assert_eq!(tm.clock_state, before, "clock must not change");
}

#[test]
fn test_reset_to_manual_break_sets_nominal_break_and_clears_schedule() {
    initialize();
    let config = GameConfig {
        nominal_break: Duration::from_secs(180),
        ..Default::default()
    };
    let mut tm = TournamentManager::new(config);
    // Load portal-style next-game info and a grid slot, as if a schedule were active.
    tm.set_next_game(NextGameInfo {
        number: "5".to_string(),
        timing: None,
        start_time: Some(OffsetDateTime::now_utc()),
    });
    tm.set_game_start(Instant::now()); // test helper sets next_scheduled_start = Some(..)

    tm.reset_to_manual_break();

    assert!(tm.next_game.is_none(), "next_game should be cleared");
    assert_eq!(tm.next_scheduled_start, None, "grid slot should be cleared");
    assert_eq!(
        tm.clock_state,
        ClockState::Stopped { clock_time: Duration::from_secs(180) },
        "clock should be stopped at the nominal break",
    );
}

#[test]
fn test_kept_game_break_falls_back_to_nominal_after_clear() {
    initialize();
    let config = GameConfig {
        half_play_duration: Duration::from_secs(10),
        half_time_duration: Duration::from_secs(3),
        nominal_break: Duration::from_secs(30),
        minimum_break: Duration::from_secs(2),
        game_block: Duration::from_secs(40),
        overtime_allowed: false,
        sudden_death_allowed: false,
        ..Default::default()
    };
    let mut tm = TournamentManager::new(config);
    let now = Instant::now();
    tm.start_clock(now);
    tm.start_play_now(now).unwrap(); // next_scheduled_start = now + 40 (grid)
    tm.clear_portal_next_game(); // operator switched to manual, kept the running game
    tm.stop_clock(now).unwrap();
    tm.set_period_and_game_clock_time(GamePeriod::SecondHalf, Duration::ZERO);
    tm.end_game(now);

    assert_eq!(tm.current_period(), GamePeriod::BetweenGames);
    // No leftover grid/portal time: the break falls back to the nominal break (30s), not 40s.
    assert_eq!(tm.game_clock_time(now), Some(Duration::from_secs(30)));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p refbox reset_to_manual_break clear_portal_next_game kept_game_break`
Expected: FAIL — `no method named clear_portal_next_game` / `reset_to_manual_break`.

- [ ] **Step 3: Implement the two methods**

Add near `clear_scheduled_game_start` (~line 166):

```rust
/// Drop any loaded next-game info and Game Block grid slot WITHOUT touching the
/// running clock. Used on KeepGameAndApply when switching to manual mid-game: the
/// in-progress game keeps running, and when it ends the between-games break falls
/// back to the nominal break.
pub fn clear_portal_next_game(&mut self) {
    self.next_game = None;
    self.next_scheduled_start = None;
}

/// Return to the fresh-manual-launch before-game state: forget any loaded next-game
/// info and the Game Block grid slot, and stop the clock at the nominal break.
/// Precondition: called only in `BetweenGames` (the apply path and the
/// EndGameAndApply confirmation guarantee this).
pub fn reset_to_manual_break(&mut self) {
    self.clear_portal_next_game();
    self.clock_state = ClockState::Stopped {
        clock_time: self.config.nominal_break,
    };
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p refbox reset_to_manual_break clear_portal_next_game kept_game_break`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add refbox/src/tournament_manager/mod.rs
git commit -m "feat(refbox): add reset_to_manual_break + clear_portal_next_game engine routines"
```

---

### Task 2: New confirmation kind + view (header text and buttons)

**Files:**
- Modify: `refbox/src/app/mod.rs` (`enum ConfirmationKind`, ~line 322)
- Modify: `refbox/src/app/view_builders/confirmation.rs` (header `match` ~line 24, buttons `match` ~line 51)
- Modify: `refbox/translations/*/*.ftl` (15 locales) — new header key

**Interfaces:**
- Produces: `ConfirmationKind::SwitchToManualFromApply` (unit variant). Offers buttons `GoBack`, `DiscardChanges`, `KeepGameAndApply`, `EndGameAndApply` — the same set as `GameNumberChangedFromApply`.

- [ ] **Step 1: Add the variant**

In `enum ConfirmationKind` (after `UwhPortalIncompleteFromApply`, ~line 329):

```rust
    // Raised by per-page Apply on Game Options when the operator turns the portal
    // OFF mid-game. Switching to manual clears the loaded schedule and resets the
    // before-game clock to the nominal break; this confirms whether to end the
    // current game first or keep it running.
    SwitchToManualFromApply,
```

- [ ] **Step 2: Add the new translation key to all 15 locales**

Add key `apply-switch-to-manual` to every `refbox/translations/*/*.ftl`. English (`en`/`en-US`) value:

```
apply-switch-to-manual = Switching to manual will clear the loaded schedule and reset the time before the next game. A game is in progress.
```

Provide a best-guess translation in each of the other 14 locales (mirror the tone of the neighboring `apply-this-game-number-change` / `game-configuration-can-not-be-changed` keys). Do not leave English placeholders.

- [ ] **Step 3: Render header text**

In `build_confirmation_page`'s `header_text` match (`confirmation.rs` ~line 24), add:

```rust
        ConfirmationKind::SwitchToManualFromApply => fl!("apply-switch-to-manual"),
```

- [ ] **Step 4: Render buttons**

In the `buttons` match (~line 51), add an arm identical in shape to `GameNumberChangedFromApply` (GoBack / DiscardChanges / KeepGameAndApply / EndGameAndApply), reusing existing button keys:

```rust
        ConfirmationKind::SwitchToManualFromApply => vec![
            (fl!("go-back-to-editor"), green_button, ConfirmationOption::GoBack),
            (fl!("discard-changes"), yellow_button, ConfirmationOption::DiscardChanges),
            (
                fl!("keep-current-game-and-apply-change"),
                orange_button,
                ConfirmationOption::KeepGameAndApply,
            ),
            (
                fl!("end-current-game-and-apply-change"),
                red_button,
                ConfirmationOption::EndGameAndApply,
            ),
        ],
```

- [ ] **Step 5: Verify it compiles (exhaustive match check)**

Run: `cargo build -p refbox`
Expected: builds clean. If any other `match` on `ConfirmationKind` is non-exhaustive (e.g. the `matches!` guard near `mod.rs:3368`), add `SwitchToManualFromApply` there too — see Task 4.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/app/mod.rs refbox/src/app/view_builders/confirmation.rs refbox/translations
git commit -m "feat(refbox): add SwitchToManualFromApply confirmation kind + view + locales"
```

---

### Task 3: Detect the portal off-switch in `apply_game_options`

**Files:**
- Modify: `refbox/src/app/mod.rs` (`apply_game_options`, ~line 941)

**Interfaces:**
- Consumes: `self.using_uwhportal` (prior committed value, still unchanged at function entry), `edited.using_uwhportal`, `tm.reset_to_manual_break()` (Task 1), `ConfirmationKind::SwitchToManualFromApply` (Task 2).
- Produces: a `switching_to_manual` branch handled before the existing config/game-number branches.

**Implementation notes (mirror the surrounding patterns — `std::mem::drop(tm)` before `&mut self` calls, route event id through `set_current_event_id`):**

Near the top of `apply_game_options`, after `edited` is bound and `tm` is locked, capture the transition and handle it first:

```rust
let switching_to_manual = self.using_uwhportal && !edited.using_uwhportal;
if switching_to_manual {
    if tm.current_period() != GamePeriod::BetweenGames {
        return Some(ConfirmationKind::SwitchToManualFromApply);
    }
    // Between games: commit the clean manual slate directly.
    tm.set_config(edited.config.clone()).unwrap(); // safe: BetweenGames checked above
    tm.reset_to_manual_break();
    std::mem::drop(tm);
    self.commit_switch_to_manual(edited_config_snapshot, /* see below */);
    return None;
}
```

Because the exact borrow choreography (dropping the `edited` borrow before `&mut self` calls) matches the existing config-change branch at `mod.rs:990-1006`, factor the app-state clearing into a helper so both the direct path (this task) and the confirmation arms (Task 4) share it:

```rust
/// Clear the on-screen portal selections back to a fresh manual slate. The TM-side
/// reset (clock + next-game) is done separately by the caller via the engine routines.
/// Keeps `config.uwhportal.token` (no logout).
fn clear_portal_selections_to_manual(&mut self) {
    self.using_uwhportal = false;
    self.set_current_event_id(None); // keeps portal_event_id handle in sync (ADR 011)
    self.current_court = None;
    self.schedule = None;
    self.config.game = /* the committed manual config */;
    self.persist_config();
}
```

The executor decides whether to thread the manual config in as a parameter or read it from `self`/`edited` before the borrow ends — follow whichever matches the existing branch's style. Keep `self.game_number`/TM game number at the manual default (do not blank it; matching a fresh launch).

- [ ] **Step 1: Implement the branch + helper** (code above; adapt borrows to mirror `mod.rs:990-1006`).
- [ ] **Step 2: Build**

Run: `cargo build -p refbox`
Expected: builds clean.

- [ ] **Step 3: Manual check (between-games path)**

`cargo build -p refbox` then run it (see run rules). Load a portal schedule between games, turn the portal off, Apply. Confirm: event/court/game cleared, before-game clock shows the nominal break (stopped), still logged in.

- [ ] **Step 4: Commit**

```bash
git add refbox/src/app/mod.rs
git commit -m "feat(refbox): reset to manual slate when portal toggled off between games"
```

---

### Task 4: Handle the mid-game confirmation in `apply_game_confirmation`

**Files:**
- Modify: `refbox/src/app/mod.rs` (`apply_game_confirmation`, ~line 1052; the `matches!` guard near ~line 3368)

**Interfaces:**
- Consumes: `ConfirmationKind::SwitchToManualFromApply`, `tm.reset_game(now)`, `tm.reset_to_manual_break()`, `tm.clear_portal_next_game()`, `self.clear_portal_selections_to_manual()` (Task 3).

**Behavior for the new kind:**
- `EndGameAndApply` → `tm.reset_game(now)` (ends game → BetweenGames), then `tm.reset_to_manual_break()` (overrides reset_game's minimum-break clock with the nominal break), then clear selections, regenerate snapshot, land back in settings.
- `KeepGameAndApply` → `tm.clear_portal_next_game()` (no clock touch), then clear selections, regenerate snapshot. The running game is untouched; its end-of-game break falls back to the nominal break.
- `DiscardChanges` / `GoBack` → same as the existing arms (revert / return to editor); no TM change.

- [ ] **Step 1: Extend the guard** that recognizes which confirmation kinds route into `apply_game_confirmation` (near `mod.rs:3368`): add `| ConfirmationKind::SwitchToManualFromApply` to the matched set.

- [ ] **Step 2: Add handling** inside `apply_game_confirmation`. The function currently special-cases the `new_config` extraction for `GameConfigChangedFromApply`; add a parallel branch for `SwitchToManualFromApply`. Sketch for the two action options (mirror the existing `EndGameAndApply`/`KeepGameAndApply` arms' drop/snapshot/persist choreography at `mod.rs:1079-1135`):

```rust
// inside the SwitchToManualFromApply handling:
ConfirmationOption::EndGameAndApply => {
    let now = Instant::now();
    {
        let mut tm = self.tm.lock().unwrap();
        tm.reset_game(now);
        tm.reset_to_manual_break();
    }
    self.clear_portal_selections_to_manual();
    let new_snapshot = self.tm.lock().unwrap().generate_snapshot(now).unwrap();
    task = self.apply_snapshot(new_snapshot);
    AppState::EditGameConfig(ConfigPage::Main)
}
ConfirmationOption::KeepGameAndApply => {
    {
        let mut tm = self.tm.lock().unwrap();
        tm.clear_portal_next_game();
    }
    self.clear_portal_selections_to_manual();
    let new_snapshot = self.tm.lock().unwrap().generate_snapshot(Instant::now()).unwrap();
    task = self.apply_snapshot(new_snapshot);
    AppState::EditGameConfig(ConfigPage::Main)
}
```

Keep `DiscardChanges`/`GoBack` behaving exactly as the existing shared arms. The executor reconciles this with the function's existing structure (it may be cleaner to branch on the `ConfirmationKind` once at the top than to duplicate the option `match`).

- [ ] **Step 3: Build**

Run: `cargo build -p refbox`
Expected: builds clean.

- [ ] **Step 4: Manual check (mid-game paths)**

Run the binary. Start a game (clock running), enter Game Options, turn the portal off, Apply → confirmation appears.
- Choose **End game & apply** → game ends, clean manual slate, clock at nominal break.
- Repeat, choose **Keep game & apply** → game keeps running; let it end → next break is the nominal break.
- Repeat, choose **Discard** / **Go back** → nothing changes.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/app/mod.rs
git commit -m "feat(refbox): handle mid-game switch-to-manual confirmation (end vs keep)"
```

---

### Task 5: Clear staged selections when the portal toggle flips off

**Files:**
- Modify: `refbox/src/app/mod.rs` (toggle handler `BoolGameParameter::UsingUwhPortal`, ~line 3199)

**Interfaces:**
- Consumes: `edited_settings` (the staged edit). Mirrors the existing OFF → ON blank-slate block.

- [ ] **Step 1: Add the symmetric OFF branch**

The existing block handles `!was_using && using` (OFF → ON). Add the inverse so the editor reflects manual immediately:

```rust
                                if was_using && !edited_settings.using_uwhportal {
                                    // ON -> OFF: switching to manual is a clean slate
                                    // (reverses ADR 017's "no proactive clearing").
                                    edited_settings.current_event_id = None;
                                    edited_settings.current_court = None;
                                    edited_settings.schedule = None;
                                    edited_settings.game_number = String::new();
                                }
```

(Leave `config.uwhportal.token` and `uwhportal_token_valid` untouched — no logout. Do not set `trigger_event_list_fetch`.)

- [ ] **Step 2: Build + quick check**

Run: `cargo build -p refbox`. Run the binary; toggle the portal off in Game Options and confirm the event/court/game fields visibly clear in the editor before Apply.

- [ ] **Step 3: Commit**

```bash
git add refbox/src/app/mod.rs
git commit -m "feat(refbox): clear staged portal selections when toggled off in editor"
```

---

### Task 6: Amend ADR 017

**Files:**
- Modify: `docs/decisions/017-portal-data-lifecycle.md`

- [ ] **Step 1: Add an amendment** to the *"Cached data on toggle transitions"* section recording the reversal:

```markdown
### Amendment 2026-06-22 — ON → OFF is now a clean wipe

The original ON → OFF decision ("no proactive clearing") is reversed. Switching the
portal off now returns the refbox to a fresh-manual-launch state: the loaded event,
court, game, and schedule are cleared, and the before-game clock is reset to the
nominal break (`TournamentManager::reset_to_manual_break`). The saved portal token is
kept (this is not a logout). Mid-game, the switch is gated by the
`SwitchToManualFromApply` confirmation (End game & apply / Keep game & apply), matching
other mid-game parameter changes. Rationale: a leftover portal-scheduled start time
silently driving the manual countdown is confusing; "switch to manual = clean slate" is
more predictable. The original network-cost rationale is unaffected (no fetches fire
while the portal is off). See `docs/superpowers/specs/2026-06-22-portal-off-manual-reset-design.md`.
```

- [ ] **Step 2: Commit**

```bash
git add docs/decisions/017-portal-data-lifecycle.md
git commit -m "docs(refbox): amend ADR 017 — portal off-switch is now a clean wipe"
```

---

### Task 7: Full verification

- [ ] **Step 1: Full test suite**

Run: `just test`
Expected: all tests pass (including the 3 new engine tests).

- [ ] **Step 2: Golden-trace check**

Run: `just test` covers the golden traces; confirm zero drift in the golden-trace tests. Expected: unchanged (the golden scenarios do not toggle the portal). If any golden test fails, STOP — that indicates the reset unexpectedly altered time progression; investigate before proceeding.

- [ ] **Step 3: Lint + format**

Run: `cargo clippy -p refbox -- -D warnings` then `just fmt-check`
Expected: clean.

- [ ] **Step 4: Translation coverage check**

Confirm `apply-switch-to-manual` exists with a non-English-placeholder value in all 15 locale `.ftl` files (diff each locale's value against `en-US`).

- [ ] **Step 5: Full manual walkthrough** (rebuild first: `cargo build -p refbox`)

Walk the three acceptance scenarios from the spec:
1. Between games: load schedule → portal off → clean slate, nominal-break clock (stopped).
2. Mid-game: portal off → confirmation → End (ends game, nominal break) / Keep (game runs, nominal break after) / Discard (no change).
3. Re-enable portal → still logged in; pickers blank.

- [ ] **Step 6: Final commit (if any fixups)** and prepare PR per `.claude/rules/pr-review.md`.

---

## Self-Review

**Spec coverage:**
- Trigger detection → Task 3. Fresh manual slate (selections) → Tasks 3 & 5. Engine reset (clock + next-game) → Task 1. Nominal-vs-minimum correctness → Task 1 (`reset_to_manual_break` sets nominal) + Task 4 (End path calls it after `reset_game`). Between-games path → Task 3. Mid-game confirmation (End/Keep/Discard/GoBack) → Tasks 2 & 4. Keep-game fallback to nominal → Task 1 test + Task 4. Token kept → Tasks 3 & 5. ADR 017 amendment → Task 6. Testing/acceptance → Task 7. All spec sections covered.
- **Out of scope honored:** OFF → ON unchanged; no logout; no cross-crate changes; game-block play-time behavior untouched.

**Placeholder scan:** No TBD/TODO. The app-layer tasks intentionally describe borrow choreography rather than scripting every line, per the project's right-sized-plan rule and the "mirror the closest sibling's source" convention — each references the exact existing code (`mod.rs:990-1006`, `mod.rs:1079-1135`) to copy from.

**Type consistency:** `reset_to_manual_break`, `clear_portal_next_game`, `clear_portal_selections_to_manual`, `ConfirmationKind::SwitchToManualFromApply`, and the reused `ConfirmationOption` variants are named identically across Tasks 1–4. `NextGameInfo { number, timing, start_time }` matches the struct at `mod.rs:2453`.
