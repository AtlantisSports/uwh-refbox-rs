# Beep-Test 15-Cap + Locked Column Widths — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land Chunk 8 of the BeepTest redesign — raise the level cap from 10 to 15, and lock the editor and main-view table column widths to a 15-cell grid by filler-padding shorter rows.

**Architecture:** Two surgical changes layered on Chunk 7's just-landed code. The cap value bumps from inline-literal `10` to inline-literal `15` at the two enforcement sites (view-side `add_disabled`, handler-side defense-in-depth guard). The column-width lock reintroduces the per-row right-side filler-cell padding that Chunk 7 removed for partial bands — applied to every header row and every cell row in both view builders, with a single fixed target of 15 cells (no wrapping, no band loop).

**Tech Stack:** Rust 2024, MSRV 1.85; iced 0.13.

**Spec:** `docs/superpowers/specs/2026-05-21-beep-test-edit-levels-15-cap-column-lock-design.md` (committed at `f9c268a0`).

**Process:** Lean (per `.claude/rules/plan-execution.md` — refbox UI work, no state-machine or wire-format change). One final code review at PR time; no per-task review. No new unit tests; verification via walkthrough.

---

## File Structure

| File | Role | Tasks |
|------|------|-------|
| `refbox/src/app/view_builders/beep_test_settings.rs` | `build_edit_panel` cap value; `build_editor_levels_table` filler padding | Task 1 (cap), Task 2 (padding) |
| `refbox/src/app/mod.rs` | `Message::BeepTestEditAddLevel` handler cap guard | Task 1 |
| `refbox/src/app/view_builders/beep_test.rs` | `build_levels_table` filler padding | Task 2 |

Two implementation tasks plus a walkthrough verification task.

---

### Task 1: Bump cap value from 10 to 15

**Files:**
- Modify: `refbox/src/app/view_builders/beep_test_settings.rs` (`build_edit_panel`, around the `add_disabled = levels.len() >= 10` line that landed in Chunk 7)
- Modify: `refbox/src/app/mod.rs` (`Message::BeepTestEditAddLevel` handler, around the `if levels.len() < 10` guard that landed in Chunk 7)

- [ ] **Step 1: Update the view-side gate in `beep_test_settings.rs`**

In `refbox/src/app/view_builders/beep_test_settings.rs`, find the line introduced by Chunk 7 inside `build_edit_panel`:

```rust
let add_disabled = levels.len() >= 10;
```

Change to:

```rust
let add_disabled = levels.len() >= 15;
```

No other change in this file for Task 1.

- [ ] **Step 2: Update the handler-side guard in `mod.rs`**

In `refbox/src/app/mod.rs`, find the `Message::BeepTestEditAddLevel` handler. The Chunk 7 code is:

```rust
            Message::BeepTestEditAddLevel => {
                if let Some(ref mut edited) = self.edited_settings {
                    if let Some(ref mut levels) = edited.beep_test_levels {
                        if levels.len() < 10 {
                            levels.push(crate::config::Level {
                                count: 4,
                                duration: std::time::Duration::from_secs(20),
                            });
                            edited.selected_level = levels.len() - 1;
                        }
                    }
                }
                Task::none()
            }
```

Change the `if levels.len() < 10` to `if levels.len() < 15`. The rest of the handler stays the same.

- [ ] **Step 3: Verify the workspace compiles**

```
cargo build -p refbox 2>&1 | tail -5
```

Expected: clean compile.

- [ ] **Step 4: Run `just check`**

```
just check
```

Expected: PASS.

- [ ] **Step 5: Commit**

```
git add refbox/src/app/view_builders/beep_test_settings.rs refbox/src/app/mod.rs
git commit -m "feat(refbox): raise beep-test level cap from 10 to 15"
```

With Co-Authored-By footer:

```
Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

---

### Task 2: Lock column widths via 15-cell filler padding (both views)

**Files:**
- Modify: `refbox/src/app/view_builders/beep_test_settings.rs` (`build_editor_levels_table`)
- Modify: `refbox/src/app/view_builders/beep_test.rs` (`build_levels_table`)

The mechanism is the same in both views: after the header row is populated with real cells, push `(15 - levels.len())` `filler_cell()` calls onto the header row before pushing the row to the column. Then inside the `for row_idx in 0..max_count` loop, after the cell row is populated, push `(15 - levels.len())` `filler_cell()` calls onto the cell row before pushing the row to the column. The `filler_cell()` helper is already in scope in both files (defined in `beep_test_settings.rs` and re-imported as needed).

- [ ] **Step 1: Update `build_editor_levels_table` in `beep_test_settings.rs`**

In `refbox/src/app/view_builders/beep_test_settings.rs`, find `build_editor_levels_table`. After Chunk 7 it looks like:

```rust
fn build_editor_levels_table(levels: &[Level], selected: usize) -> Element<'_, Message> {
    let max_count = levels
        .iter()
        .map(|l| l.count as usize)
        .max()
        .unwrap_or(0);

    let mut rows = Column::new().spacing(SPACING);

    // Header row: column headers (1-indexed for the operator).
    let mut header_row = Row::new().spacing(SPACING);
    for (col_idx, _level) in levels.iter().enumerate() {
        let is_selected = col_idx == selected;
        header_row = header_row.push(editor_header_cell(
            (col_idx + 1).to_string(),
            col_idx,
            is_selected,
        ));
    }
    rows = rows.push(header_row);

    // Cell rows: stacked vertically. Each cell is a tappable button that
    // selects the column it belongs to. Empty rows beyond a column's count
    // render as filler.
    for row_idx in 0..max_count {
        let mut cell_row = Row::new().spacing(SPACING);
        for (col_idx, level) in levels.iter().enumerate() {
            if row_idx < level.count as usize {
                let is_selected = col_idx == selected;
                cell_row = cell_row.push(editor_value_cell(
                    level.duration.as_secs().to_string(),
                    col_idx,
                    is_selected,
                ));
            } else {
                cell_row = cell_row.push(filler_cell());
            }
        }
        rows = rows.push(cell_row);
    }

    container(rows)
        .padding(EDIT_TABLE_CELL_SPACING)
        .width(Length::Fill)
        .center_x(Length::Fill)
        .into()
}
```

Insert one right-padding loop in the header section (between the `for (col_idx, _level)` loop and the `rows.push(header_row)`), and one right-padding loop in the cell-row section (between the inner per-column `for` loop and `rows.push(cell_row)`). After the change:

```rust
fn build_editor_levels_table(levels: &[Level], selected: usize) -> Element<'_, Message> {
    let max_count = levels
        .iter()
        .map(|l| l.count as usize)
        .max()
        .unwrap_or(0);

    let mut rows = Column::new().spacing(SPACING);

    // Header row: column headers (1-indexed for the operator). Pad the
    // right side with filler cells so the row always has 15 cells; this
    // locks each real column's width to 1/15 of the row regardless of
    // levels.len().
    let mut header_row = Row::new().spacing(SPACING);
    for (col_idx, _level) in levels.iter().enumerate() {
        let is_selected = col_idx == selected;
        header_row = header_row.push(editor_header_cell(
            (col_idx + 1).to_string(),
            col_idx,
            is_selected,
        ));
    }
    for _ in levels.len()..15 {
        header_row = header_row.push(filler_cell());
    }
    rows = rows.push(header_row);

    // Cell rows: stacked vertically. Each cell is a tappable button that
    // selects the column it belongs to. Empty rows beyond a column's count
    // render as filler. Each row is padded on the right to 15 cells so the
    // column widths stay locked.
    for row_idx in 0..max_count {
        let mut cell_row = Row::new().spacing(SPACING);
        for (col_idx, level) in levels.iter().enumerate() {
            if row_idx < level.count as usize {
                let is_selected = col_idx == selected;
                cell_row = cell_row.push(editor_value_cell(
                    level.duration.as_secs().to_string(),
                    col_idx,
                    is_selected,
                ));
            } else {
                cell_row = cell_row.push(filler_cell());
            }
        }
        for _ in levels.len()..15 {
            cell_row = cell_row.push(filler_cell());
        }
        rows = rows.push(cell_row);
    }

    container(rows)
        .padding(EDIT_TABLE_CELL_SPACING)
        .width(Length::Fill)
        .center_x(Length::Fill)
        .into()
}
```

- [ ] **Step 2: Update `build_levels_table` in `beep_test.rs`**

In `refbox/src/app/view_builders/beep_test.rs`, find `build_levels_table`. After Chunk 7 it looks like:

```rust
fn build_levels_table(
    levels: &[crate::config::Level],
    active_level: Option<usize>,
    active_within_lap: Option<u32>,
    clock_running: bool,
) -> Element<'_, Message> {
    let max_count = levels.iter().map(|l| l.count as usize).max().unwrap_or(0);

    // Pre-compute the column state for each level so we don't call
    // `compute_column_state` repeatedly in both the header loop and the
    // inner cell-row loop.
    let column_states: Vec<ColumnState> = levels
        .iter()
        .enumerate()
        .map(|(col_idx, _)| compute_column_state(active_level, col_idx + 1))
        .collect();

    let mut rows = Column::new().spacing(SPACING);

    // Header row: level numbers (1-indexed). Past columns are grayed
    // out (disabled look); active and future columns use green headers.
    let mut header_row = Row::new().spacing(SPACING);
    for (col_idx, _level) in levels.iter().enumerate() {
        header_row = header_row.push(header_cell(
            (col_idx + 1).to_string(),
            column_states[col_idx],
        ));
    }
    rows = rows.push(header_row);

    // Cell rows: stacked vertically. Row 0 is the first lap, row 1 the
    // second, etc. A column has `level.count` cells; rows beyond a
    // column's count are empty space.
    for row_idx in 0..max_count {
        let mut cell_row = Row::new().spacing(SPACING);
        for (col_idx, level) in levels.iter().enumerate() {
            if row_idx < level.count as usize {
                let column_state = column_states[col_idx];
                let active_now = if clock_running {
                    CellState::Active
                } else {
                    CellState::ActivePaused
                };
                let cell_state = match column_state {
                    ColumnState::Past => CellState::Completed,
                    ColumnState::Active => match active_within_lap {
                        Some(within) if (within as usize) == row_idx + 1 => active_now,
                        Some(within) if (within as usize) > row_idx + 1 => CellState::Completed,
                        _ => CellState::Default,
                    },
                    ColumnState::Future => CellState::Default,
                };
                cell_row =
                    cell_row.push(value_cell(level.duration.as_secs().to_string(), cell_state));
            } else {
                cell_row = cell_row.push(filler_cell());
            }
        }
        rows = rows.push(cell_row);
    }

    container(rows)
        .padding(TABLE_CELL_SPACING)
        .width(Length::Fill)
        .center_x(Length::Fill)
        .into()
}
```

Insert the same two right-padding loops as in Step 1 (after the header `for` loop, before `rows.push(header_row)`; and after the inner per-column cell `for` loop, before `rows.push(cell_row)`). After the change:

```rust
fn build_levels_table(
    levels: &[crate::config::Level],
    active_level: Option<usize>,
    active_within_lap: Option<u32>,
    clock_running: bool,
) -> Element<'_, Message> {
    let max_count = levels.iter().map(|l| l.count as usize).max().unwrap_or(0);

    // Pre-compute the column state for each level so we don't call
    // `compute_column_state` repeatedly in both the header loop and the
    // inner cell-row loop.
    let column_states: Vec<ColumnState> = levels
        .iter()
        .enumerate()
        .map(|(col_idx, _)| compute_column_state(active_level, col_idx + 1))
        .collect();

    let mut rows = Column::new().spacing(SPACING);

    // Header row: level numbers (1-indexed). Past columns are grayed
    // out (disabled look); active and future columns use green headers.
    // Pad the right side with filler cells so the row always has 15 cells;
    // this locks each real column's width to 1/15 of the row regardless of
    // levels.len().
    let mut header_row = Row::new().spacing(SPACING);
    for (col_idx, _level) in levels.iter().enumerate() {
        header_row = header_row.push(header_cell(
            (col_idx + 1).to_string(),
            column_states[col_idx],
        ));
    }
    for _ in levels.len()..15 {
        header_row = header_row.push(filler_cell());
    }
    rows = rows.push(header_row);

    // Cell rows: stacked vertically. Row 0 is the first lap, row 1 the
    // second, etc. A column has `level.count` cells; rows beyond a
    // column's count are empty space. Each row is padded on the right to
    // 15 cells so the column widths stay locked.
    for row_idx in 0..max_count {
        let mut cell_row = Row::new().spacing(SPACING);
        for (col_idx, level) in levels.iter().enumerate() {
            if row_idx < level.count as usize {
                let column_state = column_states[col_idx];
                let active_now = if clock_running {
                    CellState::Active
                } else {
                    CellState::ActivePaused
                };
                let cell_state = match column_state {
                    ColumnState::Past => CellState::Completed,
                    ColumnState::Active => match active_within_lap {
                        Some(within) if (within as usize) == row_idx + 1 => active_now,
                        Some(within) if (within as usize) > row_idx + 1 => CellState::Completed,
                        _ => CellState::Default,
                    },
                    ColumnState::Future => CellState::Default,
                };
                cell_row =
                    cell_row.push(value_cell(level.duration.as_secs().to_string(), cell_state));
            } else {
                cell_row = cell_row.push(filler_cell());
            }
        }
        for _ in levels.len()..15 {
            cell_row = cell_row.push(filler_cell());
        }
        rows = rows.push(cell_row);
    }

    container(rows)
        .padding(TABLE_CELL_SPACING)
        .width(Length::Fill)
        .center_x(Length::Fill)
        .into()
}
```

- [ ] **Step 3: Verify the workspace compiles**

```
cargo build -p refbox 2>&1 | tail -5
```

Expected: clean compile.

- [ ] **Step 4: Run `just check`**

```
just check
```

Expected: PASS.

- [ ] **Step 5: Commit**

```
git add refbox/src/app/view_builders/beep_test_settings.rs refbox/src/app/view_builders/beep_test.rs
git commit -m "feat(refbox): lock beep-test column widths to 15-cell grid"
```

With Co-Authored-By footer.

---

### Task 3: Walkthrough verification

**Files:** none. Smoke-test the running refbox.

Per `.claude/rules/pr-review.md`: "Smoke-tested locally — refbox (or the affected artifact) was launched and the change exercised in a real session before any push/PR/merge/tag-push. CI green ≠ smoke-tested."

- [ ] **Step 1: Launch the refbox**

```
WAYLAND_DISPLAY= cargo run --manifest-path /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign/Cargo.toml -p refbox
```

(Operator config has `mode = "BeepTest"`, so refbox comes up directly in BeepTest mode.)

- [ ] **Step 2: Scenario 1 — single-level locked width**

Navigate **Main → Settings → Edit Levels**. If the staged config has > 1 level, tap REMOVE LEVEL repeatedly until exactly 1 level remains.

Confirm:
- The single active column occupies roughly 1/15 of the table width (visually narrow, on the left).
- The rest of the table area is transparent / empty space.

- [ ] **Step 3: Scenario 2 — adding levels does not resize existing columns**

From 1 level, tap ADD LEVEL repeatedly while watching the existing columns. Confirm:
- Each new column appears at the right edge of the active region.
- The width of the existing real columns does NOT change.
- The transparent space on the right shrinks as more real columns appear.

- [ ] **Step 4: Scenario 3 — 15-cap reached**

Continue tapping ADD LEVEL until 15 levels exist. Confirm:
- At exactly 15, the table fills the full row (no transparent space on the right).
- ADD LEVEL grays out (disabled style).
- Tapping the disabled ADD LEVEL does nothing — no 16th column appears.

- [ ] **Step 5: Scenario 4 — removing shifts left**

With 15 levels, tap REMOVE LEVEL once. Confirm:
- A column disappears (the selected one moves out; selection moves to a remaining column).
- ADD LEVEL re-enables (green, responsive).
- The remaining real columns stay at their locked 1/15 width — they do not visibly expand.

- [ ] **Step 6: Scenario 5 — main view (BeepTest screen) layout sanity**

Save the staged levels (Apply on the Edit Levels page) and return to the main BeepTest screen. Confirm the main view's transposed levels table also renders in the 15-cell-locked grid:
- If the saved config has fewer than 15 levels, the table's real columns occupy the left side at locked-width and the right side is transparent space.
- The active-column highlight (if a beep test is running) still highlights the correct column.

- [ ] **Step 7: Hand back to operator**

Report walkthrough results for Scenarios 1–5. If any scenario fails or behaves unexpectedly, stop and report — do not push/PR.

---

## Deviations

(Append notes here if execution deviates from the plan. Per `.claude/rules/plan-execution.md`, fold deviation notes into the code commit that introduced the deviation; no standalone doc-only deviation commits.)

---

## Self-review notes

- **Spec coverage:**
  - Spec §Cap value change → Task 1 Steps 1–2 (both sites).
  - Spec §Column-width lock via 15-cell row padding → Task 2 Steps 1–2 (both views, both per-row padding sites).
  - Spec §Testing (5 walkthrough scenarios) → Task 3 Steps 2–6.
  - Spec §Acceptance criteria (`just check` green + walkthrough pass) → Task 1 Step 4 + Task 2 Step 4 + Task 3.
- **No placeholders:** all steps show concrete code/commands. Both view-file edits show the complete function body before and after (no "similar to Task N" handwave).
- **Type consistency:** No new types or identifiers introduced. `filler_cell()`, `editor_header_cell`, `editor_value_cell`, `header_cell`, `value_cell`, `Column`, `Row`, `Length`, and the `levels.len()` predicate are already in scope in each respective file. Both view files already use `filler_cell()` (added in Chunk 7) for the per-column rows-shorter-than-max_count case.
- **Task ordering:** cap value bump first (Task 1) — minimal change, isolates the cap-related behaviour change. Filler padding second (Task 2) — the visual layout change. Each task is independently green via `just check`. The walkthrough scenarios specifically test both behaviours and their interaction (Scenario 3 hits the 15-cap; Scenarios 1–2/4 exercise the locked width).
- **Why no separate cap-related-test:** there's no test infrastructure for `Message::BeepTestEditAddLevel` in the codebase (zero `#[test]` attributes in `mod.rs`), and adding it for a one-character change is disproportionate; the handler-side guard is a defense-in-depth mirror of a view-side gate that is itself a literal value change. Walkthrough Scenario 3 verifies end-to-end.
