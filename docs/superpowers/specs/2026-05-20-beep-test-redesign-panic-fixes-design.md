# Beep-Test Redesign — Panic Fixes Design Spec

**Date:** 2026-05-20
**Branch:** `feat/refbox/beep-test-redesign`
**Scope:** Fix two runtime panics surfaced during the operator walkthrough of Branch 2.
**Process:** Lean — refbox-only, no `uwh-common`, no wire-format changes.

---

## Goal

The BeepTest Settings redesign on Branch 2 surfaced two iced runtime panics during the
operator walkthrough. This spec captures the design for fixing both before the branch is
considered complete. No new features are added; this is strictly defect resolution.

---

## Panic #1: `CycleParameter` unwrap on the Settings landing

### Symptom

Tapping the APP MODE tile on the BeepTest Settings landing after returning from a sub-page
(e.g., Sound Settings → Cancel → back to landing → tap APP MODE) panics with
`called Option::unwrap() on a None value` at `refbox/src/app/mod.rs:2648`. The unwrap is in
the shared `Message::CycleParameter` handler:

```rust
Message::CycleParameter(param) => {
    let settings = &mut self.edited_settings.as_mut().unwrap();
    ...
}
```

### Root cause

The BeepTest sub-page `Cancel` and `Save/Apply` handlers all clear `edited_settings = None`
on their way back to the landing. The landing's APP MODE cycle button uses the shared
`CycleParameter(Mode)` message, whose handler expects `edited_settings` to be `Some`. After
any sub-page round-trip, `edited_settings` is `None`, so the next cycle press panics.

The game-mode (Hockey/Rugby) Settings flow does not have this problem: its sub-page
Cancel/Apply messages (`Message::CancelConfigPage` / `Message::ApplyConfigPage`) do not
clear `edited_settings`. `edited_settings` survives across sub-page navigation and is only
cleared at the final exit point.

### Fix

Align the BeepTest Settings flow with the Hockey/Rugby flow: do **not** clear
`edited_settings` on sub-page Cancel/Apply. Only clear it at the true exit point —
`Message::BeepTestCloseSettings`, which is fired by BACK on the landing.

Specifically, remove `self.edited_settings = None;` from these six handlers in
`refbox/src/app/mod.rs`:

- `Message::BeepTestSoundSettingsSave`
- `Message::BeepTestSoundSettingsCancel`
- `Message::BeepTestEditLevelsSave`
- `Message::BeepTestEditLevelsCancel`
- `Message::BeepTestLanguageApply`
- `Message::BeepTestLanguageCancel`

Each sub-page entry message (`BeepTestEditOpenSound`, `BeepTestEditOpenLevels`,
`BeepTestEditOpenLanguage`) already rebuilds `edited_settings` from scratch with current
config values, so stale data from a previous sub-page never persists across entries. The
landing's seeded `edited_settings.mode` from `BeepTestOpenSettings` survives.

This is a 6-line removal — no new code.

### Verification

- Open BeepTest Settings → open Sound Settings → Cancel → back on landing → tap APP MODE:
  cycles without panicking.
- Same with Sound Settings → Save, Edit Levels → Cancel/Save, Language → Cancel/Apply.

---

## Panic #2: `Quad with non-normal height!` on the Edit Levels page

### Symptom

Tapping EDIT LEVELS on the BeepTest Settings landing causes refbox to panic with
`called panic!('Quad with non-normal height!')` in `iced_tiny_skia::engine::draw_quad`. The
page is unreachable as a result.

### Suspected root cause

The Edit Levels view uses `container(table).height(Length::FillPortion(3))` and
`container(edit_panel).height(Length::FillPortion(2))`. The visually-equivalent main view
in `beep_test.rs` uses `container(table).height(Length::Fill)` for the same widget pattern
and renders without panic.

Theory: iced 0.13's `FillPortion` math interacts badly with the nested
`container(bands).height(Length::Fill)` inside the table, in this specific window-size
range, producing a child quad with a non-normal computed height. Switching the wrapper to
`Length::Fill` will match the main view's pattern and likely eliminate the panic.

The theory is unconfirmed. If the swap does not fix the panic, we use the
`superpowers:systematic-debugging` skill to bisect the layout tree and identify the actual
culprit (likely candidates: the `editor_value_cell` button's implicit height in a
`FillPortion`-wrapped column, or an unguarded division by zero in band-layout arithmetic).

### Fix

Two-step approach:

1. **Try the FillPortion → Fill swap.** Change the Edit Levels page's outer column from:
   ```rust
   column![
       banner,
       container(table).height(FillPortion(3)),
       container(edit_panel).height(FillPortion(2)),
       footer,
   ]
   ```
   to a structure that mirrors the main view: either a single Fill column containing both
   table and panel side by side, or two equal Fill rows. The exact restructure is decided
   after step 1 confirms the FillPortion theory.

2. **If the swap doesn't fix it,** invoke `superpowers:systematic-debugging` and bisect the
   layout tree. Confirm the offending widget by progressively simplifying the page (remove
   panel, simplify table, etc.) until the panic disappears, then identify the minimal
   reproduction and fix accordingly.

### Verification

- Open BeepTest Settings → tap EDIT LEVELS: the page renders without panic.
- The table shows the default 10 levels with correct column heights matching the main view.
- Selecting a level highlights its column.
- `[+NEW]`, `[REMOVE LEVEL]`, `[-]`/`[+]` for Time and Count all work.
- Save commits and Cancel discards correctly.

---

## Out of scope (deferred)

These were surfaced during the walkthrough but are not part of this fix:

- Cell-highlight color and timing tweaks on the main view (Scenario M follow-up)
- Branch 3 (LED panel cross-crate changes)
- Any further UI redesign requests

---

## Acceptance criteria

1. APP MODE cycling on the BeepTest Settings landing does not panic after any sub-page
   round-trip.
2. Tapping EDIT LEVELS on the BeepTest Settings landing renders the page without panic.
3. The Edit Levels page is functionally usable (selection, +NEW, REMOVE LEVEL, time/count
   increment/decrement, Save, Cancel).
4. The BeepTest Language picker works end-to-end (no panic on entry, no panic on
   Save/Cancel, no panic returning to landing).
5. `just check` passes cleanly.

---

## Spec self-review

| Check | Result |
|-------|--------|
| Placeholders | None — both panics have concrete root-cause and fix sketches. The Edit Levels fix is conditional on the FillPortion theory, but the fallback path (systematic-debugging) is fully specified. |
| Internal consistency | Yes — both fixes are independent and the verification steps are concrete. |
| Scope | Single coherent unit: defect resolution for two related panics on the same branch. |
| Ambiguity | Edit Levels Step 1 could be interpreted two ways (single column vs side-by-side). The plan will pick one based on the live page-after-swap result. |
