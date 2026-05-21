# Beep-Test ADD LEVEL Inserts After Selected — Design

**Date:** 2026-05-21
**Status:** Approved through brainstorming; awaiting implementation plan
**Owner:** e-straily (with Claude)
**Branch:** `feat/refbox/beep-test-redesign` (continuing after Chunk 8 at `f2b19ab0`)
**Chunk:** 9 of the beep-test redesign follow-on series
**Process gate:** Lean (per `.claude/rules/plan-execution.md` — refbox UI work, no state-machine or wire-format change).

---

## Goal

Change the `Message::BeepTestEditAddLevel` handler so that ADD LEVEL inserts a new level **immediately to the right of the currently selected column** (and selects the newly-inserted level), rather than always appending at the right-most end.

---

## Motivation

Surfaced 2026-05-21 during the Chunk 8 walkthrough. With Chunks 7/8 in place, the operator confirmed the column-width-lock layout works correctly, but observed that ADD LEVEL always pushes the new column to the right end of the active region — even when a non-last column is selected. Operator's intent: ADD LEVEL should let them insert a new level *next to the one they're currently editing*, not require them to add at the end and then re-order (no reorder UI exists today).

The change is a single handler-body rewrite. No view, no `Message`, no state-machine, no signature affected.

---

## Scope

### Files touched

- `refbox/src/app/mod.rs` — the `Message::BeepTestEditAddLevel` arm of `update()` (around line 3580; current body landed in Chunk 7 and was nudged by Chunk 8's cap bump). Replace the `levels.push(...)` + `edited.selected_level = levels.len() - 1` pair with a clamped insert-at-position-plus-one + select-the-new-one.

### Not touched

- The view-side `add_disabled = levels.len() >= 15` predicate or its `gray_button` rendering.
- The handler's 15-cap guard (`if levels.len() < 15`).
- The new-level defaults (`count: 4`, `duration: 20s`).
- `view_builders/beep_test.rs`, `view_builders/beep_test_settings.rs`, or any other file.
- Translations.
- Chunk 1's per-level lap-count cap.
- `uwh-common`, `wireless-remote`, `overlay`, the LED panel, Chunk 6 work.

---

## Design

### Approach A — clamp via `.min(levels.len())` (approved through brainstorming)

After the existing 15-cap guard, compute the insert position from `edited.selected_level + 1`, clamped to `levels.len()` so the position is always in-range:

```rust
let insert_at = (edited.selected_level + 1).min(levels.len());
levels.insert(insert_at, crate::config::Level {
    count: 4,
    duration: std::time::Duration::from_secs(20),
});
edited.selected_level = insert_at;
```

### Why `.min(levels.len())`

The clamp handles three cases without separate branches:

| Scenario | `selected_level` | `levels.len()` | `(selected_level + 1).min(levels.len())` | Result |
|---|---|---|---|---|
| Mid-list selection | 2 | 5 | `min(3, 5) = 3` | Insert at index 3 (between cols 3 and 4) |
| Last-column selection | 4 | 5 | `min(5, 5) = 5` | Insert at index 5 = append (same end-position as Chunk 7's `push` behaviour) |
| Empty list, default 0 | 0 | 0 | `min(1, 0) = 0` | Insert at front (first level) |
| Stale `selected_level` past end | 10 | 3 | `min(11, 3) = 3` | Append (defense-in-depth; should not occur in practice — the view clamps `selected.min(levels.len().saturating_sub(1))` before render) |

The `.min(levels.len())` is conceptually "saturating insert position" — analogous to Chunk 1's `level.count.saturating_add(1)` defensive arithmetic style in adjacent handler code.

### Why select the new level

The current Chunk 7/8 handler sets `edited.selected_level = levels.len() - 1` after `push` — i.e. it selects the newly-added level. Approach A preserves that invariant ("newly-added is selected") at the new insert position, so the operator's per-level edit panel immediately reflects the level they just added. The selection-after-insert behaviour does not change from the operator's perspective; only the insert position changes.

### Pattern reference (per `.claude/rules/patterns.md`)

Saturating arithmetic on indices (`.min(len)` / `.saturating_sub(1)` / `.saturating_add(1)`) is the established style in this codebase's BeepTest handler chain — Chunk 1 used `level.count.saturating_add(1)`, the view's `selected.min(levels.len().saturating_sub(1))` clamp is in `build_beep_test_edit_levels_page`, and the existing `Message::BeepTestEditRemoveLevel` handler uses `if edited.selected_level >= levels.len() { ... }` defensive clamps after removal. The new line continues that pattern.

---

## Testing

No new unit test. Lean process (refbox UI, no state-machine change). Verification via walkthrough.

**Walkthrough scenarios:**

1. **Insert in the middle.** Start with at least 3 levels staged. Open Edit Levels. Tap a non-last column (e.g. column 3 of 5) to select it. Tap ADD LEVEL. Confirm:
   - A new column appears at position 4 (immediately to the right of the selected column).
   - The new column is selected (blue-highlighted).
   - The previous columns 4 and 5 have shifted right to positions 5 and 6.
   - The per-level edit panel shows the new level's default count and duration.
2. **Insert at end (last-column selection).** Select the last column. Tap ADD LEVEL. Confirm the new column appears at the very end and is selected — same as pre-Chunk-9 behaviour for this case.
3. **15-cap still works.** Keep selecting and adding levels until 15 exist. Confirm ADD LEVEL grays out at 15 regardless of which column is selected; tapping does nothing.
4. **REMOVE LEVEL still shifts left correctly.** After inserting in the middle (scenario 1), tap REMOVE LEVEL. Confirm the previously-selected new column disappears and selection moves to a remaining column without disrupting the layout.

`just check` passes.

---

## Acceptance criteria

The four walkthrough scenarios pass; `just check` is green.

---

## Out of scope (intentionally deferred)

- **Insertion before the selected column** (left-of-selected) or **drag-to-reorder** UI — operator's request is specifically "to the right of selected"; alternative insertion positions are not on the table.
- **Changing the new-level defaults** (`count: 4`, `duration: 20s`). Same as today.
- **A separate UI affordance for "add at end"** — the operator can still get the previous behaviour by selecting the last column first.
- **Adjusting selection after REMOVE LEVEL** — current `BeepTestEditRemoveLevel` behaviour is unchanged.
- **Migrating away from `selected_level` as a plain `usize`** (e.g. to `Option<usize>` for explicit "no selection" handling) — the view-side clamp already keeps `selected_level` in range when `levels` is non-empty, and the new handler clamps on read regardless.
