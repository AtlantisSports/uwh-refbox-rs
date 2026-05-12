# Game Options Snapshot Expansion Implementation Plan

> **For agentic workers:** This is a heavy-process plan (TM-adjacent state-machine change). Execute inline with verification between major task groups. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend Game Options' page-entry snapshot and per-page Apply path to fully cover the four App-slice fields editable from Game Options in portal mode (`using_uwhportal`, `current_event_id`, `current_court`, `schedule`). After this change, per-page Cancel/Discard on Game Options reverts everything the page edited, and per-page Apply commits everything the page edited.

**Architecture:** `PageEntrySnapshot::Game` grows from 2 fields to 6. Four functions update in lockstep (`capture_snapshot_for`, `revert_from_snapshot`, `page_has_changes`, `apply_game_options`). `apply_app_options` is unchanged — both pages now commit the same overlap fields, but writes are idempotent because identical values are written. No new ConfirmationKind variants. No UI layout changes.

**Tech Stack:** Rust 2024, refbox crate, iced 0.13. Heavy process per `.claude/rules/plan-execution.md` (touches state-machine commit logic).

**Out of scope:**
- Removing the toggle/event/court pickers from Game Options (UX redesign — separate ADR if anyone proposes it).
- Reorganising slice naming (the `App` snapshot variant still exists; only Game's variant grows).
- Tasks 9–13 of ADR 009 (other pages' Cancel/Apply chrome).

---

## Background

Surfaced during Task 8 manual review (code-reviewer issue I-2):

Game Options in portal mode lets the operator edit four fields whose values live in `App`-slice snapshot — `using_uwhportal` (toggle in the picker row), `current_event_id` (event picker), `current_court` (court picker), and `schedule` (auto-fetched when event_id changes). Pre-Task-8 this didn't matter because Game Options had no per-page Cancel — only the global Done committed everything. Task 8 added per-page Cancel/Apply, exposing two gaps:

1. **Discard gap:** Cancel/Discard on Game Options reverts only `config` + `game_number`. If the operator changed the event from inside Game Options and pressed Cancel, the event change stays.
2. **Apply gap (worse):** Apply on Game commits only `config` + `game_number` to live state. The new `current_event_id` lives in `edited_settings` but isn't written to `self.current_event_id`. If the operator then leaves settings via the global Back, `edited_settings` is dropped on next entry — the event change is **silently lost** with no warning.

This plan closes both gaps by making Game's snapshot match what Game Options can actually edit, and making Game's apply commit those same fields.

---

## File Structure

| File | Change |
|------|--------|
| `refbox/src/app/mod.rs` | `PageEntrySnapshot::Game` variant expansion; `capture_snapshot_for`, `revert_from_snapshot`, `apply_game_options` updates |
| `refbox/src/app/view_builders/configuration.rs` | `page_has_changes` Game arm extension |

No new files. No tests added (matches the existing convention — `apply_*_options` functions are not unit-tested; verification is manual + lint + integration).

---

## Task 1: Extend `PageEntrySnapshot::Game` variant

**Files:**
- Modify: `refbox/src/app/mod.rs` — `PageEntrySnapshot` enum (around line 132–162)

- [ ] **Step 1: Add four fields to the `Game` variant.**

Replace:
```rust
pub(crate) enum PageEntrySnapshot {
    Game {
        config: GameConfig,
        game_number: GameNumber,
    },
```

With:
```rust
pub(crate) enum PageEntrySnapshot {
    Game {
        config: GameConfig,
        game_number: GameNumber,
        using_uwhportal: bool,
        current_event_id: Option<EventId>,
        current_court: Option<String>,
        schedule: Option<Schedule>,
    },
```

The remaining variants (`App`, `Display`, `Sound`, `Remotes`, `Language`) are unchanged.

- [ ] **Step 2: `cargo check -p refbox`.**

Expected: FAIL — pattern matches against `PageEntrySnapshot::Game { config, game_number }` are now incomplete, and field destructurings need updating in three places (`capture_snapshot_for`, `revert_from_snapshot`, `page_has_changes`). The compiler errors will guide subsequent tasks.

---

## Task 2: Update `capture_snapshot_for(Game)` arm

**Files:**
- Modify: `refbox/src/app/mod.rs` — `capture_snapshot_for` (around line 525–562)

- [ ] **Step 1: Capture the four new fields.**

Replace the Game arm:
```rust
ConfigPage::Game => PageEntrySnapshot::Game {
    config: edited.config.clone(),
    game_number: edited.game_number.clone(),
},
```

With:
```rust
ConfigPage::Game => PageEntrySnapshot::Game {
    config: edited.config.clone(),
    game_number: edited.game_number.clone(),
    using_uwhportal: edited.using_uwhportal,
    current_event_id: edited.current_event_id.clone(),
    current_court: edited.current_court.clone(),
    schedule: edited.schedule.clone(),
},
```

---

## Task 3: Update `revert_from_snapshot` Game arm

**Files:**
- Modify: `refbox/src/app/mod.rs` — `revert_from_snapshot` (around line 564–620)

- [ ] **Step 1: Restore the four new fields on revert.**

Replace:
```rust
PageEntrySnapshot::Game {
    config,
    game_number,
} => {
    edited.config = config;
    edited.game_number = game_number;
}
```

With:
```rust
PageEntrySnapshot::Game {
    config,
    game_number,
    using_uwhportal,
    current_event_id,
    current_court,
    schedule,
} => {
    edited.config = config;
    edited.game_number = game_number;
    edited.using_uwhportal = using_uwhportal;
    edited.current_event_id = current_event_id;
    edited.current_court = current_court;
    edited.schedule = schedule;
}
```

---

## Task 4: Update `page_has_changes` Game arm

**Files:**
- Modify: `refbox/src/app/view_builders/configuration.rs` — `page_has_changes` (around line 110–178)

- [ ] **Step 1: Compare the four new fields too.**

Replace:
```rust
(
    ConfigPage::Game,
    PageEntrySnapshot::Game {
        config,
        game_number,
    },
) => edited.config != *config || edited.game_number != *game_number,
```

With:
```rust
(
    ConfigPage::Game,
    PageEntrySnapshot::Game {
        config,
        game_number,
        using_uwhportal,
        current_event_id,
        current_court,
        schedule,
    },
) => {
    edited.config != *config
        || edited.game_number != *game_number
        || edited.using_uwhportal != *using_uwhportal
        || edited.current_event_id != *current_event_id
        || edited.current_court != *current_court
        || edited.schedule != *schedule
},
```

- [ ] **Step 2: `cargo check -p refbox`.**

Expected: PASS for everything *except* `apply_game_options`, which still doesn't write the new fields to live state. (No compile error here — it's a behavioural gap, not a type gap.)

---

## Task 5: Update `apply_game_options` to commit the four overlap fields

**Files:**
- Modify: `refbox/src/app/mod.rs` — `apply_game_options` (the no-gate-fired branch and the BetweenGames game-number-changed branch).

The function currently has three commit sites where it writes to live state on the no-gate-fired path:

1. **Config-changed BetweenGames branch:** writes `tm.set_config(...)`, `self.config.game = ...`, sets next game.
2. **Game-number-changed BetweenGames branch:** sets next game.
3. **No-change-detected fall-through:** no writes (returns `None`).

In all three cases, after the existing writes, we now also commit the four App-slice overlap fields to live state.

- [ ] **Step 1: Add a single helper inside `apply_game_options` for the overlap-field commit.**

Inside `apply_game_options` (still inside the function body, after the gate logic), introduce a closure or a sequence of writes immediately before each `return None` / fall-through. The cleanest approach is a single block placed at the very end of the function after all gate checks, since all three success paths return `None` from inside the function:

Restructure the existing function so each gate-passes branch falls through to a common commit-overlap block at the bottom. The current structure has early `return None;` from the config-changed BetweenGames branch — replace that with `let _ = ...;` style fall-through, OR extract the overlap commit into a lambda called before each return.

**Concrete approach:** add overlap commits inline at each of the three success paths, since the borrow checker plays better with explicit writes than with a closure that captures `&mut self`:

In the **config-changed BetweenGames** branch (after `std::mem::drop(tm); self.config.game = new_config;`), append:
```rust
self.using_uwhportal = edited.using_uwhportal;
self.current_event_id = edited.current_event_id.clone();
self.current_court = edited.current_court.clone();
self.schedule = edited.schedule.clone();
return None;
```

In the **game-number-changed BetweenGames** branch (after the `tm.set_next_game(...)` and possible `tm.apply_next_game_start(...)`), drop the tm guard, then append the same four lines:
```rust
std::mem::drop(tm);
self.using_uwhportal = edited.using_uwhportal;
self.current_event_id = edited.current_event_id.clone();
self.current_court = edited.current_court.clone();
self.schedule = edited.schedule.clone();
```

In the **no-change-detected** fall-through (the final `None` at the end of the function): same four lines before returning. This handles the case where the user toggled `using_uwhportal` without changing config or game_number — Apply must still commit the toggle.

**Borrow-check note:** `edited` is the immutable borrow of `self.edited_settings`. Each of the four `self.X = edited.Y.clone()` lines reads from `edited` then writes to a *different* field of `self`. NLL with field-splitting handles this fine — the same pattern is used in `apply_app_options` already.

- [ ] **Step 2: `cargo check -p refbox`.**

Expected: PASS.

- [ ] **Step 3: `just lint`.**

Expected: PASS. If clippy flags the cloning pattern (e.g., redundant clone), keep `.clone()` — the borrow checker requires it.

---

## Task 6: Verification

**Files:** none.

- [ ] **Step 1: `just check` (lint + tests + fmt).**

Expected: lint, tests, fmt PASS. Audit fail is pre-existing (out of scope).

- [ ] **Step 2: Manual smoke test — non-portal mode.**

Launch refbox with the canonical X11/dev command. Tests:
1. Settings → Game Options. Apply disabled. Toggle `using-uwh-portal` ON. Apply enables. Press **Cancel**. Toggle reverts to OFF. Land on Settings Main. ✓
2. Game Options → Toggle ON, **Apply**. Toggle commits to live state. Re-enter Game Options — Apply disabled (clean snapshot reflects new state). Toggle OFF. Apply enables. Apply. Toggle reverts to OFF in live state.

- [ ] **Step 3: Manual smoke test — portal mode (between games).**

Toggle `using-uwh-portal` ON and Apply. Now in portal mode.
1. Game Options → pick an event. Apply enables. **Cancel** — event reverts to prior value. ✓
2. Game Options → pick an event, pick a court. **Apply** — both commit to live state. Re-enter Game Options — Apply disabled. ✓
3. Game Options → pick a different event. Apply enables. Press the global Back from Settings Main. Re-enter settings. Re-enter Game Options. The previously-applied event/court should still be live (i.e. the abandoned change was correctly dropped, not silently committed). ✓

- [ ] **Step 4: Manual smoke test — portal mode (mid-game).**

Start a game in simulator mode.
1. Game Options → pick a different event but no matching court. **Apply** → UwhPortalIncomplete confirmation. **Discard Changes** → event reverts to prior value (this is the bug we set out to fix). Land on Settings Main. ✓
2. Re-enter Game Options. Pick a different event AND matching court. Apply → if config from schedule differs from current, GameConfigChangedFromApply confirmation. Otherwise commits and lands on Settings Main.

- [ ] **Step 5: Hand off to user for spot-check.**

User confirms the four mode/event/court/portal-toggle changes are now reverted by Cancel/Discard and committed by Apply.

---

## Task 7: Commit

**Files:** none.

- [ ] **Step 1: Update the ADR-009 plan's Deviations log.**

In `docs/superpowers/plans/2026-04-20-adr-009-settings-navigation.md`, append to the Deviations log a Task 8 sub-entry for "Snapshot expansion" describing the change and pointing at this plan.

- [ ] **Step 2: Stage and commit (after user approval).**

Bundle into the Task 8 commit (squash this expansion into Task 8's diff, single commit). Per `.claude/rules/plan-execution.md`, deviations bundle into the same commit; this is the same shape as Task 7's deviations bundled into Task 7's commit (commit `4ba2753`).

```bash
git add refbox/src/app/mod.rs \
        refbox/src/app/view_builders/configuration.rs \
        refbox/src/app/view_builders/confirmation.rs \
        refbox/src/app/message.rs \
        docs/superpowers/plans/2026-04-20-adr-009-settings-navigation.md
git commit -m "feat(refbox): Game Options gains Cancel/Apply chrome"
```

(The commit message matches the plan's Task 8 Step 6 verbatim. The expanded snapshot is part of the same Task 8 unit of work, recorded in deviations.)

---

## Self-review

**Spec coverage:**
- Discard gap fixed by Tasks 1–4 (snapshot covers all editable fields, revert restores them, page_has_changes detects them).
- Apply gap fixed by Task 5 (apply commits the four overlap fields to live state).
- Background section covers the operator-visible bugs the change addresses.

**Placeholder scan:** none. Every step has concrete code blocks or commands.

**Type consistency:** field names match throughout (`using_uwhportal`, `current_event_id`, `current_court`, `schedule`); types match `EditableSettings` definitions; `Schedule` and `EventId` already imported in `mod.rs` (used by existing `App` variant).
