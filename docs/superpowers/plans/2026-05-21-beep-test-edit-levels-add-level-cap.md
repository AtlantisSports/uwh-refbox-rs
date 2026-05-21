# Beep-Test Edit Levels: ADD LEVEL + 10-Cap + Drop Wrapping — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land Chunk 7 of the BeepTest redesign — rename the `+ NEW` button to **ADD LEVEL** with proper translations in all 15 locales (plus the paired REMOVE LEVEL translations in 12 locales that currently use English fallback), cap the number of levels at 10 with a view-side disable and a handler-side guard, and strip the now-dead `chunks(BAND_WIDTH)` row-wrapping logic from both the main view and the Edit Levels editor.

**Architecture:** Three independent changes. Translations are pure `.ftl` value edits across 15 locale files. The cap is one view-side predicate plus one handler-side guard, both using inline literal `10` (matching Chunk 1's inline-literal `5` pattern). The wrapping removal strips the `levels.chunks(BAND_WIDTH)` outer loop and band-padding logic in two view-builder files and removes the two `BAND_WIDTH = 10` / `EDIT_BAND_WIDTH = 10` constants. No `Message` enum changes, no signature changes, no new types.

**Tech Stack:** Rust 2024, MSRV 1.85; iced 0.13; Fluent for translations.

**Spec:** `docs/superpowers/specs/2026-05-21-beep-test-edit-levels-add-level-cap-design.md` (committed at `b81a1df`).

**Process:** Lean (per `.claude/rules/plan-execution.md` — refbox UI work, no state-machine or wire-format change). One final code review at PR time; no per-task review. No new unit tests; verification via walkthrough.

---

## File Structure

| File | Role | Tasks |
|------|------|-------|
| `refbox/translations/{en-US,fr,es,de-DE,id-ID,it-IT,ja-JP,ko-KR,ms-MY,nl-NL,pt-PT,th-TH,tl-PH,tr-TR,zh-CN}/refbox.ftl` | Per-locale string values for `beep-test-edit-new` (all 15) and `beep-test-edit-remove` (12 of them) | Task 1 |
| `refbox/src/app/view_builders/beep_test_settings.rs` | `build_edit_panel` — add `add_disabled` gate at both ADD LEVEL call sites | Task 2 |
| `refbox/src/app/mod.rs` | `Message::BeepTestEditAddLevel` handler — add `if levels.len() < 10` defense-in-depth guard | Task 2 |
| `refbox/src/app/view_builders/beep_test_settings.rs` | `build_editor_levels_table` — strip `chunks(EDIT_BAND_WIDTH)` loop + band-padding; remove `EDIT_BAND_WIDTH` constant | Task 3 |
| `refbox/src/app/view_builders/beep_test.rs` | Main view's table builder — strip `chunks(BAND_WIDTH)` loop + band-padding; remove `BAND_WIDTH` constant; update doc comment | Task 3 |

Three implementation tasks plus a walkthrough verification task.

---

### Task 1: Update translation values in all 15 locale files

**Files:**
- Modify: `refbox/translations/en-US/refbox.ftl`
- Modify: `refbox/translations/fr/refbox.ftl`
- Modify: `refbox/translations/es/refbox.ftl`
- Modify: `refbox/translations/de-DE/refbox.ftl`
- Modify: `refbox/translations/id-ID/refbox.ftl`
- Modify: `refbox/translations/it-IT/refbox.ftl`
- Modify: `refbox/translations/ja-JP/refbox.ftl`
- Modify: `refbox/translations/ko-KR/refbox.ftl`
- Modify: `refbox/translations/ms-MY/refbox.ftl`
- Modify: `refbox/translations/nl-NL/refbox.ftl`
- Modify: `refbox/translations/pt-PT/refbox.ftl`
- Modify: `refbox/translations/th-TH/refbox.ftl`
- Modify: `refbox/translations/tl-PH/refbox.ftl`
- Modify: `refbox/translations/tr-TR/refbox.ftl`
- Modify: `refbox/translations/zh-CN/refbox.ftl`

- [ ] **Step 1: Update `beep-test-edit-new` in every locale**

Per the spec's translation table:

| File | Old line | New line |
|---|---|---|
| `refbox/translations/en-US/refbox.ftl` | `beep-test-edit-new = + NEW` | `beep-test-edit-new = ADD LEVEL` |
| `refbox/translations/fr/refbox.ftl` | `beep-test-edit-new = + NOUVEAU` | `beep-test-edit-new = AJOUTER NIVEAU` |
| `refbox/translations/es/refbox.ftl` | `beep-test-edit-new = + NUEVO` | `beep-test-edit-new = AÑADIR NIVEL` |
| `refbox/translations/de-DE/refbox.ftl` | `beep-test-edit-new = + NEW` | `beep-test-edit-new = STUFE HINZUFÜGEN` |
| `refbox/translations/id-ID/refbox.ftl` | `beep-test-edit-new = + NEW` | `beep-test-edit-new = TAMBAH LEVEL` |
| `refbox/translations/it-IT/refbox.ftl` | `beep-test-edit-new = + NEW` | `beep-test-edit-new = AGGIUNGI LIVELLO` |
| `refbox/translations/ja-JP/refbox.ftl` | `beep-test-edit-new = + NEW` | `beep-test-edit-new = レベル追加` |
| `refbox/translations/ko-KR/refbox.ftl` | `beep-test-edit-new = + NEW` | `beep-test-edit-new = 레벨 추가` |
| `refbox/translations/ms-MY/refbox.ftl` | `beep-test-edit-new = + NEW` | `beep-test-edit-new = TAMBAH TAHAP` |
| `refbox/translations/nl-NL/refbox.ftl` | `beep-test-edit-new = + NEW` | `beep-test-edit-new = NIVEAU TOEVOEGEN` |
| `refbox/translations/pt-PT/refbox.ftl` | `beep-test-edit-new = + NEW` | `beep-test-edit-new = ADICIONAR NÍVEL` |
| `refbox/translations/th-TH/refbox.ftl` | `beep-test-edit-new = + NEW` | `beep-test-edit-new = เพิ่มระดับ` |
| `refbox/translations/tl-PH/refbox.ftl` | `beep-test-edit-new = + NEW` | `beep-test-edit-new = MAGDAGDAG NG ANTAS` |
| `refbox/translations/tr-TR/refbox.ftl` | `beep-test-edit-new = + NEW` | `beep-test-edit-new = SEVİYE EKLE` |
| `refbox/translations/zh-CN/refbox.ftl` | `beep-test-edit-new = + NEW` | `beep-test-edit-new = 添加级别` |

Apply each as a direct line replacement. The line numbers may differ very slightly between locales; rely on string match, not line number.

- [ ] **Step 2: Update `beep-test-edit-remove` in the 12 currently-English-fallback locales**

The `beep-test-edit-remove` key currently has English fallback `REMOVE LEVEL` in these 12 locales. Replace each:

| File | Old line | New line |
|---|---|---|
| `refbox/translations/de-DE/refbox.ftl` | `beep-test-edit-remove = REMOVE LEVEL` | `beep-test-edit-remove = STUFE ENTFERNEN` |
| `refbox/translations/id-ID/refbox.ftl` | `beep-test-edit-remove = REMOVE LEVEL` | `beep-test-edit-remove = HAPUS LEVEL` |
| `refbox/translations/it-IT/refbox.ftl` | `beep-test-edit-remove = REMOVE LEVEL` | `beep-test-edit-remove = RIMUOVI LIVELLO` |
| `refbox/translations/ja-JP/refbox.ftl` | `beep-test-edit-remove = REMOVE LEVEL` | `beep-test-edit-remove = レベル削除` |
| `refbox/translations/ko-KR/refbox.ftl` | `beep-test-edit-remove = REMOVE LEVEL` | `beep-test-edit-remove = 레벨 제거` |
| `refbox/translations/ms-MY/refbox.ftl` | `beep-test-edit-remove = REMOVE LEVEL` | `beep-test-edit-remove = BUANG TAHAP` |
| `refbox/translations/nl-NL/refbox.ftl` | `beep-test-edit-remove = REMOVE LEVEL` | `beep-test-edit-remove = NIVEAU VERWIJDEREN` |
| `refbox/translations/pt-PT/refbox.ftl` | `beep-test-edit-remove = REMOVE LEVEL` | `beep-test-edit-remove = REMOVER NÍVEL` |
| `refbox/translations/th-TH/refbox.ftl` | `beep-test-edit-remove = REMOVE LEVEL` | `beep-test-edit-remove = ลบระดับ` |
| `refbox/translations/tl-PH/refbox.ftl` | `beep-test-edit-remove = REMOVE LEVEL` | `beep-test-edit-remove = ALISIN ANG ANTAS` |
| `refbox/translations/tr-TR/refbox.ftl` | `beep-test-edit-remove = REMOVE LEVEL` | `beep-test-edit-remove = SEVİYE KALDIR` |
| `refbox/translations/zh-CN/refbox.ftl` | `beep-test-edit-remove = REMOVE LEVEL` | `beep-test-edit-remove = 删除级别` |

The English (`en-US`), French (`fr`), and Spanish (`es`) `beep-test-edit-remove` values are already correctly translated and stay unchanged.

- [ ] **Step 3: Verify all 27 line updates applied**

```
cd /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign/
grep -rn "beep-test-edit-new\|beep-test-edit-remove" refbox/translations/ | sort
```

Expected: 30 lines total (15 `-new` + 15 `-remove`), with NO occurrence of `+ NEW`, `+ NOUVEAU`, `+ NUEVO`, or — among the 12 non-EN/FR/ES locales — `REMOVE LEVEL`. The en-US value should remain `REMOVE LEVEL`.

- [ ] **Step 4: Run `just check`**

```
just check
```

Expected: PASS. The "Missing keys in translations/*" build-warnings about `portal-row-attempt-suffix` are pre-existing and unrelated; do NOT attempt to fix them here (out of scope).

If `just check` flags any *new* "Missing keys" warning related to `beep-test-edit-new` or `beep-test-edit-remove`, that means a locale's value was accidentally deleted instead of replaced — re-inspect that file.

- [ ] **Step 5: Commit**

```
git add refbox/translations/
git commit -m "feat(refbox): translate ADD LEVEL + REMOVE LEVEL across all 15 locales"
```

With Co-Authored-By footer:

```
Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

---

### Task 2: Cap number of levels at 10

**Files:**
- Modify: `refbox/src/app/view_builders/beep_test_settings.rs` (`build_edit_panel`, around lines 474–521)
- Modify: `refbox/src/app/mod.rs` (`Message::BeepTestEditAddLevel` handler, around line 3580)

- [ ] **Step 1: Add the view-side disable in `build_edit_panel`**

In `refbox/src/app/view_builders/beep_test_settings.rs`, the function `build_edit_panel(levels: &[Level], selected: usize)` (starting at line 474) currently has two ADD LEVEL button construction sites — one in the empty-list fallback at line 478–488 and one in the normal management row at line 492–494.

Compute the gate at the top of the function, *after* the `levels.get(selected)` check but reusable in both branches. Change the function body so that, immediately after the `let Some(level) = ...` else-fallback returns, the `add_disabled` predicate is in scope and gates the management-row button as well.

The cleanest structure: compute `let add_disabled = levels.len() >= 10;` at the very top of the function, BEFORE the `let Some(level)` check. Use it in both branches.

Replace lines 474–494 (function header through the management-row `add_button` definition). Current code:

```rust
fn build_edit_panel(levels: &[Level], selected: usize) -> Element<'_, Message> {
    // Safe to index because the caller already clamped `selected` to be
    // in range. If the list is empty we fall through to a placeholder
    // with just a `[+NEW]` button.
    let Some(level) = levels.get(selected) else {
        let add_button = make_smaller_button(fl!("beep-test-edit-new"))
            .style(green_button)
            .on_press(Message::BeepTestEditAddLevel);
        return container(add_button)
            .padding(PADDING)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
    };

    // Top management row: [+NEW] | Selected: Level N | [REMOVE LEVEL]
    let add_button = make_smaller_button(fl!("beep-test-edit-new"))
        .style(green_button)
        .on_press(Message::BeepTestEditAddLevel);
```

After:

```rust
fn build_edit_panel(levels: &[Level], selected: usize) -> Element<'_, Message> {
    // The ADD LEVEL button is disabled when the number of staged levels is
    // already at the cap. Mirrors the existing remove_disabled + count_inc_disabled
    // patterns and the per-level count cap from Chunk 1. Defense-in-depth in
    // the handler at Message::BeepTestEditAddLevel.
    let add_disabled = levels.len() >= 10;

    // Safe to index because the caller already clamped `selected` to be
    // in range. If the list is empty we fall through to a placeholder
    // with just a `[ADD LEVEL]` button.
    let Some(level) = levels.get(selected) else {
        let add_button = if add_disabled {
            make_smaller_button(fl!("beep-test-edit-new")).style(gray_button)
        } else {
            make_smaller_button(fl!("beep-test-edit-new"))
                .style(green_button)
                .on_press(Message::BeepTestEditAddLevel)
        };
        return container(add_button)
            .padding(PADDING)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
    };

    // Top management row: [ADD LEVEL] | Selected: Level N | [REMOVE LEVEL]
    let add_button = if add_disabled {
        make_smaller_button(fl!("beep-test-edit-new")).style(gray_button)
    } else {
        make_smaller_button(fl!("beep-test-edit-new"))
            .style(green_button)
            .on_press(Message::BeepTestEditAddLevel)
    };
```

Note: also updated the two inline comments referring to `[+NEW]` to `[ADD LEVEL]` to match the new label.

- [ ] **Step 2: Add the handler-side guard in `mod.rs`**

In `refbox/src/app/mod.rs`, around line 3580. Current handler:

```rust
            Message::BeepTestEditAddLevel => {
                if let Some(ref mut edited) = self.edited_settings {
                    if let Some(ref mut levels) = edited.beep_test_levels {
                        levels.push(crate::config::Level {
                            count: 4,
                            duration: std::time::Duration::from_secs(20),
                        });
                        edited.selected_level = levels.len() - 1;
                    }
                }
                Task::none()
            }
```

After (wrap the push + selection update in `if levels.len() < 10`):

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
git commit -m "feat(refbox): cap beep-test number of levels at 10"
```

With Co-Authored-By footer.

---

### Task 3: Strip the `chunks(BAND_WIDTH)` row-wrapping logic in both views

**Files:**
- Modify: `refbox/src/app/view_builders/beep_test_settings.rs` (`build_editor_levels_table`, around lines 329–402; constant at line 261)
- Modify: `refbox/src/app/view_builders/beep_test.rs` (main view's table builder around lines 175–269; constant at line 37; doc comment at lines 177–178)

- [ ] **Step 1: Strip the band-wrapping logic in `build_editor_levels_table`**

In `refbox/src/app/view_builders/beep_test_settings.rs`, replace `build_editor_levels_table` (lines 329–402). Current code:

```rust
fn build_editor_levels_table(levels: &[Level], selected: usize) -> Element<'_, Message> {
    let mut bands = Column::new().spacing(SPACING);

    for (band_idx, band_levels) in levels.chunks(EDIT_BAND_WIDTH).enumerate() {
        let level_index_offset = band_idx * EDIT_BAND_WIDTH;
        let max_count = band_levels
            .iter()
            .map(|l| l.count as usize)
            .max()
            .unwrap_or(0);

        // Header row: column headers (1-indexed for the operator).
        let mut header_row = Row::new().spacing(SPACING);
        for (col_idx, _level) in band_levels.iter().enumerate() {
            let level_number = level_index_offset + col_idx + 1;
            let zero_based = level_index_offset + col_idx;
            let is_selected = zero_based == selected;
            header_row = header_row.push(editor_header_cell(
                level_number.to_string(),
                zero_based,
                is_selected,
            ));
        }
        // Pad with filler cells on partially-populated bands so columns
        // stay aligned with full bands above.
        let cols_used_in_band = band_levels.len();
        for _ in cols_used_in_band..EDIT_BAND_WIDTH {
            header_row = header_row.push(filler_cell());
        }
        bands = bands.push(header_row);

        // Cell rows: stacked vertically. Each cell is a tappable button
        // that selects the column it belongs to. Empty rows beyond a
        // column's count render as filler.
        for row_idx in 0..max_count {
            let mut cell_row = Row::new().spacing(SPACING);
            for (col_idx, level) in band_levels.iter().enumerate() {
                let zero_based = level_index_offset + col_idx;
                if row_idx < level.count as usize {
                    let is_selected = zero_based == selected;
                    cell_row = cell_row.push(editor_value_cell(
                        level.duration.as_secs().to_string(),
                        zero_based,
                        is_selected,
                    ));
                } else {
                    cell_row = cell_row.push(filler_cell());
                }
            }
            for _ in cols_used_in_band..EDIT_BAND_WIDTH {
                cell_row = cell_row.push(filler_cell());
            }
            bands = bands.push(cell_row);
        }

        // Pad odd-layer bands with one blank row so every band's rendered
        // height is an exact multiple of MIN_BUTTON_SIZE. Mirrors the main
        // view's band-sizing behavior.
        let layer_count = 1 + max_count;
        if layer_count % 2 == 1 {
            let mut blank_row = Row::new().spacing(SPACING);
            for _ in 0..EDIT_BAND_WIDTH {
                blank_row = blank_row.push(filler_cell());
            }
            bands = bands.push(blank_row);
        }
    }

    container(bands)
        .padding(EDIT_TABLE_CELL_SPACING)
        .width(Length::Fill)
        .center_x(Length::Fill)
        .into()
}
```

After (drop the outer `chunks(...)` loop, drop the band-padding for partial bands, drop the odd-layer blank-row padding — all three of those existed only because of multi-band layouts):

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

- [ ] **Step 2: Remove the `EDIT_BAND_WIDTH` constant**

In the same file, around line 261, delete:

```rust
const EDIT_BAND_WIDTH: usize = 10;
```

and the doc comment immediately above it (the one explaining "Mirrors the main view's BAND_WIDTH so the editor's transposed table…"). Compilation will fail in this file until Step 1's replacement is in place, so apply Step 1 before Step 2 if working incrementally.

- [ ] **Step 3: Strip the band-wrapping logic in `beep_test.rs`'s main view table builder**

In `refbox/src/app/view_builders/beep_test.rs`, locate the main-view table builder function (around line 175–269; identifiable by its `for ... in levels.chunks(BAND_WIDTH)` at line 187).

The transformation mirrors Step 1: replace the outer `chunks(...)` loop with a single pass over `levels`. Compute `max_count` once at the top; build one header row and a stack of cell rows directly. Drop the per-band index offset arithmetic, the partial-band filler padding (`for _ in band_levels.len()..BAND_WIDTH`), and the odd-layer blank-row padding.

Update the doc comment at lines 177–178 from:

```rust
/// `BAND_WIDTH` columns wrap onto additional rows when there are more
/// user levels than `BAND_WIDTH`.
```

to a single sentence describing the new single-row layout — e.g.:

```rust
/// Renders a single row of column headers followed by a stack of cell rows;
/// the level cap (10) is enforced at the editor's ADD LEVEL gate so this
/// table never needs to wrap.
```

Use the cell-builder helpers already in scope in this file (the main view has its own `header_cell` / `value_cell` helpers — match the existing names; do NOT cross-import from `beep_test_settings.rs`). If unsure of the exact helper names, grep `^fn ` at the top of `beep_test.rs` and use the ones the existing `chunks(...)` loop body calls.

- [ ] **Step 4: Remove the `BAND_WIDTH` constant**

In the same file, around line 37, delete:

```rust
const BAND_WIDTH: usize = 10;
```

and any leading comment that references it.

- [ ] **Step 5: Verify the workspace compiles**

```
cargo build -p refbox 2>&1 | tail -5
```

Expected: clean compile. If `cargo build` fails with `cannot find value 'BAND_WIDTH'` or `cannot find value 'EDIT_BAND_WIDTH'`, search for any remaining reference and remove it.

- [ ] **Step 6: Run `just check`**

```
just check
```

Expected: PASS.

- [ ] **Step 7: Commit**

```
git add refbox/src/app/view_builders/beep_test_settings.rs refbox/src/app/view_builders/beep_test.rs
git commit -m "refactor(refbox): drop dead band-wrapping logic from beep-test views"
```

With Co-Authored-By footer.

---

### Task 4: Walkthrough verification

**Files:** none. Smoke-test the running refbox.

Per `.claude/rules/pr-review.md`: "Smoke-tested locally — refbox (or the affected artifact) was launched and the change exercised in a real session before any push/PR/merge/tag-push. CI green ≠ smoke-tested." The five walkthrough scenarios from the spec must all pass before the operator is asked to approve a PR.

- [ ] **Step 1: Launch the refbox in BeepTest mode**

```
WAYLAND_DISPLAY= cargo run --manifest-path /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign/Cargo.toml -p refbox
```

(The operator's config has `mode = "BeepTest"`, so refbox will come up directly in BeepTest mode.)

- [ ] **Step 2: Scenario 1 — ADD LEVEL label visible**

Navigate **Main → Settings → Edit Levels**. Confirm the upper-left button reads **ADD LEVEL** in English. If your locale is French or Spanish, confirm `AJOUTER NIVEAU` / `AÑADIR NIVEL` respectively (and the paired `SUPPRIMER NIVEAU` / `ELIMINAR NIVEL` for REMOVE LEVEL).

- [ ] **Step 3: Scenario 2 — Add works under cap**

With fewer than 10 levels staged (the default config has 1–5), tap ADD LEVEL. Confirm:
- A new level appends to the right of the table.
- The new column becomes the selected (blue-highlighted) column.
- The per-level edit panel updates to show the new level's count and duration.

- [ ] **Step 4: Scenario 3 — Cap reached at 10**

Keep tapping ADD LEVEL until 10 levels exist. Confirm:
- At exactly 10 levels, the ADD LEVEL button visibly **grays out** (disabled style, gray-button background).
- Tapping ADD LEVEL after 10 does nothing — no 11th column appears.

- [ ] **Step 5: Scenario 4 — Removing re-enables Add**

With 10 levels, tap REMOVE LEVEL once. Confirm:
- One column disappears (the selected one, with selection moving to a remaining column).
- ADD LEVEL re-enables (green style, responsive).

- [ ] **Step 6: Scenario 5 — Layout sanity at boundary counts**

Use ADD LEVEL / REMOVE LEVEL to reach 1, 5, and 10 levels in turn. At each:
- Confirm the table renders as a single horizontal row of columns.
- Confirm no visual artifacts where the old band-padding / odd-layer blank-row code used to render (specifically, no unexpected gap below the level rows).

- [ ] **Step 7: Hand back to operator**

Report walkthrough results for Scenarios 1–5. If any scenario fails or behaves unexpectedly, stop and report — do not push/PR.

---

## Deviations

(Append notes here if execution deviates from the plan. Per `.claude/rules/plan-execution.md`, fold deviation notes into the code commit that introduced the deviation; no standalone doc-only deviation commits.)

---

## Self-review notes

- **Spec coverage:**
  - Spec §Item 1 (rename + per-locale translations) → Task 1 Steps 1–2 (29 string updates across 15 files).
  - Spec §Item 2 (cap at 10) → Task 2 Steps 1 (view-side disable in both ADD LEVEL call sites) + 2 (handler-side guard).
  - Spec §Item 3 (drop wrapping) → Task 3 Steps 1–4 (strip both views' band loops; remove both constants).
  - Spec §Testing (lean, walkthrough) → Task 4.
  - Spec §Acceptance criteria (5 walkthrough scenarios + `just check`) → Task 4 Steps 2–6 + `just check` runs in Tasks 1/2/3.
- **No placeholders:** all steps show concrete code/commands. The Task 3 Step 3 helper-name guidance ("use existing helpers") is a deliberate pragmatic note, not a placeholder — the helper names are already defined in `beep_test.rs` and the executing engineer reads them as part of editing the file.
- **Type consistency:** No new types or function signatures introduced. The existing `editor_header_cell`, `editor_value_cell`, `filler_cell`, `make_smaller_button`, `gray_button`, `green_button`, `fl!` are all already in scope where used. The new local `add_disabled: bool` and the `if levels.len() < 10` literal use only existing types.
- **Lean process:** one `just check` gate per code task; no per-task code review; final review at PR time. No new unit tests.
- **Task ordering:** translations land first (zero behaviour change, no compile risk), then the cap (small behaviour change, single function + handler), then the wrapping strip (refactor, requires the cap to be in place semantically but not for compilation). Each task is its own commit for clean review-time history.
- **Side-effect-free verification:** `just check` after each task confirms each step is independently green; if a task introduces an issue, it's localised to that commit.
