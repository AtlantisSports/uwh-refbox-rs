# Beep-Test Redesign Panic Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix two iced runtime panics surfaced during the operator walkthrough of Branch 2 — the App Mode cycle unwrap on the Settings landing, and the `Quad with non-normal height` crash on the Edit Levels page.

**Architecture:** Two independent fixes. (1) Remove `self.edited_settings = None;` from six sub-page Cancel/Save handlers so `edited_settings` survives sub-page round-trips, matching the Hockey/Rugby Settings pattern. (2) Swap the Edit Levels page's `FillPortion(3)/FillPortion(2)` wrappers for the same `Length::Fill` pattern the panic-free main view uses; if the swap doesn't fix it, bisect the layout tree with `superpowers:systematic-debugging`.

**Tech Stack:** Rust 2024 edition (MSRV 1.85), `iced` 0.13.

**Spec:** `docs/superpowers/specs/2026-05-20-beep-test-redesign-panic-fixes-design.md`

**Process discipline:** Lean per `.claude/rules/plan-execution.md` — refbox-only, narrow defect-resolution scope.

---

## Branch and worktree

- **Branch:** `feat/refbox/beep-test-redesign` (existing — this is a continuation, not a new branch)
- **Worktree:** `.worktrees/feat-refbox-beep-test-redesign/` — already in use
- **Working directory for all commands:** `/home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign/`

---

## File Structure

### Files modified

| Path | Why |
|------|-----|
| `refbox/src/app/mod.rs` | Remove `self.edited_settings = None;` from six sub-page handlers |
| `refbox/src/app/view_builders/beep_test_settings.rs` | Change `Length::FillPortion` to `Length::Fill` on the Edit Levels container wrappers (Task 2 — may be adjusted based on live diagnosis) |

### Files created

None.

### Files deleted

None.

---

## Task 1: Stop sub-page Cancel/Save handlers from clearing `edited_settings`

**Goal:** Align the BeepTest sub-page lifecycle with the Hockey/Rugby pattern. After this task, the Settings landing's APP MODE cycle button does not panic when tapped after a sub-page round-trip.

**Files:**
- Modify: `refbox/src/app/mod.rs` (six handler bodies)

### Steps

- [ ] **Step 1: Locate the six handlers**

Run:

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
grep -nE "Message::BeepTest(SoundSettings|EditLevels|Language)(Save|Cancel|Apply)" refbox/src/app/mod.rs
```

Expected output:

```
3371:            Message::BeepTestLanguageCancel => {
3377:            Message::BeepTestLanguageApply => {
3456:            Message::BeepTestSoundSettingsSave => {
3468:            Message::BeepTestSoundSettingsCancel => {
3599:            Message::BeepTestEditLevelsSave => {
3611:            Message::BeepTestEditLevelsCancel => {
```

(Line numbers may have drifted slightly — use them as guides, not exact targets.)

- [ ] **Step 2: Remove `self.edited_settings = None;` from each handler body**

For each of the six handlers, locate the line `self.edited_settings = None;` inside the arm
and delete it. Do NOT delete the line in `Message::BeepTestCloseSettings` (that one stays —
it's the true exit point).

The remaining lines in each handler (Save: commit + persist + navigate; Cancel: navigate)
stay unchanged.

Specifically:

- `BeepTestSoundSettingsSave`: keep `self.apply_sound_options()`, `self.persist_config()`,
  `self.app_state = AppState::BeepTestSettings(BeepTestConfigPage::Main)`, `trace!(...)`,
  `Task::none()`. Remove only the `self.edited_settings = None;` line.
- `BeepTestSoundSettingsCancel`: keep `self.app_state = ...`, `trace!(...)`, `Task::none()`.
  Remove only the `self.edited_settings = None;` line.
- `BeepTestEditLevelsSave`: keep the commit logic (copy `edited.beep_test_levels` to
  `self.config.beep_test.levels`, persist, navigate). Remove only the
  `self.edited_settings = None;` line.
- `BeepTestEditLevelsCancel`: keep navigation. Remove only the `self.edited_settings = None;`
  line.
- `BeepTestLanguageApply`: keep the language commit + restart logic. Remove only the
  `self.edited_settings = None;` line (it currently happens before the
  `if let Some(lang) = lang_opt` block; that block stays).
- `BeepTestLanguageCancel`: keep navigation. Remove only the `self.edited_settings = None;`
  line.

After the edits, each handler still navigates back to `AppState::BeepTestSettings(BeepTestConfigPage::Main)`,
but `edited_settings` is left intact (preserving the landing-seeded `.mode`).

- [ ] **Step 3: Verify build**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
just check
```

Expected: PASS (exit 0). Six lines removed, no other changes.

- [ ] **Step 4: Commit**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
git add refbox/src/app/mod.rs && \
git commit -m "fix(refbox): keep edited_settings alive across beep-test sub-pages"
```

- [ ] **Step 5: Operator verification**

Launch refbox in BeepTest mode (the operator drives the UI):

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
WAYLAND_DISPLAY= cargo run -p refbox
```

Verification scenario:

1. Refbox opens on the BeepTest main view.
2. Tap SETTINGS → Settings landing.
3. Tap SOUND SETTINGS → Sound Settings sub-page → tap CANCEL → back to landing.
4. Tap the APP MODE tile. Verify: no panic. The mode value cycles (Hockey 6v6 → Hockey 3v3 → Rugby → Beep Test → repeats).
5. Repeat with SOUND SETTINGS → APPLY (after toggling something), EDIT LEVELS → CANCEL,
   LANGUAGE → CANCEL, etc. APP MODE on the landing should never panic.
6. Tap BACK on the landing — returns to BeepTest main view.

If APP MODE still panics after this fix, **STOP** and re-check that the right line was
removed in each handler.

---

## Task 2: Fix the Edit Levels page render panic

**Goal:** The Edit Levels page renders without panic and is functionally usable.

**Files:**
- Modify: `refbox/src/app/view_builders/beep_test_settings.rs` (the column structure inside `build_beep_test_edit_levels_page`)

### Steps

- [ ] **Step 1: Read the current Edit Levels page column structure**

Find `build_beep_test_edit_levels_page` in `refbox/src/app/view_builders/beep_test_settings.rs`.
Read the column construction at the bottom of the function (currently around lines 321-337):

```rust
column![
    make_game_time_button(snapshot, false, false, mode, clock_running, portal_indicator),
    container(table)
        .width(Length::Fill)
        .height(Length::FillPortion(3)),
    container(edit_panel)
        .width(Length::Fill)
        .height(Length::FillPortion(2)),
    make_beep_test_cancel_apply_footer(
        Message::BeepTestEditLevelsCancel,
        Message::BeepTestEditLevelsSave,
        has_changes,
    ),
]
.spacing(SPACING)
.height(Length::Fill)
.into()
```

- [ ] **Step 2: Compare against the main view (which doesn't panic)**

Find the main view column in `refbox/src/app/view_builders/beep_test.rs` around lines 140-152.
Note the key difference: the main view wraps its table in `container(levels_table).width(Fill).height(Fill)`,
**not** `FillPortion`.

- [ ] **Step 3: Swap `FillPortion(3)` / `FillPortion(2)` for `Fill` shares**

Replace the column in `build_beep_test_edit_levels_page` with this structure:

```rust
column![
    make_game_time_button(snapshot, false, false, mode, clock_running, portal_indicator),
    container(table)
        .width(Length::Fill)
        .height(Length::Fill),
    container(edit_panel)
        .width(Length::Fill)
        .height(Length::Fill),
    make_beep_test_cancel_apply_footer(
        Message::BeepTestEditLevelsCancel,
        Message::BeepTestEditLevelsSave,
        has_changes,
    ),
]
.spacing(SPACING)
.height(Length::Fill)
.into()
```

This gives the table and edit panel each an equal Fill share of the remaining vertical
space — the same pattern the main view uses for its table wrapper. (The previous 3:2 split
is sacrificed for the layout to render at all; if a 3:2 visual ratio is desired later, it
can be reintroduced via `Length::FillPortion(3)` / `Length::FillPortion(2)` AFTER the root
cause is fully understood.)

- [ ] **Step 4: Verify build**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
just check
```

Expected: PASS (exit 0).

- [ ] **Step 5: Operator verification — try opening Edit Levels**

Launch refbox in BeepTest mode (operator drives):

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
WAYLAND_DISPLAY= cargo run -p refbox
```

Verification scenario:

1. Refbox opens on the BeepTest main view.
2. Tap SETTINGS → Settings landing.
3. Tap EDIT LEVELS.
4. **If the page renders without panic:** verify the table shows 10 columns (Level 1..10) with
   stacked time values matching the schedule. Verify the edit panel below shows
   "Selected: Level 1" with time/count `[-]` `[+]` buttons. Tap a column → it highlights blue.
   Tap `[+NEW]` → a new column appears at the end. Tap `[-]` on count for the selected level →
   the column shrinks. Tap CANCEL → returns to landing without committing. Re-enter,
   tap SAVE → commits and returns to landing.
5. **If the page still panics:** capture the new panic backtrace from the log file at
   `/tmp/claude-1000/.../tasks/<task_id>.output` and proceed to Step 6.

- [ ] **Step 6 (only if Step 5 still panics): Invoke systematic-debugging skill**

Invoke `superpowers:systematic-debugging` and bisect the Edit Levels page layout:

1. Reduce `build_beep_test_edit_levels_page` to just the banner + footer (no table, no edit_panel).
   Render — does it panic?
2. Add back the edit_panel only. Render — does it panic?
3. Add back the table only. Render — does it panic?
4. If the table is the culprit, simplify `build_editor_levels_table` step by step (remove
   the `[+NEW]` cell, remove filler cells, remove the band loop, etc.) until the panic
   disappears. The minimal reproduction reveals the offending widget.
5. Apply the targeted fix and re-verify.

This step's output should be a concrete, root-causable fix — not another speculative swap.

- [ ] **Step 7: Commit**

If Step 5 passed:

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
git add refbox/src/app/view_builders/beep_test_settings.rs && \
git commit -m "fix(refbox): use Length::Fill instead of FillPortion on edit levels page"
```

If Step 6 produced a different fix, the commit message describes that fix instead:

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
git add refbox/src/app/view_builders/beep_test_settings.rs && \
git commit -m "fix(refbox): <root-cause fix for edit levels render panic>"
```

---

## Task 3: Final operator walkthrough

**Goal:** Confirm both panics are gone and all BeepTest Settings flows work end-to-end.

**Files:** None modified.

### Steps

- [ ] **Step 1: Launch refbox in BeepTest mode**

```bash
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign && \
WAYLAND_DISPLAY= cargo run -p refbox
```

- [ ] **Step 2: Walk through all sub-pages**

The operator drives:

| # | Action | Expected outcome |
|---|--------|------------------|
| 1 | BeepTest main view | Renders as before; Start/Reset/Settings work |
| 2 | Settings → Sound Settings → toggle a setting → Cancel | Returns to landing, no commit |
| 3 | Settings landing → APP MODE tap | Cycles in place, no panic |
| 4 | Settings → Sound Settings → toggle → APPLY | Returns to landing, change persisted |
| 5 | Settings landing → APP MODE tap | Still cycles, no panic |
| 6 | Settings → EDIT LEVELS | Page renders, no panic |
| 7 | Edit Levels → select a level → `[+]` time → SAVE | Returns to landing, levels updated |
| 8 | Settings landing → APP MODE tap | Still cycles, no panic |
| 9 | Settings → LANGUAGE → pick a language → CANCEL | Returns to landing, no change |
| 10 | Settings landing → APP MODE tap | Still cycles, no panic |
| 11 | Settings → APP MODE cycle → BACK | Returns to BeepTest main, staged mode discarded |
| 12 | Settings → APP MODE cycle → RESTART TO APPLY | Refbox restarts in new mode |

- [ ] **Step 3: If anything fails**

Report the failing scenario back; this plan does not auto-resolve unknown failures.
A new round of fixes is needed.

- [ ] **Step 4 (only on success): Done — Branch 2 ready for PR**

No commit needed — Tasks 1 and 2 already committed their fixes.

---

## Self-review

| Check | Result |
|-------|--------|
| Spec coverage (Panic #1) | Task 1 (six handlers, removal, verification). |
| Spec coverage (Panic #2) | Task 2 (FillPortion swap with systematic-debugging fallback). |
| Spec coverage (Verification) | Task 3 (12-scenario walkthrough). |
| Placeholder scan | No TBDs. Step 6 (debugging fallback) is conditional but its method (systematic-debugging skill) is concrete. |
| Type consistency | All Message variant names checked against current `message.rs`. |
| Bite-sized tasks | Each task has ≤ 7 steps, each step ≤ 5 min. |
