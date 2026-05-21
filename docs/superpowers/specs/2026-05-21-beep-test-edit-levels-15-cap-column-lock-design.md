# Beep-Test Edit Levels: 15-Cap + Locked Column Widths — Design

**Date:** 2026-05-21
**Status:** Approved through brainstorming; awaiting implementation plan
**Owner:** e-straily (with Claude)
**Branch:** `feat/refbox/beep-test-redesign` (continuing after Chunk 7 at `3f91c22f`)
**Chunk:** 8 of the beep-test redesign follow-on series
**Process gate:** Lean (per `.claude/rules/plan-execution.md` — refbox UI work, no state-machine or wire-format change).

---

## Goal

Two operator-driven refinements to Chunk 7's just-landed Edit Levels behaviour:

1. **Raise the level cap from 10 to 15.** Up to 15 levels can be staged; ADD LEVEL disables at exactly 15.
2. **Lock the column widths to a 15-cell grid.** Active columns always render at 1/15 of the table width, regardless of how many levels exist. Adding or removing levels does not visibly resize the existing columns — extra width is reserved as transparent space on the right of the table.

---

## Motivation

Surfaced 2026-05-21 during the Chunk 7 walkthrough. The operator approved Chunk 7 in principle ("seems good") but identified two refinements after seeing the result:

- 10 was an underestimate; tournament configurations may want up to 15 progressively-faster cadence levels.
- With Chunk 7's design, the active columns *visibly resize* as the operator adds or removes levels (because `Length::Fill` on each cell makes them split available width by the actual count). The operator would prefer the columns to stay the same size so the page feels stable while editing — adding the 6th level shouldn't make levels 1–5 narrower.

The chosen mechanism (per brainstorming): bring back the per-row filler-cell padding that Chunk 7 removed for partial bands, but applied to *every* row with a single fixed target of 15 cells (no wrapping). Existing cell uses `Length::Fill`, so 15 cells in a `Row` give each cell exactly 1/15 of the row width — real cells stay locked at 1/15 regardless of how many actual levels exist; the remaining cells are transparent `filler_cell()` calls that reserve the layout slot.

The brainstorming round considered an alternative "always render 15 visible disabled-style placeholder cells" (Approach D) and rejected it: the operator preferred the cleaner visual of invisible filler over the affordance of dimmed placeholder cells.

---

## Scope

### Files touched

- `refbox/src/app/view_builders/beep_test_settings.rs` —
  - Change `add_disabled = levels.len() >= 10` to `>= 15` in `build_edit_panel`.
  - In `build_editor_levels_table`, after the existing header-row construction, push `(15 - levels.len())` `filler_cell()` calls to bring the header to 15 cells. After each cell-row construction inside the `for row_idx in 0..max_count` loop, push the same `(15 - levels.len())` filler cells.
- `refbox/src/app/view_builders/beep_test.rs` — in `build_levels_table`, apply the same right-padding to the header row and to each cell row (push `(15 - levels.len())` `filler_cell()` calls to bring each row to 15 cells total). No cap-related change in this file (caps live in the editor, not the main view).
- `refbox/src/app/mod.rs` — change `if levels.len() < 10` to `< 15` in the `Message::BeepTestEditAddLevel` handler.

### Not touched

- The `filler_cell()` helper — used as-is, in scope in both view files.
- The `Message` enum — no changes.
- Translations — Chunk 7's ADD LEVEL / REMOVE LEVEL translations stand.
- Chunk 1's per-level lap-count cap (`level.count >= 5`) — unchanged.
- `uwh-common`, `wireless-remote`, `overlay`, the LED panel, Chunk 6 work.
- Behaviour for legacy configs with > 15 levels — see Out of scope.

---

## Design

### Cap value change

Three sites change from `10` to `15`:

1. `build_edit_panel`'s view-side gate:
   ```rust
   let add_disabled = levels.len() >= 15;
   ```
2. The `Message::BeepTestEditAddLevel` handler's defense-in-depth guard:
   ```rust
   if levels.len() < 15 {
       levels.push(...);
       edited.selected_level = levels.len() - 1;
   }
   ```
3. The inline doc-comment near `add_disabled` (the one that says "the cap") is updated implicitly by the value change; the comment text doesn't mention `10` literally, so no further change there.

### Column-width lock via 15-cell row padding

The current post-Chunk 7 code in both `build_editor_levels_table` and `build_levels_table` builds:

- A header row with one `editor_header_cell` / `header_cell` per real level, then `rows.push(header_row)`.
- A series of cell rows; each cell row iterates `levels` and pushes either a real cell or a `filler_cell()` (for `row_idx >= level.count`).

The change reintroduces a *right-side padding loop* on every constructed row, with target 15 cells:

```rust
// After populating header_row with real cells:
for _ in levels.len()..15 {
    header_row = header_row.push(filler_cell());
}
rows = rows.push(header_row);

// And after each cell_row is populated with the per-column real-or-filler decision:
for _ in levels.len()..15 {
    cell_row = cell_row.push(filler_cell());
}
rows = rows.push(cell_row);
```

These padding loops are identical in structure to the (pre-Chunk-7) band-padding loops that lived at `for _ in cols_used_in_band..EDIT_BAND_WIDTH` and `for _ in band_levels.len()..BAND_WIDTH`. Chunk 7 removed them because the outer band loop also went away; Chunk 8 reintroduces just the inner per-row padding (no outer band loop) bounded by 15 (instead of the old per-band 10).

### Why this locks column widths

`Row` in iced 0.13 distributes available width among children based on each child's `Length`. A `Length::Fill` child takes one equal share. With 15 children all using `Length::Fill`, each gets 1/15 of the row's width. Whether a child is a real `editor_header_cell` / `editor_value_cell` (which use `Length::Fill` width) or a `filler_cell()` (which uses `Length::Fill` width), the per-child width is identical.

Therefore: real cells are always 1/15 of the row width. Adding a 6th level moves a filler cell out of position 6 and puts a real cell there — but the cell widths don't change.

### Pattern reference (per `.claude/rules/patterns.md`)

This change reuses the `filler_cell()` helper and the right-padding pattern from before Chunk 7. We're not introducing a new layout idiom — we're restoring half of one that was removed. The reintroduced loop is bounded by inline-literal `15` to match Chunk 7's inline-literal `10` and Chunk 1's inline-literal `5` (the established convention in this branch).

---

## Testing

No new unit test. Lean process (refbox UI, no state-machine change). Verification via walkthrough.

**Walkthrough scenarios:**

1. **One level, locked width.** Start with a config of 1 level. Open Edit Levels. The single active column should occupy roughly 1/15 of the table width; the rest of the table area is empty (transparent) space.
2. **Adding levels does not resize existing columns.** From 1 level, tap ADD LEVEL repeatedly. Each new column should appear at the *right edge of the active region* without changing the width of the existing columns.
3. **15-cap reached.** At 15 levels, the table fills the full row (every slot is a real column); ADD LEVEL grays out and tapping it does nothing.
4. **Removing a level shifts left.** From 15, tap REMOVE LEVEL once. A column disappears from the active region; ADD LEVEL re-enables; existing real columns stay at the same width.
5. **Main view (BeepTest screen) layout sanity.** Exit Edit Levels and run a beep test with the staged levels. The main view's transposed table should also render in a 15-cell-locked grid with the same column-width feel.

`just check` passes (fmt, clippy, full test suite, audit).

---

## Acceptance criteria

The five walkthrough scenarios pass; `just check` is green.

---

## Out of scope (intentionally deferred)

- **Named `MAX_LEVELS` constant.** Sticking with inline-literal `15` to match Chunk 1's `5` and Chunk 7's `10`. If the cap value ever needs another bump, a single refactor can extract a constant then; doing it now adds churn for no current benefit.
- **Visible-placeholder-cell variant (Approach D from brainstorming).** Operator chose the cleaner-looking invisible-filler approach.
- **Behaviour for legacy configs with > 15 levels.** Same as Chunk 7's stance: existing levels are not auto-truncated; the table renders single-row (now overflowing past the 15-cell layout); REMOVE LEVEL works; the operator decrements to ≤ 15 to bring the config in bounds. ADD LEVEL is disabled at any count ≥ 15.
- **Tap-to-add-here interaction on the right-side empty space.** ADD LEVEL stays the only way to append a level.
- **Translation key changes, the per-level lap-count cap, LED panel work** — all unchanged.
