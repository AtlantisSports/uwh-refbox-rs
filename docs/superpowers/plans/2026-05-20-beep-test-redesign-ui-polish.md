# Beep-Test Redesign — UI Polish — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Apply the seven UI polish items from the operator's request to the beep-test mode: new three-cell top row on the main page, green column headers with yellow active lap, completed-level disabled look, two-layer-per-standard-row band sizing, blue RESTART TO APPLY button, and removal of the time banner from all beep-test sub-pages.

**Architecture:** Pure view-builder changes in `beep_test.rs` and `beep_test_settings.rs`, plus translation-key additions and removals in 15 locale files. No state-machine, snapshot, wire-format, or message-enum changes.

**Tech Stack:** Rust 2024, MSRV 1.85, `iced` 0.13, `fluent` translations via `fl!`.

**Spec:** `docs/superpowers/specs/2026-05-20-beep-test-redesign-ui-polish-design.md`

**Process:** Lean (per `.claude/rules/plan-execution.md` — refbox UI work). No per-task deviation commits, one bundled walkthrough + code-review at the end.

---

## File Structure

| File | Role | Tasks |
|------|------|-------|
| `refbox/translations/<locale>/refbox.ftl` × 15 | Add 3 new top-row label keys; remove 2 unused inline-label keys | Task 1 |
| `refbox/src/app/view_builders/beep_test.rs` | Main-page top row + levels-table styling + band sizing | Tasks 2, 3, 4 |
| `refbox/src/app/view_builders/beep_test_settings.rs` | Remove time banner from 4 sub-pages; RESTART TO APPLY color | Tasks 5, 6 |

No new files. No file relocations.

---

## Style constants (for reference, do not change)

From `refbox/src/app/theme/mod.rs`:
- `MIN_BUTTON_SIZE = 89.0`
- `SPACING = 8.0`
- `PADDING = 8.0`

Derived in Task 4: cell height in the levels table = `(MIN_BUTTON_SIZE - SPACING) / 2.0 = 40.5`.

---

### Task 1: Translation keys

**Files:**
- Modify: all 15 of `refbox/translations/<locale>/refbox.ftl` (de-DE, en-US, es, fr, id-ID, it-IT, ja-JP, ko-KR, ms-MY, nl-NL, pt-PT, th-TH, tl-PH, tr-TR, zh-CN)

Mechanical translation-only work. Per `.claude/rules/plan-execution.md`, no verification-before-completion needed — `just check` is enough.

- [ ] **Step 1: Add three new keys to each locale**

In each `refbox.ftl` (place near the existing `beep-test-*` block around line 360 of en-US):

```ftl
beep-test-top-time-label = TIME
beep-test-top-level-label = LEVEL
beep-test-top-lap-label = LAP
```

For each non-English locale, use that locale's existing translation for "TIME", "LEVEL", "LAP". Where the locale already translates `beep-test-edit-time` ("TIME") or `beep-test-column-level` ("LEVEL"), reuse those exact translations for the new top-row keys. For "LAP" (singular), derive from the locale's existing "LAPS" (`beep-test-laps`) translation by removing the plural marker / colon — e.g. Spanish "VUELTAS:" → "VUELTA", French "TOURS:" → "TOUR". If unsure for a non-Latin locale, use the same word the locale uses for a single lap in beep-test context; non-verified flags are not required (these are existing concepts in each locale).

- [ ] **Step 2: Remove the two now-unused inline keys from each locale**

Delete these lines from every `refbox.ftl`:

```ftl
beep-test-level = LEVEL { $level }
beep-test-laps = LAPS: { $laps }
```

(Locale-specific wording, same lines.)

- [ ] **Step 3: Verify**

```
just check
```

Expected: PASS. Compilation succeeds — these keys are still referenced in `beep_test.rs` but will be replaced in Task 2; for Task 1 alone the build will fail at `beep_test.rs:75` and `:85`. Do Task 1 and Task 2 in the same working state and commit them together; if you must commit Task 1 alone, expect a transient compile failure on those two lines.

- [ ] **Step 4: Commit (with Task 2)**

Combined commit at end of Task 2.

---

### Task 2: New three-cell top row on the main page

**Files:**
- Modify: `refbox/src/app/view_builders/beep_test.rs`

- [ ] **Step 1: Replace the existing top-banner block**

In `build_beep_test_page`, delete the block that computes `time_text`, `level_label`, `laps_label`, and builds the conditional `header` (the `column!` / `row!` based on `band_count` and `BAND_COUNT_COLLAPSE_THRESHOLD`).

Replace with a single three-cell row of `make_value_button`. `make_value_button` is already in scope via the existing `use super::*;` at the top of the file (re-exported through `view_builders/mod.rs:48`). Each cell is `Length::Fill` wide, fixed `MIN_BUTTON_SIZE` tall:

```rust
let time_value = secs_to_long_time_string(snapshot.secs_in_period)
    .trim()
    .to_owned();

let level_value: String = match snapshot.current_period {
    BeepTestPeriod::Level(i) => i.to_string(),
};

let lap_value: String = match snapshot.current_period {
    BeepTestPeriod::Level(0) => 1.to_string(),
    BeepTestPeriod::Level(_) => active_within_lap.unwrap_or(1).to_string(),
};

let top_row = row![
    make_value_button(fl!("beep-test-top-time-label"), time_value, (true, true), None)
        .width(Length::Fill),
    make_value_button(fl!("beep-test-top-level-label"), level_value, (true, true), None)
        .width(Length::Fill),
    make_value_button(fl!("beep-test-top-lap-label"), lap_value, (true, true), None)
        .width(Length::Fill),
]
.spacing(SPACING);
```

The `(true, true)` for `large_text` makes both header and value MEDIUM_TEXT (matching how other refbox pages render a value tile). `None` for the message makes the buttons non-interactive (they're info-only).

- [ ] **Step 2: Delete now-unused helpers**

In the same file, delete:
- `fn time_bar` (the old full-width yellow-digit banner)
- `fn info_widget` (the old `[LEVEL: N]` / `[LAPS: N]` tile)
- the `BAND_COUNT_COLLAPSE_THRESHOLD` constant

Also remove the now-unused `use` of `matrix_drawing::secs_to_long_time_string` if it becomes unused — it's still used by the new `time_value` line, so it stays.

- [ ] **Step 3: Wire the new row into the page column**

Replace the previous `column![header, ...]` arrangement with:

```rust
column![
    top_row,
    container(levels_table).width(Length::Fill),
    row![horizontal_space()].height(Length::Fill),  // absorbs leftover vertical space
    bottom_row,
]
.spacing(SPACING)
.padding(PADDING)
.width(Length::Fill)
.height(Length::Fill)
.into()
```

Note: the `container(levels_table)` no longer has `.height(Length::Fill)` — the table sizes to its content (height-driven by Task 4's band-sizing rule). The `row![horizontal_space()].height(Length::Fill)` row absorbs leftover vertical space so the bottom action row stays anchored at the bottom. (This row pattern matches the existing usage in `beep_test_settings.rs:120`.)

- [ ] **Step 4: Verify build**

```
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign
just check
```

Expected: PASS.

- [ ] **Step 5: Commit (Task 1 + Task 2 together)**

```
git add refbox/translations refbox/src/app/view_builders/beep_test.rs
git commit -m "feat(refbox): replace beep-test top banner with 3-cell value row"
```

---

### Task 3: Levels-table cell-state and colors

**Files:**
- Modify: `refbox/src/app/view_builders/beep_test.rs`

Extends the existing `CellState` enum and rewrites the per-column state derivation so past columns (entire prior levels) render disabled in addition to in-active-column completed laps.

- [ ] **Step 1: Change header-cell colors**

In `fn header_cell`, replace the existing `if is_active { yellow_button } else { light_gray_container }` block with three-state logic. Introduce a new parameter so the caller passes column state:

```rust
#[derive(Clone, Copy)]
enum ColumnState {
    Past,
    Active,
    Future,
}

fn header_cell<'a>(label: String, state: ColumnState) -> Element<'a, Message> {
    let inner = text(label)
        .size(SMALL_TEXT)
        .align_x(Horizontal::Center)
        .width(Length::Fill);
    match state {
        ColumnState::Past => container(inner)
            .style(disabled_container)
            .padding(TABLE_CELL_SPACING)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),
        ColumnState::Active | ColumnState::Future => button(
            container(inner)
                .center_x(Length::Fill)
                .center_y(Length::Fill),
        )
        .style(green_button)
        .padding(TABLE_CELL_SPACING)
        .width(Length::Fill)
        .into(),
    }
}
```

- [ ] **Step 2: Update cell-state derivation for value cells**

In `build_levels_table`, replace the existing per-cell state computation so it uses `ColumnState`:

```rust
let column_state = match active_level {
    None => ColumnState::Future,
    Some(active) if level_number < active => ColumnState::Past,
    Some(active) if level_number == active => ColumnState::Active,
    _ => ColumnState::Future,
};

// Header
header_row = header_row.push(header_cell(level_number.to_string(), column_state));

// Value cells (inside the loop)
let cell_state = match column_state {
    ColumnState::Past => CellState::Completed,
    ColumnState::Active => match active_within_lap {
        Some(within) if (within as usize) == row_idx + 1 => CellState::Active,
        Some(within) if (within as usize) > row_idx + 1 => CellState::Completed,
        _ => CellState::Default,
    },
    ColumnState::Future => CellState::Default,
};
```

`CellState` itself stays the same — `Default` / `Active` / `Completed` already render correctly. Only the input mapping changes.

- [ ] **Step 3: Add unit tests for the new column-state logic**

Append to the `#[cfg(test)] mod tests` block at the bottom of `beep_test.rs`:

```rust
#[test]
fn column_state_no_active_level_is_all_future() {
    // When the engine is in warmup (Level 0), `active_level` is None and
    // every column should render as Future.
    // Direct construction is sufficient — these helpers are internal.
    assert!(matches!(
        compute_column_state(None, 1),
        ColumnState::Future
    ));
    assert!(matches!(
        compute_column_state(None, 5),
        ColumnState::Future
    ));
}

#[test]
fn column_state_active_marks_only_active_column() {
    assert!(matches!(
        compute_column_state(Some(3), 1),
        ColumnState::Past
    ));
    assert!(matches!(
        compute_column_state(Some(3), 2),
        ColumnState::Past
    ));
    assert!(matches!(
        compute_column_state(Some(3), 3),
        ColumnState::Active
    ));
    assert!(matches!(
        compute_column_state(Some(3), 4),
        ColumnState::Future
    ));
}
```

This requires extracting the column-state computation into a testable helper:

```rust
fn compute_column_state(active_level: Option<usize>, column_one_based: usize) -> ColumnState {
    match active_level {
        None => ColumnState::Future,
        Some(active) if column_one_based < active => ColumnState::Past,
        Some(active) if column_one_based == active => ColumnState::Active,
        _ => ColumnState::Future,
    }
}
```

Use this helper in `build_levels_table` instead of the inline match (DRY).

- [ ] **Step 4: Run tests**

```
cd refbox && cargo test -p refbox --lib app::view_builders::beep_test::tests
```

Expected: PASS, including the two new tests plus the existing `within_level_lap_*` tests.

- [ ] **Step 5: Verify build**

```
just check
```

Expected: PASS.

- [ ] **Step 6: Commit**

```
git add refbox/src/app/view_builders/beep_test.rs
git commit -m "feat(refbox): green level headers; past levels show disabled look"
```

---

### Task 4: Two-layer-per-standard-row band sizing

**Files:**
- Modify: `refbox/src/app/view_builders/beep_test.rs`

Fixes the cell height so two stacked layers + standard SPACING = one MIN_BUTTON_SIZE row. Pads odd-layer bands with one blank row.

- [ ] **Step 1: Add a cell-height constant**

Near the existing constants at the top of `beep_test.rs`:

```rust
/// Height of one table-cell layer (header or lap row). Sized so that two
/// stacked layers plus the standard SPACING between them equal one
/// MIN_BUTTON_SIZE row, matching the rest of refbox's row rhythm.
const TABLE_CELL_HEIGHT: f32 = (MIN_BUTTON_SIZE - SPACING) / 2.0;
```

- [ ] **Step 2: Apply the fixed height to every cell**

Update `header_cell`, `value_cell`, and `filler_cell` so each returns a container/button of `Length::Fixed(TABLE_CELL_HEIGHT)` height.

For `header_cell` (in both `Past` and `Active | Future` arms): add `.height(Length::Fixed(TABLE_CELL_HEIGHT))` to the container/button.

For `value_cell` (all three states): same — add `.height(Length::Fixed(TABLE_CELL_HEIGHT))`.

For `filler_cell`: change from `Space::with_width(Length::Fill)` to:

```rust
fn filler_cell<'a>() -> Element<'a, Message> {
    Space::new(Length::Fill, Length::Fixed(TABLE_CELL_HEIGHT)).into()
}
```

- [ ] **Step 3: Change inter-row spacing within a band to standard SPACING**

In `build_levels_table`, the per-band layout uses `Row::new().spacing(TABLE_CELL_SPACING)` for horizontal spacing (between columns inside a row — this stays as the tight 2.0 spacing for column-width tightness).

But the band-level `Column::new().spacing(SPACING)` for the vertical stack: it's currently already `SPACING`. Verify and leave it. If you find the existing code uses a smaller value for the vertical spacing inside a band, change it to `SPACING`.

- [ ] **Step 4: Pad odd-layer bands with a blank row**

After pushing the header row and all `max_count` lap rows for a band, compute:

```rust
let layer_count = 1 + max_count; // 1 header + max_count lap rows
if layer_count % 2 == 1 {
    // Append one blank row of filler cells so the band's total layer count
    // is even — keeps the band's total height a whole number of MIN_BUTTON_SIZE
    // rows when combined with SPACING.
    let mut blank_row = Row::new().spacing(TABLE_CELL_SPACING);
    for _ in 0..BAND_WIDTH {
        blank_row = blank_row.push(filler_cell());
    }
    bands = bands.push(blank_row);
}
```

(`BAND_WIDTH` stays = 10. The blank row's full BAND_WIDTH filler count keeps column alignment with neighbouring bands.)

- [ ] **Step 5: Verify build**

```
just check
```

Expected: PASS.

- [ ] **Step 6: Commit**

```
git add refbox/src/app/view_builders/beep_test.rs
git commit -m "feat(refbox): size beep-test table cells to two-layer-per-row standard"
```

---

### Task 5: Remove the time banner from BeepTest sub-pages

**Files:**
- Modify: `refbox/src/app/view_builders/beep_test_settings.rs`

Removes the `make_game_time_button(...)` call and its row from each of four sub-pages, then redistributes the column layout so the bottom footer stays at the bottom and content keeps the standard row height.

- [ ] **Step 1: Settings landing — `build_beep_test_settings_landing`**

Locate the existing column:

```rust
column![
    make_game_time_button(snapshot, false, false, mode, clock_running, portal_indicator),
    row_top,
    row_bottom,
    row![horizontal_space()].height(Length::Fill),
    row![horizontal_space()].height(Length::Fill),
    row![horizontal_space()].height(Length::Fill),
    bottom_row,
]
```

Remove the `make_game_time_button` line. Result: 2 content rows + 3 filler rows + bottom_row. The filler-row count stays at 3 — losing the top banner gives all rows more vertical space evenly. No row-count math beyond that.

Remove the `snapshot`, `mode`, `clock_running`, `portal_indicator` parameters from the function signature **if they become unused** after this edit. Also propagate that signature change to the dispatch site in `refbox/src/app/mod.rs` (around line 3824 for the landing; around 3844, 3869, 3890 for the other three sub-pages).

- [ ] **Step 2: Sound Settings — `build_beep_test_sound_settings_page`**

Same change: delete the `make_game_time_button` row from the column macro. Drop the four banner-only parameters from the function signature if unused, update the caller.

- [ ] **Step 3: Edit Levels — `build_beep_test_edit_levels_page`**

Same change: delete the `make_game_time_button` row. Drop unused parameters; update the caller. (Per spec, this is the ONLY change to the Edit Levels page in this task; do not touch the table + edit-panel layout.)

- [ ] **Step 4: Language picker — `build_beep_test_language_picker`**

Same change. The language picker's column has 4 language rows + 1 filler + footer; with the banner removed it becomes 4 language rows + 1 filler + footer with each Fill share slightly larger. Keep the existing filler-row count.

- [ ] **Step 5: Remove the `make_game_time_button` import**

If after the four removals it's no longer used in `beep_test_settings.rs`, delete the `use` from the top of the file. Same for `GameSnapshot` and `PortalIndicatorState` if they become unused.

- [ ] **Step 6: Verify build**

```
just check
```

Expected: PASS.

- [ ] **Step 7: Commit**

```
git add refbox/src/app/view_builders/beep_test_settings.rs refbox/src/app/mod.rs
git commit -m "feat(refbox): remove time banner from beep-test sub-pages"
```

(Include `mod.rs` if call sites changed.)

---

### Task 6: Change RESTART TO APPLY button from green to blue

**Files:**
- Modify: `refbox/src/app/view_builders/beep_test_settings.rs`

One-line change.

- [ ] **Step 1: Swap the style**

In `build_beep_test_settings_landing`, find the line:

```rust
let restart_button = make_button(fl!("restart-to-apply"))
    .style(green_button)
    .on_press(Message::BeepTestRestartToApply);
```

Change `green_button` → `blue_button`. Verify `blue_button` is already imported in this file via the `use super::*;` at the top (`blue_button` is a standard theme style; the `use super::*` already brings it in).

- [ ] **Step 2: Verify build**

```
just check
```

Expected: PASS.

- [ ] **Step 3: Commit**

```
git add refbox/src/app/view_builders/beep_test_settings.rs
git commit -m "feat(refbox): beep-test restart-to-apply button now blue"
```

---

### Task 7: Walkthrough verification

**Files:** none. Smoke-test the running refbox.

- [ ] **Step 1: Launch the refbox**

```
WAYLAND_DISPLAY= cargo run -p refbox
```

(`dangerouslyDisableSandbox: true` needed when launching via Claude — see memory.)

- [ ] **Step 2: Switch into BeepTest mode if not already there**

From the main view, settings → app config → Mode → BeepTest → restart, OR if already in BeepTest, skip.

- [ ] **Step 3: Verify each acceptance criterion from the spec**

Walk through scenarios 1–7 from the design spec's Acceptance Criteria section. Mark each as PASS / FAIL in a brief note. Specifically:

1. Top row shows three side-by-side cells (TIME / LEVEL / LAP) at standard row height.
2. All column headers in the levels table are green when stopped.
3. Start the cadence engine; observe the active lap go yellow, watch the engine advance through a full level, and verify the prior column's header + all its cells flip to the disabled look and stay there.
4. Visually confirm two stacked cells fit in one standard row height; a band with an odd layer count shows one blank row at the band's bottom.
5. Side-by-side comparison with a hockey page — top/bottom rows and spacing identical.
6. Settings → change App Mode → bottom RESTART TO APPLY button is blue.
7. Open each of the 4 sub-pages — no time banner at top, footer stays at the bottom.

- [ ] **Step 4: Run the full check**

```
just check
```

Expected: PASS.

- [ ] **Step 5: Hand back to operator**

Report walkthrough results. Do not push. Do not open a PR — per memory `project_beep_test_redesign_b2_complete`, this branch is being held for a stacked PR with Branch 3.

---

## Deviations

(Append notes here if execution deviates from the plan. Per `.claude/rules/plan-execution.md`, no per-task deviation commits — single note per deviation, with rationale.)

### Task 2 — top-row tiles use a local `top_row_tile` helper instead of `make_value_button`

The plan and spec both name `make_value_button` for the three top-row tiles. During implementation (commit `050d818`), `make_value_button(..., None)` rendered the cells with iced's disabled style (gray text on washed-out background) because iced 0.13 puts any `Button` without `on_press` into `Status::Disabled`. The fix-up commit (`5b4ccb1`) replaced the three calls with a local `top_row_tile` helper that mirrors `make_value_button`'s internal layout (label / `horizontal_space` / value, `MEDIUM_TEXT`, `MIN_BUTTON_SIZE` tall) but is built as a `Container` styled with `light_gray_container`. Visually equivalent to a non-disabled `make_value_button`, no new `Message` variant needed.

### Task 3 — `header_cell` and `value_cell::Active` use containers, not buttons; added `yellow_container` style

Same iced-disabled-button pitfall as Task 2 hit again. The plan prescribed `button(...).style(green_button)` for the `Active | Future` arm of `header_cell` and the existing code used `button(...).style(yellow_button)` for the `CellState::Active` arm of `value_cell`. Both render gray under `Status::Disabled` (no `on_press`). The fix-up commit (`d682a03`) replaced both with `container(...).style(<color>_container)` patterns. `green_container` already existed in `refbox/src/app/theme/container.rs`; `yellow_container` was added there (six lines, mirroring the existing `green_container` definition) and re-exported from `theme/mod.rs`. This expands the file scope of the change beyond what the spec listed (theme files were originally out of scope), but the addition is mechanical and follows established patterns.

The `value_cell::Active` fix also corrects a pre-existing bug from Branch 2 of the redesign (active lap rendered gray instead of yellow). This was not previously caught because Branch 2's walkthrough didn't run the cadence engine far enough for the operator to notice the active lap colour.

---

## Self-review notes

- **Spec coverage:** items 1–7 of the spec each map to one of Tasks 2–7 (item 5 is the standard-row-rhythm goal, achieved as a side-effect of Tasks 2 + 4). Translation work covered by Task 1.
- **Type consistency:** `ColumnState` and the `compute_column_state` helper introduced in Task 3 are used consistently in the same task; `TABLE_CELL_HEIGHT` introduced in Task 4 is consumed by `header_cell` / `value_cell` / `filler_cell` updated in the same task. No forward references.
- **No placeholders:** all steps contain the actual code or command needed.
- **Lean process:** seven tasks, ~250 lines of plan, no per-task verification ceremony on Task 1 (mechanical). Final walkthrough at Task 7.
